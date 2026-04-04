use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;
use std::sync::Arc;

use clap::{Parser, ValueEnum};
use cmakefmt::spec::registry::CommandRegistry;
use cmakefmt::{
    convert_legacy_config_files, default_config_template_for,
    files::{discover_cmake_files_with_options, is_cmake_file, matches_filter, DiscoveryOptions},
    format_source_with_registry, format_source_with_registry_debug, CaseStyle, Config,
    DumpConfigFormat,
};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use pest::error::{ErrorVariant, LineColLocation};
use rayon::prelude::*;
use regex::Regex;
use serde::Serialize;
use similar::TextDiff;

const LONG_ABOUT: &str = "

Parse CMake listfiles and format them nicely.

Formatting is configurable with one or more YAML or TOML configuration files.
If no config file is specified on the command line, cmakefmt will try to find
the nearest .cmakefmt.yaml, .cmakefmt.yml, or .cmakefmt.toml for each input by
walking up through parent directories to the repository root or filesystem
root. If no project-local config exists, cmakefmt falls back to the same files
in the home directory when present.

Direct file arguments are always processed, even if ignore files would skip
them during recursive discovery. Ignore rules only affect files discovered
from directories, --files-from, or Git-aware selection modes.

cmakefmt can print a commented starter configuration for you as a customization
starting point with --dump-config. By default this emits YAML; pass
--dump-config toml for TOML output.

Legacy cmake-format JSON, YAML, and Python config files can be converted to
.cmakefmt.toml with --convert-legacy-config. YAML is the recommended user
config format for larger custom-command specs.";

/// A fast, correct CMake formatter.
#[derive(Parser, Debug)]
#[command(
    name = "cmakefmt",
    version,
    about = "Parse CMake listfiles and format them nicely.",
    long_about = LONG_ABOUT
)]
struct Cli {
    /// Files or directories to format. Use `-` for stdin.
    ///
    /// If omitted, `cmakefmt` recursively finds CMake files under the current
    /// working directory.
    files: Vec<String>,

    /// Read more formatting targets from a file, or `-` for stdin.
    ///
    /// Accepts newline-delimited or NUL-delimited path lists. This is useful
    /// for scripted workflows that already know which files to pass to
    /// `cmakefmt`.
    #[arg(
        long = "files-from",
        value_name = "PATH",
        help_heading = "Input Selection",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    files_from: Vec<String>,

    /// Rewrite files on disk instead of printing formatted output.
    #[arg(
        short = 'i',
        long = "in-place",
        help_heading = "Output Modes",
        conflicts_with = "list_files",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    in_place: bool,

    /// Exit with code 1 if any selected file would change.
    #[arg(
        long,
        help_heading = "Output Modes",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    check: bool,

    /// Print only the files that would change, without modifying them.
    #[arg(
        long = "list-files",
        help_heading = "Output Modes",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    list_files: bool,

    /// Filter recursively discovered CMake paths with a regex.
    ///
    /// This only affects discovery from directories or Git/file-list driven
    /// inputs. Direct file arguments are always kept.
    #[arg(
        long = "path-regex",
        value_name = "REGEX",
        help_heading = "Input Selection",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    file_regex: Option<String>,

    /// Add one or more extra ignore files during recursive discovery.
    ///
    /// This only affects discovered files, not direct file arguments.
    #[arg(
        long = "ignore-path",
        value_name = "PATH",
        help_heading = "Input Selection",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    ignore_paths: Vec<PathBuf>,

    /// Ignore `.gitignore` files during recursive discovery.
    #[arg(
        long = "no-gitignore",
        help_heading = "Input Selection",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    no_gitignore: bool,

