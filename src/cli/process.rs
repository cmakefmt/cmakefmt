// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Path discovery, target processing, caching, and config-context wiring.

use std::collections::{BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};

use cmakefmt::spec::registry::CommandRegistry;
use cmakefmt::{
    files::{discover_cmake_files_with_options, is_cmake_file, matches_filter, DiscoveryOptions},
    format_source_with_registry, format_source_with_registry_debug, parser,
    render_effective_config,
    semantic::{normalize_command_literals, normalize_keyword_args, normalize_line_endings},
    Config, DumpConfigFormat, IoResultExt,
};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::cli::diff::{
    apply_line_ranges, build_unified_diff, changed_formatted_line_numbers, highlight_changed_lines,
    split_lines_with_endings,
};
use crate::cli::runtime::{log_debug, needs_debug_lines};
use crate::{CacheStrategy, Cli, ReportFormat};

#[derive(Clone)]
pub(crate) enum InputTarget {
    Stdin,
    Path(PathBuf),
}

impl InputTarget {
    pub(crate) fn is_path(&self) -> bool {
        matches!(self, InputTarget::Path(_))
    }

    pub(crate) fn display_name(&self, stdin_path: Option<&Path>) -> String {
        match self {
            Self::Stdin => stdin_path
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<stdin>".to_owned()),
            Self::Path(path) => path.display().to_string(),
        }
    }
}

pub(crate) struct ProcessedTarget {
    pub(crate) path: Option<PathBuf>,
    pub(crate) display_name: String,
    pub(crate) formatted: String,
    pub(crate) highlighted_output: Option<String>,
    pub(crate) unified_diff: Option<String>,
    pub(crate) changed_lines: Vec<usize>,
    pub(crate) would_change: bool,
    pub(crate) skipped: bool,
    pub(crate) skip_reason: Option<String>,
    pub(crate) debug_lines: Vec<String>,
    pub(crate) source_lines: usize,
    pub(crate) formatted_lines: usize,
    pub(crate) elapsed: std::time::Duration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VerificationMode {
    Disabled,
    Enabled,
}

pub(crate) struct FailedTarget {
    pub(crate) display_name: String,
    pub(crate) rendered_error: String,
}

#[derive(Clone, Debug)]
struct CacheContext {
    cache_file: PathBuf,
    tool_signature: String,
    config_signature: String,
    source_signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    tool_signature: String,
    config_signature: String,
    source_signature: String,
    formatted: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ConfigSourceMode {
    Disabled,
    Explicit,
    Discovered,
    DefaultsOnly,
}

#[derive(Clone, Debug)]
pub(crate) struct ConfigContext {
    pub(crate) mode: ConfigSourceMode,
    pub(crate) sources: Vec<PathBuf>,
}

#[derive(Copy, Clone)]
enum GitSelectionMode<'a> {
    Staged,
    Changed(Option<&'a str>),
}

#[derive(Clone)]
pub(crate) struct ProgressReporter {
    inner: Option<Arc<ProgressBar>>,
}

impl ProgressReporter {
    pub(crate) fn new(enabled: bool, total: usize) -> Self {
        let inner = enabled.then(|| {
            let progress = ProgressBar::new(total as u64);
            progress.set_draw_target(ProgressDrawTarget::stderr());
            progress.set_style(
                ProgressStyle::with_template(
                    "{spinner:.green} [Elapsed: {elapsed_precise}] |{bar:50.green/green}| ({eta_precise}) {pos}/{len} ({percent}%) files",
                )
                .expect("progress template should be valid")
                .progress_chars("=> "),
            );
            Arc::new(progress)
        });
        Self { inner }
    }

    pub(crate) fn finish_one(&self) {
        let Some(inner) = &self.inner else {
            return;
        };

        inner.inc(1);
        if inner.position() == inner.length().unwrap_or(0) {
            inner.finish();
        }
    }

