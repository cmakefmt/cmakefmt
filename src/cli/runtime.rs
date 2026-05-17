// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Per-target run-loop helpers: stdout emission, in-place writes, summary
//! gating, and CLI validation. Kept here so `main.rs` can focus on argument
//! parsing and the top-level `run()` dispatch.

use std::io::{self, Write};
use std::path::Path;

use crate::cli::diff::colorize_unified_diff;
use crate::cli::errors::render_cli_error;
use crate::cli::process::{FailedTarget, ProcessedTarget, ProgressReporter};
use crate::cli::summary::{render_explain_output, render_summary_failed_line, render_summary_line};
use crate::{Cli, ReportFormat, RunSummary};

pub(crate) struct RunState {
    pub(crate) results: Vec<ProcessedTarget>,
    pub(crate) failures: Vec<FailedTarget>,
    pub(crate) summary: RunSummary,
    pub(crate) human_output: HumanOutputState,
}

pub(crate) struct HumanOutputState {
    multi_target_stdout: bool,
    wrote_stdout_block: bool,
}

impl HumanOutputState {
    pub(crate) fn new(multi_target_stdout: bool) -> Self {
        Self {
            multi_target_stdout,
            wrote_stdout_block: false,
        }
    }
}

pub(crate) fn handle_completed_target(
    target_result: Result<ProcessedTarget, cmakefmt::Error>,
    cli: &Cli,
    colorize_stdout: bool,
    colorize_stderr: bool,
    progress: &ProgressReporter,
    state: &mut RunState,
) -> Result<(), cmakefmt::Error> {
    match target_result {
        Ok(result) => {
            if result.skipped {
                state.summary.skipped += 1;
            } else if result.would_change {
                state.summary.changed += 1;
                state.summary.total_changed_lines += result.changed_lines.len();
            } else {
                state.summary.unchanged += 1;
            }

            if cli.output_modes.summary && cli.output_modes.report_format == ReportFormat::Human {
                progress.eprintln(&render_summary_line(&result, colorize_stderr))?;
            }

            if cli.output_modes.report_format == ReportFormat::Human {
                emit_human_result(
                    &result,
                    cli,
                    colorize_stdout,
                    progress,
                    &mut state.human_output,
                )?;
            }

            state.results.push(result);
            Ok(())
        }
        Err(err) => {
            if !cli.execution.keep_going {
                return Err(err);
            }

            state.summary.failed += 1;
            let failure = FailedTarget {
                display_name: error_display_name(&err),
                rendered_error: render_cli_error(&err),
            };

            if cli.output_modes.summary && cli.output_modes.report_format == ReportFormat::Human {
                progress.eprintln(&render_summary_failed_line(
                    &failure.display_name,
                    colorize_stderr,
                ))?;
            }

            if cli.output_modes.report_format == ReportFormat::Human {
                emit_human_failure(&failure, progress)?;
            }

            state.failures.push(failure);
            Ok(())
        }
    }
}

fn emit_human_result(
    result: &ProcessedTarget,
    cli: &Cli,
    colorize_stdout: bool,
    progress: &ProgressReporter,
    human_output: &mut HumanOutputState,
) -> Result<(), cmakefmt::Error> {
    if cli.execution.debug {
        for line in &result.debug_lines {
            progress.eprintln(&format!("debug: {line}"))?;
        }
    }

    if cli.output_modes.explain {
        render_explain_output(result, progress)?;
        return Ok(());
    }

    if result.skipped {
        if is_stdout_mode(cli) && !cli.execution.quiet && !cli.output_modes.summary {
            write_stdout_result(result, colorize_stdout, human_output)?;
        }
        return Ok(());
    }

    if cli.output_modes.list_changed_files {
        if result.would_change {
            writeln!(io::stdout(), "{}", result.display_name).map_err(cmakefmt::Error::Io)?;
            flush_stdout()?;
        }
        return Ok(());
    }

    if cli.output_modes.check {
        if result.would_change {
            if cli.output_modes.diff {
                write_diff_to_stdout(result, colorize_stdout)?;
                flush_stdout()?;
            }
            if !cli.execution.quiet && !cli.output_modes.summary {
                progress.eprintln(&format!("{} would be reformatted", result.display_name))?;
            }
        }
        return Ok(());
    }

    if cli.output_modes.in_place {
        return Ok(());
    }

    if cli.output_modes.diff {
        if result.would_change {
            write_diff_to_stdout(result, colorize_stdout)?;
            flush_stdout()?;
        }
        return Ok(());
    }

    if !cli.output_modes.summary && !cli.execution.quiet {
        write_stdout_result(result, colorize_stdout, human_output)?;
    }

    Ok(())
}

fn emit_human_failure(
    failure: &FailedTarget,
    progress: &ProgressReporter,
) -> Result<(), cmakefmt::Error> {
    progress.eprintln(&failure.rendered_error)
}

