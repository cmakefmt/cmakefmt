// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

mod cli;

use cli::commands::{
    install_git_hook, render_man_page, run_config_subcommand, run_dump_subcommand,
    run_list_unknown_commands, run_watch,
};
use cli::errors::render_cli_error;
use cli::process::{collect_targets, compile_file_filter, process_targets, ProgressReporter};
use cli::report::{machine_mode_exit_code, print_non_human_report};
use cli::runtime::{
    check_required_version, debug_parallel_suffix, handle_completed_target, is_stdout_mode,
    log_debug, progress_bar_suppressed_reason, resolve_parallel_jobs, should_enable_progress_bar,
    should_print_human_summary, validate_cli, write_diff_to_stdout, write_in_place_updates,
    HumanOutputState, RunState,
};
use cli::summary::{render_human_summary, render_stat_summary};

use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
use cmakefmt::{CaseStyle, DumpConfigFormat};
use serde::Serialize;

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

Use `cmakefmt config init` to generate a starter .cmakefmt.yaml, or
`cmakefmt config dump` to print the full default template.

Legacy cmake-format config files can be converted with
`cmakefmt config convert <path>`.

Use `cmakefmt config path` to inspect which config file was selected,
`cmakefmt config show` for the effective config, and `cmakefmt config explain`
for a human-readable explanation of config resolution.";

fn cli_styles() -> clap::builder::Styles {
    use clap::builder::styling::{AnsiColor, Effects, Style};
    clap::builder::Styles::styled()
        .header(
            Style::new()
                .fg_color(Some(AnsiColor::Green.into()))
                .effects(Effects::BOLD),
        )
        .usage(
            Style::new()
                .fg_color(Some(AnsiColor::Green.into()))
                .effects(Effects::BOLD),
        )
        .literal(Style::new().fg_color(Some(AnsiColor::Cyan.into())))
        .placeholder(Style::new().fg_color(Some(AnsiColor::Cyan.into())))
        .valid(Style::new().fg_color(Some(AnsiColor::Green.into())))
        .invalid(
            Style::new()
                .fg_color(Some(AnsiColor::Red.into()))
                .effects(Effects::BOLD),
        )
        .error(
            Style::new()
                .fg_color(Some(AnsiColor::Red.into()))
                .effects(Effects::BOLD),
        )
}

/// A fast, correct CMake formatter.
#[derive(Parser, Debug)]
#[command(
    name = "cmakefmt",
    version,
    long_version = env!("CMAKEFMT_CLI_LONG_VERSION"),
    about = "Parse CMake listfiles and format them nicely.",
    long_about = LONG_ABOUT,
    styles = cli_styles(),
)]
struct Cli {
    #[command(flatten)]
    input_selection: InputSelectionArgs,

    #[command(flatten)]
    output_modes: OutputModesArgs,

    #[command(flatten)]
    execution: ExecutionArgs,

    #[command(flatten)]
    config_overrides: ConfigOverridesArgs,

    /// Subcommand (e.g. `cmakefmt config dump`).
    #[command(subcommand)]
    command: Option<CliCommand>,
}

