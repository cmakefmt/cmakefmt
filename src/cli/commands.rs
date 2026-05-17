// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Implementations of `cmakefmt` subcommands (config, dump, manpage,
//! install-hook) and the watch + list-unknown-commands modes that branch off
//! `run()` before normal target processing kicks in.

use std::collections::{BTreeSet, HashMap};
use std::io;
use std::path::{Path, PathBuf};

use clap::CommandFactory;
use cmakefmt::files::{discover_cmake_files_with_options, is_cmake_file, DiscoveryOptions};
use cmakefmt::{
    convert_legacy_config_files, default_config_template_for, format_source_with_registry,
    generate_json_schema, parser, render_effective_config, Config, DumpConfigFormat, IoResultExt,
};
use regex::Regex;

use crate::cli::process::{
    build_context, describe_cli_overrides, describe_config_mode, resolve_config_context,
    resolve_config_probe_target, InputTarget,
};
use crate::cli::runtime::atomic_write;
use crate::cli::spec_coverage::run_spec_coverage;
use crate::{
    should_colorize_stderr, should_colorize_stdout, Cli, ConfigAction, DumpAction, EXIT_OK,
};

pub(crate) fn run_list_unknown_commands(
    cli: &Cli,
    targets: &[InputTarget],
) -> Result<u8, cmakefmt::Error> {
    use cmakefmt::parser;
    use std::collections::BTreeMap;

    // command_name -> vec of (file, line)
    let mut unknown: BTreeMap<String, Vec<(String, usize)>> = BTreeMap::new();

    for target in targets {
        let (display_name, source) = match target {
            InputTarget::Stdin => {
                let mut buf = String::new();
                io::Read::read_to_string(&mut io::stdin(), &mut buf)
                    .map_err(cmakefmt::Error::Io)?;
                ("<stdin>".to_owned(), buf)
            }
            InputTarget::Path(path) => {
                let source = std::fs::read_to_string(path).with_path(path)?;
                (path.display().to_string(), source)
            }
        };

        let (_, registry, _) = build_context(
            cli,
            match target {
                InputTarget::Path(p) => Some(p.as_path()),
                InputTarget::Stdin => cli.input_selection.stdin_path.as_deref().map(Path::new),
            },
        )?;

        let file = match parser::parse(&source) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("warning: {display_name}: parse error, skipping ({e})");
                continue;
            }
        };

        for statement in &file.statements {
            if let parser::ast::Statement::Command(command) = statement {
                if !registry.contains(&command.name) {
                    let line = source[..command.span.0]
                        .chars()
                        .filter(|&c| c == '\n')
                        .count()
                        + 1;
                    unknown
                        .entry(command.name.to_ascii_lowercase())
                        .or_default()
                        .push((display_name.clone(), line));
                }
            }
        }
    }

    if unknown.is_empty() {
        eprintln!("No unknown commands found.");
        return Ok(EXIT_OK);
    }

    for (name, locations) in &unknown {
        println!("{name}");
        for (file, line) in locations {
            println!("  {file}:{line}");
        }
    }

    Ok(EXIT_OK)
}