fn write_stdout_result(
    result: &ProcessedTarget,
    colorize_stdout: bool,
    human_output: &mut HumanOutputState,
) -> Result<(), cmakefmt::Error> {
    if human_output.multi_target_stdout {
        if human_output.wrote_stdout_block {
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
    flush_stdout()?;
    human_output.wrote_stdout_block = true;
    Ok(())
}

fn flush_stdout() -> Result<(), cmakefmt::Error> {
    io::stdout().flush().map_err(cmakefmt::Error::Io)
}

fn error_display_name(err: &cmakefmt::Error) -> String {
    match err {
        cmakefmt::Error::Parse(parse) => parse.display_name.clone(),
        cmakefmt::Error::Config(config) => config.path.display().to_string(),
        cmakefmt::Error::Spec(spec) => spec.path.display().to_string(),
        cmakefmt::Error::Formatter(message) => message
            .split(':')
            .next()
            .unwrap_or("<unknown>")
            .trim()
            .to_owned(),
        cmakefmt::Error::IoAt { path, .. } => path.display().to_string(),
        cmakefmt::Error::LegacyMigration { path, .. } => path.display().to_string(),
        cmakefmt::Error::Io(_)
        | cmakefmt::Error::CliArg { .. }
        | cmakefmt::Error::InvalidRegex { .. }
        | cmakefmt::Error::Render { .. }
        | cmakefmt::Error::LayoutTooWide { .. } => "<unknown>".to_owned(),
        _ => "<unknown>".to_owned(),
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

pub(crate) fn is_stdout_mode(cli: &Cli) -> bool {
    !cli.output_modes.list_changed_files
        && !cli.output_modes.list_input_files
        && !cli.output_modes.check
        && !cli.output_modes.in_place
}

pub(crate) fn streams_stdout_during_run(cli: &Cli) -> bool {
    if cli.output_modes.report_format != ReportFormat::Human {
        return false;
    }

    if cli.output_modes.list_changed_files || cli.output_modes.diff {
        return true;
    }

    is_stdout_mode(cli) && !cli.output_modes.summary && !cli.execution.quiet
}

pub(crate) fn should_enable_progress_bar(
    cli: &Cli,
    total: usize,
    stdout_is_terminal: bool,
    stderr_is_terminal: bool,
) -> bool {
    cli.execution.progress_bar
        && total > 1
        && stderr_is_terminal
        && (!streams_stdout_during_run(cli) || !stdout_is_terminal)
}

/// Returns a human-readable reason if the progress bar was requested but
/// suppressed, or `None` if it was enabled (or not requested).
pub(crate) fn progress_bar_suppressed_reason(
    cli: &Cli,
    total: usize,
    stdout_is_terminal: bool,
    stderr_is_terminal: bool,
) -> Option<&'static str> {
    if !cli.execution.progress_bar
        || should_enable_progress_bar(cli, total, stdout_is_terminal, stderr_is_terminal)
    {
        return None;
    }
    if !stderr_is_terminal {
        Some("stderr is not a terminal")
    } else if total <= 1 {
        Some("only one file to process")
    } else {
        Some("output is streaming to the terminal; pipe stdout to enable")
    }
}

pub(crate) fn write_in_place_updates(results: &[ProcessedTarget]) -> Result<(), cmakefmt::Error> {
    for result in results {
        if let Some(path) = &result.path {
            if result.would_change {
                atomic_write(path, &result.formatted)?;
            }
        }
    }
    Ok(())
}

/// Write `contents` to `path` atomically by writing to a temporary file in the
/// same directory and then renaming. This prevents partial writes and avoids
/// TOCTOU races where the target could be replaced with a symlink between read
/// and write.
pub(crate) fn write_diff_to_stdout(
    result: &ProcessedTarget,
    colorize: bool,
) -> Result<(), cmakefmt::Error> {
    let diff_output = result.unified_diff.as_deref().unwrap_or_default();
    let display_output = if colorize {
        colorize_unified_diff(diff_output)
    } else {
        diff_output.to_owned()
    };
    io::stdout()
        .write_all(display_output.as_bytes())
        .map_err(cmakefmt::Error::Io)
}

pub(crate) fn atomic_write(path: &Path, contents: &str) -> Result<(), cmakefmt::Error> {
    use cmakefmt::IoResultExt;
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(dir).with_path(dir)?;
    tmp.write_all(contents.as_bytes()).with_path(path)?;
    tmp.persist(path)
        .map_err(|e| cmakefmt::Error::io_at(path, e.error))?;
    Ok(())
}

pub(crate) fn should_print_human_summary(
    cli: &Cli,
    summary: &RunSummary,
    failures: &[FailedTarget],
    successful_results: usize,
) -> bool {
    if cli.output_modes.report_format != ReportFormat::Human {
        return false;
    }

    let stdout_mode = !cli.output_modes.list_changed_files
        && !cli.output_modes.list_input_files
        && !cli.output_modes.check
        && !cli.output_modes.in_place
        && !cli.output_modes.diff;
    if stdout_mode {
        return cli.execution.quiet || cli.output_modes.summary || !failures.is_empty();
    }

    cli.execution.quiet
        || !failures.is_empty()
        || cli.output_modes.check
        || cli.output_modes.in_place
        || (cli.output_modes.diff && successful_results > 1)
        || summary.selected > 1
}

pub(crate) fn needs_debug_lines(cli: &Cli) -> bool {
    cli.execution.debug || cli.output_modes.explain
}

pub(crate) fn log_debug(message: impl AsRef<str>) {
    eprintln!("debug: {}", message.as_ref());
}

pub(crate) fn debug_parallel_suffix(parallel_jobs: usize) -> String {
    if parallel_jobs > 1 {
        format!(" (parallel jobs: {parallel_jobs})")
    } else {
        String::new()
    }
}

pub(crate) fn check_required_version(cli: &Cli) -> Result<(), cmakefmt::Error> {
    let Some(required) = &cli.execution.required_version else {
        return Ok(());
    };

    let required = required.trim().trim_start_matches('v');
    let current = env!("CARGO_PKG_VERSION");
    if required == current {
        Ok(())
    } else {
        Err(cmakefmt::Error::cli_arg(format!(
            "required cmakefmt version {required} does not match current version {current}"
        )))
    }
}

pub(crate) fn resolve_parallel_jobs(requested: Option<usize>) -> Result<usize, cmakefmt::Error> {
    match requested {
        None => {
            // Default: available CPUs minus 1, minimum 1.
            let cpus = std::thread::available_parallelism()
                .map(|p| p.get())
                .unwrap_or(1);
            Ok(cpus.saturating_sub(1).max(1))
        }
        Some(0) => std::thread::available_parallelism()
            .map(|parallelism| parallelism.get())
            .map_err(cmakefmt::Error::Io),
        Some(jobs) => Ok(jobs.max(1)),
    }
}

pub(crate) fn validate_cli(cli: &Cli) -> Result<(), cmakefmt::Error> {
    if cli.config_overrides.no_config && !cli.config_overrides.config_paths.is_empty() {
        return Err(cmakefmt::Error::cli_arg(
            "--no-config cannot be combined with --config-file",
        ));
    }

    if cli.execution.verify && cli.execution.no_verify {
        return Err(cmakefmt::Error::cli_arg(
            "--verify cannot be combined with --no-verify",
        ));
    }

    if (cli.input_selection.staged || cli.input_selection.changed)
        && (!cli.input_selection.files.is_empty() || !cli.input_selection.files_from.is_empty())
    {
        return Err(cmakefmt::Error::cli_arg(
            "--staged/--changed cannot be combined with explicit input paths or --files-from",
        ));
    }

    if cli.input_selection.stdin_path.is_some()
        && !cli.input_selection.files.iter().any(|file| file == "-")
    {
        return Err(cmakefmt::Error::cli_arg(
            "--stdin-path requires stdin input via `cmakefmt -`",
        ));
    }

    if cli.execution.generate_man_page
        && (!cli.input_selection.files.is_empty()
            || !cli.input_selection.files_from.is_empty()
            || !cli.config_overrides.config_paths.is_empty()
            || cli.config_overrides.no_config
            || cli.execution.debug
            || cli.execution.quiet
            || cli.execution.keep_going
            || cli.output_modes.diff
            || cli.output_modes.check
            || cli.output_modes.in_place
            || cli.output_modes.list_changed_files
            || cli.output_modes.list_input_files
            || cli.input_selection.staged
            || cli.input_selection.changed
            || cli.input_selection.stdin_path.is_some()
            || !cli.input_selection.line_ranges.is_empty())
    {
        return Err(cmakefmt::Error::cli_arg(
            "completion/man-page generation cannot be combined with formatting or config-introspection inputs",
        ));
    }

    if cli.output_modes.diff && cli.output_modes.list_changed_files {
        return Err(cmakefmt::Error::cli_arg(
            "--diff cannot be combined with --list-changed-files",
        ));
    }

    if cli.output_modes.list_input_files && cli.output_modes.report_format != ReportFormat::Human {
        return Err(cmakefmt::Error::cli_arg(
            "--list-input-files only supports human output",
        ));
    }

    if cli.output_modes.list_input_files && !cli.input_selection.line_ranges.is_empty() {
        return Err(cmakefmt::Error::cli_arg(
            "--list-input-files cannot be combined with --lines",
        ));
    }

    if cli.execution.watch && cli.input_selection.files.iter().any(|f| f == "-") {
        return Err(cmakefmt::Error::cli_arg("--watch cannot read from stdin"));
    }

    Ok(())
}