    pub(crate) fn eprintln(&self, message: &str) -> Result<(), cmakefmt::Error> {
        if let Some(inner) = &self.inner {
            inner.println(message);
        } else {
            eprintln!("{message}");
        }
        io::stderr().flush().map_err(cmakefmt::Error::Io)
    }
}

pub(crate) fn process_targets<F>(
    targets: &[InputTarget],
    cli: &Cli,
    parallel_jobs: usize,
    colorize_stdout: bool,
    progress: &ProgressReporter,
    mut on_result: F,
) -> Result<(), cmakefmt::Error>
where
    F: FnMut(Result<ProcessedTarget, cmakefmt::Error>) -> Result<(), cmakefmt::Error>,
{
    if parallel_jobs > 1 && targets.iter().all(InputTarget::is_path) {
        process_targets_parallel(
            targets,
            cli,
            parallel_jobs,
            colorize_stdout,
            progress,
            &mut on_result,
        )
    } else {
        if cli.execution.debug
            && parallel_jobs > 1
            && targets.iter().any(|target| !target.is_path())
        {
            log_debug("parallel mode ignored because stdin input must run serially");
        }
        process_targets_serial(targets, cli, colorize_stdout, progress, &mut on_result)
    }
}

fn process_targets_serial<F>(
    targets: &[InputTarget],
    cli: &Cli,
    colorize_stdout: bool,
    progress: &ProgressReporter,
    on_result: &mut F,
) -> Result<(), cmakefmt::Error>
where
    F: FnMut(Result<ProcessedTarget, cmakefmt::Error>) -> Result<(), cmakefmt::Error>,
{
    for target in targets {
        on_result(process_target(target, cli, colorize_stdout, progress))?;
    }
    Ok(())
}

fn process_targets_parallel<F>(
    targets: &[InputTarget],
    cli: &Cli,
    parallel_jobs: usize,
    colorize_stdout: bool,
    progress: &ProgressReporter,
    on_result: &mut F,
) -> Result<(), cmakefmt::Error>
where
    F: FnMut(Result<ProcessedTarget, cmakefmt::Error>) -> Result<(), cmakefmt::Error>,
{
    let worker_count = parallel_jobs.min(targets.len().max(1));
    let next_work = AtomicUsize::new(0);
    let cancelled = AtomicBool::new(false);

    std::thread::scope(|scope| {
        let (tx, rx) = mpsc::channel();

        for _ in 0..worker_count {
            let tx = tx.clone();
            let next_work = &next_work;
            let cancelled = &cancelled;
            scope.spawn(move || loop {
                if cancelled.load(Ordering::Relaxed) {
                    break;
                }
                let index = next_work.fetch_add(1, Ordering::Relaxed);
                let Some(target) = targets.get(index) else {
                    break;
                };
                if tx
                    .send((
                        index,
                        process_target(target, cli, colorize_stdout, progress),
                    ))
                    .is_err()
                {
                    break;
                }
            });
        }
        drop(tx);

        // Buffer out-of-order results and flush in input order. Uses a
        // HashMap so memory scales with the actual backlog, not total
        // target count.
        let mut next_emit = 0;
        let mut pending: HashMap<usize, Result<ProcessedTarget, cmakefmt::Error>> = HashMap::new();
        let mut first_error: Option<cmakefmt::Error> = None;

        while let Ok((index, result)) = rx.recv() {
            // Cancel workers immediately on errors, even if we can't
            // emit them yet due to ordering.
            if first_error.is_none() && result.is_err() && !cli.execution.keep_going {
                cancelled.store(true, Ordering::Relaxed);
            }

            pending.insert(index, result);

            // Drain all contiguous results starting from next_emit.
            while pending.contains_key(&next_emit) {
                let result = pending.remove(&next_emit).unwrap();
                match on_result(result) {
                    Ok(()) => {}
                    Err(err) => {
                        cancelled.store(true, Ordering::Relaxed);
                        first_error = Some(err);
                    }
                }
                next_emit += 1;
            }
        }

        match first_error {
            Some(err) => Err(err),
            None => Ok(()),
        }
    })
}

fn process_target(
    target: &InputTarget,
    cli: &Cli,
    colorize_stdout: bool,
    progress: &ProgressReporter,
) -> Result<ProcessedTarget, cmakefmt::Error> {
    let start = std::time::Instant::now();
    let mut result = match target {
        InputTarget::Stdin => process_stdin(cli, colorize_stdout),
        InputTarget::Path(path) => process_path(path, cli, colorize_stdout),
    };
    if let Ok(ref mut r) = result {
        r.elapsed = start.elapsed();
    }
    progress.finish_one();
    result
}

fn process_stdin(cli: &Cli, colorize_stdout: bool) -> Result<ProcessedTarget, cmakefmt::Error> {
    let mut source = String::new();
    io::stdin()
        .read_to_string(&mut source)
        .map_err(cmakefmt::Error::Io)?;

    let stdin_path = cli.input_selection.stdin_path.as_deref();
    let display_name = stdin_path
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<stdin>".to_owned());
    if cli.execution.require_pragma && !has_enable_pragma(&source) {
        return Ok(skipped_target(
            stdin_path.map(Path::to_path_buf),
            display_name,
            source,
            "missing format opt-in pragma".to_owned(),
            cli.execution.debug,
        ));
    }
    let (config, registry, config_context) = build_context(cli, stdin_path)?;
    let collect_debug = needs_debug_lines(cli);
    let mut debug_lines = if collect_debug {
        vec![
            format!("processing {display_name}"),
            describe_config_context(&config_context),
        ]
    } else {
        Vec::new()
    };
    let (formatted, mut formatter_debug) = if collect_debug {
        match format_source_with_registry_debug(&source, &config, &registry) {
            Ok(result) => result,
            Err(err) => return Err(err.with_display_name(&display_name)),
        }
    } else {
        match format_source_with_registry(&source, &config, &registry) {
            Ok(formatted) => (formatted, Vec::new()),
            Err(err) => return Err(err.with_display_name(&display_name)),
        }
    };
    if collect_debug {
        debug_lines.append(&mut formatter_debug);
    }

    if verification_mode(cli) == VerificationMode::Enabled {
        verify_semantics(&source, &formatted, &registry, &display_name)?;
        if collect_debug {
            debug_lines.push(format!(
                "result {display_name}: semantic verification passed"
            ));
        }
    }

    let formatted = apply_line_ranges(
        &source,
        &formatted,
        &cli.input_selection.line_ranges,
        &display_name,
    )?;

    let would_change = formatted != source;
    let source_lines = source.lines().count();
    let formatted_lines = formatted.lines().count();
    let changed_lines = if needs_changed_lines(cli, colorize_stdout) {
        changed_formatted_line_numbers(
            &split_lines_with_endings(&source),
            &split_lines_with_endings(&formatted),
        )
    } else {
        Vec::new()
    };
    if collect_debug {
        debug_lines.push(format!(
            "result {display_name}: would_change={would_change}"
        ));
        debug_lines.push(format!(
            "result {display_name}: changed_lines={}",
            changed_lines.len()
        ));
    }
    let highlighted_output = colorize_stdout
        .then(|| highlight_changed_lines(&source, &formatted))
        .filter(|_| would_change);
    let unified_diff = (would_change && needs_unified_diff(cli))
        .then(|| build_unified_diff(&display_name, &source, &formatted));

    Ok(ProcessedTarget {
        path: stdin_path.map(Path::to_path_buf),
        display_name,
        formatted,
        highlighted_output,
        unified_diff,
        changed_lines,
        would_change,
        skipped: false,
        skip_reason: None,
        debug_lines,
        source_lines,
        formatted_lines,
        elapsed: std::time::Duration::ZERO,
    })
}

fn process_path(
    path: &Path,
    cli: &Cli,
    colorize_stdout: bool,
) -> Result<ProcessedTarget, cmakefmt::Error> {
    let source = std::fs::read_to_string(path).with_path(path)?;
    if cli.execution.require_pragma && !has_enable_pragma(&source) {
        return Ok(skipped_target(
            Some(path.to_path_buf()),
            path.display().to_string(),
            source,
            "missing format opt-in pragma".to_owned(),
            cli.execution.debug,
        ));
    }
    let (config, registry, config_context) = build_context(cli, Some(path))?;
    let collect_debug = needs_debug_lines(cli);
    let mut debug_lines = if collect_debug {
        vec![
            format!("processing {}", path.display()),
            describe_config_context(&config_context),
            describe_cli_overrides(cli),
        ]
    } else {
        Vec::new()
    };
    let cache_context = if cli.execution.cache || cli.execution.cache_location.is_some() {
        Some(cache_context(
            path,
            &source,
            &config,
            &config_context,
            cli.execution.cache_location.as_deref(),
            cli.execution.cache_strategy,
        )?)
    } else {
        None
    };

    let mut cache_hit = false;
    let (formatted, mut formatter_debug) = if let Some(cache) = &cache_context {
        if let Some(cached) = read_cache_entry(cache)? {
            cache_hit = true;
            if collect_debug {
                debug_lines.push(format!(
                    "cache hit {} ({})",
                    path.display(),
                    cache.cache_file.display()
                ));
            }
            (cached.formatted, Vec::new())
        } else {
            if collect_debug {
                debug_lines.push(format!(
                    "cache miss {} ({})",
                    path.display(),
                    cache.cache_file.display()
                ));
            }
            if collect_debug {
                match format_source_with_registry_debug(&source, &config, &registry) {
                    Ok(result) => result,
                    Err(err) => return Err(err.with_display_name(path.display().to_string())),
                }
            } else {
                match format_source_with_registry(&source, &config, &registry) {
                    Ok(formatted) => (formatted, Vec::new()),
                    Err(err) => return Err(err.with_display_name(path.display().to_string())),
                }
            }
        }
    } else if collect_debug {
        match format_source_with_registry_debug(&source, &config, &registry) {
            Ok(result) => result,
            Err(err) => return Err(err.with_display_name(path.display().to_string())),
        }
    } else {
        match format_source_with_registry(&source, &config, &registry) {
            Ok(formatted) => (formatted, Vec::new()),
            Err(err) => return Err(err.with_display_name(path.display().to_string())),
        }
    };
    if collect_debug {
        debug_lines.append(&mut formatter_debug);
    }

    if verification_mode(cli) == VerificationMode::Enabled {
        verify_semantics(&source, &formatted, &registry, &path.display().to_string())?;
        if collect_debug {
            debug_lines.push(format!(
                "result {}: semantic verification passed",
                path.display()
            ));
        }
    }

    let formatted = apply_line_ranges(
        &source,
        &formatted,
        &cli.input_selection.line_ranges,
        &path.display().to_string(),
    )?;
    let would_change = formatted != source;
    let source_lines = source.lines().count();
    let formatted_lines = formatted.lines().count();
    let changed_lines = if needs_changed_lines(cli, colorize_stdout) {
        changed_formatted_line_numbers(
            &split_lines_with_endings(&source),
            &split_lines_with_endings(&formatted),
        )
    } else {
        Vec::new()
    };
    if collect_debug {
        debug_lines.push(format!(
            "result {}: would_change={would_change}",
            path.display()
        ));
        debug_lines.push(format!(
            "result {}: changed_lines={}",
            path.display(),
            changed_lines.len()
        ));
    }
    let highlighted_output = colorize_stdout
        .then(|| highlight_changed_lines(&source, &formatted))
        .filter(|_| would_change);
    let unified_diff = (would_change && needs_unified_diff(cli))
        .then(|| build_unified_diff(&path.display().to_string(), &source, &formatted));

    if let Some(cache) = &cache_context {
        if !cache_hit {
            write_cache_entry(
                cache,
                CacheEntry {
                    tool_signature: cache.tool_signature.clone(),
                    config_signature: cache.config_signature.clone(),
                    source_signature: cache.source_signature.clone(),
                    formatted: formatted.clone(),
                },
            )?;
        }
    }

    Ok(ProcessedTarget {
        path: Some(path.to_path_buf()),
        display_name: path.display().to_string(),
        formatted,
        highlighted_output,
        unified_diff,
        changed_lines,
        would_change,
        skipped: false,
        skip_reason: None,
        debug_lines,
        source_lines,
        formatted_lines,
        elapsed: std::time::Duration::ZERO,
    })
}

pub(crate) fn needs_changed_lines(cli: &Cli, colorize_stdout: bool) -> bool {
    colorize_stdout
        || !cli.input_selection.line_ranges.is_empty()
        || cli.execution.debug
        || cli.output_modes.summary
        || cli.execution.stat
        || cli.output_modes.report_format != ReportFormat::Human
}

/// Check whether the current CLI invocation actually needs a unified diff.
/// Computing the diff (Myers algorithm via `similar`) is expensive on large
/// files — only pay for it when the result will be consumed.
pub(crate) fn needs_unified_diff(cli: &Cli) -> bool {
    cli.output_modes.diff
        || matches!(
            cli.output_modes.report_format,
            ReportFormat::Junit | ReportFormat::Checkstyle
        )
}

fn verification_mode(cli: &Cli) -> VerificationMode {
    if cli.execution.verify || (cli.output_modes.in_place && !cli.execution.no_verify) {
        VerificationMode::Enabled
    } else {
        VerificationMode::Disabled
    }
}

pub(crate) fn has_enable_pragma(source: &str) -> bool {
    source.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.contains("cmakefmt: enable")
            || trimmed.contains("fmt: enable")
            || trimmed.contains("cmake-format: enable")
    })
}

