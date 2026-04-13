// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Command-invocation formatting logic.

use crate::config::{
    CaseStyle, CommandConfig, CompiledPatterns, Config, DangleAlign, FractionalTabPolicy,
};
use crate::error::Result;
use crate::formatter::comment;
use crate::parser::ast::{Argument, CommandInvocation};
use crate::spec::registry::CommandRegistry;
use crate::spec::{CommandForm, CommandSpec, NArgs};

use super::DebugLog;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HeaderKind {
    Keyword,
    Flag,
}

#[derive(Debug)]
pub(crate) struct Section<'a> {
    pub(crate) header: Option<&'a str>,
    pub(crate) header_kind: Option<HeaderKind>,
    pub(crate) arguments: Vec<&'a Argument>,
}

/// Format a single parsed command invocation.
///
/// The formatter chooses between inline, hanging-wrap, and vertical layouts
/// using command specs from the registry plus the effective per-command
/// configuration.
pub(crate) fn format_command(
    command: &CommandInvocation,
    config: &Config,
    patterns: &CompiledPatterns,
    registry: &CommandRegistry,
    block_depth: usize,
    debug: &mut DebugLog<'_>,
) -> Result<String> {
    let cmd_config = config.for_command(&command.name);
    let spec = registry.get(&command.name);
    let first_arg = first_argument(command).map(Argument::as_str);
    let form = spec.form_for(first_arg);
    let mut sections = split_sections(command, form)?;

    if config.enable_sort {
        sort_sections(&mut sections, form, config.autosort);
    }

    debug.log(format!(
        "formatter: command {} form={} first_arg={} effective_config(line_width={}, tab_size={}, dangle_parens={}, max_hanging_wrap_lines={}, max_hanging_wrap_positional_args={}, max_hanging_wrap_groups={})",
        command.name,
        describe_selected_form(spec, first_arg),
        first_arg.unwrap_or("<none>"),
        cmd_config.line_width(),
        cmd_config.tab_size(),
        cmd_config.dangle_parens(),
        cmd_config.global().max_lines_hwrap,
        cmd_config.max_pargs_hwrap(),
        cmd_config.max_subgroups_hwrap(),
    ));

    // Check whether this command must always be laid out vertically: either
    // the global config lists it, or the resolved command spec requests it.
    let spec_always_wrap = form
        .layout
        .as_ref()
        .and_then(|l| l.always_wrap)
        .unwrap_or(false);
    let config_always_wrap = config
        .always_wrap
        .iter()
        .any(|n| n.eq_ignore_ascii_case(&command.name));
    let force_vertical = spec_always_wrap || config_always_wrap;

    let spec_wrap_first = form.layout.as_ref().and_then(|l| l.wrap_after_first_arg);
    let wrap_after_first_arg = cmd_config.wrap_after_first_arg(spec_wrap_first);

    let output = if force_vertical {
        debug.log(format!(
            "formatter: command {} layout=vertical (always_wrap)",
            command.name
        ));
        format_command_vertical(
            command,
            &sections,
            &cmd_config,
            patterns,
            block_depth,
            wrap_after_first_arg,
        )?
    } else if let Some(inline) = try_format_inline(
        command,
        &sections,
        &cmd_config,
        block_depth,
        config.line_width,
    ) {
        debug.log(format!(
            "formatter: command {} layout=inline sections={} positional_args={}",
            command.name,
            sections.len(),
            sections
                .iter()
                .find(|section| section.header.is_none())
                .map_or(0, |section| section.arguments.len())
        ));
        inline
    } else if let Some(hanging) = try_format_hanging(
        command,
        &sections,
        &cmd_config,
        patterns,
        block_depth,
        config.line_width,
    ) {
        debug.log(format!(
            "formatter: command {} layout=hanging-wrap thresholds(line_width={}, max_hanging_wrap_lines={}, max_hanging_wrap_positional_args={})",
            command.name,
            cmd_config.line_width(),
            cmd_config.global().max_lines_hwrap,
            cmd_config.max_pargs_hwrap()
        ));
        hanging
    } else {
        debug.log(format!(
            "formatter: command {} layout=vertical thresholds(line_width={}, max_hanging_wrap_lines={}, max_hanging_wrap_positional_args={}, max_hanging_wrap_groups={})",
            command.name,
            cmd_config.line_width(),
            cmd_config.global().max_lines_hwrap,
            cmd_config.max_pargs_hwrap(),
            cmd_config.max_subgroups_hwrap()
        ));
        format_command_vertical(
            command,
            &sections,
            &cmd_config,
            patterns,
            block_depth,
            wrap_after_first_arg,
        )?
    };

    if config.use_tabchars {
        Ok(spaces_to_tabs(
            &output,
            cmd_config.tab_size(),
            config.fractional_tab_policy,
        ))
    } else {
        Ok(output)
    }
}