#[derive(Args, Debug, Clone)]
struct InputSelectionArgs {
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
        help_heading = "Input Selection"
    )]
    files_from: Vec<String>,

    /// Filter recursively discovered CMake paths with a regex.
    ///
    /// This only affects discovery from directories or Git/file-list driven
    /// inputs. Direct file arguments are always kept.
    #[arg(
        long = "path-regex",
        value_name = "REGEX",
        help_heading = "Input Selection"
    )]
    file_regex: Option<String>,

    /// Add one or more extra ignore files during recursive discovery.
    ///
    /// This only affects discovered files, not direct file arguments.
    #[arg(
        long = "ignore-path",
        value_name = "PATH",
        help_heading = "Input Selection"
    )]
    ignore_paths: Vec<PathBuf>,

    /// Ignore `.gitignore` files during recursive discovery.
    ///
    /// By default, cmakefmt honours `.gitignore` rules when discovering
    /// files from directories.
    #[arg(long = "no-gitignore", help_heading = "Input Selection")]
    no_gitignore: bool,

    /// Sort discovered files by path before processing.
    ///
    /// Guarantees alphabetical output order regardless of filesystem
    /// discovery order. Direct file arguments are sorted too.
    #[arg(long, help_heading = "Input Selection")]
    sorted: bool,

    /// Select modified Git-tracked files instead of explicit input paths.
    ///
    /// Use `--since` to compare against a specific base ref; otherwise
    /// `cmakefmt` compares the working tree against `HEAD`.
    #[arg(long, help_heading = "Input Selection", conflicts_with = "staged")]
    changed: bool,

    /// Select staged Git-tracked files instead of explicit input paths.
    ///
    /// Useful for pre-commit hooks that should only check files in the
    /// current changeset.
    #[arg(long, help_heading = "Input Selection", conflicts_with = "changed")]
    staged: bool,

    /// Git base ref used together with `--changed`.
    ///
    /// Without this flag, `--changed` compares against `HEAD`.
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
}

#[derive(Args, Debug, Clone)]
struct OutputModesArgs {
    /// Rewrite files on disk instead of printing formatted output.
    ///
    /// Semantic verification is enabled by default for in-place rewrites.
    /// Use `--no-verify` to skip it.
    #[arg(
        short = 'i',
        long = "in-place",
        help_heading = "Output Modes",
        conflicts_with = "list_changed_files",
        conflicts_with = "list_input_files"
    )]
    in_place: bool,

    /// Exit with code 1 if any selected file would change.
    ///
    /// No files are modified on disk.
    #[arg(
        long,
        help_heading = "Output Modes",
        conflicts_with = "list_input_files"
    )]
    check: bool,

    /// Print only the files that would change, without modifying them.
    #[arg(
        long = "list-changed-files",
        alias = "list-files",
        help_heading = "Output Modes",
        conflicts_with = "quiet",
        conflicts_with = "list_input_files"
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
        conflicts_with = "quiet"
    )]
    list_input_files: bool,

    /// List commands that don't match any built-in or user-defined spec.
    ///
    /// Parses the selected files and prints each unrecognized command name
    /// with its file and line number. Useful for discovering project-specific
    /// commands that should be added to the `commands:` config section.
    #[arg(
        long = "list-unknown-commands",
        help_heading = "Output Modes",
        conflicts_with = "check",
        conflicts_with = "in_place",
        conflicts_with = "diff",
        conflicts_with = "list_changed_files",
        conflicts_with = "list_input_files",
        conflicts_with = "explain",
        conflicts_with = "watch",
        conflicts_with = "quiet",
        conflicts_with = "progress_bar"
    )]
    list_unknown_commands: bool,

    /// Show a per-file status summary instead of formatted output.
    ///
    /// Prints a status line for each file to stderr with change details,
    /// line counts, and elapsed time. In stdout mode (no `--check`,
    /// `--in-place`, or `--diff`), formatted output is suppressed.
    #[arg(short, long, help_heading = "Output Modes", conflicts_with = "quiet")]
    summary: bool,

    /// Print a unified diff instead of the full formatted output.
    #[arg(
        short,
        long,
        help_heading = "Output Modes",
        conflicts_with = "in_place"
    )]
    diff: bool,

    /// Show why each command was formatted the way it was.
    ///
    /// Prints a per-command explanation of the layout decision (inline,
    /// hanging, or vertical) and the config values that influenced it.
    /// Requires exactly one formatting target.
    #[arg(
        long,
        help_heading = "Output Modes",
        conflicts_with = "check",
        conflicts_with = "in_place",
        conflicts_with = "diff",
        conflicts_with = "list_changed_files",
        conflicts_with = "list_input_files",
        conflicts_with = "quiet",
        conflicts_with = "progress_bar"
    )]
    explain: bool,

    /// Choose the output report format.
    #[arg(
        long = "report-format",
        value_enum,
        default_value_t = ReportFormat::Human,
        help_heading = "Output Modes"
    )]
    report_format: ReportFormat,

    /// Control ANSI color output.
    #[arg(
        long = "color",
        alias = "colour",
        value_enum,
        default_value_t = ColorChoice::Auto,
        help_heading = "Output Modes"
    )]
    color: ColorChoice,
}

