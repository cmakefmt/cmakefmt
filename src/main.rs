// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::hash::{Hash, Hasher};
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;
use std::sync::Arc;

use clap::{CommandFactory, Parser, ValueEnum};
use clap_complete::{generate, Shell};
use cmakefmt::spec::registry::CommandRegistry;
use cmakefmt::{
    convert_legacy_config_files, default_config_template_for,
    files::{discover_cmake_files_with_options, is_cmake_file, matches_filter, DiscoveryOptions},
    format_source_with_registry, format_source_with_registry_debug, parser,
    render_effective_config, CaseStyle, Config, DumpConfigFormat,
};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use pest::error::{ErrorVariant, LineColLocation};
use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
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
config format for larger custom-command specs.

Use --show-config-path to inspect which config file was selected, --show-config
to inspect the effective config after CLI overrides, and --explain-config for
a fuller human-readable explanation of config resolution for a target or the
current working directory.";

/// A fast, correct CMake formatter.
#[derive(Parser, Debug)]
#[command(
    name = "cmakefmt",
    version,
    long_version = env!("CMAKEFMT_CLI_LONG_VERSION"),
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
        conflicts_with = "list_changed_files",
        conflicts_with = "list_input_files",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    in_place: bool,

    /// Exit with code 1 if any selected file would change.
    #[arg(
        long,
        help_heading = "Output Modes",
        conflicts_with = "list_input_files",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    check: bool,

    /// Print only the files that would change, without modifying them.
    #[arg(
        long = "list-changed-files",
        alias = "list-files",
        help_heading = "Output Modes",
        conflicts_with = "quiet",
        conflicts_with = "list_input_files",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    list_changed_files: bool,

    /// Print the selected input files after discovery/filtering, without formatting them.
    #[arg(
        long = "list-input-files",
        help_heading = "Output Modes",
        conflicts_with = "check",
        conflicts_with = "list_changed_files",
        conflicts_with = "in_place",
        conflicts_with = "diff",
        conflicts_with = "quiet",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    list_input_files: bool,

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

    /// Generate shell completion scripts and print them to stdout.
    #[arg(
        long = "generate-completion",
        value_enum,
        value_name = "SHELL",
        help_heading = "Release And Packaging",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths",
        conflicts_with = "show_config",
        conflicts_with = "show_config_path",
        conflicts_with = "explain_config"
    )]
    generate_completion: Option<Shell>,

    /// Generate a roff man page and print it to stdout.
    #[arg(
        long = "generate-man-page",
        help_heading = "Release And Packaging",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths",
        conflicts_with = "generate_completion",
        conflicts_with = "show_config",
        conflicts_with = "show_config_path",
        conflicts_with = "explain_config"
    )]
    generate_man_page: bool,

    /// Print the effective config for a single target and exit.
    ///
    /// By default this emits YAML. Pass `toml` to print TOML instead.
    #[arg(
        long = "show-config",
        value_name = "FORMAT",
        help_heading = "Config And Conversion",
        num_args = 0..=1,
        default_missing_value = "yaml",
        require_equals = true,
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths",
        conflicts_with = "show_config_path",
        conflicts_with = "explain_config"
    )]
    show_config: Option<DumpConfigFormat>,

    /// Print the config file path selected for a single target and exit.
    #[arg(
        long = "show-config-path",
        visible_alias = "find-config-path",
        help_heading = "Config And Conversion",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths",
        conflicts_with = "show_config",
        conflicts_with = "explain_config"
    )]
    show_config_path: bool,

    /// Explain config resolution for a single target or the current directory and exit.
    #[arg(
        long = "explain-config",
        help_heading = "Config And Conversion",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths",
        conflicts_with = "show_config",
        conflicts_with = "show_config_path"
    )]
    explain_config: bool,

    /// Convert legacy cmake-format JSON, YAML, or Python config files.
    ///
    /// The converted config is printed to stdout as `.cmakefmt.toml`.
    #[arg(
        long = "convert-legacy-config",
        value_name = "PATH",
        help_heading = "Config And Conversion",
        conflicts_with = "dump_config",
        conflicts_with = "check",
        conflicts_with = "list_changed_files",
        conflicts_with = "list_input_files",
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

    /// Suppress per-file human output and emit only end-of-run summaries.
    ///
    /// This is intended for quieter `--check` and `--in-place` automation
    /// workflows. It does not suppress actual errors.
    #[arg(
        long,
        help_heading = "Execution",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    quiet: bool,

    /// Continue processing other files after a file-level parse or format error.
    ///
    /// Without this flag, human runs still fail at the first file error.
    #[arg(
        long = "keep-going",
        help_heading = "Execution",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    keep_going: bool,

    /// Print a unified diff instead of full formatted output.
    #[arg(
        long,
        help_heading = "Output Modes",
        conflicts_with = "in_place",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    diff: bool,

    /// Cache formatted results for repeated runs on the same files.
    #[arg(long, help_heading = "Execution")]
    cache: bool,

    /// Override the cache directory used by `--cache`.
    ///
    /// Supplying a cache location also enables caching.
    #[arg(
        long = "cache-location",
        value_name = "PATH",
        help_heading = "Execution"
    )]
    cache_location: Option<PathBuf>,

    /// Choose whether cache invalidation tracks file metadata or file contents.
    #[arg(
        long = "cache-strategy",
        value_enum,
        default_value_t = CacheStrategy::Metadata,
        help_heading = "Execution"
    )]
    cache_strategy: CacheStrategy,

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
        short = 'c',
        long = "config-file",
        visible_alias = "config",
        value_name = "PATH",
        help_heading = "Config Overrides"
    )]
    config_paths: Vec<PathBuf>,

    /// Disable config discovery and ignore explicit config files.
    #[arg(long, help_heading = "Config Overrides")]
    no_config: bool,

    /// Override the maximum line width.
    #[arg(short = 'l', long, help_heading = "Config Overrides")]
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

    /// Refuse to run unless the current cmakefmt version matches exactly.
    #[arg(long, value_name = "VERSION", help_heading = "Execution")]
    required_version: Option<String>,

    /// Verify that formatting preserves the parsed CMake semantics.
    ///
    /// In-place rewrites verify semantics by default; use this flag to enable
    /// the same safety check in stdout, diff, and check modes.
    #[arg(long, help_heading = "Execution", conflicts_with = "fast")]
    verify: bool,

    /// Skip semantic verification, even for in-place rewrites.
    #[arg(long, help_heading = "Execution", conflicts_with = "verify")]
    fast: bool,

    /// Format only files that opt in with a `# cmakefmt: enable` style pragma.
    #[arg(long, help_heading = "Execution")]
    require_pragma: bool,
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
    /// GitHub Actions workflow commands.
    Github,
    /// Checkstyle XML.
    Checkstyle,
    /// JUnit XML.
    Junit,
    /// SARIF JSON.
    Sarif,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum CacheStrategy {
    /// Use file size and modification time to detect cache invalidation.
    Metadata,
    /// Hash file contents to detect cache invalidation.
    Content,
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
    check_required_version(cli)?;

    if let Some(format) = cli.dump_config {
        print!("{}", default_config_template_for(format));
        return Ok(EXIT_OK);
    }

    if let Some(shell) = cli.generate_completion {
        let mut command = Cli::command();
        generate(shell, &mut command, "cmakefmt", &mut io::stdout());
        return Ok(EXIT_OK);
    }

    if cli.generate_man_page {
        let command = Cli::command();
        clap_mangen::Man::new(command)
            .render(&mut io::stdout())
            .map_err(cmakefmt::Error::Io)?;
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

    if cli.show_config.is_some() || cli.show_config_path || cli.explain_config {
        return run_config_introspection(cli);
    }

    let stdout_mode =
        !cli.list_changed_files && !cli.list_input_files && !cli.check && !cli.in_place;
    let colorize_stdout = stdout_mode && should_colorize_stdout(cli.colour);
    let file_filter = compile_file_filter(cli.file_regex.as_deref())?;
    let targets = collect_targets(cli, file_filter.as_ref())?;
    if cli.list_input_files {
        for target in &targets {
            println!("{}", target.display_name(cli.stdin_path.as_deref()));
        }
        return Ok(EXIT_OK);
    }
    if !cli.line_ranges.is_empty() && targets.len() != 1 {
        return Err(cmakefmt::Error::Formatter(
            "--lines requires exactly one formatting target".to_owned(),
        ));
    }
    let parallel_jobs = resolve_parallel_jobs(cli.parallel)?;
    let progress = ProgressReporter::new(cli.progress_bar, cli.in_place, targets.len());

    if cli.debug {
        log_debug(format!(
            "discovered {} target(s){}",
            targets.len(),
            debug_parallel_suffix(parallel_jobs)
        ));
    }

    let target_results = if parallel_jobs > 1 && targets.iter().all(InputTarget::is_path) {
        process_targets_parallel(&targets, cli, parallel_jobs, colorize_stdout, &progress)?
    } else {
        if cli.debug && parallel_jobs > 1 && targets.iter().any(|target| !target.is_path()) {
            log_debug("parallel mode ignored because stdin input must run serially");
        }
        process_targets_serial(&targets, cli, colorize_stdout, &progress)
    };
    let mut results = Vec::new();
    let mut failures = Vec::new();
    let mut summary = RunSummary {
        selected: targets.len(),
        ..RunSummary::default()
    };

    for target_result in target_results {
        match target_result {
            Ok(result) => {
                if result.skipped {
                    summary.skipped += 1;
                } else if result.would_change {
                    summary.changed += 1;
                } else {
                    summary.unchanged += 1;
                }
                results.push(result);
            }
            Err(err) => {
                if !cli.keep_going {
                    return Err(err);
                }
                summary.failed += 1;
                let rendered_error = render_cli_error(&err);
                let display_name = match &err {
                    cmakefmt::Error::ParseContext { display_name, .. } => display_name.clone(),
                    cmakefmt::Error::Config { path, .. } => path.display().to_string(),
                    cmakefmt::Error::Spec { path, .. } => path.display().to_string(),
                    cmakefmt::Error::Formatter(message) => message
                        .split(':')
                        .next()
                        .unwrap_or("<unknown>")
                        .trim()
                        .to_owned(),
                    cmakefmt::Error::Io(_) | cmakefmt::Error::Parse(_) => "<unknown>".to_owned(),
                };
                failures.push(FailedTarget {
                    display_name,
                    rendered_error,
                });
            }
        }
    }
    let multi_target_stdout = stdout_mode && results.len() > 1;

    if cli.report_format != ReportFormat::Human {
        if cli.in_place {
            write_in_place_updates(&results)?;
        }
        print_non_human_report(cli, &results, &failures, &summary)?;
        return machine_mode_exit_code(&results, &failures, &summary, cli);
    }

    for (index, result) in results.iter().enumerate() {
        if cli.debug {
            for line in &result.debug_lines {
                log_debug(line);
            }
        }

        if result.skipped {
            if stdout_mode && !cli.quiet {
                if multi_target_stdout && index > 0 {
                    io::stdout().write_all(b"\n").map_err(cmakefmt::Error::Io)?;
                }
                if multi_target_stdout {
                    write_stdout_header(&result.display_name, colorize_stdout)?;
                }
                io::stdout()
                    .write_all(result.formatted.as_bytes())
                    .map_err(cmakefmt::Error::Io)?;
            }
            continue;
        }

        if cli.list_changed_files {
            if result.would_change {
                println!("{}", result.display_name);
            }
        } else if cli.check {
            if result.would_change && !cli.quiet {
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
                    let diff_output = result.unified_diff.as_deref().unwrap_or_default();
                    let display_output = if colorize_stdout {
                        colorize_unified_diff(diff_output)
                    } else {
                        diff_output.to_owned()
                    };
                    io::stdout()
                        .write_all(display_output.as_bytes())
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

    if !failures.is_empty() {
        for failure in &failures {
            eprintln!("{}", failure.rendered_error);
        }
    }

    if should_print_human_summary(cli, &summary, &failures, results.len()) {
        eprintln!("{}", render_human_summary(&summary));
    }

    if !failures.is_empty() {
        Ok(EXIT_ERROR)
    } else if (cli.check || cli.list_changed_files) && summary.changed > 0 {
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
    if cli.no_config && !cli.config_paths.is_empty() {
        return Err(cmakefmt::Error::Formatter(
            "--no-config cannot be combined with --config-file".to_owned(),
        ));
    }

    if cli.verify && cli.fast {
        return Err(cmakefmt::Error::Formatter(
            "--verify cannot be combined with --fast".to_owned(),
        ));
    }

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

    if (cli.generate_completion.is_some() || cli.generate_man_page)
        && (!cli.files.is_empty()
            || !cli.files_from.is_empty()
            || !cli.config_paths.is_empty()
            || cli.no_config
            || cli.debug
            || cli.quiet
            || cli.keep_going
            || cli.diff
            || cli.check
            || cli.in_place
            || cli.list_changed_files
            || cli.list_input_files
            || cli.staged
            || cli.changed
            || cli.stdin_path.is_some()
            || !cli.line_ranges.is_empty())
    {
        return Err(cmakefmt::Error::Formatter(
            "completion/man-page generation cannot be combined with formatting or config-introspection inputs".to_owned(),
        ));
    }

    if cli.diff && cli.list_changed_files {
        return Err(cmakefmt::Error::Formatter(
            "--diff cannot be combined with --list-changed-files".to_owned(),
        ));
    }

    if cli.list_input_files && cli.report_format != ReportFormat::Human {
        return Err(cmakefmt::Error::Formatter(
            "--list-input-files only supports human output".to_owned(),
        ));
    }

    if cli.list_input_files && !cli.line_ranges.is_empty() {
        return Err(cmakefmt::Error::Formatter(
            "--list-input-files cannot be combined with --lines".to_owned(),
        ));
    }

    if is_config_introspection_mode(cli) {
        if cli.check || cli.list_changed_files || cli.list_input_files || cli.in_place || cli.diff {
            return Err(cmakefmt::Error::Formatter(
                "config introspection flags cannot be combined with formatting output modes"
                    .to_owned(),
            ));
        }
        if cli.report_format != ReportFormat::Human {
            return Err(cmakefmt::Error::Formatter(
                "config introspection flags only support human output".to_owned(),
            ));
        }
        if cli.staged || cli.changed || !cli.files_from.is_empty() {
            return Err(cmakefmt::Error::Formatter(
                "config introspection expects a single explicit path or stdin context".to_owned(),
            ));
        }
    }

    Ok(())
}

fn is_config_introspection_mode(cli: &Cli) -> bool {
    cli.show_config.is_some() || cli.show_config_path || cli.explain_config
}

fn run_config_introspection(cli: &Cli) -> Result<u8, cmakefmt::Error> {
    if cli.explain_config {
        let target = resolve_config_probe_target(cli)?;
        return explain_config(cli, target.as_deref().unwrap_or(Path::new(".")));
    }

    let target = resolve_config_probe_target(cli)?;
    let target_path = target.as_deref();
    let config_context = resolve_config_context(cli, target_path);

    if cli.show_config_path {
        for path in &config_context.sources {
            println!("{}", path.display());
        }
        return Ok(EXIT_OK);
    }

    if let Some(format) = cli.show_config {
        let (config, _, _) = build_context(cli, target_path)?;
        let rendered = render_effective_config(&config, format)?;
        print!("{rendered}");
        if !rendered.ends_with('\n') {
            println!();
        }
        return Ok(EXIT_OK);
    }

    Ok(EXIT_OK)
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

fn resolve_config_probe_target(cli: &Cli) -> Result<Option<PathBuf>, cmakefmt::Error> {
    if cli.files.is_empty() {
        return Ok(Some(PathBuf::from(".")));
    }

    if cli.files.len() != 1 {
        return Err(cmakefmt::Error::Formatter(
            "config introspection expects exactly one explicit path".to_owned(),
        ));
    }

    if cli.files[0] == "-" {
        return cli.stdin_path.clone().map(Some).ok_or_else(|| {
            cmakefmt::Error::Formatter(
                "stdin config introspection requires --stdin-path".to_owned(),
            )
        });
    }

    Ok(Some(PathBuf::from(&cli.files[0])))
}

fn resolve_config_context(cli: &Cli, file_path: Option<&Path>) -> ConfigContext {
    if cli.no_config {
        return ConfigContext {
            mode: ConfigSourceMode::Disabled,
            sources: Vec::new(),
        };
    }

    if !cli.config_paths.is_empty() {
        return ConfigContext {
            mode: ConfigSourceMode::Explicit,
            sources: cli.config_paths.clone(),
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

fn describe_config_mode(mode: ConfigSourceMode) -> &'static str {
    match mode {
        ConfigSourceMode::Disabled => "disabled by --no-config",
        ConfigSourceMode::Explicit => "explicit --config-file override(s)",
        ConfigSourceMode::Discovered => "discovered from the target path",
        ConfigSourceMode::DefaultsOnly => "defaults only",
    }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConfigSourceMode {
    Disabled,
    Explicit,
    Discovered,
    DefaultsOnly,
}

#[derive(Clone, Debug)]
struct ConfigContext {
    mode: ConfigSourceMode,
    sources: Vec<PathBuf>,
}

/// Build a formatting context by layering: defaults → config files → CLI
/// overrides, and by merging any `[commands]` spec overrides from the same
/// config files into the command registry.
fn build_context(
    cli: &Cli,
    file_path: Option<&Path>,
) -> Result<(Config, CommandRegistry, ConfigContext), cmakefmt::Error> {
    let config_context = resolve_config_context(cli, file_path);

    let mut config = Config::from_files(&config_context.sources)?;
    let mut registry = CommandRegistry::builtins().clone();
    for path in &config_context.sources {
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

    Ok((config, registry, config_context))
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

    fn display_name(&self, stdin_path: Option<&Path>) -> String {
        match self {
            Self::Stdin => stdin_path
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<stdin>".to_owned()),
            Self::Path(path) => path.display().to_string(),
        }
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
    skipped: bool,
    skip_reason: Option<String>,
    debug_lines: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VerificationMode {
    Disabled,
    Enabled,
}

struct FailedTarget {
    display_name: String,
    rendered_error: String,
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

#[derive(Debug, Serialize)]
struct JsonReport {
    mode: &'static str,
    summary: RunSummary,
    files: Vec<JsonFileReport>,
    errors: Vec<JsonErrorReport>,
}

#[derive(Debug, Serialize)]
struct JsonErrorReport {
    display_name: String,
    error: String,
}

#[derive(Debug, Serialize)]
struct JsonFileReport {
    display_name: String,
    path: Option<String>,
    would_change: bool,
    skipped: bool,
    skip_reason: Option<String>,
    changed_lines: Vec<usize>,
    formatted: Option<String>,
    diff: Option<String>,
    debug_lines: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
struct RunSummary {
    selected: usize,
    changed: usize,
    unchanged: usize,
    skipped: usize,
    failed: usize,
}

fn process_targets_serial(
    targets: &[InputTarget],
    cli: &Cli,
    colorize_stdout: bool,
    progress: &ProgressReporter,
) -> Vec<Result<ProcessedTarget, cmakefmt::Error>> {
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
) -> Result<Vec<Result<ProcessedTarget, cmakefmt::Error>>, cmakefmt::Error> {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(parallel_jobs)
        .build()
        .map_err(|err| cmakefmt::Error::Formatter(format!("failed to build thread pool: {err}")))?;

    Ok(pool.install(|| {
        targets
            .par_iter()
            .map(|target| process_target(target, cli, colorize_stdout, progress))
            .collect::<Vec<Result<ProcessedTarget, cmakefmt::Error>>>()
    }))
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
    let display_name = stdin_path
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<stdin>".to_owned());
    if cli.require_pragma && !has_enable_pragma(&source) {
        return Ok(skipped_target(
            stdin_path.map(Path::to_path_buf),
            display_name,
            source,
            "missing format opt-in pragma".to_owned(),
            cli.debug,
        ));
    }
    let (config, registry, config_context) = build_context(cli, stdin_path)?;
    let mut debug_lines = if cli.debug {
        vec![
            format!("processing {display_name}"),
            describe_config_context(&config_context),
        ]
    } else {
        Vec::new()
    };
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
    if cli.debug {
        debug_lines.append(&mut formatter_debug);
    }

    if verification_mode(cli) == VerificationMode::Enabled {
        verify_semantics(&source, &formatted, &registry, &display_name)?;
        if cli.debug {
            debug_lines.push(format!(
                "result {display_name}: semantic verification passed"
            ));
        }
    }

    let formatted = apply_line_ranges(&source, &formatted, &cli.line_ranges, &display_name)?;

    let would_change = formatted != source;
    let changed_lines = if needs_changed_lines(cli, colorize_stdout) {
        changed_formatted_line_numbers(
            &split_lines_with_endings(&source),
            &split_lines_with_endings(&formatted),
        )
    } else {
        Vec::new()
    };
    if cli.debug {
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
    let unified_diff = would_change.then(|| build_unified_diff(&display_name, &source, &formatted));

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
    })
}

fn process_path(
    path: &Path,
    cli: &Cli,
    colorize_stdout: bool,
) -> Result<ProcessedTarget, cmakefmt::Error> {
    let source = std::fs::read_to_string(path)
        .map_err(|err| cmakefmt::Error::Formatter(format!("{}: {err}", path.display())))?;
    if cli.require_pragma && !has_enable_pragma(&source) {
        return Ok(skipped_target(
            Some(path.to_path_buf()),
            path.display().to_string(),
            source,
            "missing format opt-in pragma".to_owned(),
            cli.debug,
        ));
    }
    let (config, registry, config_context) = build_context(cli, Some(path))?;
    let mut debug_lines = if cli.debug {
        vec![
            format!("processing {}", path.display()),
            describe_config_context(&config_context),
            describe_cli_overrides(cli),
        ]
    } else {
        Vec::new()
    };
    let cache_context = if cli.cache || cli.cache_location.is_some() {
        Some(cache_context(
            path,
            &source,
            &config,
            &config_context,
            cli.cache_location.as_deref(),
            cli.cache_strategy,
        )?)
    } else {
        None
    };

    let mut cache_hit = false;
    let (formatted, mut formatter_debug) = if let Some(cache) = &cache_context {
        if let Some(cached) = read_cache_entry(cache)? {
            cache_hit = true;
            if cli.debug {
                debug_lines.push(format!(
                    "cache hit {} ({})",
                    path.display(),
                    cache.cache_file.display()
                ));
            }
            (cached.formatted, Vec::new())
        } else {
            if cli.debug {
                debug_lines.push(format!(
                    "cache miss {} ({})",
                    path.display(),
                    cache.cache_file.display()
                ));
            }
            if cli.debug {
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
    } else if cli.debug {
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
    if cli.debug {
        debug_lines.append(&mut formatter_debug);
    }

    if verification_mode(cli) == VerificationMode::Enabled {
        verify_semantics(&source, &formatted, &registry, &path.display().to_string())?;
        if cli.debug {
            debug_lines.push(format!(
                "result {}: semantic verification passed",
                path.display()
            ));
        }
    }

    let formatted = apply_line_ranges(
        &source,
        &formatted,
        &cli.line_ranges,
        &path.display().to_string(),
    )?;
    let would_change = formatted != source;
    let changed_lines = if needs_changed_lines(cli, colorize_stdout) {
        changed_formatted_line_numbers(
            &split_lines_with_endings(&source),
            &split_lines_with_endings(&formatted),
        )
    } else {
        Vec::new()
    };
    if cli.debug {
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
    let unified_diff =
        would_change.then(|| build_unified_diff(&path.display().to_string(), &source, &formatted));

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
    })
}

fn needs_changed_lines(cli: &Cli, colorize_stdout: bool) -> bool {
    colorize_stdout
        || !cli.line_ranges.is_empty()
        || cli.debug
        || cli.report_format != ReportFormat::Human
}

fn verification_mode(cli: &Cli) -> VerificationMode {
    if cli.verify || (cli.in_place && !cli.fast) {
        VerificationMode::Enabled
    } else {
        VerificationMode::Disabled
    }
}

fn has_enable_pragma(source: &str) -> bool {
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

    ProcessedTarget {
        path,
        display_name,
        formatted: source,
        highlighted_output: None,
        unified_diff: None,
        changed_lines: Vec::new(),
        would_change: false,
        skipped: true,
        skip_reason: Some(reason),
        debug_lines,
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
            let metadata = std::fs::metadata(path).map_err(cmakefmt::Error::Io)?;
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
        Err(err) => return Err(cmakefmt::Error::Io(err)),
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
        std::fs::create_dir_all(parent).map_err(cmakefmt::Error::Io)?;
    }
    let json = serde_json::to_string(&entry).map_err(|err| {
        cmakefmt::Error::Formatter(format!("failed to serialize cache entry: {err}"))
    })?;
    std::fs::write(&cache.cache_file, json).map_err(cmakefmt::Error::Io)
}

fn stable_hash<T: Hash + ?Sized>(value: &T) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn check_required_version(cli: &Cli) -> Result<(), cmakefmt::Error> {
    let Some(required) = &cli.required_version else {
        return Ok(());
    };

    let required = required.trim().trim_start_matches('v');
    let current = env!("CARGO_PKG_VERSION");
    if required == current {
        Ok(())
    } else {
        Err(cmakefmt::Error::Formatter(format!(
            "required cmakefmt version {required} does not match current version {current}"
        )))
    }
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
                    "{spinner:.green} [Elapsed: {elapsed_precise}] |{bar:50.green/green}| ({eta_precise}) {pos}/{len} ({percent}%) files",
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
    push_ansi_line(output, line, "\u{1b}[36m");
}

fn push_red_line(output: &mut String, line: &str) {
    push_ansi_line(output, line, "\u{1b}[31m");
}

fn push_green_line(output: &mut String, line: &str) {
    push_ansi_line(output, line, "\u{1b}[32m");
}

fn push_ansi_line(output: &mut String, line: &str, colour: &str) {
    const RESET: &str = "\u{1b}[0m";

    if let Some(stripped) = line.strip_suffix('\n') {
        output.push_str(colour);
        output.push_str(stripped);
        output.push_str(RESET);
        output.push('\n');
    } else {
        output.push_str(colour);
        output.push_str(line);
        output.push_str(RESET);
    }
}

fn colorize_unified_diff(diff: &str) -> String {
    let mut output = String::with_capacity(diff.len() + 256);
    for line in split_lines_with_endings(diff) {
        if line.starts_with('+') && !line.starts_with("+++") {
            push_green_line(&mut output, line);
        } else if line.starts_with('-') && !line.starts_with("---") {
            push_red_line(&mut output, line);
        } else {
            output.push_str(line);
        }
    }
    output
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

fn build_json_report(
    results: &[ProcessedTarget],
    failures: &[FailedTarget],
    summary: &RunSummary,
    cli: &Cli,
) -> JsonReport {
    let mode = if cli.in_place {
        "in-place"
    } else if cli.check {
        "check"
    } else if cli.list_changed_files {
        "list-changed-files"
    } else if cli.list_input_files {
        "list-input-files"
    } else if cli.diff {
        "diff"
    } else {
        "stdout"
    };

    JsonReport {
        mode,
        summary: RunSummary {
            selected: summary.selected,
            changed: summary.changed,
            unchanged: summary.unchanged,
            skipped: summary.skipped,
            failed: summary.failed,
        },
        files: results
            .iter()
            .map(|result| JsonFileReport {
                display_name: result.display_name.clone(),
                path: result.path.as_ref().map(|path| path.display().to_string()),
                would_change: result.would_change,
                skipped: result.skipped,
                skip_reason: result.skip_reason.clone(),
                changed_lines: result.changed_lines.clone(),
                formatted: (!cli.in_place
                    && !cli.check
                    && !cli.list_changed_files
                    && !cli.list_input_files
                    && !cli.diff)
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
        errors: failures
            .iter()
            .map(|failure| JsonErrorReport {
                display_name: failure.display_name.clone(),
                error: failure.rendered_error.clone(),
            })
            .collect(),
    }
}

fn build_github_report(
    results: &[ProcessedTarget],
    failures: &[FailedTarget],
    summary: &RunSummary,
) -> String {
    let mut out = String::new();

    for result in results {
        if !result.would_change {
            continue;
        }

        let line = result.changed_lines.first().copied().unwrap_or(1);
        let file = github_escape_property(
            result
                .path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| result.display_name.clone())
                .as_str(),
        );
        let message = github_escape_message("file would be reformatted by cmakefmt");
        let _ = writeln!(out, "::warning file={file},line={line}::{message}");
    }

    for failure in failures {
        let file = github_escape_property(&failure.display_name);
        let message = github_escape_message(&failure.rendered_error);
        let _ = writeln!(out, "::error file={file}::{message}");
    }

    let summary_line = github_escape_message(&render_human_summary(summary));
    let _ = writeln!(out, "::notice::{summary_line}");
    out
}

fn build_checkstyle_report(results: &[ProcessedTarget], failures: &[FailedTarget]) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    out.push_str("<checkstyle version=\"4.3\">\n");

    for result in results {
        if !result.would_change {
            continue;
        }
        let path = xml_escape(
            result
                .path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| result.display_name.clone())
                .as_str(),
        );
        let line = result.changed_lines.first().copied().unwrap_or(1);
        out.push_str(&format!("  <file name=\"{path}\">\n"));
        out.push_str(&format!(
            "    <error line=\"{line}\" severity=\"warning\" source=\"cmakefmt.format\" message=\"{}\"/>\n",
            xml_escape("file would be reformatted by cmakefmt")
        ));
        out.push_str("  </file>\n");
    }

    for failure in failures {
        let path = xml_escape(&failure.display_name);
        out.push_str(&format!("  <file name=\"{path}\">\n"));
        out.push_str(&format!(
            "    <error severity=\"error\" source=\"cmakefmt.error\" message=\"{}\"/>\n",
            xml_escape(&failure.rendered_error)
        ));
        out.push_str("  </file>\n");
    }

    out.push_str("</checkstyle>\n");
    out
}

fn build_junit_report(
    results: &[ProcessedTarget],
    failures: &[FailedTarget],
    summary: &RunSummary,
) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    out.push_str(&format!(
        "<testsuite name=\"cmakefmt\" tests=\"{}\" failures=\"{}\" errors=\"{}\">\n",
        summary.selected, summary.changed, summary.failed
    ));

    for result in results {
        out.push_str(&format!(
            "  <testcase classname=\"cmakefmt\" name=\"{}\">",
            xml_escape(&result.display_name)
        ));
        if result.would_change {
            out.push_str(&format!(
                "<failure message=\"{}\">{}</failure>",
                xml_escape("file would be reformatted by cmakefmt"),
                xml_escape(
                    result
                        .unified_diff
                        .as_deref()
                        .unwrap_or("file would be reformatted by cmakefmt")
                )
            ));
        }
        out.push_str("</testcase>\n");
    }

    for failure in failures {
        out.push_str(&format!(
            "  <testcase classname=\"cmakefmt\" name=\"{}\"><error message=\"{}\">{}</error></testcase>\n",
            xml_escape(&failure.display_name),
            xml_escape("cmakefmt failed to process the file"),
            xml_escape(&failure.rendered_error)
        ));
    }

    out.push_str("</testsuite>\n");
    out
}

fn build_sarif_report(results: &[ProcessedTarget], failures: &[FailedTarget]) -> serde_json::Value {
    let mut sarif_results = Vec::new();

    for result in results {
        if !result.would_change {
            continue;
        }

        let uri = result
            .path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| result.display_name.clone());
        sarif_results.push(serde_json::json!({
            "ruleId": "cmakefmt/would-reformat",
            "level": "warning",
            "message": { "text": "file would be reformatted by cmakefmt" },
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": { "uri": uri },
                    "region": { "startLine": result.changed_lines.first().copied().unwrap_or(1) }
                }
            }]
        }));
    }

    for failure in failures {
        sarif_results.push(serde_json::json!({
            "ruleId": "cmakefmt/error",
            "level": "error",
            "message": { "text": failure.rendered_error },
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": { "uri": failure.display_name }
                }
            }]
        }));
    }

    serde_json::json!({
        "version": "2.1.0",
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "cmakefmt",
                    "informationUri": "https://github.com/puneetmatharu/cmakefmt",
                    "rules": [
                        {
                            "id": "cmakefmt/would-reformat",
                            "shortDescription": { "text": "file would be reformatted" }
                        },
                        {
                            "id": "cmakefmt/error",
                            "shortDescription": { "text": "cmakefmt failed to process the file" }
                        }
                    ]
                }
            },
            "results": sarif_results
        }]
    })
}

fn github_escape_property(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

fn github_escape_message(value: &str) -> String {
    github_escape_property(value)
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\'', "&apos;")
}

fn machine_mode_exit_code(
    results: &[ProcessedTarget],
    failures: &[FailedTarget],
    _summary: &RunSummary,
    cli: &Cli,
) -> Result<u8, cmakefmt::Error> {
    if !failures.is_empty() {
        Ok(EXIT_ERROR)
    } else if (cli.check || cli.list_changed_files) && results.iter().any(|r| r.would_change) {
        Ok(EXIT_CHECK_FAILED)
    } else {
        Ok(EXIT_OK)
    }
}

fn write_in_place_updates(results: &[ProcessedTarget]) -> Result<(), cmakefmt::Error> {
    for result in results {
        if let Some(path) = &result.path {
            if result.would_change {
                std::fs::write(path, &result.formatted).map_err(cmakefmt::Error::Io)?;
            }
        }
    }
    Ok(())
}

fn print_non_human_report(
    cli: &Cli,
    results: &[ProcessedTarget],
    failures: &[FailedTarget],
    summary: &RunSummary,
) -> Result<(), cmakefmt::Error> {
    match cli.report_format {
        ReportFormat::Human => Ok(()),
        ReportFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&build_json_report(results, failures, summary, cli))
                    .map_err(|err| {
                    cmakefmt::Error::Formatter(format!("failed to build JSON report: {err}"))
                })?
            );
            Ok(())
        }
        ReportFormat::Github => {
            print!("{}", build_github_report(results, failures, summary));
            Ok(())
        }
        ReportFormat::Checkstyle => {
            print!("{}", build_checkstyle_report(results, failures));
            Ok(())
        }
        ReportFormat::Junit => {
            print!("{}", build_junit_report(results, failures, summary));
            Ok(())
        }
        ReportFormat::Sarif => {
            println!(
                "{}",
                serde_json::to_string_pretty(&build_sarif_report(results, failures)).map_err(
                    |err| cmakefmt::Error::Formatter(format!(
                        "failed to build SARIF report: {err}"
                    ))
                )?
            );
            Ok(())
        }
    }
}