fn describe_selected_form(spec: &CommandSpec, first_arg: Option<&str>) -> String {
    match spec {
        CommandSpec::Single(_) => "single".to_owned(),
        CommandSpec::Discriminated { forms, fallback } => match first_arg {
            Some(token) if forms.contains_key(token) => format!("discriminated:{token}"),
            Some(token) => {
                let normalized = token.to_ascii_uppercase();
                if forms.contains_key(&normalized) {
                    format!("discriminated:{normalized}")
                } else if fallback.is_some() {
                    format!("fallback:{token}")
                } else {
                    format!("first-form:{token}")
                }
            }
            None if fallback.is_some() => "fallback:<none>".to_owned(),
            None => "first-form:<none>".to_owned(),
        },
    }
}

fn first_argument(command: &CommandInvocation) -> Option<&Argument> {
    command
        .arguments
        .iter()
        .find(|argument| !argument.is_comment())
}

fn format_name(command: &CommandInvocation, cmd_config: &CommandConfig<'_>) -> String {
    let name = apply_case(cmd_config.command_case(), &command.name);
    if cmd_config.space_before_paren() {
        let mut spaced = String::with_capacity(name.len() + 1);
        spaced.push_str(&name);
        spaced.push(' ');
        spaced
    } else {
        name
    }
}

pub(crate) fn split_sections<'a>(
    command: &'a CommandInvocation,
    form: &'a CommandForm,
) -> Result<Vec<Section<'a>>> {
    let mut sections = Vec::with_capacity(command.arguments.len().min(8));

    for argument in &command.arguments {
        if argument.is_comment() {
            if sections.is_empty() {
                sections.push(Section {
                    header: None,
                    header_kind: None,
                    arguments: Vec::new(),
                });
            }
            sections
                .last_mut()
                .expect("section list contains at least one section")
                .arguments
                .push(argument);
            continue;
        }

        let token = argument.as_str();
        if nested_token_belongs_to_current_section(&sections, form, token) {
            sections
                .last_mut()
                .expect("section list contains at least one section")
                .arguments
                .push(argument);
            continue;
        }

        let header_kind = if contains_kwarg(form, token) {
            Some(HeaderKind::Keyword)
        } else if contains_flag(form, token) {
            Some(HeaderKind::Flag)
        } else {
            None
        };

        if let Some(header_kind) = header_kind {
            sections.push(Section {
                header: Some(token),
                header_kind: Some(header_kind),
                arguments: Vec::new(),
            });
            continue;
        }

        if sections.is_empty() {
            sections.push(Section {
                header: None,
                header_kind: None,
                arguments: Vec::new(),
            });
        }

        sections
            .last_mut()
            .expect("section list contains at least one section")
            .arguments
            .push(argument);
    }

    Ok(sections)
}