#[derive(Args, Debug, Clone)]
struct ExecutionArgs {
    /// Deprecated. Use `cmakefmt manpage` instead. Hidden to keep
    /// help output focused on the canonical subcommand form; the flag
    /// remains accepted so existing release scripts (e.g.
    /// `cmakefmt --generate-man-page > cmakefmt.1`) keep working.
    #[arg(long = "generate-man-page", hide = true)]
    generate_man_page: bool,

    /// Print detailed discovery, config, and formatter diagnostics to stderr.
    #[arg(long, help_heading = "Execution")]
    debug: bool,

    /// Suppress per-file output and emit only end-of-run summaries.
    ///
    /// In stdout mode, formatted output is suppressed. In `--check` mode,
    /// "would be reformatted" lines are suppressed. Errors and the summary
    /// line are always printed.
    #[arg(short, long, help_heading = "Execution")]
    quiet: bool,

    /// Print a git-style summary after formatting (e.g. "3 files changed, 12 lines reformatted").
    ///
    /// This works with all output modes (`--check`, `--diff`, `--in-place`, and
    /// stdout). When combined with `--quiet`, the stat line is still printed.
    #[arg(long, help_heading = "Execution")]
    stat: bool,

    /// Continue processing other files after a file-level parse or format error.
    ///
    /// Without this flag, human runs still fail at the first file error.
    #[arg(long = "keep-going", help_heading = "Execution")]
    keep_going: bool,

    /// Watch for file changes and reformat automatically.
    ///
    /// Watches the specified files or directories for changes and reformats
    /// them in-place. Press Ctrl+C to stop.
    #[arg(
        long,
        help_heading = "Execution",
        conflicts_with = "check",
        conflicts_with = "diff",
        conflicts_with = "list_changed_files",
        conflicts_with = "list_input_files",
        conflicts_with = "quiet",
        conflicts_with = "explain",
        conflicts_with = "progress_bar"
    )]
    watch: bool,

    /// Cache formatted results for repeated runs on the same files.
    ///
    /// Speeds up large-repo checks by skipping files that haven't changed.
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

    /// Set the number of parallel formatting jobs.
    ///
    /// Defaults to the available CPU count minus one (minimum 1). Pass an
    /// explicit value to override, or `--parallel 1` to force serial.
    #[arg(
        short = 'j',
        long,
        value_name = "JOBS",
        help_heading = "Execution",
        num_args = 0..=1,
        default_missing_value = "0",
    )]
    parallel: Option<usize>,

    /// Show a progress bar on stderr while processing files.
    ///
    /// The progress bar is intended for directory or multi-file runs.
    #[arg(short, long = "progress-bar", help_heading = "Execution")]
    progress_bar: bool,

    /// Refuse to run unless the current cmakefmt version matches exactly.
    ///
    /// Useful for pinned CI and editor wrappers that need a specific version.
    #[arg(long, value_name = "VERSION", help_heading = "Execution")]
    required_version: Option<String>,

    /// Verify that formatting preserves the parsed CMake semantics.
    ///
    /// In-place rewrites verify semantics by default; use this flag to enable
    /// the same safety check in stdout, diff, and check modes.
    #[arg(long, help_heading = "Execution", conflicts_with = "no_verify")]
    verify: bool,

    /// Skip semantic verification, even for in-place rewrites.
    ///
    /// Improves throughput on trusted inputs at the cost of safety.
    /// `--fast` is a deprecated hidden alias retained for
    /// backwards compatibility; new usage should write `--no-verify`.
    #[arg(
        long = "no-verify",
        alias = "fast",
        help_heading = "Execution",
        conflicts_with = "verify"
    )]
    no_verify: bool,

    /// Format only files that opt in with a `# cmakefmt: enable` style pragma.
    ///
    /// Useful for gradually rolling out formatting across a large codebase.
    #[arg(long, help_heading = "Execution")]
    require_pragma: bool,
}