    /// Print the default config template and exit.
    ///
    /// By default this emits YAML. Pass `toml` to print TOML instead.
    #[arg(
        long = "dump-config",
        value_name = "FORMAT",
        help_heading = "Config And Conversion",
        num_args = 0..=1,
        default_missing_value = "yaml"
    )]
    dump_config: Option<DumpConfigFormat>,

    /// Convert legacy cmake-format JSON, YAML, or Python config files.
    ///
    /// The converted config is printed to stdout as `.cmakefmt.toml`.
    #[arg(
        long = "convert-legacy-config",
        value_name = "PATH",
        help_heading = "Config And Conversion",
        conflicts_with = "dump_config",
        conflicts_with = "check",
        conflicts_with = "list_files",
        conflicts_with = "in_place",
        conflicts_with = "debug",
        conflicts_with = "parallel",
        conflicts_with = "config_paths",
        conflicts_with = "line_width",
        conflicts_with = "tab_size",
        conflicts_with = "command_case",
        conflicts_with = "keyword_case",
        conflicts_with = "dangle_parens"
    )]
    convert_config_paths: Vec<PathBuf>,

    /// Print detailed discovery, config, and formatter diagnostics to stderr.
    #[arg(
        long,
        help_heading = "Execution",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    debug: bool,

    /// Print a unified diff instead of full formatted output.
    #[arg(
        long,
        help_heading = "Output Modes",
        conflicts_with = "in_place",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    diff: bool,

    /// Select modified Git-tracked files instead of explicit input paths.
    ///
    /// Use `--since` to compare against a specific base ref; otherwise
    /// `cmakefmt` compares the working tree against `HEAD`.
    #[arg(
        long,
        help_heading = "Input Selection",
        conflicts_with = "staged",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    changed: bool,

    /// Select staged Git-tracked files instead of explicit input paths.
    #[arg(
        long,
        help_heading = "Input Selection",
        conflicts_with = "changed",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    staged: bool,

    /// Git base ref used together with `--changed`.
    #[arg(
        long,
        requires = "changed",
        value_name = "REF",
        help_heading = "Input Selection"
    )]
    since: Option<String>,

    /// Virtual path used for config discovery and diagnostics when reading stdin.
    ///
    /// This does not read from disk; it only gives stdin formatting a real
    /// project-relative path to work from.
    #[arg(
        long = "stdin-path",
        value_name = "PATH",
        help_heading = "Input Selection"
    )]
    stdin_path: Option<PathBuf>,

    /// Restrict formatting to one or more 1-based inclusive line ranges.
    ///
    /// This is intended for editor integrations and only works on a single
    /// formatting target.
    #[arg(
        long = "lines",
        value_name = "START:END",
        help_heading = "Input Selection"
    )]
    line_ranges: Vec<LineRange>,

    /// Choose human terminal output or machine-readable JSON reporting.
    #[arg(
        long = "report-format",
        value_enum,
        default_value_t = ReportFormat::Human,
        help_heading = "Output Modes"
    )]
    report_format: ReportFormat,

    /// Control ANSI colour when printing formatted output to stdout.
    #[arg(
        long = "colour",
        alias = "color",
        value_enum,
        default_value_t = ColorChoice::Auto,
        help_heading = "Output Modes"
    )]
    colour: ColorChoice,

    /// Format files in parallel when explicitly requested.
    ///
    /// If omitted entirely, formatting stays single-threaded. If provided
    /// without a value, cmakefmt uses the available CPU count.
    #[arg(
        short = 'j',
        long,
        value_name = "JOBS",
        help_heading = "Execution",
        num_args = 0..=1,
        default_missing_value = "0",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    parallel: Option<usize>,

    /// Show a progress bar while formatting files in-place.
    ///
    /// The progress bar is intended for directory or multi-file runs and is
    /// only available together with `--in-place`.
    #[arg(
        long = "progress-bar",
        requires = "in_place",
        help_heading = "Execution"
    )]
    progress_bar: bool,

    /// Use one or more explicit config files instead of config discovery.
    ///
    /// Later files override earlier ones.
    #[arg(
        long = "config-file",
        visible_alias = "config",
        value_name = "PATH",
        help_heading = "Config Overrides"
    )]
    config_paths: Vec<PathBuf>,

    /// Override the maximum line width.
    #[arg(long, help_heading = "Config Overrides")]
    line_width: Option<usize>,

    /// Override the number of spaces per indent level.
    #[arg(long, help_heading = "Config Overrides")]
    tab_size: Option<usize>,

    /// Normalise command name case (lower, upper, unchanged).
    #[arg(long, help_heading = "Config Overrides")]
    command_case: Option<CaseStyle>,

    /// Normalise keyword case (lower, upper, unchanged).
    #[arg(long, help_heading = "Config Overrides")]
    keyword_case: Option<CaseStyle>,

    /// Place closing paren on its own line when wrapping.
    #[arg(long, help_heading = "Config Overrides")]
    dangle_parens: Option<bool>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum ColorChoice {
    /// Use colour only when stdout looks like an interactive terminal.
    Auto,
    /// Always emit ANSI colour codes.
    Always,
    /// Never emit ANSI colour codes.
    Never,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum ReportFormat {
    /// Human-friendly terminal output.
    Human,
    /// Machine-readable JSON output.
    Json,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LineRange {
    start: usize,
    end: usize,
}

impl LineRange {
    fn contains(&self, line: usize) -> bool {
        self.start <= line && line <= self.end
    }
}

impl FromStr for LineRange {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let Some((start, end)) = value.split_once(':') else {
            return Err("expected START:END".to_owned());
        };
        let start = start
            .parse::<usize>()
            .map_err(|_| "line range start must be a positive integer".to_owned())?;
        let end = end
            .parse::<usize>()
            .map_err(|_| "line range end must be a positive integer".to_owned())?;
        if start == 0 || end == 0 {
            return Err("line ranges are 1-based".to_owned());
        }
        if end < start {
            return Err("line range end must be >= start".to_owned());
        }
        Ok(Self { start, end })
    }
}

/// Exit codes matching the spec in ARCHITECTURE.md.
const EXIT_OK: u8 = 0;
const EXIT_CHECK_FAILED: u8 = 1;
const EXIT_ERROR: u8 = 2;

fn main() -> ExitCode {
    let cli = Cli::parse();

    match run(&cli) {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("{}", render_cli_error(&err));
            ExitCode::from(EXIT_ERROR)
        }
    }
}