fn skipped_target(
    path: Option<PathBuf>,
    display_name: String,
    source: String,
    reason: String,
    debug: bool,
) -> ProcessedTarget {
    let mut debug_lines = Vec::new();
    if debug {
        debug_lines.push(format!("processing {display_name}"));
        debug_lines.push(format!("skipped {display_name}: {reason}"));
    }
    let source_lines = source.lines().count();

    ProcessedTarget {
        path,
        display_name,
        formatted_lines: source_lines,
        formatted: source,
        highlighted_output: None,
        unified_diff: None,
        changed_lines: Vec::new(),
        would_change: false,
        skipped: true,
        skip_reason: Some(reason),
        debug_lines,
        source_lines,
        elapsed: std::time::Duration::ZERO,
    }
}

fn cache_context(
    path: &Path,
    source: &str,
    config: &Config,
    config_context: &ConfigContext,
    cache_location: Option<&Path>,
    cache_strategy: CacheStrategy,
) -> Result<CacheContext, cmakefmt::Error> {
    let cache_root = cache_location
        .map(Path::to_path_buf)
        .unwrap_or_else(|| default_cache_dir(path));
    let cache_key = stable_hash(&path.display().to_string());
    let cache_file = cache_root.join(format!("{cache_key}.json"));
    let rendered_config = render_effective_config(config, DumpConfigFormat::Toml)?;
    let mut config_fingerprint = format!(
        "{}\n{}",
        env!("CMAKEFMT_CLI_LONG_VERSION"),
        rendered_config.trim_end()
    );
    for source_path in &config_context.sources {
        config_fingerprint.push('\n');
        config_fingerprint.push_str(
            &std::fs::read_to_string(source_path)
                .unwrap_or_else(|_| format!("<unreadable:{}>", source_path.display())),
        );
    }

    Ok(CacheContext {
        cache_file,
        tool_signature: env!("CMAKEFMT_CLI_LONG_VERSION").to_owned(),
        config_signature: stable_hash(&config_fingerprint),
        source_signature: source_signature(path, source, cache_strategy)?,
    })
}

