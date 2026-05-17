// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Human-readable summary, stat, and per-file status rendering.

use std::fmt::Write as _;

use crate::cli::process::{ProcessedTarget, ProgressReporter};
use crate::RunSummary;

pub(crate) fn render_human_summary(summary: &RunSummary) -> String {
    let mut rendered = format!(
        "summary: selected={}, changed={}, unchanged={}",
        summary.selected, summary.changed, summary.unchanged
    );
    if summary.skipped > 0 {
        let _ = write!(rendered, ", skipped={}", summary.skipped);
    }
    let _ = write!(rendered, ", failed={}", summary.failed);
    if summary.elapsed.as_millis() > 0 {
        let _ = write!(rendered, " in {:.2}s", summary.elapsed.as_secs_f64());
    }
    rendered
}

pub(crate) fn render_stat_summary(summary: &RunSummary) -> String {
    let files_word = if summary.changed == 1 {
        "file changed"
    } else {
        "files changed"
    };
    let lines_word = if summary.total_changed_lines == 1 {
        "line reformatted"
    } else {
        "lines reformatted"
    };
    format!(
        "{} {}, {} {}",
        summary.changed, files_word, summary.total_changed_lines, lines_word
    )
}

pub(crate) fn format_elapsed(elapsed: std::time::Duration) -> String {
    let ms = elapsed.as_millis();
    if ms == 0 {
        "<1ms".to_owned()
    } else if ms < 1000 {
        format!("{ms}ms")
    } else {
        format!("{:.2}s", elapsed.as_secs_f64())
    }
}

pub(crate) fn render_explain_output(
    result: &ProcessedTarget,
    progress: &ProgressReporter,
) -> Result<(), cmakefmt::Error> {
    progress.eprintln(&format!(
        "Formatting decisions for {}\n",
        result.display_name
    ))?;

    let mut found_any = false;
    for line in &result.debug_lines {
        if let Some(rest) = line.strip_prefix("formatter: ") {
            progress.eprintln(&format!("  {rest}"))?;
            found_any = true;
        }
    }

    if !found_any {
        progress.eprintln("  (no formatting decisions — file may be empty or fully disabled)")?;
    }
    progress.eprintln("")?;
    Ok(())
}

pub(crate) fn render_summary_line(result: &ProcessedTarget, colorize: bool) -> String {
    let display_name = &result.display_name;
    let would_change = result.would_change;
    let skipped = result.skipped;
    let skip_reason = result.skip_reason.as_deref();
    let changed_lines = result.changed_lines.len();
    let source_lines = result.source_lines;
    let formatted_lines = result.formatted_lines;
    let elapsed = result.elapsed;
    let elapsed_str = format_elapsed(elapsed);

    if skipped {
        let reason = skip_reason.unwrap_or("skipped");
        if colorize {
            return format!(
                "\u{1b}[2m-\u{1b}[0m {display_name}\n  \u{2514}\u{2500} \u{1b}[2mskipped ({reason})\u{1b}[0m"
            );
        }
        return format!("[-]  {display_name}\n     skipped ({reason})");
    }

    if would_change {
        let line_counts = if source_lines == formatted_lines {
            format!("{source_lines} lines")
        } else {
            format!("{source_lines} \u{2192} {formatted_lines} lines")
        };
        let detail = format!("{changed_lines} lines changed, {line_counts}, {elapsed_str}");
        if colorize {
            return format!("\u{1b}[1;93m!\u{1b}[0m {display_name}\n  \u{2514}\u{2500} {detail}");
        }
        return format!("[!]  {display_name}\n     {detail}");
    }

    // Unchanged
    let detail = format!("unchanged, {source_lines} lines, {elapsed_str}");
    if colorize {
        format!(
            "\u{1b}[1;92m\u{2714}\u{1b}[0m \u{1b}[2m{display_name}\u{1b}[0m\n  \u{2514}\u{2500} \u{1b}[2m{detail}\u{1b}[0m"
        )
    } else {
        format!("[ok] {display_name}\n     {detail}")
    }
}

pub(crate) fn render_summary_failed_line(display_name: &str, colorize: bool) -> String {
    if colorize {
        format!(
            "\u{1b}[1;91m\u{2717}\u{1b}[0m {display_name}\n  \u{2514}\u{2500} \u{1b}[91mparse error\u{1b}[0m"
        )
    } else {
        format!("[!!] {display_name}\n     parse error")
    }
}

#[cfg(test)]
mod tests {
    use super::{format_elapsed, render_summary_failed_line, render_summary_line};
    use crate::cli::process::ProcessedTarget;
    use std::time::Duration;

    #[allow(clippy::too_many_arguments)]
    fn test_target(
        name: &str,
        would_change: bool,
        skipped: bool,
        skip_reason: Option<&str>,
        changed_lines: usize,
        source_lines: usize,
        formatted_lines: usize,
        elapsed: Duration,
    ) -> ProcessedTarget {
        ProcessedTarget {
            path: None,
            display_name: name.to_owned(),
            formatted: String::new(),
            highlighted_output: None,
            unified_diff: None,
            changed_lines: vec![0; changed_lines],
            would_change,
            skipped,
            skip_reason: skip_reason.map(str::to_owned),
            debug_lines: Vec::new(),
            source_lines,
            formatted_lines,
            elapsed,
        }
    }