pub(crate) fn run_watch(
    cli: &Cli,
    initial_targets: &[InputTarget],
    file_filter: Option<&Regex>,
) -> Result<u8, cmakefmt::Error> {
    use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    let colorize_stderr = should_colorize_stderr(cli.output_modes.color);

    // Collect directories to watch from the initial targets.
    let mut watch_roots = Vec::new();
    for target in initial_targets {
        match target {
            InputTarget::Path(path) => {
                if path.is_dir() {
                    watch_roots.push(path.clone());
                } else if let Some(parent) = path.parent() {
                    watch_roots.push(parent.to_path_buf());
                }
            }
            InputTarget::Stdin => {}
        }
    }
    if watch_roots.is_empty() {
        watch_roots.push(std::env::current_dir().map_err(cmakefmt::Error::Io)?);
    }
    watch_roots.sort();
    watch_roots.dedup();
    let mut known_mtimes = HashMap::new();

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();
    ctrlc::set_handler(move || {
        shutdown_clone.store(true, Ordering::Relaxed);
    })
    .map_err(|e| cmakefmt::Error::Formatter(format!("failed to set Ctrl+C handler: {e}")))?;

    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_millis(300), tx)
        .map_err(|e| cmakefmt::Error::Formatter(format!("failed to create file watcher: {e}")))?;

    for root in &watch_roots {
        debouncer
            .watcher()
            .watch(root, notify::RecursiveMode::Recursive)
            .map_err(|e| {
                cmakefmt::Error::Formatter(format!("failed to watch {}: {e}", root.display()))
            })?;
    }

    eprintln!(
        "watching {} for changes (Ctrl+C to stop)...",
        watch_roots
            .iter()
            .map(|r| r.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    while !shutdown.load(Ordering::Relaxed) {
        let should_poll = match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(Ok(events)) => events.into_iter().any(|event| {
                matches!(
                    event.kind,
                    DebouncedEventKind::Any | DebouncedEventKind::AnyContinuous
                )
            }),
            Ok(Err(err)) => {
                eprintln!("watch error: {err}");
                true
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => true,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        };

        if !should_poll {
            continue;
        }

        let changed_paths = poll_watch_changes(&watch_roots, cli, file_filter, &mut known_mtimes);
        let mut formatted_paths = BTreeSet::new();
        for path in changed_paths {
            if formatted_paths.contains(&path) {
                continue;
            }
            formatted_paths.insert(path.clone());

            match watch_format_file(cli, &path, colorize_stderr) {
                Ok(msg) => eprintln!("{msg}"),
                Err(e) => eprintln!("error: {}: {e}", path.display()),
            }
        }
    }

    eprintln!("stopped.");
    Ok(EXIT_OK)
}

fn poll_watch_changes(
    watch_roots: &[PathBuf],
    cli: &Cli,
    file_filter: Option<&Regex>,
    known_mtimes: &mut HashMap<PathBuf, Option<std::time::SystemTime>>,
) -> Vec<PathBuf> {
    let current_paths = collect_watch_candidates(watch_roots, cli, file_filter);
    let mut changed = Vec::new();

    for path in current_paths.iter().cloned() {
        let modified = watch_modified_time(&path);
        let previous = known_mtimes.insert(path.clone(), modified);
        if previous != Some(modified) {
            changed.push(path);
        }
    }

    known_mtimes.retain(|path, _| current_paths.contains(path));
    changed.sort();
    changed
}

fn collect_watch_candidates(
    watch_roots: &[PathBuf],
    cli: &Cli,
    file_filter: Option<&Regex>,
) -> BTreeSet<PathBuf> {
    let mut candidates = BTreeSet::new();

    for root in watch_roots {
        if root.is_file() {
            if is_cmake_file(root) {
                candidates.insert(root.clone());
            }
            continue;
        }

        for path in discover_cmake_files_with_options(
            root,
            DiscoveryOptions {
                file_filter,
                honor_gitignore: !cli.input_selection.no_gitignore,
                explicit_ignore_paths: &cli.input_selection.ignore_paths,
            },
        ) {
            candidates.insert(path);
        }
    }

    candidates
}

fn watch_modified_time(path: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

fn watch_format_file(cli: &Cli, path: &Path, colorize: bool) -> Result<String, cmakefmt::Error> {
    let source = std::fs::read_to_string(path).with_path(path)?;
    let (config, registry, _) = build_context(cli, Some(path))?;
    let formatted = format_source_with_registry(&source, &config, &registry)
        .map_err(|e| e.with_display_name(path.display().to_string()))?;

    let would_change = formatted != source;
    if would_change {
        atomic_write(path, &formatted)?;
        if colorize {
            Ok(format!("\x1b[1;93m!\x1b[0m {}", path.display()))
        } else {
            Ok(format!("[!] {}", path.display()))
        }
    } else if colorize {
        Ok(format!(
            "\x1b[1;32m✔\x1b[0m \x1b[2m{}\x1b[0m",
            path.display()
        ))
    } else {
        Ok(format!("[ok] {}", path.display()))
    }
}

pub(crate) fn run_config_subcommand(
    cli: &Cli,
    action: &ConfigAction,
) -> Result<u8, cmakefmt::Error> {
    match action {
        ConfigAction::Dump { format } => {
            print!("{}", default_config_template_for(*format));
            Ok(EXIT_OK)
        }
        ConfigAction::Schema => {
            println!("{}", generate_json_schema());
            Ok(EXIT_OK)
        }
        ConfigAction::Check { path } => {
            let path_arg = path.as_deref().unwrap_or("");
            run_check_config(cli, path_arg)
        }
        ConfigAction::Show { format, path } => {
            if let Some(p) = path {
                if !Path::new(p).exists() {
                    return Err(cmakefmt::Error::cli_arg(format!("file not found: {p}")));
                }
            }
            let target = path
                .as_ref()
                .map(PathBuf::from)
                .or_else(|| resolve_config_probe_target(cli).ok().flatten());
            let (config, _, _) = build_context(cli, target.as_deref())?;
            let rendered = render_effective_config(&config, *format)?;
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(EXIT_OK)
        }
        ConfigAction::Path { path } => {
            if let Some(p) = path {
                if !Path::new(p).exists() {
                    return Err(cmakefmt::Error::cli_arg(format!("file not found: {p}")));
                }
            }
            let target = path
                .as_ref()
                .map(PathBuf::from)
                .or_else(|| resolve_config_probe_target(cli).ok().flatten());
            let config_context = resolve_config_context(cli, target.as_deref());
            for p in &config_context.sources {
                println!("{}", p.display());
            }
            Ok(EXIT_OK)
        }
        ConfigAction::Explain { path } => {
            if let Some(p) = path {
                if !Path::new(p).exists() {
                    return Err(cmakefmt::Error::cli_arg(format!("file not found: {p}")));
                }
            }
            let target = path.as_deref().map(Path::new).unwrap_or(Path::new("."));
            explain_config(cli, target)
        }
        ConfigAction::Convert { paths, format } => {
            if paths.is_empty() {
                return Err(cmakefmt::Error::cli_arg(
                    "cmakefmt config convert requires at least one config file path",
                ));
            }
            let output = convert_legacy_config_files(paths, *format)?;
            print!("{output}");
            Ok(EXIT_OK)
        }
        ConfigAction::Init => {
            let path = Path::new(".cmakefmt.yaml");
            if path.exists() {
                return Err(cmakefmt::Error::cli_arg(".cmakefmt.yaml already exists"));
            }
            std::fs::write(path, default_config_template_for(DumpConfigFormat::Yaml))
                .map_err(cmakefmt::Error::Io)?;
            eprintln!("created .cmakefmt.yaml");
            Ok(EXIT_OK)
        }
    }
}

pub(crate) fn run_dump_subcommand(
    cli: &Cli,
    action: &DumpAction,
    file: Option<&Path>,
) -> Result<u8, cmakefmt::Error> {
    // `dump spec-coverage` is a read-only introspection of the
    // formatter's built-in registry — no source input is needed, so
    // skip the file/stdin read that the source-consuming variants
    // share.
    if let DumpAction::SpecCoverage { format, status } = action {
        return run_spec_coverage(*format, *status);
    }

    let source = match file {
        Some(path) if path.as_os_str() != "-" => std::fs::read_to_string(path).with_path(path)?,
        _ => {
            let mut buf = String::new();
            io::Read::read_to_string(&mut io::stdin(), &mut buf).map_err(cmakefmt::Error::Io)?;
            buf
        }
    };

    let parsed = parser::parse(&source)?;
    let color = should_colorize_stdout(cli.output_modes.color);

    let tree = match action {
        DumpAction::Ast => cmakefmt::dump::dump_ast(&parsed, color),
        DumpAction::Parse => {
            let config_path = file.filter(|p| p.as_os_str() != "-");
            let (_, registry, _) = build_context(cli, config_path)?;
            cmakefmt::dump::dump_parse(&parsed, &registry, color)
        }
        DumpAction::SpecCoverage { .. } => unreachable!("handled above"),
    };

    print!("{tree}");
    Ok(EXIT_OK)
}

/// Render the clap-derived CLI as a roff man page and write it to
/// stdout. Shared between the `Manpage` subcommand and the
/// deprecated `--generate-man-page` flag so both forms emit
/// byte-identical output during the transition window.
pub(crate) fn render_man_page() -> Result<u8, cmakefmt::Error> {
    let command = Cli::command();
    clap_mangen::Man::new(command)
        .render(&mut io::stdout())
        .map_err(cmakefmt::Error::Io)?;
    Ok(EXIT_OK)
}

pub(crate) fn install_git_hook() -> Result<u8, cmakefmt::Error> {
    let hooks_dir = Path::new(".git/hooks");
    if !hooks_dir.exists() {
        return Err(cmakefmt::Error::cli_arg(
            "not a git repository (no .git/hooks directory)",
        ));
    }
    let hook_path = hooks_dir.join("pre-commit");
    if hook_path.exists() {
        return Err(cmakefmt::Error::cli_arg(format!(
            "{} already exists; remove it first or add cmakefmt manually",
            hook_path.display()
        )));
    }
    let hook_content = "#!/bin/sh\n\
        # Installed by cmakefmt install-hook\n\
        cmakefmt --check --staged\n";
    std::fs::write(&hook_path, hook_content).with_path(&hook_path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755))
            .with_path(&hook_path)?;
    }
    eprintln!("installed pre-commit hook: {}", hook_path.display());
    Ok(EXIT_OK)
}