fn default_cache_dir(path: &Path) -> PathBuf {
    find_git_root(path)
        .unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| {
                path.parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| PathBuf::from("."))
            })
        })
        .join(".cmakefmt-cache")
}

fn find_git_root(path: &Path) -> Option<PathBuf> {
    let mut current = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()?.to_path_buf()
    };

    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn source_signature(
    path: &Path,
    source: &str,
    cache_strategy: CacheStrategy,
) -> Result<String, cmakefmt::Error> {
    match cache_strategy {
        CacheStrategy::Metadata => {
            let metadata = std::fs::metadata(path).with_path(path)?;
            let modified = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_nanos())
                .unwrap_or_default();
            Ok(format!("metadata:{}:{}", metadata.len(), modified))
        }
        CacheStrategy::Content => Ok(format!("content:{}", stable_hash(source))),
    }
}

fn read_cache_entry(cache: &CacheContext) -> Result<Option<CacheEntry>, cmakefmt::Error> {
    let contents = match std::fs::read_to_string(&cache.cache_file) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(cmakefmt::Error::io_at(&cache.cache_file, err)),
    };

    let entry: CacheEntry = match serde_json::from_str(&contents) {
        Ok(entry) => entry,
        Err(_) => return Ok(None),
    };

    Ok((entry.tool_signature == cache.tool_signature
        && entry.config_signature == cache.config_signature
        && entry.source_signature == cache.source_signature)
        .then_some(entry))
}