/// Sort arguments within sections that are marked sortable.
fn sort_sections(sections: &mut [Section<'_>], form: &CommandForm, autosort: bool) {
    for section in sections.iter_mut() {
        let Some(header) = section.header else {
            continue;
        };
        if section.arguments.is_empty() {
            continue;
        }

        // Check if the spec marks this keyword section as sortable.
        let spec_sortable = form
            .kwargs
            .get(&header.to_ascii_uppercase())
            .or_else(|| form.kwargs.get(header))
            .is_some_and(|kwarg| kwarg.sortable);

        let should_sort = if spec_sortable {
            true
        } else if autosort {
            // Heuristic: all non-comment arguments are simple unquoted tokens
            // (no variables, generator expressions, or quoted strings).
            section
                .arguments
                .iter()
                .filter(|arg| !arg.is_comment())
                .all(|arg| {
                    matches!(arg, Argument::Unquoted(s) if !s.contains("${") && !s.contains("$<") && !s.contains("$ENV{") && !s.contains("$CACHE{"))
                })
        } else {
            false
        };

        if should_sort {
            // Partition into non-comment arguments and inline comments.
            // Sort only the non-comment arguments, preserving comment positions.
            let non_comment_positions: Vec<usize> = section
                .arguments
                .iter()
                .enumerate()
                .filter(|(_, a)| !a.is_comment())
                .map(|(i, _)| i)
                .collect();

            let mut sortable_args: Vec<&Argument> = non_comment_positions
                .iter()
                .map(|&i| section.arguments[i])
                .collect();

            sortable_args.sort_by(|a, b| {
                a.as_str()
                    .to_ascii_lowercase()
                    .cmp(&b.as_str().to_ascii_lowercase())
            });

            for (j, &pos) in non_comment_positions.iter().enumerate() {
                section.arguments[pos] = sortable_args[j];
            }
        }
    }
}

fn nested_token_belongs_to_current_section(
    sections: &[Section<'_>],
    form: &CommandForm,
    token: &str,
) -> bool {
    let Some(section) = sections.last() else {
        return false;
    };
    let Some(HeaderKind::Keyword) = section.header_kind else {
        return false;
    };
    let Some(header) = section.header else {
        return false;
    };
    let Some(spec) = lookup_kwarg(form, header) else {
        return false;
    };

    matches!(spec.nargs, NArgs::Fixed(0))
        && (contains_nested_kwarg(spec, token) || contains_nested_flag(spec, token))
}

fn try_format_inline(
    command: &CommandInvocation,
    sections: &[Section<'_>],
    cmd_config: &CommandConfig<'_>,
    block_depth: usize,
    line_width: usize,
) -> Option<String> {
    if command.arguments.iter().any(argument_has_newline)
        || command.arguments.iter().any(Argument::is_comment)
    {
        return None;
    }

    if sections
        .iter()
        .any(|section| section.arguments.len() > cmd_config.max_pargs_hwrap())
    {
        return None;
    }

    let base_indent = cmd_config.indent_str().repeat(block_depth);
    let mut output = format!("{base_indent}{}(", format_name(command, cmd_config));

    let mut first_token = true;
    for section in sections {
        if let Some(header) = section.header {
            if !first_token {
                output.push(' ');
            }
            output.push_str(&apply_case(cmd_config.keyword_case(), header));
            first_token = false;
        }

        for argument in &section.arguments {
            if !first_token {
                output.push(' ');
            }
            output.push_str(argument.as_str());
            first_token = false;
        }
    }

    output.push(')');
    (output.chars().count() <= line_width).then_some(output)
}

fn try_format_hanging(
    command: &CommandInvocation,
    sections: &[Section<'_>],
    cmd_config: &CommandConfig<'_>,
    _patterns: &CompiledPatterns,
    block_depth: usize,
    line_width: usize,
) -> Option<String> {
    if command.arguments.iter().any(Argument::is_comment)
        || command.arguments.iter().any(argument_has_newline)
    {
        return None;
    }

    if sections.len() != 1 || sections[0].header.is_some() {
        return None;
    }

    let is_condition_command = is_condition_command(&command.name);

    if !is_condition_command && sections[0].arguments.len() > cmd_config.max_pargs_hwrap() {
        return None;
    }

    let base_indent = cmd_config.indent_str().repeat(block_depth);
    let prefix = format!("{base_indent}{}(", format_name(command, cmd_config));
    let continuation = " ".repeat(prefix.chars().count());
    let tokens: Vec<&str> = sections[0]
        .arguments
        .iter()
        .map(|argument| argument.as_str())
        .collect();
    let break_before = match_condition_breaks(&command.name);

    let mut lines = pack_tokens(
        &prefix,
        &continuation,
        &tokens,
        line_width,
        cmd_config.global().max_lines_hwrap,
        break_before,
    )?;
    // Reject the hanging layout if it produces more rows than the cmdline
    // threshold allows.
    if lines.len() > cmd_config.global().max_rows_cmdline {
        return None;
    }
    if lines.len() == 1 {
        lines[0].push(')');
        return Some(lines.remove(0));
    }

    Some(close_multiline(
        lines,
        &base_indent,
        format_name(command, cmd_config).len(),
        cmd_config,
    ))
}

fn format_command_vertical(
    command: &CommandInvocation,
    sections: &[Section<'_>],
    cmd_config: &CommandConfig<'_>,
    patterns: &CompiledPatterns,
    block_depth: usize,
    wrap_after_first_arg: bool,
) -> Result<String> {
    let base_indent = cmd_config.indent_str().repeat(block_depth);
    let indent = format!("{base_indent}{}", cmd_config.indent_str());
    let nested_indent = format!("{indent}{}", cmd_config.indent_str());
    let mut output = String::new();

    let name = format_name(command, cmd_config);
    output.push_str(&base_indent);
    output.push_str(&name);

    // When wrap_after_first_arg is enabled and the first section is
    // positional (no keyword header), keep the first argument on the
    // command line and align the rest to the open parenthesis.
    let first_is_positional = sections
        .first()
        .is_some_and(|s| s.header.is_none() && !s.arguments.is_empty());

    if wrap_after_first_arg && first_is_positional {
        let first_section = &sections[0];

        // Find the first non-comment argument to keep on the command line.
        let first_real_idx = first_section
            .arguments
            .iter()
            .position(|a| !a.is_comment())
            .unwrap_or(0);
        let first_arg = first_section.arguments[first_real_idx];
        let paren_indent = " ".repeat(base_indent.len() + name.len() + 1);

        output.push('(');
        output.push_str(first_arg.as_str());

        // If the next argument is an inline comment, try to keep it attached.
        let mut consumed = first_real_idx + 1;
        if consumed < first_section.arguments.len()
            && first_section.arguments[consumed].is_comment()
        {
            let comment = first_section.arguments[consumed].as_str();
            let line_so_far = base_indent.len() + name.len() + 1 + first_arg.as_str().len();
            if line_so_far + 1 + comment.len() <= cmd_config.line_width() {
                output.push(' ');
                output.push_str(comment);
                consumed += 1;
            }
        }

        // Remaining arguments in the first section — try to pack them on
        // the same line as the first arg before wrapping to a new line.
        // Skip inline packing if the line already ends with a comment.
        let remaining = &first_section.arguments[consumed..];
        let line_has_comment = output.lines().last().is_some_and(|l| l.contains('#'));

        if !remaining.is_empty() {
            let line_so_far = output.lines().last().map_or(0, |l| l.len());
            let mut inline_candidate = String::new();
            let mut fits_inline = !line_has_comment;
            let mut candidate_width = line_so_far;
            if fits_inline {
                for arg in remaining {
                    if arg.is_comment() {
                        fits_inline = false;
                        break;
                    }
                    let token = arg.as_str();
                    let token_width = token.chars().count();
                    if candidate_width + 1 + token_width > cmd_config.line_width() {
                        fits_inline = false;
                        break;
                    }
                    inline_candidate.push(' ');
                    inline_candidate.push_str(token);
                    candidate_width += 1 + token_width;
                }
            }
            if fits_inline {
                output.push_str(&inline_candidate);
                if sections.len() > 1 {
                    output.push('\n');
                }
            } else {
                // Either they don't fit or there are keyword sections that
                // will follow — wrap to aligned lines.
                output.push('\n');
                if remaining.len() > cmd_config.max_pargs_hwrap() {
                    write_vertical_arguments(
                        &mut output,
                        remaining,
                        &paren_indent,
                        cmd_config.global(),
                        patterns,
                    );
                } else {
                    write_packed_arguments(
                        &mut output,
                        remaining,
                        &paren_indent,
                        cmd_config.global(),
                        patterns,
                        cmd_config.line_width(),
                    );
                }
            }
        } else if sections.len() > 1 {
            output.push('\n');
        }

        // Remaining sections (keywords, flags).
        for section in &sections[1..] {
            match section.header {
                None => {
                    if section.arguments.len() > cmd_config.max_pargs_hwrap() {
                        write_vertical_arguments(
                            &mut output,
                            &section.arguments,
                            &paren_indent,
                            cmd_config.global(),
                            patterns,
                        );
                    } else {
                        write_packed_arguments(
                            &mut output,
                            &section.arguments,
                            &paren_indent,
                            cmd_config.global(),
                            patterns,
                            cmd_config.line_width(),
                        );
                    }
                }
                Some(header) => {
                    let header = apply_case(cmd_config.keyword_case(), header);
                    let kw_nested = format!("{paren_indent}{}", cmd_config.indent_str());
                    if section.arguments.is_empty() {
                        output.push_str(&paren_indent);
                        output.push_str(&header);
                        output.push('\n');
                        continue;
                    }
                    output.push_str(&paren_indent);
                    output.push_str(&header);
                    if section.arguments.len() > cmd_config.max_pargs_hwrap() {
                        output.push('\n');
                        write_vertical_arguments(
                            &mut output,
                            &section.arguments,
                            &kw_nested,
                            cmd_config.global(),
                            patterns,
                        );
                    } else if let Some(line) = format_section_inline(
                        &header,
                        section.header_kind,
                        &section.arguments,
                        &paren_indent,
                        cmd_config.global(),
                        patterns,
                        cmd_config.line_width(),
                    ) {
                        output.truncate(output.len() - header.len());
                        output.push_str(&line);
                        output.push('\n');
                    } else {
                        output.push('\n');
                        write_packed_arguments(
                            &mut output,
                            &section.arguments,
                            &kw_nested,
                            cmd_config.global(),
                            patterns,
                            cmd_config.line_width(),
                        );
                    }
                }
            }
        }

        // Close the command.
        if output.ends_with('\n') {
            output.pop();
        }
        if cmd_config.dangle_parens() {
            output.push('\n');
            match cmd_config.dangle_align() {
                DangleAlign::Prefix | DangleAlign::Close => output.push_str(&base_indent),
                DangleAlign::Open => {
                    output.push_str(&base_indent);
                    output.push_str(&" ".repeat(name.len()));
                }
            }
            output.push(')');
        } else if last_output_line_has_comment(&output) {
            output.push('\n');
            output.push_str(&base_indent);
            output.push(')');
        } else {
            output.push(')');
        }
        return Ok(output);
    }

    output.push_str("(\n");

    for section in sections {
        match section.header {
            None => {
                if section.arguments.len() > cmd_config.max_pargs_hwrap() {
                    write_vertical_arguments(
                        &mut output,
                        &section.arguments,
                        &indent,
                        cmd_config.global(),
                        patterns,
                    );
                } else {
                    write_packed_arguments(
                        &mut output,
                        &section.arguments,
                        &indent,
                        cmd_config.global(),
                        patterns,
                        cmd_config.line_width(),
                    );
                }
            }
            Some(header) => {
                let header = apply_case(cmd_config.keyword_case(), header);
                if section.arguments.is_empty() {
                    output.push_str(&indent);
                    output.push_str(&header);
                    output.push('\n');
                    continue;
                }

                output.push_str(&indent);
                output.push_str(&header);
                if section.arguments.len() > cmd_config.max_pargs_hwrap() {
                    output.push('\n');
                    write_vertical_arguments(
                        &mut output,
                        &section.arguments,
                        &nested_indent,
                        cmd_config.global(),
                        patterns,
                    );
                } else {
                    if let Some(line) = format_section_inline(
                        &header,
                        section.header_kind,
                        &section.arguments,
                        &indent,
                        cmd_config.global(),
                        patterns,
                        cmd_config.line_width(),
                    ) {
                        output.truncate(output.len() - header.len());
                        output.push_str(&line);
                        output.push('\n');
                    } else {
                        output.push('\n');
                        write_packed_arguments(
                            &mut output,
                            &section.arguments,
                            &nested_indent,
                            cmd_config.global(),
                            patterns,
                            cmd_config.line_width(),
                        );
                    }
                }
            }
        }
    }

    if output.ends_with('\n') {
        output.pop();
    }

    if cmd_config.dangle_parens() {
        output.push('\n');
        match cmd_config.dangle_align() {
            DangleAlign::Prefix | DangleAlign::Close => output.push_str(&base_indent),
            DangleAlign::Open => {
                output.push_str(&base_indent);
                output.push_str(&" ".repeat(name.len()));
            }
        }
        output.push(')');
    } else if last_output_line_has_comment(&output) {
        output.push('\n');
        output.push_str(&base_indent);
        output.push(')');
    } else {
        output.push(')');
    }

    Ok(output)
}

fn format_section_inline(
    header: &str,
    header_kind: Option<HeaderKind>,
    arguments: &[&Argument],
    indent: &str,
    config: &Config,
    patterns: &CompiledPatterns,
    line_width: usize,
) -> Option<String> {
    if arguments
        .iter()
        .any(|argument| argument_has_newline(argument))
    {
        return None;
    }

    let indent_width = indent.chars().count();
    let mut line = String::from(header);
    let mut line_width_count = line.chars().count();
    let comment_indent = indent_width + line_width_count;

    for (index, argument) in arguments.iter().enumerate() {
        match argument {
            Argument::InlineComment(comment) => {
                if index + 1 != arguments.len() {
                    return None;
                }
                let comment_lines = comment::format_comment_lines(
                    comment,
                    config,
                    patterns,
                    comment_indent + 1,
                    line_width,
                );
                if comment_lines.len() != 1 {
                    return None;
                }

                let mut candidate = String::with_capacity(line.len() + 1 + comment_lines[0].len());
                candidate.push_str(&line);
                candidate.push(' ');
                candidate.push_str(&comment_lines[0]);
                let candidate_width = line_width_count + 1 + comment_lines[0].chars().count();
                if indent_width + candidate_width > line_width {
                    return None;
                }
                line = candidate;
                line_width_count = candidate_width;
            }
            _ => {
                let token = argument.as_str();
                let token_width = token.chars().count();
                let candidate_width = if line.is_empty() {
                    token_width
                } else {
                    line_width_count + 1 + token_width
                };
                if indent_width + candidate_width > line_width {
                    if matches!(header_kind, Some(HeaderKind::Flag)) && arguments.len() == 1 {
                        return None;
                    }
                    return None;
                }
                if line.is_empty() {
                    line.push_str(token);
                } else {
                    line.push(' ');
                    line.push_str(token);
                }
                line_width_count = candidate_width;
            }
        }
    }

    Some(line)
}

fn write_packed_arguments(
    output: &mut String,
    arguments: &[&Argument],
    indent: &str,
    config: &Config,
    patterns: &CompiledPatterns,
    line_width: usize,
) {
    let mut current = String::new();
    let indent_width = indent.chars().count();
    let mut current_width = 0usize;

    for argument in arguments {
        match argument {
            Argument::InlineComment(comment) => {
                let comment_lines = comment::format_comment_lines(
                    comment,
                    config,
                    patterns,
                    indent.chars().count(),
                    line_width,
                );
                if comment_lines.len() == 1 && !current.is_empty() {
                    let comment_width = comment_lines[0].chars().count();
                    let candidate_width = current_width + 1 + comment_width;
                    if indent_width + candidate_width <= line_width {
                        // Append comment inline and flush — nothing can
                        // follow a trailing comment on the same line.
                        current.push(' ');
                        current.push_str(&comment_lines[0]);
                        flush_current_line(output, &mut current, indent);
                        current_width = 0;
                        continue;
                    }
                }

                flush_current_line(output, &mut current, indent);
                current_width = 0;
                for line in comment_lines {
                    output.push_str(indent);
                    output.push_str(&line);
                    output.push('\n');
                }
            }
            _ if argument_has_newline(argument) => {
                flush_current_line(output, &mut current, indent);
                current_width = 0;
                write_multiline_argument(output, indent, argument.as_str());
            }
            _ => {
                let token = argument.as_str();
                let token_width = token.chars().count();
                let candidate_width = if current.is_empty() {
                    token_width
                } else {
                    current_width + 1 + token_width
                };

                if current.is_empty() || indent_width + candidate_width <= line_width {
                    if current.is_empty() {
                        current.push_str(token);
                    } else {
                        current.push(' ');
                        current.push_str(token);
                    }
                    current_width = candidate_width;
                } else {
                    flush_current_line(output, &mut current, indent);
                    current_width = token_width;
                    current = token.to_owned();
                }
            }
        }
    }

    flush_current_line(output, &mut current, indent);
}

fn write_vertical_arguments(
    output: &mut String,
    arguments: &[&Argument],
    indent: &str,
    config: &Config,
    patterns: &CompiledPatterns,
) {
    for argument in arguments {
        match argument {
            Argument::InlineComment(comment) => {
                let comment_text = comment.as_str();

                // Try to keep the comment on the same line as the preceding
                // argument. This preserves the common pattern:
                //   dep1 # first dep
                //   dep2 # second dep
                if output.ends_with('\n') {
                    let last_line_start =
                        output[..output.len() - 1].rfind('\n').map_or(0, |p| p + 1);
                    let last_line_width = output[last_line_start..output.len() - 1].chars().count();
                    let comment_width = comment_text.chars().count();
                    if last_line_width + 1 + comment_width <= config.line_width {
                        output.pop(); // remove trailing newline
                        output.push(' ');
                        output.push_str(comment_text);
                        output.push('\n');
                        continue;
                    }
                }

                // Comment doesn't fit inline — render on its own line(s).
                for line in comment::format_comment_lines(
                    comment,
                    config,
                    patterns,
                    indent.chars().count(),
                    config.line_width,
                ) {
                    output.push_str(indent);
                    output.push_str(&line);
                    output.push('\n');
                }
            }
            _ if argument_has_newline(argument) => {
                write_multiline_argument(output, indent, argument.as_str())
            }
            _ => {
                output.push_str(indent);
                output.push_str(argument.as_str());
                output.push('\n');
            }
        }
    }
}

fn write_multiline_argument(output: &mut String, indent: &str, source: &str) {
    let normalized = source.replace("\r\n", "\n");
    let mut lines = normalized.split('\n');

    output.push_str(indent);
    output.push_str(lines.next().unwrap_or_default());
    output.push('\n');

    for line in lines {
        output.push_str(line);
        output.push('\n');
    }
}

fn flush_current_line(output: &mut String, current: &mut String, indent: &str) {
    if current.is_empty() {
        return;
    }

    output.push_str(indent);
    output.push_str(current);
    output.push('\n');
    current.clear();
}

fn pack_tokens(
    prefix: &str,
    continuation: &str,
    tokens: &[&str],
    line_width: usize,
    max_lines: usize,
    break_before: &[&str],
) -> Option<Vec<String>> {
    if tokens.is_empty() {
        return Some(vec![prefix.to_owned()]);
    }

    let prefix_width = prefix.chars().count();
    let continuation_width = continuation.chars().count();
    let mut lines = vec![prefix.to_owned()];
    let mut current_width = prefix_width;

    for &token in tokens {
        if break_before
            .iter()
            .any(|candidate| token.eq_ignore_ascii_case(candidate))
            && lines.last().is_some_and(|line| line != prefix)
            && lines.len() < max_lines
        {
            let mut next = String::with_capacity(continuation.len() + token.len());
            next.push_str(continuation);
            next.push_str(token);
            lines.push(next);
            current_width = continuation_width + token.chars().count();
            continue;
        }

        let current = lines.last_mut().expect("at least one line");
        let needs_space = current_width != prefix_width && current_width != continuation_width;
        let candidate_width = current_width + usize::from(needs_space) + token.chars().count();

        if candidate_width <= line_width {
            if needs_space {
                current.push(' ');
            }
            current.push_str(token);
            current_width = candidate_width;
            continue;
        }

        if lines.len() >= max_lines {
            return None;
        }

        let mut next = String::with_capacity(continuation.len() + token.len());
        next.push_str(continuation);
        next.push_str(token);
        lines.push(next);
        current_width = continuation_width + token.chars().count();
    }

    Some(lines)
}

fn close_multiline(
    mut lines: Vec<String>,
    base_indent: &str,
    name_len: usize,
    cmd_config: &CommandConfig<'_>,
) -> String {
    if cmd_config.dangle_parens() {
        let closer = match cmd_config.dangle_align() {
            DangleAlign::Prefix | DangleAlign::Close => format!("{base_indent})"),
            DangleAlign::Open => format!("{base_indent}{}{})", " ".repeat(name_len), ""),
        };
        lines.push(closer);
        return lines.join("\n");
    }

    if lines.last().is_some_and(|last| last.contains('#')) {
        lines.push(format!("{base_indent})"));
        lines.join("\n")
    } else {
        if let Some(last) = lines.last_mut() {
            last.push(')');
        }
        lines.join("\n")
    }
}

fn last_output_line_has_comment(output: &str) -> bool {
    output.lines().last().is_some_and(|line| line.contains('#'))
}

fn argument_has_newline(argument: &Argument) -> bool {
    argument.as_str().contains('\n') || argument.as_str().contains('\r')
}

fn apply_case(style: CaseStyle, s: &str) -> String {
    match style {
        CaseStyle::Lower => s.to_ascii_lowercase(),
        CaseStyle::Upper => s.to_ascii_uppercase(),
        CaseStyle::Unchanged => s.to_string(),
    }
}

fn has_ascii_lowercase(s: &str) -> bool {
    s.bytes().any(|byte| byte.is_ascii_lowercase())
}

fn lookup_kwarg<'a>(form: &'a CommandForm, token: &str) -> Option<&'a crate::spec::KwargSpec> {
    form.kwargs.get(token).or_else(|| {
        has_ascii_lowercase(token)
            .then(|| token.to_ascii_uppercase())
            .and_then(|normalized| form.kwargs.get(&normalized))
    })
}

fn contains_kwarg(form: &CommandForm, token: &str) -> bool {
    lookup_kwarg(form, token).is_some()
}

fn contains_flag(form: &CommandForm, token: &str) -> bool {
    form.flags.contains(token)
        || (has_ascii_lowercase(token) && form.flags.contains(&token.to_ascii_uppercase()))
}

fn contains_nested_kwarg(spec: &crate::spec::KwargSpec, token: &str) -> bool {
    spec.kwargs.get(token).is_some()
        || (has_ascii_lowercase(token) && spec.kwargs.contains_key(&token.to_ascii_uppercase()))
}

fn contains_nested_flag(spec: &crate::spec::KwargSpec, token: &str) -> bool {
    spec.flags.contains(token)
        || (has_ascii_lowercase(token) && spec.flags.contains(&token.to_ascii_uppercase()))
}

fn is_condition_command(name: &str) -> bool {
    !match_condition_breaks(name).is_empty()
}

fn match_condition_breaks(name: &str) -> &'static [&'static str] {
    if name.eq_ignore_ascii_case("if")
        || name.eq_ignore_ascii_case("elseif")
        || name.eq_ignore_ascii_case("while")
    {
        &["AND", "OR"]
    } else {
        &[]
    }
}

/// Replace leading spaces with tab characters.
fn spaces_to_tabs(output: &str, tab_size: usize, policy: FractionalTabPolicy) -> String {
    if tab_size == 0 {
        return output.to_string();
    }

    let mut result = String::with_capacity(output.len());
    for (i, line) in output.split('\n').enumerate() {
        if i > 0 {
            result.push('\n');
        }
        let leading = line.len() - line.trim_start_matches(' ').len();
        let tabs = leading / tab_size;
        let remaining = leading % tab_size;
        for _ in 0..tabs {
            result.push('\t');
        }
        match policy {
            FractionalTabPolicy::UseSpace => {
                for _ in 0..remaining {
                    result.push(' ');
                }
            }
            FractionalTabPolicy::RoundUp => {
                if remaining > 0 {
                    result.push('\t');
                }
            }
        }
        result.push_str(&line[leading..]);
    }
    result
}