    #[test]
    fn summary_changed_file_no_color() {
        let target = test_target(
            "src/CMakeLists.txt",
            true,
            false,
            None,
            12,
            84,
            86,
            Duration::from_millis(2),
        );
        let line = render_summary_line(&target, false);
        assert!(line.starts_with("[!]  src/CMakeLists.txt"));
        assert!(line.contains("12 lines changed"));
        assert!(line.contains("84 \u{2192} 86 lines"));
        assert!(line.contains("2ms"));
        assert!(line.contains('\n'));
    }

    #[test]
    fn summary_changed_file_same_line_count_no_color() {
        let target = test_target(
            "CMakeLists.txt",
            true,
            false,
            None,
            3,
            42,
            42,
            Duration::from_millis(5),
        );
        let line = render_summary_line(&target, false);
        assert!(line.contains("3 lines changed"));
        assert!(line.contains("42 lines"));
        // Should not contain an arrow when line count is unchanged
        assert!(!line.contains('\u{2192}'));
    }

    #[test]
    fn summary_unchanged_file_no_color() {
        let target = test_target(
            "tests/CMakeLists.txt",
            false,
            false,
            None,
            0,
            42,
            42,
            Duration::from_millis(1),
        );
        let line = render_summary_line(&target, false);
        assert!(line.starts_with("[ok] tests/CMakeLists.txt"));
        assert!(line.contains("unchanged"));
        assert!(line.contains("42 lines"));
        assert!(line.contains("1ms"));
    }

    #[test]
    fn summary_skipped_file_no_color() {
        let target = test_target(
            "docs/CMakeLists.txt",
            false,
            true,
            Some("missing format opt-in pragma"),
            0,
            10,
            10,
            Duration::ZERO,
        );
        let line = render_summary_line(&target, false);
        assert!(line.starts_with("[-]  docs/CMakeLists.txt"));
        assert!(line.contains("skipped (missing format opt-in pragma)"));
    }

    #[test]
    fn summary_failed_file_no_color() {
        let line = render_summary_failed_line("lib/CMakeLists.txt", false);
        assert!(line.starts_with("[!!] lib/CMakeLists.txt"));
        assert!(line.contains("parse error"));
    }

    #[test]
    fn summary_changed_file_with_color() {
        let target = test_target(
            "src/CMakeLists.txt",
            true,
            false,
            None,
            5,
            50,
            52,
            Duration::from_millis(3),
        );
        let line = render_summary_line(&target, true);
        // Bold bright yellow exclamation mark
        assert!(line.contains("\u{1b}[1;93m!\u{1b}[0m"));
        assert!(line.contains("src/CMakeLists.txt"));
        assert!(line.contains("5 lines changed"));
    }

    #[test]
    fn summary_unchanged_file_with_color() {
        let target = test_target(
            "tests/CMakeLists.txt",
            false,
            false,
            None,
            0,
            42,
            42,
            Duration::from_millis(1),
        );
        let line = render_summary_line(&target, true);
        // Bold bright green checkmark
        assert!(line.contains("\u{1b}[1;92m\u{2714}\u{1b}[0m"));
        assert!(line.contains("unchanged"));
    }

    #[test]
    fn summary_skipped_file_with_color() {
        let target = test_target(
            "docs/CMakeLists.txt",
            false,
            true,
            Some("missing pragma"),
            0,
            10,
            10,
            Duration::ZERO,
        );
        let line = render_summary_line(&target, true);
        // Dim hyphen
        assert!(line.contains("\u{1b}[2m-\u{1b}[0m"));
        assert!(line.contains("skipped (missing pragma)"));
    }

    #[test]
    fn summary_failed_file_with_color() {
        let line = render_summary_failed_line("lib/CMakeLists.txt", true);
        // Bold bright red ballot x
        assert!(line.contains("\u{2717}"));
        assert!(line.contains("\u{1b}[1;91m"));
        assert!(line.contains("parse error"));
    }

    #[test]
    fn summary_line_has_tree_branch() {
        let target = test_target(
            "CMakeLists.txt",
            true,
            false,
            None,
            1,
            10,
            10,
            Duration::from_millis(1),
        );
        let line = render_summary_line(&target, true);
        // Should contain the tree branch connector
        assert!(line.contains("\u{2514}\u{2500}"));
    }

    #[test]
    fn summary_failed_line_has_tree_branch() {
        let line = render_summary_failed_line("CMakeLists.txt", true);
        assert!(line.contains("\u{2514}\u{2500}"));
    }

    #[test]
    fn summary_no_color_uses_indentation_not_branch() {
        let target = test_target(
            "CMakeLists.txt",
            false,
            false,
            None,
            0,
            10,
            10,
            Duration::from_millis(1),
        );
        let line = render_summary_line(&target, false);
        let lines: Vec<&str> = line.split('\n').collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[1].starts_with("     "));
    }

    #[test]
    fn format_elapsed_sub_millisecond() {
        assert_eq!(format_elapsed(Duration::ZERO), "<1ms");
        assert_eq!(format_elapsed(Duration::from_micros(500)), "<1ms");
    }

    #[test]
    fn format_elapsed_milliseconds() {
        assert_eq!(format_elapsed(Duration::from_millis(1)), "1ms");
        assert_eq!(format_elapsed(Duration::from_millis(42)), "42ms");
        assert_eq!(format_elapsed(Duration::from_millis(999)), "999ms");
    }

    #[test]
    fn format_elapsed_seconds() {
        assert_eq!(format_elapsed(Duration::from_millis(1000)), "1.00s");
        assert_eq!(format_elapsed(Duration::from_millis(2500)), "2.50s");
    }
}