fn run(cli: &Cli) -> Result<u8, cmakefmt::Error> {
    if let Some(format) = cli.dump_config {
        print!("{}", default_config_template_for(format));
        return Ok(EXIT_OK);
    }

    if !cli.convert_config_paths.is_empty() {
        if !cli.files.is_empty() {
            return Err(cmakefmt::Error::Formatter(
                "--convert-legacy-config does not accept formatting input paths".to_owned(),
            ));
        }
        print!(
            "{}",
            convert_legacy_config_files(&cli.convert_config_paths)?
        );
        return Ok(EXIT_OK);
    }

    validate_cli(cli)?;

    let stdout_mode = !cli.list_files && !cli.check && !cli.in_place;
    let colorize_stdout = stdout_mode && should_colorize_stdout(cli.colour);
    let file_filter = compile_file_filter(cli.file_regex.as_deref())?;
    let targets = collect_targets(cli, file_filter.as_ref())?;
    if !cli.line_ranges.is_empty() && targets.len() != 1 {
        return Err(cmakefmt::Error::Formatter(
            "--lines requires exactly one formatting target".to_owned(),
        ));
    }
    let parallel_jobs = resolve_parallel_jobs(cli.parallel)?;
    let mut any_would_change = false;
    let progress = ProgressReporter::new(cli.progress_bar, cli.in_place, targets.len());

    if cli.debug {
        log_debug(format!(
            "discovered {} target(s){}",
            targets.len(),
            debug_parallel_suffix(parallel_jobs)
        ));
    }

    let results = if parallel_jobs > 1 && targets.iter().all(InputTarget::is_path) {
        process_targets_parallel(&targets, cli, parallel_jobs, colorize_stdout, &progress)?
    } else {
        if cli.debug && parallel_jobs > 1 && targets.iter().any(|target| !target.is_path()) {
            log_debug("parallel mode ignored because stdin input must run serially");
        }
        process_targets_serial(&targets, cli, colorize_stdout, &progress)?
    };
    let multi_target_stdout = stdout_mode && results.len() > 1;

    if cli.report_format == ReportFormat::Json {
        if cli.in_place {
            for result in &results {
                if let Some(path) = &result.path {
                    if result.would_change {
                        std::fs::write(path, &result.formatted).map_err(cmakefmt::Error::Io)?;
                    }
                }
            }
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&build_json_report(&results, cli)).map_err(|err| {
                cmakefmt::Error::Formatter(format!("failed to build JSON report: {err}"))
            })?
        );
        return if (cli.check || cli.list_files) && results.iter().any(|r| r.would_change) {
            Ok(EXIT_CHECK_FAILED)
        } else {
            Ok(EXIT_OK)
        };
    }

    for (index, result) in results.iter().enumerate() {
        if cli.debug {
            for line in &result.debug_lines {
                log_debug(line);
            }
        }

        any_would_change |= result.would_change;

        if cli.list_files {
            if result.would_change {
                println!("{}", result.display_name);
            }
        } else if cli.check {
            if result.would_change {
                eprintln!("{} would be reformatted", result.display_name);
            }
        } else if cli.in_place {
            if let Some(path) = &result.path {
                if result.would_change {
                    std::fs::write(path, &result.formatted).map_err(cmakefmt::Error::Io)?;
                }
            }
        } else {
            if cli.diff {
                if result.would_change {
                    io::stdout()
                        .write_all(
                            result
                                .unified_diff
                                .as_deref()
                                .unwrap_or_default()
                                .as_bytes(),
                        )
                        .map_err(cmakefmt::Error::Io)?;
                }
            } else {
                if multi_target_stdout {
                    if index > 0 {
                        io::stdout().write_all(b"\n").map_err(cmakefmt::Error::Io)?;
                    }
                    write_stdout_header(&result.display_name, colorize_stdout)?;
                }
                let display_output = result
                    .highlighted_output
                    .as_deref()
                    .unwrap_or(&result.formatted);
                io::stdout()
                    .write_all(display_output.as_bytes())
                    .map_err(cmakefmt::Error::Io)?;
            }
        }
    }

    if (cli.check || cli.list_files) && any_would_change {
        Ok(EXIT_CHECK_FAILED)
    } else {
        Ok(EXIT_OK)
    }
}

fn write_stdout_header(display_name: &str, colorize: bool) -> Result<(), cmakefmt::Error> {
    if colorize {
        writeln!(io::stdout(), "\u{1b}[1;36m### {display_name}\u{1b}[0m")
            .map_err(cmakefmt::Error::Io)?;
    } else {
        writeln!(io::stdout(), "### {display_name}").map_err(cmakefmt::Error::Io)?;
    }
    Ok(())
}

fn validate_cli(cli: &Cli) -> Result<(), cmakefmt::Error> {
    if (cli.staged || cli.changed) && (!cli.files.is_empty() || !cli.files_from.is_empty()) {
        return Err(cmakefmt::Error::Formatter(
            "--staged/--changed cannot be combined with explicit input paths or --files-from"
                .to_owned(),
        ));
    }

    if cli.stdin_path.is_some() && !cli.files.iter().any(|file| file == "-") {
        return Err(cmakefmt::Error::Formatter(
            "--stdin-path requires stdin input via `cmakefmt -`".to_owned(),
        ));
    }

    if cli.diff && cli.list_files {
        return Err(cmakefmt::Error::Formatter(
            "--diff cannot be combined with --list-files".to_owned(),
        ));
    }

    Ok(())
}

fn compile_file_filter(pattern: Option<&str>) -> Result<Option<Regex>, cmakefmt::Error> {
    pattern
        .map(|pattern| {
            Regex::new(pattern).map_err(|err| {
                cmakefmt::Error::Formatter(format!("invalid file regex {pattern:?}: {err}"))
            })
        })
        .transpose()
}

fn collect_targets(
    cli: &Cli,
    file_filter: Option<&Regex>,
) -> Result<Vec<InputTarget>, cmakefmt::Error> {
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
            for discovered in discover_cmake_files_with_options(
                &path,
                DiscoveryOptions {
                    file_filter,
                    honor_gitignore: !cli.no_gitignore,
                    explicit_ignore_paths: &cli.ignore_paths,
                },
            ) {
                push_unique_path(&mut targets, &mut seen_paths, discovered);
            }
            continue;
        }

        return Err(cmakefmt::Error::Formatter(format!(
            "{}: no such file or directory",
            path.display()
        )));
    }

    Ok(targets)
}

fn collect_input_arguments(
    cli: &Cli,
    file_filter: Option<&Regex>,
) -> Result<Vec<String>, cmakefmt::Error> {
    let mut inputs = Vec::new();

    if cli.staged {
        inputs.extend(collect_git_paths(GitSelectionMode::Staged, file_filter)?);
    } else if cli.changed {
        inputs.extend(collect_git_paths(
            GitSelectionMode::Changed(cli.since.as_deref()),
            file_filter,
        )?);
    }

    for files_from in &cli.files_from {
        inputs.extend(read_files_from(files_from)?);
    }

    inputs.extend(cli.files.clone());

    if inputs.is_empty() {
        inputs.push(".".to_owned());
    }

    Ok(inputs)
}