fn write_cache_entry(cache: &CacheContext, entry: CacheEntry) -> Result<(), cmakefmt::Error> {
    if let Some(parent) = cache.cache_file.parent() {
        std::fs::create_dir_all(parent).with_path(parent)?;
    }
    let json = serde_json::to_string(&entry)
        .map_err(|err| cmakefmt::Error::render("cache entry (JSON)", err.to_string()))?;
    std::fs::write(&cache.cache_file, json).with_path(&cache.cache_file)
}

fn stable_hash<T: Hash + ?Sized>(value: &T) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn verify_semantics(
    original: &str,
    formatted: &str,
    registry: &CommandRegistry,
    display_name: &str,
) -> Result<(), cmakefmt::Error> {
    let original_ast =
        parser::parse(original).map_err(|err| err.with_display_name(display_name.to_owned()))?;
    let formatted_ast =
        parser::parse(formatted).map_err(|err| err.with_display_name(display_name.to_owned()))?;

    if normalize_semantics(original_ast, registry) == normalize_semantics(formatted_ast, registry) {
        Ok(())
    } else {
        Err(cmakefmt::Error::Formatter(format!(
            "{display_name}: semantic verification failed; formatted output changes the parsed CMake structure"
        )))
    }
}

fn normalize_semantics(
    mut file: parser::ast::File,
    registry: &CommandRegistry,
) -> parser::ast::File {
    // Strip standalone comments and blank lines — they have no CMake semantic
    // meaning and may change structure when the formatter reflows them.
    file.statements.retain(|s| {
        !matches!(
            s,
            parser::ast::Statement::Comment(_) | parser::ast::Statement::BlankLines(_)
        )
    });

    for statement in &mut file.statements {
        match statement {
            parser::ast::Statement::Command(command) => {
                command.span = (0, 0);
                command.name.make_ascii_lowercase();
                normalize_command_literals(command);
                normalize_keyword_args(command, registry);
            }
            parser::ast::Statement::TemplatePlaceholder(value) => normalize_line_endings(value),
            parser::ast::Statement::Comment(_) | parser::ast::Statement::BlankLines(_) => {
                unreachable!()
            }
        }
    }

    file
}