fn run_check_config(cli: &Cli, path_arg: &str) -> Result<u8, cmakefmt::Error> {
    if !path_arg.is_empty() {
        let path = Path::new(path_arg);
        if !path.exists() {
            return Err(cmakefmt::Error::cli_arg(format!(
                "config file not found: {}",
                path.display()
            )));
        }
        match Config::from_files(&[path.to_path_buf()]) {
            Ok(_) => {
                println!("config is valid: {}", path.display());
                Ok(EXIT_OK)
            }
            Err(err) => Err(err),
        }
    } else {
        let context = resolve_config_context(cli, Some(Path::new(".")));
        if context.sources.is_empty() {
            return Err(cmakefmt::Error::cli_arg("no config file found"));
        }
        match Config::from_files(&context.sources) {
            Ok(_) => {
                for source in &context.sources {
                    println!("config is valid: {}", source.display());
                }
                Ok(EXIT_OK)
            }
            Err(err) => Err(err),
        }
    }
}

fn explain_config(cli: &Cli, path: &Path) -> Result<u8, cmakefmt::Error> {
    let (config, _, config_context) = build_context(cli, Some(path))?;
    println!("target: {}", path.display());
    println!("config mode: {}", describe_config_mode(config_context.mode));
    if config_context.sources.is_empty() {
        println!("config files: none");
    } else {
        println!("config files:");
        for source in &config_context.sources {
            println!("  - {}", source.display());
        }
    }

    let cli_overrides = describe_cli_overrides(cli);
    println!("{cli_overrides}");
    println!();
    println!("effective config:");
    let rendered = render_effective_config(&config, DumpConfigFormat::Yaml)?;
    print!("{rendered}");
    if !rendered.ends_with('\n') {
        println!();
    }
    Ok(EXIT_OK)
}