#[derive(Copy, Clone)]
enum GitSelectionMode<'a> {
    Staged,
    Changed(Option<&'a str>),
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
        std::fs::read_to_string(source)
            .map_err(|err| cmakefmt::Error::Formatter(format!("{source}: {err}")))?
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
fn build_context(
    cli: &Cli,
    file_path: Option<&Path>,
) -> Result<(Config, CommandRegistry, Vec<PathBuf>), cmakefmt::Error> {
    let config_sources = if !cli.config_paths.is_empty() {
        cli.config_paths.clone()
    } else if let Some(path) = file_path {
        Config::config_sources_for(path)
    } else {
        Vec::new()
    };

    let mut config = Config::from_files(&config_sources)?;
    let mut registry = CommandRegistry::builtins().clone();
    for path in &config_sources {
        registry.merge_override_file(path)?;
    }

    if let Some(v) = cli.line_width {
        config.line_width = v;
    }
    if let Some(v) = cli.tab_size {
        config.tab_size = v;
    }
    if let Some(v) = cli.command_case {
        config.command_case = v;
    }
    if let Some(v) = cli.keyword_case {
        config.keyword_case = v;
    }
    if let Some(v) = cli.dangle_parens {
        config.dangle_parens = v;
    }

    Ok((config, registry, config_sources))
}

#[derive(Clone)]
enum InputTarget {
    Stdin,
    Path(PathBuf),
}

impl InputTarget {
    fn is_path(&self) -> bool {
        matches!(self, InputTarget::Path(_))
    }
}

struct ProcessedTarget {
    path: Option<PathBuf>,
    display_name: String,
    formatted: String,
    highlighted_output: Option<String>,
    unified_diff: Option<String>,
    changed_lines: Vec<usize>,
    would_change: bool,
    debug_lines: Vec<String>,
}

#[derive(Debug, Serialize)]
struct JsonReport {
    mode: &'static str,
    files: Vec<JsonFileReport>,
}

#[derive(Debug, Serialize)]
struct JsonFileReport {
    display_name: String,
    path: Option<String>,
    would_change: bool,
    changed_lines: Vec<usize>,
    formatted: Option<String>,
    diff: Option<String>,
    debug_lines: Vec<String>,
}

fn process_targets_serial(
    targets: &[InputTarget],
    cli: &Cli,
    colorize_stdout: bool,
    progress: &ProgressReporter,
) -> Result<Vec<ProcessedTarget>, cmakefmt::Error> {
    targets
        .iter()
        .map(|target| process_target(target, cli, colorize_stdout, progress))
        .collect()
}

fn process_targets_parallel(
    targets: &[InputTarget],
    cli: &Cli,
    parallel_jobs: usize,
    colorize_stdout: bool,
    progress: &ProgressReporter,
) -> Result<Vec<ProcessedTarget>, cmakefmt::Error> {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(parallel_jobs)
        .build()
        .map_err(|err| cmakefmt::Error::Formatter(format!("failed to build thread pool: {err}")))?;

    pool.install(|| {
        targets
            .par_iter()
            .map(|target| process_target(target, cli, colorize_stdout, progress))
            .collect()
    })
}

fn process_target(
    target: &InputTarget,
    cli: &Cli,
    colorize_stdout: bool,
    progress: &ProgressReporter,
) -> Result<ProcessedTarget, cmakefmt::Error> {
    let result = match target {
        InputTarget::Stdin => process_stdin(cli, colorize_stdout),
        InputTarget::Path(path) => process_path(path, cli, colorize_stdout),
    };
    progress.finish_one();
    result
}

fn process_stdin(cli: &Cli, colorize_stdout: bool) -> Result<ProcessedTarget, cmakefmt::Error> {
    let mut source = String::new();
    io::stdin()
        .read_to_string(&mut source)
        .map_err(cmakefmt::Error::Io)?;

    let stdin_path = cli.stdin_path.as_deref();
    let (config, registry, config_sources) = build_context(cli, stdin_path)?;
    let display_name = stdin_path
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<stdin>".to_owned());
    let mut debug_lines = vec![
        format!("processing {display_name}"),
        describe_config_sources(&config_sources),
    ];
    let (formatted, mut formatter_debug) = if cli.debug {
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
    debug_lines.append(&mut formatter_debug);

    let formatted = apply_line_ranges(&source, &formatted, &cli.line_ranges, &display_name)?;

    let would_change = formatted != source;
    let changed_lines = changed_formatted_line_numbers(
        &split_lines_with_endings(&source),
        &split_lines_with_endings(&formatted),
    );
    debug_lines.push(format!(
        "result {display_name}: would_change={would_change}"
    ));
    debug_lines.push(format!(
        "result {display_name}: changed_lines={}",
        changed_lines.len()
    ));
    let highlighted_output = colorize_stdout
        .then(|| highlight_changed_lines(&source, &formatted))
        .filter(|_| would_change);
    let unified_diff = would_change.then(|| build_unified_diff(&display_name, &source, &formatted));

    Ok(ProcessedTarget {
        path: stdin_path.map(Path::to_path_buf),
        display_name,
        formatted,
        highlighted_output,
        unified_diff,
        changed_lines,
        would_change,
        debug_lines,
    })
}

fn process_path(
    path: &Path,
    cli: &Cli,
    colorize_stdout: bool,
) -> Result<ProcessedTarget, cmakefmt::Error> {
    let source = std::fs::read_to_string(path)
        .map_err(|err| cmakefmt::Error::Formatter(format!("{}: {err}", path.display())))?;
    let (config, registry, config_sources) = build_context(cli, Some(path))?;
    let mut debug_lines = vec![
        format!("processing {}", path.display()),
        describe_config_sources(&config_sources),
        describe_cli_overrides(cli),
    ];

    let (formatted, mut formatter_debug) = if cli.debug {
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
    debug_lines.append(&mut formatter_debug);

    let formatted = apply_line_ranges(
        &source,
        &formatted,
        &cli.line_ranges,
        &path.display().to_string(),
    )?;
    let would_change = formatted != source;
    let changed_lines = changed_formatted_line_numbers(
        &split_lines_with_endings(&source),
        &split_lines_with_endings(&formatted),
    );
    debug_lines.push(format!(
        "result {}: would_change={would_change}",
        path.display()
    ));
    debug_lines.push(format!(
        "result {}: changed_lines={}",
        path.display(),
        changed_lines.len()
    ));
    let highlighted_output = colorize_stdout
        .then(|| highlight_changed_lines(&source, &formatted))
        .filter(|_| would_change);
    let unified_diff =
        would_change.then(|| build_unified_diff(&path.display().to_string(), &source, &formatted));

    Ok(ProcessedTarget {
        path: Some(path.to_path_buf()),
        display_name: path.display().to_string(),
        formatted,
        highlighted_output,
        unified_diff,
        changed_lines,
        would_change,
        debug_lines,
    })
}

fn resolve_parallel_jobs(requested: Option<usize>) -> Result<usize, cmakefmt::Error> {
    match requested {
        None => Ok(1),
        Some(0) => std::thread::available_parallelism()
            .map(|parallelism| parallelism.get())
            .map_err(cmakefmt::Error::Io),
        Some(jobs) => Ok(jobs.max(1)),
    }
}

#[derive(Clone)]
struct ProgressReporter {
    inner: Option<Arc<ProgressBar>>,
}

impl ProgressReporter {
    fn new(requested: bool, in_place: bool, total: usize) -> Self {
        let enabled = requested && in_place && total > 1 && io::stderr().is_terminal();
        let inner = enabled.then(|| {
            let progress = ProgressBar::new(total as u64);
            progress.set_draw_target(ProgressDrawTarget::stderr());
            progress.set_style(
                ProgressStyle::with_template(
                    "{spinner:.cyan} [{elapsed_precise}] [{bar:50.cyan/blue}] {pos}/{len} files",
                )
                .expect("progress template should be valid")
                .progress_chars("=> "),
            );
            Arc::new(progress)
        });
        Self { inner }
    }

    fn finish_one(&self) {
        let Some(inner) = &self.inner else {
            return;
        };

        inner.inc(1);
        if inner.position() == inner.length().unwrap_or(0) {
            inner.finish();
        }
    }
}

fn should_colorize_stdout(choice: ColorChoice) -> bool {
    match choice {
        ColorChoice::Auto => {
            io::stdout().is_terminal()
                && std::env::var_os("NO_COLOR").is_none()
                && std::env::var("TERM").map_or(true, |term| term != "dumb")
        }
        ColorChoice::Always => true,
        ColorChoice::Never => false,
    }
}

fn highlight_changed_lines(source: &str, formatted: &str) -> String {
    let original_lines = split_lines_with_endings(source);
    let formatted_lines = split_lines_with_endings(formatted);
    let changed_mask = changed_formatted_line_mask(&original_lines, &formatted_lines);
    let mut output = String::with_capacity(formatted.len() + changed_mask.len() * 10);

    for (line, changed) in formatted_lines.iter().zip(changed_mask) {
        if changed {
            push_cyan_line(&mut output, line);
        } else {
            output.push_str(line);
        }
    }

    output
}

fn split_lines_with_endings(source: &str) -> Vec<&str> {
    if source.is_empty() {
        Vec::new()
    } else {
        source.split_inclusive('\n').collect()
    }
}

fn changed_formatted_line_mask<'a>(original: &[&'a str], formatted: &[&'a str]) -> Vec<bool> {
    let mut prefix_len = 0;
    while prefix_len < original.len()
        && prefix_len < formatted.len()
        && original[prefix_len] == formatted[prefix_len]
    {
        prefix_len += 1;
    }

    let mut suffix_len = 0;
    while suffix_len < original.len().saturating_sub(prefix_len)
        && suffix_len < formatted.len().saturating_sub(prefix_len)
        && original[original.len() - 1 - suffix_len] == formatted[formatted.len() - 1 - suffix_len]
    {
        suffix_len += 1;
    }

    let original_mid = &original[prefix_len..original.len() - suffix_len];
    let formatted_mid = &formatted[prefix_len..formatted.len() - suffix_len];

    let mut changed = vec![false; prefix_len];
    changed.extend(diff_middle_mask(original_mid, formatted_mid));
    changed.extend(std::iter::repeat_n(false, suffix_len));
    changed
}

fn changed_formatted_line_numbers(original: &[&str], formatted: &[&str]) -> Vec<usize> {
    changed_formatted_line_mask(original, formatted)
        .into_iter()
        .enumerate()
        .filter_map(|(index, changed)| changed.then_some(index + 1))
        .collect()
}

fn diff_middle_mask<'a>(original: &[&'a str], formatted: &[&'a str]) -> Vec<bool> {
    const MAX_DP_CELLS: usize = 2_000_000;

    if formatted.is_empty() {
        return Vec::new();
    }

    if original.is_empty() {
        return vec![true; formatted.len()];
    }

    if original.len().saturating_mul(formatted.len()) > MAX_DP_CELLS {
        return vec![true; formatted.len()];
    }

    let mut dp = vec![vec![0u32; formatted.len() + 1]; original.len() + 1];

    for i in (0..original.len()).rev() {
        for j in (0..formatted.len()).rev() {
            dp[i][j] = if original[i] == formatted[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }

    let mut changed = vec![true; formatted.len()];
    let mut i = 0;
    let mut j = 0;

    while i < original.len() && j < formatted.len() {
        if original[i] == formatted[j] {
            changed[j] = false;
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }

    changed
}

fn push_cyan_line(output: &mut String, line: &str) {
    const CYAN: &str = "\u{1b}[36m";
    const RESET: &str = "\u{1b}[0m";

    if let Some(stripped) = line.strip_suffix('\n') {
        output.push_str(CYAN);
        output.push_str(stripped);
        output.push_str(RESET);
        output.push('\n');
    } else {
        output.push_str(CYAN);
        output.push_str(line);
        output.push_str(RESET);
    }
}

fn build_unified_diff(display_name: &str, source: &str, formatted: &str) -> String {
    TextDiff::from_lines(source, formatted)
        .unified_diff()
        .context_radius(3)
        .header(&format!("a/{display_name}"), &format!("b/{display_name}"))
        .to_string()
}

fn apply_line_ranges(
    source: &str,
    formatted: &str,
    line_ranges: &[LineRange],
    display_name: &str,
) -> Result<String, cmakefmt::Error> {
    if line_ranges.is_empty() {
        return Ok(formatted.to_owned());
    }

    let changed_lines = changed_formatted_line_numbers(
        &split_lines_with_endings(source),
        &split_lines_with_endings(formatted),
    );
    let mut outside = Vec::new();
    for line in changed_lines {
        if !line_ranges.iter().any(|range| range.contains(line)) {
            outside.push(line);
        }
    }

    if outside.is_empty() {
        Ok(formatted.to_owned())
    } else {
        Err(cmakefmt::Error::Formatter(format!(
            "{display_name}: selected line ranges would affect lines outside the requested ranges ({})",
            outside
                .into_iter()
                .map(|line| line.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }
}

fn build_json_report(results: &[ProcessedTarget], cli: &Cli) -> JsonReport {
    let mode = if cli.in_place {
        "in-place"
    } else if cli.check {
        "check"
    } else if cli.list_files {
        "list-files"
    } else if cli.diff {
        "diff"
    } else {
        "stdout"
    };

    JsonReport {
        mode,
        files: results
            .iter()
            .map(|result| JsonFileReport {
                display_name: result.display_name.clone(),
                path: result.path.as_ref().map(|path| path.display().to_string()),
                would_change: result.would_change,
                changed_lines: result.changed_lines.clone(),
                formatted: (!cli.in_place && !cli.check && !cli.list_files && !cli.diff)
                    .then(|| result.formatted.clone()),
                diff: cli
                    .diff
                    .then(|| result.unified_diff.clone().unwrap_or_default()),
                debug_lines: if cli.debug {
                    result.debug_lines.clone()
                } else {
                    Vec::new()
                },
            })
            .collect(),
    }
}

fn describe_config_sources(config_sources: &[PathBuf]) -> String {
    if config_sources.is_empty() {
        "config sources: defaults only".to_owned()
    } else {
        format!(
            "config sources: {}",
            config_sources
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn describe_cli_overrides(cli: &Cli) -> String {
    let mut parts = Vec::new();
    if let Some(line_width) = cli.line_width {
        parts.push(format!("line_width={line_width}"));
    }
    if let Some(tab_size) = cli.tab_size {
        parts.push(format!("tab_size={tab_size}"));
    }
    if let Some(command_case) = cli.command_case {
        parts.push(format!("command_case={command_case:?}"));
    }
    if let Some(keyword_case) = cli.keyword_case {
        parts.push(format!("keyword_case={keyword_case:?}"));
    }
    if let Some(dangle_parens) = cli.dangle_parens {
        parts.push(format!("dangle_parens={dangle_parens}"));
    }

    if parts.is_empty() {
        "cli overrides: none".to_owned()
    } else {
        format!("cli overrides: {}", parts.join(", "))
    }
}

fn debug_parallel_suffix(parallel_jobs: usize) -> String {
    if parallel_jobs > 1 {
        format!(" (parallel jobs: {parallel_jobs})")
    } else {
        String::new()
    }
}

fn log_debug(message: impl AsRef<str>) {
    eprintln!("debug: {}", message.as_ref());
}

fn render_cli_error(err: &cmakefmt::Error) -> String {
    match err {
        cmakefmt::Error::ParseContext {
            display_name,
            source_text,
            start_line,
            barrier_context,
            source,
        } => render_parse_error(
            display_name,
            source_text,
            *start_line,
            *barrier_context,
            source,
        ),
        cmakefmt::Error::Config { path, details, .. } => {
            render_file_parse_error("config", path, details)
        }
        cmakefmt::Error::Spec { path, details, .. } => {
            render_file_parse_error("spec", path, details)
        }
        cmakefmt::Error::Formatter(message) => render_formatter_error(message),
        cmakefmt::Error::Io(source) => format!("error: I/O failure: {source}"),
        cmakefmt::Error::Parse(source) => {
            format!("error: parse failure\n\nparser detail: {source}")
        }
    }
}

fn render_parse_error(
    display_name: &str,
    source_text: &str,
    start_line: usize,
    barrier_context: bool,
    source: &pest::error::Error<cmakefmt::parser::Rule>,
) -> String {
    let (local_line, local_column) = line_col_from_pest(source);
    let absolute_line = start_line + local_line.saturating_sub(1);
    let source_lines: Vec<&str> = source_text.lines().collect();
    let line_text = source_lines
        .get(local_line.saturating_sub(1))
        .copied()
        .or_else(|| source_lines.last().copied())
        .unwrap_or_default();
    let (summary, mut hints) = classify_parse_failure(display_name, line_text, source);
    if barrier_context {
        hints.push(
            "this file contains formatter barriers or fences; disabled regions are passed through verbatim"
                .to_owned(),
        );
    }

    let mut rendered = String::new();
    let _ = writeln!(
        rendered,
        "error: {summary}\n  --> {display_name}:{absolute_line}:{local_column}"
    );
    if !source_text.is_empty() {
        rendered.push('\n');
        rendered.push_str(&render_source_snippet(
            source_text,
            start_line,
            local_line,
            local_column,
        ));
        rendered.push('\n');
    }
    for hint in hints {
        let _ = writeln!(rendered, "hint: {hint}");
    }
    let _ = writeln!(
        rendered,
        "parser detail: {}",
        describe_pest_expectation(source)
    );
    if display_name != "<stdin>" {
        let _ = writeln!(rendered, "repro: cmakefmt --debug --check {display_name}");
    }
    rendered.trim_end().to_owned()
}

fn render_file_parse_error(
    kind: &str,
    path: &Path,
    source: &cmakefmt::error::FileParseError,
) -> String {
    let contents = std::fs::read_to_string(path).ok();
    let detail = source.message.as_ref();
    let mut rendered = String::new();
    let _ = writeln!(
        rendered,
        "error: invalid {kind} file ({})\n  --> {}",
        source.format,
        path.display()
    );

    if let (Some(contents), Some(line), Some(column)) =
        (contents.as_deref(), source.line, source.column)
    {
        let _ = writeln!(rendered, "      at {line}:{column}");
        rendered.push('\n');
        rendered.push_str(&render_source_snippet(contents, 1, line, column));
        rendered.push('\n');
    }

    let mut hints = Vec::new();
    if let Some((field, expected)) = extract_unknown_field_hint(detail) {
        if let Some(updated) = renamed_config_key(&field) {
            hints.push(format!(
                "`{field}` is not a valid cmakefmt key; use `{updated}` or run --convert-legacy-config"
            ));
        } else if let Some(suggestion) = best_match(&field, &expected) {
            hints.push(format!(
                "unknown key `{field}`; did you mean `{suggestion}`?"
            ));
        } else {
            hints.push(format!("unknown key `{field}` in {kind} file"));
        }
    }
    if kind == "config" {
        hints.push(
            "config files are applied in order; later files override earlier ones".to_owned(),
        );
    }
    for hint in hints {
        let _ = writeln!(rendered, "hint: {hint}");
    }
    let _ = writeln!(rendered, "detail: {detail}");
    rendered.trim_end().to_owned()
}

fn render_formatter_error(message: &str) -> String {
    let mut rendered = String::new();
    let _ = writeln!(rendered, "error: {message}");
    if let Some(pattern) = extract_invalid_regex_pattern(message) {
        let _ = writeln!(
            rendered,
            "hint: check the --path-regex pattern {pattern:?}; Rust regex syntax does not support every PCRE feature"
        );
    } else if message.contains("no such file or directory") {
        let _ = writeln!(
            rendered,
            "hint: pass an existing file or directory, or omit input paths to recurse from the current working directory"
        );
    }
    rendered.trim_end().to_owned()
}

fn line_col_from_pest(source: &pest::error::Error<cmakefmt::parser::Rule>) -> (usize, usize) {
    match source.line_col {
        LineColLocation::Pos((line, column)) => (line, column),
        LineColLocation::Span((line, column), _) => (line, column),
    }
}

fn describe_pest_expectation(source: &pest::error::Error<cmakefmt::parser::Rule>) -> String {
    match &source.variant {
        ErrorVariant::ParsingError { positives, .. } if !positives.is_empty() => {
            format!(
                "expected {}",
                positives
                    .iter()
                    .map(|rule| format!("{rule:?}").replace('_', " "))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        ErrorVariant::CustomError { message } => message.clone(),
        _ => source.to_string(),
    }
}

fn classify_parse_failure(
    display_name: &str,
    line_text: &str,
    source: &pest::error::Error<cmakefmt::parser::Rule>,
) -> (String, Vec<String>) {
    let detail = describe_pest_expectation(source);
    let trimmed = line_text.trim();
    let mut hints = Vec::new();

    if line_text.contains("\\\"") {
        hints.push(
            "possible malformed quoted argument or escaped quote sequence inside this command"
                .to_owned(),
        );
        hints.push(
            "if the caret looks late, the real problem may be earlier in the same command invocation"
                .to_owned(),
        );
        return ("failed to parse a quoted argument".to_owned(), hints);
    }

    if trimmed.contains("[=[") || trimmed.contains("[[") || trimmed.contains("]=]") {
        hints.push(
            "check that bracket argument or bracket comment delimiters use matching `=` counts"
                .to_owned(),
        );
        return (
            "failed to parse a bracket argument or comment".to_owned(),
            hints,
        );
    }

    if display_name.ends_with(".cmake.in") && trimmed.starts_with('@') && trimmed.ends_with('@') {
        hints.push("top-level configure-file placeholders like @PACKAGE_INIT@ are only valid as standalone template lines".to_owned());
        return (
            "failed to parse a configure-file template line".to_owned(),
            hints,
        );
    }

    if trimmed.contains('(') || trimmed.contains(')') {
        hints.push(
            "check for an unbalanced command invocation or control-flow condition".to_owned(),
        );
        hints.push(
            "the reported location can be after the real problem if an earlier line left the parser out of sync"
                .to_owned(),
        );
        return ("failed to parse a command invocation".to_owned(), hints);
    }

    if detail.contains("quoted element") {
        hints.push("a quoted string may be unterminated or contain malformed escapes".to_owned());
    }

    ("failed to parse CMake input".to_owned(), hints)
}

fn render_source_snippet(
    source: &str,
    start_line: usize,
    focus_line: usize,
    focus_column: usize,
) -> String {
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() {
        return String::new();
    }

    let focus_index = focus_line
        .saturating_sub(1)
        .min(lines.len().saturating_sub(1));
    let start_index = focus_index.saturating_sub(1);
    let end_index = (focus_index + 2).min(lines.len());
    let max_line_no = start_line + end_index.saturating_sub(1);
    let width = max_line_no.to_string().len();
    let mut rendered = String::new();

    for index in start_index..end_index {
        let absolute_line = start_line + index;
        let marker = if index == focus_index { '>' } else { ' ' };
        let _ = writeln!(
            rendered,
            "{marker} {absolute_line:>width$} | {}",
            lines[index],
            width = width
        );
        if index == focus_index {
            let visible_column = if focus_line > lines.len() {
                lines[index].chars().count() + 1
            } else {
                focus_column
            };
            let caret_padding = visible_column.saturating_sub(1);
            let _ = writeln!(
                rendered,
                "  {space:>width$} | {pad}^",
                space = "",
                pad = " ".repeat(caret_padding),
                width = width
            );
        }
    }

    rendered.trim_end().to_owned()
}

fn extract_unknown_field_hint(detail: &str) -> Option<(String, Vec<String>)> {
    let field = extract_between(detail, "unknown field `", "`")
        .or_else(|| extract_between(detail, "unknown field '", "'"))?;
    let expected = detail
        .split("expected one of")
        .nth(1)
        .map(|tail| {
            tail.split(',')
                .map(|part| {
                    part.trim()
                        .trim_matches('`')
                        .trim_matches('\'')
                        .trim_matches('"')
                        .to_owned()
                })
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some((field, expected))
}

fn extract_between(input: &str, start: &str, end: &str) -> Option<String> {
    let tail = input.split(start).nth(1)?;
    let field = tail.split(end).next()?;
    Some(field.to_owned())
}

fn best_match<'a>(needle: &str, candidates: &'a [String]) -> Option<&'a str> {
    candidates
        .iter()
        .filter_map(|candidate| {
            let distance = levenshtein(needle, candidate);
            (distance <= 6).then_some((distance, candidate.as_str()))
        })
        .min_by_key(|(distance, candidate)| (*distance, candidate.len()))
        .map(|(_, candidate)| candidate)
}

fn levenshtein(left: &str, right: &str) -> usize {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    let mut prev: Vec<usize> = (0..=right.len()).collect();
    let mut curr = vec![0; right.len() + 1];

    for (i, lch) in left.iter().enumerate() {
        curr[0] = i + 1;
        for (j, rch) in right.iter().enumerate() {
            let cost = usize::from(lch != rch);
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[right.len()]
}

fn renamed_config_key(key: &str) -> Option<&'static str> {
    match key {
        "use_tabchars" => Some("use_tabs"),
        "max_lines_hwrap" => Some("max_hanging_wrap_lines"),
        "max_pargs_hwrap" => Some("max_hanging_wrap_positional_args"),
        "max_subgroups_hwrap" => Some("max_hanging_wrap_groups"),
        "min_prefix_chars" => Some("min_prefix_length"),
        "max_prefix_chars" => Some("max_prefix_length"),
        "separate_ctrl_name_with_space" => Some("space_before_control_paren"),
        "separate_fn_name_with_space" => Some("space_before_definition_paren"),
        _ => None,
    }
}

fn extract_invalid_regex_pattern(message: &str) -> Option<&str> {
    let tail = message.strip_prefix("invalid file regex ")?;
    let start = tail.find('"')?;
    let end = tail[start + 1..].find('"')?;
    Some(&tail[start..=start + end + 1])
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;
    use cmakefmt::{default_config_template, default_config_template_for, DumpConfigFormat};

    #[test]
    fn dump_config_covers_config_backed_long_flags() {
        let template = default_config_template();
        let non_config_flags = [
            "check",
            "config-file",
            "convert-legacy-config",
            "colour",
            "changed",
            "debug",
            "diff",
            "path-regex",
            "files-from",
            "help",
            "dump-config",
            "ignore-path",
            "lines",
            "list-files",
            "no-gitignore",
            "parallel",
            "progress-bar",
            "report-format",
            "since",
            "staged",
            "stdin-path",
            "version",
            "in-place",
        ];

        for arg in Cli::command().get_arguments() {
            let Some(long) = arg.get_long() else {
                continue;
            };

            if non_config_flags.contains(&long) {
                continue;
            }

            let template_key = long.replace('-', "_");
            assert!(
                template.contains(&template_key),
                "CLI flag --{long} is not represented in default_config_template(); \
                 update src/config/file.rs or add --{long} to the non-config flag allowlist in src/main.rs tests"
            );
        }
    }

    #[test]
    fn toml_dump_config_covers_config_backed_long_flags() {
        let template = default_config_template_for(DumpConfigFormat::Toml);
        for key in [
            "line_width",
            "tab_size",
            "use_tabs",
            "max_empty_lines",
            "max_hanging_wrap_lines",
            "max_hanging_wrap_positional_args",
            "max_hanging_wrap_groups",
            "dangle_parens",
            "dangle_align",
            "min_prefix_length",
            "max_prefix_length",
            "space_before_control_paren",
            "space_before_definition_paren",
            "command_case",
            "keyword_case",
        ] {
            assert!(
                template.contains(key),
                "TOML dump template is missing {key}"
            );
        }
    }
}