pub(crate) fn compile_file_filter(pattern: Option<&str>) -> Result<Option<Regex>, cmakefmt::Error> {
    pattern
        .map(|pattern| {
            Regex::new(pattern).map_err(|source| cmakefmt::Error::invalid_regex(pattern, source))
        })
        .transpose()
}

pub(crate) fn collect_targets(
    cli: &Cli,
    file_filter: Option<&Regex>,
) -> Result<Vec<InputTarget>, cmakefmt::Error> {
    if cli.execution.debug {
        log_discovery_context(cli, file_filter);
    }

    let inputs = collect_input_arguments(cli, file_filter)?;

    let mut targets = Vec::new();
    let mut seen_paths = BTreeSet::new();

    for input in inputs {
        if input == "-" {
            targets.push(InputTarget::Stdin);
            continue;
        }

        let path = PathBuf::from(&input);
        if path.is_file() {
            push_unique_path(&mut targets, &mut seen_paths, path);
            continue;
        }

        if path.is_dir() {
            let all_cmake = discover_cmake_files_with_options(
                &path,
                DiscoveryOptions {
                    file_filter: None,
                    honor_gitignore: !cli.input_selection.no_gitignore,
                    explicit_ignore_paths: &cli.input_selection.ignore_paths,
                },
            );
            let filtered = if file_filter.is_some() {
                discover_cmake_files_with_options(
                    &path,
                    DiscoveryOptions {
                        file_filter,
                        honor_gitignore: !cli.input_selection.no_gitignore,
                        explicit_ignore_paths: &cli.input_selection.ignore_paths,
                    },
                )
            } else {
                all_cmake.clone()
            };

            if cli.execution.debug && file_filter.is_some() {
                let filtered_set: BTreeSet<_> = filtered.iter().collect();
                for skipped in &all_cmake {
                    if !filtered_set.contains(skipped) {
                        log_debug(format!("skipped by --path-regex: {}", skipped.display()));
                    }
                }
            }

            for discovered in filtered {
                push_unique_path(&mut targets, &mut seen_paths, discovered);
            }
            continue;
        }

        return Err(cmakefmt::Error::io_at(
            path.clone(),
            io::Error::new(io::ErrorKind::NotFound, "no such file or directory"),
        ));
    }

    Ok(targets)
}

fn log_discovery_context(cli: &Cli, file_filter: Option<&Regex>) {
    if cli.input_selection.staged {
        log_debug("discovery mode: --staged (git staged files only)");
    } else if cli.input_selection.changed {
        let since = cli.input_selection.since.as_deref().unwrap_or("HEAD");
        log_debug(format!("discovery mode: --changed --since {since}"));
    }
    if !cli.input_selection.no_gitignore {
        log_debug("discovery: .gitignore rules active (use --no-gitignore to disable)");
    }
    if !cli.input_selection.ignore_paths.is_empty() {
        for p in &cli.input_selection.ignore_paths {
            log_debug(format!("discovery: explicit ignore path: {}", p.display()));
        }
    }
    if let Some(re) = file_filter {
        log_debug(format!("discovery: --path-regex filter active: {re}"));
    }
    if cli.execution.require_pragma {
        log_debug("discovery: --require-pragma active (files without pragma will be skipped)");
    }
}

