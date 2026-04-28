// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Command-invocation formatting logic.

use crate::config::{
    apply_case, CommandConfig, CompiledPatterns, Config, DangleAlign, FractionalTabPolicy,
};
use crate::error::Result;
use crate::formatter::comment;
use crate::parser::ast::{Argument, CommandInvocation};
use crate::spec::registry::CommandRegistry;
use crate::spec::{has_ascii_lowercase, CommandForm, CommandSpec, NArgs};

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

    let spec_continuation = form.layout.as_ref().and_then(|l| l.continuation_align);
    let continuation_align = cmd_config.continuation_align(spec_continuation);

    let output = if force_vertical {
        debug.log(format!(
            "formatter: command {} layout=vertical (always_wrap)",
            command.name
        ));
        format_command_vertical(
            command,
            &sections,
            form,
            &cmd_config,
            patterns,
            block_depth,
            wrap_after_first_arg,
            continuation_align,
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
            form,
            &cmd_config,
            patterns,
            block_depth,
            wrap_after_first_arg,
            continuation_align,
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
    // Force-attach the next `pending_consume` non-comment tokens to the
    // current section, regardless of how they classify. This prevents a
    // subkwarg's value (e.g. `Runtime` after `COMPONENT`) from being
    // mis-parsed as an ancestor kwarg that happens to share the name.
    let mut pending_consume: usize = 0;

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

        if pending_consume > 0 {
            sections
                .last_mut()
                .expect("section list contains at least one section")
                .arguments
                .push(argument);
            pending_consume -= 1;
            continue;
        }

        if nested_token_belongs_to_current_section(&sections, form, token) {
            sections
                .last_mut()
                .expect("section list contains at least one section")
                .arguments
                .push(argument);
            pending_consume = nested_kwarg_forced_nargs(&sections, form, token);
            continue;
        }

        let header_kind = classify_token(form, token);

        if let Some(header_kind) = header_kind {
            sections.push(Section {
                header: Some(token),
                header_kind: Some(header_kind),
                arguments: Vec::new(),
            });
            pending_consume = form_kwarg_forced_nargs(form, token);
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

/// Minimum number of positional-argument tokens that must be
/// force-consumed after a kwarg so its values are never reinterpreted
/// as sibling or ancestor keywords. `OneOrMore` requires at least 1
/// (per CMake semantics). `ZeroOrMore` and `Optional` have no minimum.
fn forced_consumption_count(nargs: &NArgs) -> usize {
    match nargs {
        NArgs::Fixed(n) => *n,
        NArgs::AtLeast(n) => *n,
        NArgs::OneOrMore => 1,
        NArgs::ZeroOrMore | NArgs::Optional => 0,
    }
}

fn form_kwarg_forced_nargs(form: &CommandForm, token: &str) -> usize {
    lookup_kwarg(form, token)
        .map(|spec| forced_consumption_count(&spec.nargs))
        .unwrap_or(0)
}

fn nested_kwarg_forced_nargs(sections: &[Section<'_>], form: &CommandForm, token: &str) -> usize {
    let Some(section) = sections.last() else {
        return 0;
    };
    let Some(header) = section.header else {
        return 0;
    };
    let Some(parent) = lookup_kwarg(form, header) else {
        return 0;
    };
    let spec = parent.kwargs.get(token).or_else(|| {
        has_ascii_lowercase(token)
            .then(|| token.to_ascii_uppercase())
            .and_then(|normalized| parent.kwargs.get(&normalized))
    });
    spec.map(|s| forced_consumption_count(&s.nargs))
        .unwrap_or(0)
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

        let header_spec = form
            .kwargs
            .get(&header.to_ascii_uppercase())
            .or_else(|| form.kwargs.get(header));

        // Sections whose header spec carries nested subkwargs or
        // nested flags can't be flat-sorted — the sort would separate
        // a subkwarg from its value or move a nested flag across a
        // kwarg boundary, silently changing the command's semantics
        // (e.g. FILE_SET HEADERS DESTINATION include COMPONENT
        // Development). Skip sorting in those cases regardless of
        // whether the sort request came from `sortable = true` in
        // the spec or the autosort heuristic.
        //
        // Pure flat-list kwargs (PUBLIC, ITEMS, …) have neither
        // nested kwargs nor nested flags and remain sortable.
        let structural_section =
            header_spec.is_some_and(|spec| !spec.kwargs.is_empty() || !spec.flags.is_empty());
        if structural_section {
            continue;
        }

        // Check if the spec marks this keyword section as sortable.
        let spec_sortable = header_spec.is_some_and(|kwarg| kwarg.sortable);

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

            let mut sortable_args: Vec<(String, &Argument)> = non_comment_positions
                .iter()
                .map(|&i| {
                    let arg = section.arguments[i];
                    (arg.as_str().to_ascii_lowercase(), arg)
                })
                .collect();

            sortable_args.sort_by(|(key_a, _), (key_b, _)| key_a.cmp(key_b));

            for (j, &pos) in non_comment_positions.iter().enumerate() {
                section.arguments[pos] = sortable_args[j].1;
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

    // A kwarg header that declares nested kwargs/flags (e.g. INCLUDES, or
    // any of the install(TARGETS) artifact-kind subgroups, or FILE_SET
    // after its positional set-name has been force-consumed) accepts
    // nested tokens. Kwargs without nested declarations short-circuit
    // here — is_nested_keyword_or_flag returns false.
    is_nested_keyword_or_flag(spec, token)
}

fn try_format_inline(
    command: &CommandInvocation,
    sections: &[Section<'_>],
    cmd_config: &CommandConfig<'_>,
    block_depth: usize,
    line_width: usize,
) -> Option<String> {
    if command
        .arguments
        .iter()
        .any(|a| argument_has_newline(a) || a.is_comment())
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
    if command
        .arguments
        .iter()
        .any(|a| a.is_comment() || argument_has_newline(a))
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

#[allow(clippy::too_many_arguments)]
fn format_command_vertical(
    command: &CommandInvocation,
    sections: &[Section<'_>],
    form: &CommandForm,
    cmd_config: &CommandConfig<'_>,
    patterns: &CompiledPatterns,
    block_depth: usize,
    wrap_after_first_arg: bool,
    continuation_align: crate::config::ContinuationAlign,
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
                Some(header_raw) => {
                    let header = apply_case(cmd_config.keyword_case(), header_raw);
                    let kw_nested = format!("{paren_indent}{}", cmd_config.indent_str());
                    if section.arguments.is_empty() {
                        output.push_str(&paren_indent);
                        output.push_str(&header);
                        output.push('\n');
                        continue;
                    }
                    output.push_str(&paren_indent);
                    output.push_str(&header);
                    let parent_spec = lookup_kwarg(form, header_raw);
                    let grouped_spec = parent_spec.filter(|s| !s.kwargs.is_empty());
                    if section.arguments.len() > cmd_config.max_pargs_hwrap() {
                        if let Some(spec) = grouped_spec {
                            write_header_line_and_group(
                                &mut output,
                                &section.arguments,
                                spec,
                                &kw_nested,
                                cmd_config.global(),
                                patterns,
                                cmd_config.line_width(),
                                continuation_align,
                            );
                        } else {
                            output.push('\n');
                            write_vertical_arguments(
                                &mut output,
                                &section.arguments,
                                &kw_nested,
                                cmd_config.global(),
                                patterns,
                            );
                        }
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
                    } else if let Some(spec) = grouped_spec {
                        write_header_line_and_group(
                            &mut output,
                            &section.arguments,
                            spec,
                            &kw_nested,
                            cmd_config.global(),
                            patterns,
                            cmd_config.line_width(),
                            continuation_align,
                        );
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

        close_command_output(&mut output, cmd_config, &base_indent, &name);
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
            Some(header_raw) => {
                let header = apply_case(cmd_config.keyword_case(), header_raw);
                if section.arguments.is_empty() {
                    output.push_str(&indent);
                    output.push_str(&header);
                    output.push('\n');
                    continue;
                }

                output.push_str(&indent);
                output.push_str(&header);
                let parent_spec = lookup_kwarg(form, header_raw);
                let grouped_spec = parent_spec.filter(|s| !s.kwargs.is_empty());
                if section.arguments.len() > cmd_config.max_pargs_hwrap() {
                    if let Some(spec) = grouped_spec {
                        write_header_line_and_group(
                            &mut output,
                            &section.arguments,
                            spec,
                            &nested_indent,
                            cmd_config.global(),
                            patterns,
                            cmd_config.line_width(),
                            continuation_align,
                        );
                    } else {
                        output.push('\n');
                        write_vertical_arguments(
                            &mut output,
                            &section.arguments,
                            &nested_indent,
                            cmd_config.global(),
                            patterns,
                        );
                    }
                } else if let Some(line) = format_section_inline(
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
                } else if let Some(spec) = grouped_spec {
                    write_header_line_and_group(
                        &mut output,
                        &section.arguments,
                        spec,
                        &nested_indent,
                        cmd_config.global(),
                        patterns,
                        cmd_config.line_width(),
                        continuation_align,
                    );
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

    close_command_output(&mut output, cmd_config, &base_indent, &name);

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
    write_packed_arguments_with_continuation(
        output, arguments, indent, indent, config, patterns, line_width,
    );
}

/// Same as [`write_packed_arguments`], but uses a separate indent for
/// continuation (wrap) lines. When `first_indent == continuation_indent`
/// the behaviour is identical to [`write_packed_arguments`]. Used by
/// the pair-aware grouped writer to implement
/// [`crate::config::ContinuationAlign::UnderFirstValue`] — continuation
/// lines align under the first value column after the subkwarg, rather
/// than at the subkwarg's own indent.
#[allow(clippy::too_many_arguments)]
fn write_packed_arguments_with_continuation(
    output: &mut String,
    arguments: &[&Argument],
    first_indent: &str,
    continuation_indent: &str,
    config: &Config,
    patterns: &CompiledPatterns,
    line_width: usize,
) {
    let mut current = String::new();
    let mut used_first_line = false;

    let indent_for = |used_first_line: bool| -> &str {
        if used_first_line {
            continuation_indent
        } else {
            first_indent
        }
    };

    let mut current_indent_width = first_indent.chars().count();
    let mut current_width = 0usize;

    let flush = |output: &mut String,
                 current: &mut String,
                 used_first_line: &mut bool,
                 current_indent_width: &mut usize| {
        if current.is_empty() {
            return;
        }
        let ind = if *used_first_line {
            continuation_indent
        } else {
            first_indent
        };
        output.push_str(ind);
        output.push_str(current);
        output.push('\n');
        current.clear();
        *used_first_line = true;
        *current_indent_width = continuation_indent.chars().count();
    };

    for argument in arguments {
        match argument {
            Argument::InlineComment(comment) => {
                let comment_lines = comment::format_comment_lines(
                    comment,
                    config,
                    patterns,
                    current_indent_width,
                    line_width,
                );
                if comment_lines.len() == 1 && !current.is_empty() {
                    let comment_width = comment_lines[0].chars().count();
                    let candidate_width = current_width + 1 + comment_width;
                    if current_indent_width + candidate_width <= line_width {
                        current.push(' ');
                        current.push_str(&comment_lines[0]);
                        flush(
                            output,
                            &mut current,
                            &mut used_first_line,
                            &mut current_indent_width,
                        );
                        current_width = 0;
                        continue;
                    }
                }

                flush(
                    output,
                    &mut current,
                    &mut used_first_line,
                    &mut current_indent_width,
                );
                current_width = 0;
                for line in comment_lines {
                    output.push_str(indent_for(used_first_line));
                    output.push_str(&line);
                    output.push('\n');
                    used_first_line = true;
                    current_indent_width = continuation_indent.chars().count();
                }
            }
            _ if argument_has_newline(argument) => {
                flush(
                    output,
                    &mut current,
                    &mut used_first_line,
                    &mut current_indent_width,
                );
                current_width = 0;
                write_multiline_argument(output, indent_for(used_first_line), argument.as_str());
                used_first_line = true;
                current_indent_width = continuation_indent.chars().count();
            }
            _ => {
                let token = argument.as_str();
                let token_width = token.chars().count();
                let candidate_width = if current.is_empty() {
                    token_width
                } else {
                    current_width + 1 + token_width
                };

                if current.is_empty() || current_indent_width + candidate_width <= line_width {
                    if current.is_empty() {
                        current.push_str(token);
                    } else {
                        current.push(' ');
                        current.push_str(token);
                    }
                    current_width = candidate_width;
                } else {
                    flush(
                        output,
                        &mut current,
                        &mut used_first_line,
                        &mut current_indent_width,
                    );
                    current_width = token_width;
                    current = token.to_owned();
                }
            }
        }
    }

    flush(
        output,
        &mut current,
        &mut used_first_line,
        &mut current_indent_width,
    );
}

/// Pair-aware writer used when the surrounding section header declares
/// nested kwargs (e.g. `install(TARGETS ... LIBRARY ...)` artifact-kind
/// subgroups). Each nested kwarg and its forced-nargs value(s) are
/// rendered as a single logical line so that `COMPONENT Runtime` and
/// `NAMELINK_COMPONENT Development` never split across lines.
#[allow(clippy::too_many_arguments)]
fn write_grouped_arguments(
    output: &mut String,
    arguments: &[&Argument],
    indent: &str,
    parent_spec: &crate::spec::KwargSpec,
    config: &Config,
    patterns: &CompiledPatterns,
    line_width: usize,
    continuation_align: crate::config::ContinuationAlign,
) {
    let mut i = 0;
    while i < arguments.len() {
        let argument = arguments[i];
        if argument.is_comment() || argument_has_newline(argument) {
            // Defer non-token arguments to the packed writer one at a time
            // so existing comment and multi-line handling still applies.
            write_packed_arguments(
                output,
                std::slice::from_ref(&arguments[i]),
                indent,
                config,
                patterns,
                line_width,
            );
            i += 1;
            continue;
        }

        let end = group_end(arguments, i, parent_spec);
        let group = &arguments[i..end];

        // Under UnderFirstValue, wrap lines of a subkwarg group land
        // aligned under the column of the first value after the
        // subkwarg token: `indent + subkwarg + " "`.
        let hanging_indent = (continuation_align
            == crate::config::ContinuationAlign::UnderFirstValue
            && group.len() > 1
            && lookup_nested_kwarg_in(parent_spec, group[0].as_str()).is_some())
        .then(|| {
            let header_width = group[0].as_str().chars().count();
            format!("{indent}{}", " ".repeat(header_width + 1))
        });

        if let Some(continuation) = hanging_indent.as_deref() {
            write_packed_arguments_with_continuation(
                output,
                group,
                indent,
                continuation,
                config,
                patterns,
                line_width,
            );
        } else {
            write_packed_arguments(output, group, indent, config, patterns, line_width);
        }
        i = end;
    }
}

/// Count of non-comment arguments that must stay attached to the section
/// header line because they are positionals of the header kwarg itself
/// (e.g. `FILE_SET HEADERS` or `PATTERN *.h`).
fn header_positional_count(parent_spec: &crate::spec::KwargSpec) -> usize {
    match &parent_spec.nargs {
        NArgs::Fixed(n) => *n,
        NArgs::AtLeast(n) => *n,
        NArgs::OneOrMore => 1,
        NArgs::Optional | NArgs::ZeroOrMore => 0,
    }
}

/// Split off any leading inline single-line comments from `arguments`.
/// Callers carry these onto the preceding section header line so that
/// trailing comments like `RUNTIME # runtime artifacts` stay attached
/// to the header rather than floating onto their own line above the
/// grouped subkwargs.
fn split_leading_inline_line_comments<'a, 'b>(
    arguments: &'b [&'a Argument],
) -> (&'b [&'a Argument], &'b [&'a Argument]) {
    let split = arguments
        .iter()
        .position(|arg| !is_single_line_inline_comment(arg))
        .unwrap_or(arguments.len());
    (&arguments[..split], &arguments[split..])
}

fn is_single_line_inline_comment(argument: &Argument) -> bool {
    matches!(
        argument,
        Argument::InlineComment(crate::parser::ast::Comment::Line(_))
    )
}

/// Character width of the last line of `output` (i.e. the part after
/// the most recent `\n`), used to decide whether an inline trailing
/// comment still fits within the configured line budget.
fn current_line_char_count(output: &str) -> usize {
    output
        .rsplit('\n')
        .next()
        .map_or(0, |tail| tail.chars().count())
}

/// Write the header's own positional args (if any) on the same line as
/// the section header, carry any leading inline single-line comments
/// onto that same line, then newline and hand the remainder off to the
/// pair-aware grouped writer.
///
/// Crucially, any comments interleaved *before* the header's required
/// positionals are emitted *after* the positionals — a line comment
/// extends to end-of-line in CMake, so placing a positional token
/// after one would make CMake parse the positional as comment text
/// and silently change the command's semantics.
#[allow(clippy::too_many_arguments)]
fn write_header_line_and_group(
    output: &mut String,
    arguments: &[&Argument],
    spec: &crate::spec::KwargSpec,
    inner_indent: &str,
    config: &Config,
    patterns: &CompiledPatterns,
    line_width: usize,
    continuation_align: crate::config::ContinuationAlign,
) {
    let positional_count = header_positional_count(spec);

    // Walk the prefix that would naturally live on the header line,
    // separating non-comment positional args from any comments found
    // among them. Comments are deferred so that no positional is ever
    // emitted after a line comment.
    let mut positionals: Vec<&Argument> = Vec::new();
    let mut deferred_comments: Vec<&Argument> = Vec::new();
    let mut cut_at = 0usize;
    for (idx, arg) in arguments.iter().enumerate() {
        if positionals.len() == positional_count {
            cut_at = idx;
            break;
        }
        cut_at = idx + 1;
        if arg.is_comment() {
            deferred_comments.push(*arg);
        } else {
            positionals.push(*arg);
        }
    }
    let rest = &arguments[cut_at..];

    // Emit non-comment positionals on the header line first.
    for arg in &positionals {
        output.push(' ');
        output.push_str(arg.as_str());
    }

    // Combine any prefix-deferred comments with leading comments of
    // the remaining slice so they all flow after the positionals.
    let (leading_rest_comments, rest) = split_leading_inline_line_comments(rest);
    let mut inline_comments = deferred_comments;
    inline_comments.extend(leading_rest_comments.iter().copied());

    // A line comment terminates its line, so at most one line-comment
    // can stay inline with the header, and only if it fits within the
    // configured line width. Anything that doesn't fit breaks to its
    // own line at the subkwarg indent and is reflowed through the
    // shared comment formatter so an overlong comment still honours
    // the configured `line_width`.
    let mut header_line_open = true;
    for arg in inline_comments {
        let text = arg.as_str();
        if header_line_open && is_single_line_inline_comment(arg) {
            let current = current_line_char_count(output);
            if current + 1 + text.chars().count() <= line_width {
                output.push(' ');
                output.push_str(text);
                output.push('\n');
                header_line_open = false;
                continue;
            }
        }
        if header_line_open {
            output.push('\n');
            header_line_open = false;
        }
        let reflowed = if let Argument::InlineComment(comment) = arg {
            comment::format_comment_lines(
                comment,
                config,
                patterns,
                inner_indent.chars().count(),
                line_width,
            )
        } else {
            vec![text.to_owned()]
        };
        for line in reflowed {
            output.push_str(inner_indent);
            output.push_str(&line);
            output.push('\n');
        }
    }
    if header_line_open {
        output.push('\n');
    }
    if !rest.is_empty() {
        write_grouped_arguments(
            output,
            rest,
            inner_indent,
            spec,
            config,
            patterns,
            line_width,
            continuation_align,
        );
    }
}

/// Compute the end-exclusive index of the argument group that starts at
/// `start`. A group is either a nested kwarg plus the arguments it
/// consumes according to its `nargs`, or a single bare token when the
/// start token is not a known nested kwarg. Comments interleaved
/// between the kwarg and its values are carried along with the group
/// but do not count toward the nargs quota.
fn group_end(arguments: &[&Argument], start: usize, parent_spec: &crate::spec::KwargSpec) -> usize {
    let token = arguments[start].as_str();
    let Some(spec) = lookup_nested_kwarg_in(parent_spec, token) else {
        return start + 1;
    };

    match &spec.nargs {
        NArgs::Fixed(n) => advance_by_non_comment(arguments, start + 1, *n),
        NArgs::AtLeast(n) => {
            let min_end = advance_by_non_comment(arguments, start + 1, *n);
            extend_until_next_subkwarg(arguments, min_end, parent_spec)
        }
        NArgs::OneOrMore => {
            let min_end = advance_by_non_comment(arguments, start + 1, 1);
            extend_until_next_subkwarg(arguments, min_end, parent_spec)
        }
        NArgs::ZeroOrMore => extend_until_next_subkwarg(arguments, start + 1, parent_spec),
        NArgs::Optional => {
            let mut idx = start + 1;
            while idx < arguments.len() && arguments[idx].is_comment() {
                idx += 1;
            }
            if idx < arguments.len()
                && lookup_nested_kwarg_in(parent_spec, arguments[idx].as_str()).is_none()
            {
                idx + 1
            } else {
                idx
            }
        }
    }
}

/// Advance from `from` by at most `count` non-comment tokens, carrying
/// any comments encountered along the way. Returns the exclusive end
/// index into `arguments`.
fn advance_by_non_comment(arguments: &[&Argument], from: usize, count: usize) -> usize {
    let mut i = from;
    let mut taken = 0usize;
    while i < arguments.len() && taken < count {
        if !arguments[i].is_comment() {
            taken += 1;
        }
        i += 1;
    }
    i
}

fn extend_until_next_subkwarg(
    arguments: &[&Argument],
    start: usize,
    parent_spec: &crate::spec::KwargSpec,
) -> usize {
    let mut end = start;
    while end < arguments.len() {
        let arg = arguments[end];
        if arg.is_comment() {
            end += 1;
            continue;
        }
        if lookup_nested_kwarg_in(parent_spec, arg.as_str()).is_some() {
            return end;
        }
        end += 1;
    }
    end
}

fn lookup_nested_kwarg_in<'a>(
    parent: &'a crate::spec::KwargSpec,
    token: &str,
) -> Option<&'a crate::spec::KwargSpec> {
    parent.kwargs.get(token).or_else(|| {
        has_ascii_lowercase(token)
            .then(|| token.to_ascii_uppercase())
            .and_then(|normalized| parent.kwargs.get(&normalized))
    })
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
                //
                // Skip when the previous line already ends in a trailing
                // comment — appending another `#` segment would merge two
                // distinct comments into one, breaking idempotency on the
                // next format pass.
                if output.ends_with('\n') && !last_output_line_has_comment(output) {
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

/// Append the closing `)` to `output`, honouring dangle-paren config and
/// avoiding a trailing comment swallowing the paren. Shared by both
/// vertical-layout paths in `format_command_vertical`.
fn close_command_output(
    output: &mut String,
    cmd_config: &CommandConfig<'_>,
    base_indent: &str,
    name: &str,
) {
    if output.ends_with('\n') {
        output.pop();
    }
    if cmd_config.dangle_parens() {
        output.push('\n');
        match cmd_config.dangle_align() {
            DangleAlign::Prefix | DangleAlign::Close => output.push_str(base_indent),
            DangleAlign::Open => {
                output.push_str(base_indent);
                output.push_str(&" ".repeat(name.len()));
            }
        }
        output.push(')');
    } else if last_output_line_has_comment(output) {
        output.push('\n');
        output.push_str(base_indent);
        output.push(')');
    } else {
        output.push(')');
    }
}

fn argument_has_newline(argument: &Argument) -> bool {
    argument.as_str().contains('\n') || argument.as_str().contains('\r')
}

fn lookup_kwarg<'a>(form: &'a CommandForm, token: &str) -> Option<&'a crate::spec::KwargSpec> {
    form.kwargs.get(token).or_else(|| {
        has_ascii_lowercase(token)
            .then(|| token.to_ascii_uppercase())
            .and_then(|normalized| form.kwargs.get(&normalized))
    })
}

/// Classify a token as a keyword, flag, or positional in a single pass.
/// Avoids redundant case conversion by uppercasing at most once.
fn classify_token(form: &CommandForm, token: &str) -> Option<HeaderKind> {
    // Fast path: try exact-case lookup first.
    if form.kwargs.contains_key(token) {
        return Some(HeaderKind::Keyword);
    }
    if form.flags.contains(token) {
        return Some(HeaderKind::Flag);
    }

    // Slow path: normalize case once, check both.
    if has_ascii_lowercase(token) {
        let upper = token.to_ascii_uppercase();
        if form.kwargs.contains_key(&upper) {
            return Some(HeaderKind::Keyword);
        }
        if form.flags.contains(&upper) {
            return Some(HeaderKind::Flag);
        }
    }

    None
}

/// Check if a token is a nested keyword or flag in a single pass,
/// uppercasing at most once.
fn is_nested_keyword_or_flag(spec: &crate::spec::KwargSpec, token: &str) -> bool {
    if spec.kwargs.contains_key(token) || spec.flags.contains(token) {
        return true;
    }
    if has_ascii_lowercase(token) {
        let upper = token.to_ascii_uppercase();
        return spec.kwargs.contains_key(&upper) || spec.flags.contains(&upper);
    }
    false
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
