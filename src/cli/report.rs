// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Per-report-format builders (JSON, SARIF, Checkstyle, JUnit, GitHub) and the
//! shared machine-mode exit-code/print dispatcher.

use std::fmt::Write as _;

use serde::Serialize;

use crate::cli::process::{FailedTarget, ProcessedTarget};
use crate::cli::summary::render_human_summary;
use crate::{
    ExecutionArgs, OutputModesArgs, ReportFormat, RunSummary, EXIT_CHECK_FAILED, EXIT_ERROR,
    EXIT_OK,
};

#[derive(Debug, Serialize)]
pub(crate) struct JsonReport {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    elapsed_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_lines: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    formatted_lines: Option<usize>,
}

pub(crate) fn build_json_report(
    results: &[ProcessedTarget],
    failures: &[FailedTarget],
    summary: &RunSummary,
    output_modes: &OutputModesArgs,
    execution: &ExecutionArgs,
) -> JsonReport {
    let mode = if output_modes.in_place {
        "in-place"
    } else if output_modes.check {
        "check"
    } else if output_modes.list_changed_files {
        "list-changed-files"
    } else if output_modes.list_input_files {
        "list-input-files"
    } else if output_modes.diff {
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
            total_changed_lines: summary.total_changed_lines,
            ..RunSummary::default()
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
                formatted: (!output_modes.in_place
                    && !output_modes.check
                    && !output_modes.list_changed_files
                    && !output_modes.list_input_files
                    && !output_modes.diff)
                    .then(|| result.formatted.clone()),
                diff: output_modes
                    .diff
                    .then(|| result.unified_diff.clone().unwrap_or_default()),
                debug_lines: if execution.debug {
                    result.debug_lines.clone()
                } else {
                    Vec::new()
                },
                elapsed_ms: output_modes
                    .summary
                    .then_some(result.elapsed.as_millis() as u64),
                source_lines: output_modes.summary.then_some(result.source_lines),
                formatted_lines: output_modes.summary.then_some(result.formatted_lines),
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

pub(crate) fn build_github_report(
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

pub(crate) fn build_checkstyle_report(
    results: &[ProcessedTarget],
    failures: &[FailedTarget],
) -> String {
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

pub(crate) fn build_junit_report(
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

pub(crate) fn build_sarif_report(
    results: &[ProcessedTarget],
    failures: &[FailedTarget],
) -> serde_json::Value {
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
                    "informationUri": "https://github.com/cmakefmt/cmakefmt",
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

pub(crate) fn machine_mode_exit_code(
    results: &[ProcessedTarget],
    failures: &[FailedTarget],
    _summary: &RunSummary,
    output_modes: &OutputModesArgs,
) -> Result<u8, cmakefmt::Error> {
    if !failures.is_empty() {
        Ok(EXIT_ERROR)
    } else if (output_modes.check || output_modes.list_changed_files)
        && results.iter().any(|r| r.would_change)
    {
        Ok(EXIT_CHECK_FAILED)
    } else {
        Ok(EXIT_OK)
    }
}

pub(crate) fn print_non_human_report(
    output_modes: &OutputModesArgs,
    execution: &ExecutionArgs,
    results: &[ProcessedTarget],
    failures: &[FailedTarget],
    summary: &RunSummary,
) -> Result<(), cmakefmt::Error> {
    match output_modes.report_format {
        ReportFormat::Human => Ok(()),
        ReportFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&build_json_report(
                    results,
                    failures,
                    summary,
                    output_modes,
                    execution,
                ))
                .map_err(|err| cmakefmt::Error::render("JSON report", err.to_string()))?
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
                serde_json::to_string_pretty(&build_sarif_report(results, failures))
                    .map_err(|err| cmakefmt::Error::render("SARIF report", err.to_string()))?
            );
            Ok(())
        }
        ReportFormat::Edit => {
            println!(
                "{}",
                serde_json::to_string_pretty(&build_edit_report(results))
                    .map_err(|err| cmakefmt::Error::render("edit report", err.to_string()))?
            );
            Ok(())
        }
    }
}

fn build_edit_report(results: &[ProcessedTarget]) -> serde_json::Value {
    let edits: Vec<serde_json::Value> = results
        .iter()
        .filter(|r| r.would_change && !r.skipped)
        .map(|r| {
            serde_json::json!({
                "file": r.display_name,
                "replacement": r.formatted
            })
        })
        .collect();
    serde_json::json!({ "edits": edits })
}
