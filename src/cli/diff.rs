// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Line, diff, and ANSI-highlight helpers used by the CLI.

use similar::TextDiff;

use crate::LineRange;

pub(crate) fn highlight_changed_lines(source: &str, formatted: &str) -> String {
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

pub(crate) fn split_lines_with_endings(source: &str) -> Vec<&str> {
    if source.is_empty() {
        Vec::new()
    } else {
        source.split_inclusive('\n').collect()
    }
}

pub(crate) fn changed_formatted_line_mask<'a>(
    original: &[&'a str],
    formatted: &[&'a str],
) -> Vec<bool> {
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

pub(crate) fn changed_formatted_line_numbers(original: &[&str], formatted: &[&str]) -> Vec<usize> {
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

pub(crate) fn colorize_unified_diff(diff: &str) -> String {
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

pub(crate) fn build_unified_diff(display_name: &str, source: &str, formatted: &str) -> String {
    TextDiff::from_lines(source, formatted)
        .unified_diff()
        .context_radius(3)
        .header(&format!("a/{display_name}"), &format!("b/{display_name}"))
        .to_string()
}

pub(crate) fn apply_line_ranges(
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
        Err(cmakefmt::Error::cli_arg(format!(
            "{display_name}: selected line ranges would affect lines outside the requested ranges ({})",
            outside
                .into_iter()
                .map(|line| line.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }
}