fn collect_input_arguments(
    cli: &Cli,
    file_filter: Option<&Regex>,
) -> Result<Vec<String>, cmakefmt::Error> {
    let mut inputs = Vec::new();

    if cli.input_selection.staged {
        inputs.extend(collect_git_paths(GitSelectionMode::Staged, file_filter)?);
    } else if cli.input_selection.changed {
        inputs.extend(collect_git_paths(
            GitSelectionMode::Changed(cli.input_selection.since.as_deref()),
            file_filter,
        )?);
    }

    for files_from in &cli.input_selection.files_from {
        inputs.extend(read_files_from(files_from)?);
    }

    inputs.extend(cli.input_selection.files.clone());

    if inputs.is_empty() {
        inputs.push(".".to_owned());
    }

    Ok(inputs)
}

fn collect_git_paths(
    mode: GitSelectionMode<'_>,
    file_filter: Option<&Regex>,
) -> Result<Vec<String>, cmakefmt::Error> {
    let repo_root = git_command(["rev-parse", "--show-toplevel"])?;
    let repo_root = PathBuf::from(repo_root.trim());

    let diff_output = match mode {
        GitSelectionMode::Staged => {
            git_command(["diff", "--name-only", "--cached", "--diff-filter=ACMR"])?
        }
        GitSelectionMode::Changed(Some(reference)) => git_command([
            "diff",
            "--name-only",
            "--diff-filter=ACMR",
            &format!("{reference}...HEAD"),
        ])?,
        GitSelectionMode::Changed(None) => {
            git_command(["diff", "--name-only", "--diff-filter=ACMR", "HEAD"])?
        }
    };

    let mut paths = Vec::new();
    for line in diff_output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let candidate = repo_root.join(line);
        if is_cmake_file(&candidate) && matches_filter(&candidate, file_filter) {
            paths.push(candidate.display().to_string());
        }
    }
    Ok(paths)
}