fn should_print_human_summary(
    cli: &Cli,
    summary: &RunSummary,
    failures: &[FailedTarget],
    successful_results: usize,
) -> bool {
    if cli.report_format != ReportFormat::Human {
        return false;
    }

    let stdout_mode = !cli.list_changed_files
        && !cli.list_input_files
        && !cli.check
        && !cli.in_place
        && !cli.diff;
    if stdout_mode {
        return cli.quiet || !failures.is_empty();
    }

    cli.quiet
        || !failures.is_empty()
        || cli.check
        || cli.in_place
        || (cli.diff && successful_results > 1)
        || summary.selected > 1
}

fn render_human_summary(summary: &RunSummary) -> String {
    let mut rendered = format!(
        "summary: selected={}, changed={}, unchanged={}",
        summary.selected, summary.changed, summary.unchanged
    );
    if summary.skipped > 0 {
        let _ = write!(rendered, ", skipped={}", summary.skipped);
    }
    let _ = write!(rendered, ", failed={}", summary.failed);
    rendered
}

fn describe_config_context(config_context: &ConfigContext) -> String {
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

fn normalize_semantics(
    mut file: parser::ast::File,
    registry: &CommandRegistry,
) -> parser::ast::File {
    for statement in &mut file.statements {
        match statement {
            parser::ast::Statement::Command(command) => {
                command.span = (0, 0);
                command.name.make_ascii_lowercase();
                normalize_command_literals(command);
                normalize_keyword_args(command, registry);
            }
            parser::ast::Statement::TemplatePlaceholder(value) => normalize_line_endings(value),
            parser::ast::Statement::Comment(comment) => normalize_comment(comment),
            parser::ast::Statement::BlankLines(_) => {}
        }
    }

    file
}

fn normalize_command_literals(command: &mut parser::ast::CommandInvocation) {
    if let Some(comment) = &mut command.trailing_comment {
        normalize_comment(comment);
    }

    for argument in &mut command.arguments {
        match argument {
            parser::ast::Argument::Bracket(bracket) => normalize_line_endings(&mut bracket.raw),
            parser::ast::Argument::Quoted(value) | parser::ast::Argument::Unquoted(value) => {
                normalize_line_endings(value)
            }
            parser::ast::Argument::InlineComment(comment) => normalize_comment(comment),
        }
    }
}

fn normalize_comment(comment: &mut parser::ast::Comment) {
    match comment {
        parser::ast::Comment::Line(value) | parser::ast::Comment::Bracket(value) => {
            normalize_line_endings(value)
        }
    }
}

fn normalize_line_endings(value: &mut String) {
    if value.contains('\r') {
        *value = value.replace("\r\n", "\n");
    }
}

fn normalize_keyword_args(
    command: &mut parser::ast::CommandInvocation,
    registry: &CommandRegistry,
) {
    let spec = registry.get(&command.name);
    let first_arg = command.arguments.iter().find_map(first_arg_text);
    let form = spec.form_for(first_arg);
    let keyword_set = collect_keywords(form);

    for arg in &mut command.arguments {
        if let parser::ast::Argument::Unquoted(value) = arg {
            let upper = value.to_ascii_uppercase();
            if keyword_set.contains(upper.as_str()) {
                *value = upper;
            }
        }
    }
}

fn first_arg_text(argument: &parser::ast::Argument) -> Option<&str> {
    match argument {
        parser::ast::Argument::Quoted(_)
        | parser::ast::Argument::Bracket(_)
        | parser::ast::Argument::InlineComment(_) => None,
        parser::ast::Argument::Unquoted(value) => Some(value.as_str()),
    }
}

fn collect_keywords(form: &cmakefmt::spec::CommandForm) -> BTreeSet<String> {
    let mut keywords = BTreeSet::new();
    collect_form_keywords(form, &mut keywords);
    keywords
}

fn collect_form_keywords(form: &cmakefmt::spec::CommandForm, keywords: &mut BTreeSet<String>) {
    keywords.extend(form.flags.iter().cloned());

    for (name, spec) in &form.kwargs {
        keywords.insert(name.clone());
        collect_kwarg_keywords(spec, keywords);
    }
}

fn collect_kwarg_keywords(spec: &cmakefmt::spec::KwargSpec, keywords: &mut BTreeSet<String>) {
    keywords.extend(spec.flags.iter().cloned());

    for (name, child) in &spec.kwargs {
        keywords.insert(name.clone());
        collect_kwarg_keywords(child, keywords);
    }
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
            "generate-completion",
            "generate-man-page",
            "help",
            "dump-config",
            "explain-config",
            "ignore-path",
            "keep-going",
            "cache",
            "cache-location",
            "cache-strategy",
            "lines",
            "list-changed-files",
            "list-input-files",
            "no-config",
            "no-gitignore",
            "parallel",
            "progress-bar",
            "quiet",
            "report-format",
            "show-config",
            "show-config-path",
            "required-version",
            "verify",
            "fast",
            "require-pragma",
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