#[derive(Args, Debug, Clone)]
struct ConfigOverridesArgs {
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
    ///
    /// Only built-in defaults and CLI overrides remain.
    #[arg(long, help_heading = "Config Overrides")]
    no_config: bool,

    /// Disable `.editorconfig` fallback.
    ///
    /// By default, when no `.cmakefmt.yaml` config file is found, cmakefmt
    /// reads `indent_style` and `indent_size` from `.editorconfig`. This
    /// flag disables that fallback.
    #[arg(long = "no-editorconfig", help_heading = "Config Overrides")]
    no_editorconfig: bool,

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
}

#[derive(Clone, Debug, Subcommand)]
enum CliCommand {
    /// Start the cmakefmt LSP server (reads/writes JSON-RPC on stdio).
    Lsp,
    /// Generate shell completion scripts and print them to stdout.
    Completions {
        /// The shell to generate completions for.
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Generate a roff man page and print it to stdout.
    ///
    /// Use with packaging:
    ///
    ///     cmakefmt manpage > cmakefmt.1
    ///
    /// Replaces the deprecated `--generate-man-page` flag.
    Manpage,
    /// Install a git pre-commit hook that runs `cmakefmt --check` on staged
    /// CMake files.
    InstallHook,
    /// Config inspection, generation, and conversion.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Dump internal representations (AST, parse tree) for debugging.
    Dump {
        #[command(subcommand)]
        action: DumpAction,

        /// Input file to dump (reads stdin if omitted).
        #[arg(global = true)]
        file: Option<PathBuf>,
    },
}

#[derive(Clone, Debug, Subcommand)]
enum ConfigAction {
    /// Print the default config template.
    Dump {
        /// Output format.
        #[arg(long, value_enum, default_value = "yaml")]
        format: DumpConfigFormat,
    },
    /// Print the JSON Schema for the config file.
    Schema,
    /// Validate a config file without formatting.
    Check {
        /// Config file to validate (discovers automatically if omitted).
        path: Option<String>,
    },
    /// Print the effective config for a target.
    Show {
        /// Target file for config resolution.
        path: Option<String>,
        /// Output format.
        #[arg(long, value_enum, default_value = "yaml")]
        format: DumpConfigFormat,
    },
    /// Print the config file path selected for a target.
    Path {
        /// Target file for config resolution.
        path: Option<String>,
    },
    /// Explain config resolution for a target or the current directory.
    Explain {
        /// Target file for config resolution.
        path: Option<String>,
    },
    /// Convert legacy cmake-format config files.
    Convert {
        /// Legacy config file(s) to convert.
        paths: Vec<PathBuf>,
        /// Output format.
        #[arg(long, value_enum, default_value = "yaml")]
        format: DumpConfigFormat,
    },
    /// Write a starter `.cmakefmt.yaml` to the current directory.
    Init,
}

#[derive(Clone, Debug, Subcommand)]
enum DumpAction {
    /// Print the raw parser AST as a tree.
    Ast,
    /// Print the formatted parse tree (not yet implemented).
    Parse,
    /// Report spec coverage for every CMake command the formatter knows
    /// about (and every documented CMake command it doesn't).
    ///
    /// Cross-references the formatter's built-in `CommandRegistry`
    /// against a snapshot of the upstream CMake command list and
    /// classifies each entry as `missing`, `stub`, `partial`, or
    /// `full`:
    ///
    /// * `missing` — the command is not in the registry; cmakefmt
    ///   treats it as user-defined and uses the default flat layout.
    /// * `stub`    — the command is in the registry but has no kwargs
    ///   or flags modelled (just a positional shape).
    /// * `partial` — has some structure (1..=3 kwargs+flags total).
    /// * `full`    — has substantive coverage (>3 kwargs+flags total).
    ///
    /// The 3-vs-4 split is heuristic; it gives a useful three-tier
    /// signal at the cost of occasional close-call misclassification.
    /// Exit code is always 0 — this is informational, not a check.
    SpecCoverage {
        /// Output format.
        #[arg(long, value_enum, default_value = "human")]
        format: SpecCoverageFormat,
        /// Restrict output to commands with a given coverage status.
        #[arg(long, value_enum)]
        status: Option<SpecCoverageStatusFilter>,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum SpecCoverageFormat {
    /// Human-friendly aligned table.
    Human,
    /// Machine-readable JSON document.
    Json,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum SpecCoverageStatusFilter {
    /// Commands absent from the registry.
    Missing,
    /// Commands with no kwargs or flags modelled.
    Stub,
    /// Commands with 1..=3 kwargs+flags modelled.
    Partial,
    /// Commands with >3 kwargs+flags modelled.
    Full,
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
    /// Editor-friendly JSON with byte-range replacements.
    Edit,
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
    std::panic::set_hook(Box::new(|info| {
        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown".to_string()
        };

        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown".to_string());

        eprintln!(
            "\
cmakefmt encountered an internal error and crashed.

This is a bug. Please report it at:
  https://github.com/cmakefmt/cmakefmt/issues/new

Include the following in your report:
  cmakefmt version: {}
  OS: {} ({})
  panic: {}
  location: {}",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
            message,
            location,
        );
    }));

    let cli = Cli::parse();

    match run(&cli) {
        Ok(code) => ExitCode::from(code),
        Err(cmakefmt::Error::Io(ref e)) if e.kind() == std::io::ErrorKind::BrokenPipe => {
            ExitCode::from(EXIT_OK)
        }
        Err(cmakefmt::Error::IoAt { ref source, .. })
            if source.kind() == std::io::ErrorKind::BrokenPipe =>
        {
            ExitCode::from(EXIT_OK)
        }
        Err(err) => {
            eprintln!("{}", render_cli_error(&err));
            ExitCode::from(EXIT_ERROR)
        }
    }
}

fn run(cli: &Cli) -> Result<u8, cmakefmt::Error> {
    check_required_version(cli)?;

    match &cli.command {
        #[cfg(feature = "lsp")]
        Some(CliCommand::Lsp) => {
            cmakefmt::lsp::run().map_err(|e| cmakefmt::Error::Formatter(e.to_string()))?;
            return Ok(EXIT_OK);
        }
        Some(CliCommand::Completions { shell }) => {
            let mut command = Cli::command();
            generate(*shell, &mut command, "cmakefmt", &mut io::stdout());
            return Ok(EXIT_OK);
        }
        Some(CliCommand::Manpage) => {
            return render_man_page();
        }
        Some(CliCommand::InstallHook) => {
            return install_git_hook();
        }
        Some(CliCommand::Config { action }) => {
            return run_config_subcommand(cli, action);
        }
        Some(CliCommand::Dump { action, file }) => {
            return run_dump_subcommand(cli, action, file.as_deref());
        }
        None => {}
    }

    if cli.execution.generate_man_page {
        return render_man_page();
    }

    validate_cli(cli)?;

    let stdout_mode = is_stdout_mode(cli);
    let colorize_stdout = stdout_mode && should_colorize_stdout(cli.output_modes.color);
    let file_filter = compile_file_filter(cli.input_selection.file_regex.as_deref())?;
    let mut targets = collect_targets(cli, file_filter.as_ref())?;
    if cli.input_selection.sorted {
        targets.sort_by(|a, b| {
            a.display_name(cli.input_selection.stdin_path.as_deref())
                .cmp(&b.display_name(cli.input_selection.stdin_path.as_deref()))
        });
    }
    if cli.output_modes.list_input_files {
        for target in &targets {
            println!(
                "{}",
                target.display_name(cli.input_selection.stdin_path.as_deref())
            );
        }
        return Ok(EXIT_OK);
    }
    if cli.output_modes.list_unknown_commands {
        return run_list_unknown_commands(cli, &targets);
    }
    if cli.output_modes.explain && targets.len() != 1 {
        return Err(cmakefmt::Error::cli_arg(
            "--explain requires exactly one formatting target",
        ));
    }
    if cli.execution.watch {
        return run_watch(cli, &targets, file_filter.as_ref());
    }
    if !cli.input_selection.line_ranges.is_empty() && targets.len() != 1 {
        return Err(cmakefmt::Error::cli_arg(
            "--lines requires exactly one formatting target",
        ));
    }
    let parallel_jobs = resolve_parallel_jobs(cli.execution.parallel)?;
    let stdout_is_terminal = io::stdout().is_terminal();
    let stderr_is_terminal = io::stderr().is_terminal();
    let colorize_stderr = should_colorize_stderr(cli.output_modes.color);

    if let Some(reason) =
        progress_bar_suppressed_reason(cli, targets.len(), stdout_is_terminal, stderr_is_terminal)
    {
        if colorize_stderr {
            eprintln!("\n\x1b[1;93m⚠ warning: --progress-bar ignored ({reason})\x1b[0m\n");
        } else {
            eprintln!("\nwarning: --progress-bar ignored ({reason})\n");
        }
    }
    let progress = ProgressReporter::new(
        should_enable_progress_bar(cli, targets.len(), stdout_is_terminal, stderr_is_terminal),
        targets.len(),
    );

    if cli.execution.debug {
        log_debug(format!(
            "discovered {} target(s){}",
            targets.len(),
            debug_parallel_suffix(parallel_jobs)
        ));
    }

    let start_time = std::time::Instant::now();
    let mut state = RunState {
        results: Vec::new(),
        failures: Vec::new(),
        summary: RunSummary {
            selected: targets.len(),
            ..RunSummary::default()
        },
        human_output: HumanOutputState::new(stdout_mode && targets.len() > 1),
    };

    process_targets(
        &targets,
        cli,
        parallel_jobs,
        colorize_stdout,
        &progress,
        |target_result| {
            handle_completed_target(
                target_result,
                cli,
                colorize_stdout,
                colorize_stderr,
                &progress,
                &mut state,
            )
        },
    )?;
    state.summary.elapsed = start_time.elapsed();
    let RunState {
        results,
        failures,
        summary,
        ..
    } = state;

    if cli.output_modes.in_place {
        write_in_place_updates(&results)?;
    }

    if cli.output_modes.report_format != ReportFormat::Human {
        // Emit the unified diff for report formats that don't embed it in
        // their structured output. GitHub annotations are line-prefixed and
        // coexist safely with diff text; JSON/Checkstyle/JUnit/SARIF would
        // be corrupted by raw text prepended to the structured output.
        if cli.output_modes.diff && cli.output_modes.report_format == ReportFormat::Github {
            for result in &results {
                if result.would_change {
                    write_diff_to_stdout(result, colorize_stdout)?;
                }
            }
        }
        print_non_human_report(
            &cli.output_modes,
            &cli.execution,
            &results,
            &failures,
            &summary,
        )?;
        return machine_mode_exit_code(&results, &failures, &summary, &cli.output_modes);
    }

    if should_print_human_summary(cli, &summary, &failures, results.len()) {
        progress.eprintln(&render_human_summary(&summary))?;
    }

    if cli.execution.stat {
        progress.eprintln(&render_stat_summary(&summary))?;
    }

    if cli.output_modes.check
        && !cli.execution.quiet
        && summary.changed > 0
        && cli.output_modes.report_format == ReportFormat::Human
    {
        progress.eprintln("hint: run `cmakefmt --in-place .` to fix formatting")?;
    }

    if !failures.is_empty() {
        Ok(EXIT_ERROR)
    } else if (cli.output_modes.check || cli.output_modes.list_changed_files) && summary.changed > 0
    {
        Ok(EXIT_CHECK_FAILED)
    } else {
        Ok(EXIT_OK)
    }
}

#[derive(Debug, Default, Serialize)]
struct RunSummary {
    selected: usize,
    changed: usize,
    unchanged: usize,
    skipped: usize,
    failed: usize,
    total_changed_lines: usize,
    #[serde(skip)]
    elapsed: std::time::Duration,
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

fn should_colorize_stderr(choice: ColorChoice) -> bool {
    match choice {
        ColorChoice::Auto => {
            io::stderr().is_terminal()
                && std::env::var_os("NO_COLOR").is_none()
                && std::env::var("TERM").map_or(true, |term| term != "dumb")
        }
        ColorChoice::Always => true,
        ColorChoice::Never => false,
    }
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::Cli;
    use crate::cli::runtime::{should_enable_progress_bar, streams_stdout_during_run};
    use cmakefmt::{default_config_template, default_config_template_for, DumpConfigFormat};

    #[test]
    fn dump_config_covers_config_backed_long_flags() {
        let template = default_config_template();
        let non_config_flags = [
            "check",
            "config-file",
            "color",
            "changed",
            "debug",
            "diff",
            "explain",
            "path-regex",
            "files-from",
            "generate-man-page",
            "help",
            "ignore-path",
            "keep-going",
            "cache",
            "cache-location",
            "cache-strategy",
            "lines",
            "list-changed-files",
            "list-input-files",
            "list-unknown-commands",
            "no-config",
            "no-editorconfig",
            "no-gitignore",
            "sorted",
            "parallel",
            "progress-bar",
            "quiet",
            "summary",
            "stat",
            "report-format",
            "required-version",
            "verify",
            "no-verify",
            "require-pragma",
            "since",
            "staged",
            "stdin-path",
            "version",
            "watch",
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

    #[test]
    fn progress_bar_policy_disables_live_stdout_on_a_terminal() {
        for args in [
            &["cmakefmt", "--progress-bar", "CMakeLists.txt"][..],
            &["cmakefmt", "--progress-bar", "--diff", "CMakeLists.txt"][..],
            &[
                "cmakefmt",
                "--progress-bar",
                "--list-changed-files",
                "CMakeLists.txt",
            ][..],
        ] {
            let cli = Cli::parse_from(args);
            assert!(streams_stdout_during_run(&cli));
            assert!(
                !should_enable_progress_bar(&cli, 2, true, true),
                "progress bar should be disabled for args: {:?}",
                args
            );
        }
    }

    #[test]
    fn progress_bar_policy_allows_non_streaming_modes_on_a_terminal() {
        for args in [
            &["cmakefmt", "--progress-bar", "--check", "CMakeLists.txt"][..],
            &["cmakefmt", "--progress-bar", "--summary", "CMakeLists.txt"][..],
            &["cmakefmt", "--progress-bar", "--quiet", "CMakeLists.txt"][..],
            &["cmakefmt", "--progress-bar", "--in-place", "CMakeLists.txt"][..],
            &[
                "cmakefmt",
                "--progress-bar",
                "--report-format",
                "json",
                "CMakeLists.txt",
            ][..],
        ] {
            let cli = Cli::parse_from(args);
            assert!(
                should_enable_progress_bar(&cli, 2, true, true),
                "progress bar should be enabled for args: {:?}",
                args
            );
        }
    }

    #[test]
    fn progress_bar_policy_allows_streaming_stdout_when_stdout_is_piped() {
        let cli = Cli::parse_from(["cmakefmt", "--progress-bar", "--diff", "CMakeLists.txt"]);
        assert!(streams_stdout_during_run(&cli));
        assert!(should_enable_progress_bar(&cli, 2, false, true));
    }

    #[test]
    fn progress_bar_policy_requires_stderr_terminal_and_multiple_targets() {
        let cli = Cli::parse_from(["cmakefmt", "--progress-bar", "--check", "CMakeLists.txt"]);
        assert!(!should_enable_progress_bar(&cli, 1, true, true));
        assert!(!should_enable_progress_bar(&cli, 2, true, false));
    }
}