fn git_command<const N: usize>(args: [&str; N]) -> Result<String, cmakefmt::Error> {
    let output = std::process::Command::new("git")
        .args(args)
        .output()
        .map_err(cmakefmt::Error::Io)?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(cmakefmt::Error::Formatter(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

fn read_files_from(source: &str) -> Result<Vec<String>, cmakefmt::Error> {
    let contents = if source == "-" {
        let mut stdin = String::new();
        io::stdin()
            .read_to_string(&mut stdin)
            .map_err(cmakefmt::Error::Io)?;
        stdin
    } else {
        std::fs::read_to_string(source).with_path(source)?
    };

    let entries = if contents.contains('\0') {
        contents
            .split('\0')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    } else {
        contents
            .lines()
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    };
    Ok(entries)
}

fn push_unique_path(
    targets: &mut Vec<InputTarget>,
    seen_paths: &mut BTreeSet<PathBuf>,
    path: PathBuf,
) {
    if seen_paths.insert(path.clone()) {
        targets.push(InputTarget::Path(path));
    }
}

/// Build a formatting context by layering: defaults → config files → CLI
/// overrides, and by merging any `[commands]` spec overrides from the same
/// config files into the command registry.
pub(crate) fn build_context(
    cli: &Cli,
    file_path: Option<&Path>,
) -> Result<(Config, CommandRegistry, ConfigContext), cmakefmt::Error> {
    let config_context = resolve_config_context(cli, file_path);

    let mut config = Config::from_files(&config_context.sources)?;
    let mut registry = CommandRegistry::builtins().clone();
    for path in &config_context.sources {
        registry.merge_override_file(path)?;
    }

    // Apply .editorconfig fallback when no cmakefmt config file was found.
    if matches!(config_context.mode, ConfigSourceMode::DefaultsOnly)
        && !cli.config_overrides.no_editorconfig
    {
        if let Some(path) = file_path {
            let ec = cmakefmt::config::editorconfig::read_editorconfig(path);
            if let Some(use_tabs) = ec.use_tabs {
                config.use_tabchars = use_tabs;
            }
            if let Some(tab_size) = ec.tab_size {
                config.tab_size = tab_size;
            }
            if cli.execution.debug && ec.has_any() {
                log_debug(format!(
                    "editorconfig fallback: tab_size={}, use_tabs={}",
                    ec.tab_size
                        .map_or("(default)".to_owned(), |v| v.to_string()),
                    ec.use_tabs
                        .map_or("(default)".to_owned(), |v| v.to_string()),
                ));
            }
        }
    }

    if let Some(v) = cli.config_overrides.line_width {
        config.line_width = v;
    }
    if let Some(v) = cli.config_overrides.tab_size {
        config.tab_size = v;
    }
    if let Some(v) = cli.config_overrides.command_case {
        config.command_case = v;
    }
    if let Some(v) = cli.config_overrides.keyword_case {
        config.keyword_case = v;
    }
    if let Some(v) = cli.config_overrides.dangle_parens {
        config.dangle_parens = v;
    }

    Ok((config, registry, config_context))
}

pub(crate) fn resolve_config_context(cli: &Cli, file_path: Option<&Path>) -> ConfigContext {
    if cli.config_overrides.no_config {
        return ConfigContext {
            mode: ConfigSourceMode::Disabled,
            sources: Vec::new(),
        };
    }

    if !cli.config_overrides.config_paths.is_empty() {
        return ConfigContext {
            mode: ConfigSourceMode::Explicit,
            sources: cli.config_overrides.config_paths.clone(),
        };
    }

    if let Some(path) = file_path {
        let sources = Config::config_sources_for(path);
        return ConfigContext {
            mode: if sources.is_empty() {
                ConfigSourceMode::DefaultsOnly
            } else {
                ConfigSourceMode::Discovered
            },
            sources,
        };
    }

    ConfigContext {
        mode: ConfigSourceMode::DefaultsOnly,
        sources: Vec::new(),
    }
}

pub(crate) fn describe_config_mode(mode: ConfigSourceMode) -> &'static str {
    match mode {
        ConfigSourceMode::Disabled => "disabled by --no-config",
        ConfigSourceMode::Explicit => "explicit --config-file override(s)",
        ConfigSourceMode::Discovered => "discovered from the target path",
        ConfigSourceMode::DefaultsOnly => "defaults only",
    }
}

pub(crate) fn describe_config_context(config_context: &ConfigContext) -> String {
    match config_context.mode {
        ConfigSourceMode::Disabled => "config sources: disabled by --no-config".to_owned(),
        ConfigSourceMode::DefaultsOnly => "config sources: defaults only".to_owned(),
        ConfigSourceMode::Explicit | ConfigSourceMode::Discovered => format!(
            "config sources: {}",
            config_context
                .sources
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

pub(crate) fn describe_cli_overrides(cli: &Cli) -> String {
    let mut parts = Vec::new();
    if let Some(line_width) = cli.config_overrides.line_width {
        parts.push(format!("line_width={line_width}"));
    }
    if let Some(tab_size) = cli.config_overrides.tab_size {
        parts.push(format!("tab_size={tab_size}"));
    }
    if let Some(command_case) = cli.config_overrides.command_case {
        parts.push(format!("command_case={command_case:?}"));
    }
    if let Some(keyword_case) = cli.config_overrides.keyword_case {
        parts.push(format!("keyword_case={keyword_case:?}"));
    }
    if let Some(dangle_parens) = cli.config_overrides.dangle_parens {
        parts.push(format!("dangle_parens={dangle_parens}"));
    }

    if parts.is_empty() {
        "cli overrides: none".to_owned()
    } else {
        format!("cli overrides: {}", parts.join(", "))
    }
}

pub(crate) fn resolve_config_probe_target(cli: &Cli) -> Result<Option<PathBuf>, cmakefmt::Error> {
    if cli.input_selection.files.is_empty() {
        return Ok(Some(PathBuf::from(".")));
    }

    if cli.input_selection.files.len() != 1 {
        return Err(cmakefmt::Error::cli_arg(
            "config introspection expects exactly one explicit path",
        ));
    }

    if cli.input_selection.files[0] == "-" {
        return cli
            .input_selection
            .stdin_path
            .clone()
            .map(Some)
            .ok_or_else(|| {
                cmakefmt::Error::cli_arg("stdin config introspection requires --stdin-path")
            });
    }

    Ok(Some(PathBuf::from(&cli.input_selection.files[0])))
}
