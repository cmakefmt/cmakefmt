// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::formatter::{split_sections, HeaderKind};
use crate::parser::ast;
use crate::spec::registry::CommandRegistry;

// ── ANSI helpers ────────────────────────────────────────────────────────────

fn dim(s: &str, color: bool) -> String {
    if color {
        format!("\x1b[2m{s}\x1b[0m")
    } else {
        s.to_owned()
    }
}

fn bold_cyan(s: &str, color: bool) -> String {
    if color {
        format!("\x1b[1;36m{s}\x1b[0m")
    } else {
        s.to_owned()
    }
}

fn dim_green(s: &str, color: bool) -> String {
    if color {
        format!("\x1b[2;32m{s}\x1b[0m")
    } else {
        s.to_owned()
    }
}

fn bold_yellow(s: &str, color: bool) -> String {
    if color {
        format!("\x1b[1;33m{s}\x1b[0m")
    } else {
        s.to_owned()
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Write a single tree line: `{prefix}{connector} {label}  {value}\n`.
fn write_line(out: &mut String, prefix: &str, connector: &str, color: bool, parts: &str) {
    out.push_str(&format!("{prefix}{} {parts}\n", dim(connector, color)));
}

fn format_annotation(kind: &str, color: bool) -> String {
    let text = format!("({kind})");
    if color {
        format!("  {}", dim(&text, true))
    } else {
        format!("  {text}")
    }
}

fn connector_for(index: usize, total: usize) -> &'static str {
    if index + 1 == total {
        "└─"
    } else {
        "├─"
    }
}

fn child_prefix_for(index: usize, total: usize) -> &'static str {
    if index + 1 == total {
        "    "
    } else {
        "│   "
    }
}

// ── dump_ast ────────────────────────────────────────────────────────────────

/// Render the AST of a parsed CMake [`ast::File`] as a Unicode box-drawing
/// tree, optionally with ANSI colour.
pub fn dump_ast(file: &ast::File, color: bool) -> String {
    let mut out = String::new();

    let total = file.statements.len();
    out.push_str(&format!(
        "{} {}\n",
        dim("└─", color),
        bold_cyan("FILE", color),
    ));

    for (i, stmt) in file.statements.iter().enumerate() {
        let is_last = i + 1 == total;
        let connector = if is_last { "└─" } else { "├─" };
        let child_prefix = if is_last { "    " } else { "│   " };

        match stmt {
            ast::Statement::Command(cmd) => {
                out.push_str(&format!(
                    "    {} {}  {}\n",
                    dim(connector, color),
                    bold_cyan("COMMAND", color),
                    cmd.name,
                ));
                let arg_total =
                    cmd.arguments.len() + if cmd.trailing_comment.is_some() { 1 } else { 0 };
                let mut arg_idx = 0;
                for arg in &cmd.arguments {
                    arg_idx += 1;
                    let arg_last = arg_idx == arg_total;
                    let arg_conn = if arg_last { "└─" } else { "├─" };
                    match arg {
                        ast::Argument::Unquoted(s) => {
                            out.push_str(&format!(
                                "    {}  {} {}  {}{}",
                                dim(child_prefix.trim_end(), color),
                                dim(arg_conn, color),
                                bold_cyan("ARG", color),
                                s,
                                format_annotation("unquoted", color),
                            ));
                            out.push('\n');
                        }
                        ast::Argument::Quoted(s) => {
                            out.push_str(&format!(
                                "    {}  {} {}  {}{}",
                                dim(child_prefix.trim_end(), color),
                                dim(arg_conn, color),
                                bold_cyan("ARG", color),
                                s,
                                format_annotation("quoted", color),
                            ));
                            out.push('\n');
                        }
                        ast::Argument::Bracket(b) => {
                            out.push_str(&format!(
                                "    {}  {} {}  {}{}",
                                dim(child_prefix.trim_end(), color),
                                dim(arg_conn, color),
                                bold_cyan("ARG", color),
                                b.raw,
                                format_annotation("bracket", color),
                            ));
                            out.push('\n');
                        }
                        ast::Argument::InlineComment(c) => {
                            out.push_str(&format!(
                                "    {}  {} {}  {}",
                                dim(child_prefix.trim_end(), color),
                                dim(arg_conn, color),
                                bold_cyan("INLINE_COMMENT", color),
                                dim_green(c.as_str(), color),
                            ));
                            out.push('\n');
                        }
                    }
                }
                if let Some(tc) = &cmd.trailing_comment {
                    out.push_str(&format!(
                        "    {}  {} {}  {}",
                        dim(child_prefix.trim_end(), color),
                        dim("└─", color),
                        bold_cyan("TRAILING", color),
                        dim_green(tc.as_str(), color),
                    ));
                    out.push('\n');
                }
            }
            ast::Statement::Comment(c) => {
                out.push_str(&format!(
                    "    {} {}  {}",
                    dim(connector, color),
                    bold_cyan("COMMENT", color),
                    dim_green(c.as_str(), color),
                ));
                out.push('\n');
            }
            ast::Statement::BlankLines(_) => {
                out.push_str(&format!(
                    "    {} {}",
                    dim(connector, color),
                    dim("───", color),
                ));
                out.push('\n');
            }
            ast::Statement::TemplatePlaceholder(s) => {
                out.push_str(&format!(
                    "    {} {}  {}",
                    dim(connector, color),
                    bold_cyan("TEMPLATE", color),
                    s,
                ));
                out.push('\n');
            }
        }
    }

    out
}

// ── dump_parse ──────────────────────────────────────────────────────────────

/// Render a spec-resolved parse tree of a CMake [`ast::File`], grouping
/// flow-control blocks and classifying arguments via the command registry.
pub fn dump_parse(file: &ast::File, registry: &CommandRegistry, color: bool) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{} {}\n",
        dim("└─", color),
        bold_cyan("FILE", color),
    ));

    let refs: Vec<&ast::Statement> = file.statements.iter().collect();
    let groups = group_flow_control(&refs);
    render_groups(&mut out, &groups, registry, color, "    ");
    out
}

/// Render a list of flow groups at a given indent prefix.
fn render_groups(
    out: &mut String,
    groups: &[FlowGroup<'_>],
    registry: &CommandRegistry,
    color: bool,
    prefix: &str,
) {
    let total = groups.len();
    for (i, group) in groups.iter().enumerate() {
        let conn = connector_for(i, total);
        let cp = child_prefix_for(i, total);
        let child = format!("{prefix}{}", cp);
        match group {
            FlowGroup::Single(stmt) => {
                render_parse_statement(out, stmt, registry, color, prefix, conn, &child);
            }
            FlowGroup::Block {
                opener,
                bodies,
                closer,
            } => {
                render_flow_block(
                    out, opener, bodies, closer, registry, color, prefix, conn, &child,
                );
            }
        }
    }
}

/// Render a single non-flow statement in the parse tree.
fn render_parse_statement(
    out: &mut String,
    stmt: &ast::Statement,
    registry: &CommandRegistry,
    color: bool,
    prefix: &str,
    connector: &str,
    child_prefix: &str,
) {
    match stmt {
        ast::Statement::Command(cmd) => {
            render_parse_command(out, cmd, registry, color, prefix, connector, child_prefix);
        }
        ast::Statement::Comment(c) => {
            write_line(
                out,
                prefix,
                connector,
                color,
                &format!(
                    "{}  {}",
                    bold_cyan("COMMENT", color),
                    dim_green(c.as_str(), color)
                ),
            );
        }
        ast::Statement::BlankLines(_) => {
            write_line(out, prefix, connector, color, &dim("───", color));
        }
        ast::Statement::TemplatePlaceholder(s) => {
            write_line(
                out,
                prefix,
                connector,
                color,
                &format!("{}  {}", bold_cyan("TEMPLATE", color), s),
            );
        }
    }
}

/// Render a command with spec-resolved sections.
fn render_parse_command(
    out: &mut String,
    cmd: &ast::CommandInvocation,
    registry: &CommandRegistry,
    color: bool,
    prefix: &str,
    connector: &str,
    child_prefix: &str,
) {
    write_line(
        out,
        prefix,
        connector,
        color,
        &format!("{}  {}", bold_cyan("COMMAND", color), cmd.name),
    );

    let spec = registry.get(&cmd.name);
    let first_arg = cmd
        .arguments
        .iter()
        .find(|a| !a.is_comment())
        .map(ast::Argument::as_str);
    let form = spec.form_for(first_arg);

    let sections = match split_sections(cmd, form) {
        Ok(s) => s,
        Err(_) => return,
    };

    // Count the total number of child items for last-child detection.
    let mut child_count: usize = if cmd.trailing_comment.is_some() { 1 } else { 0 };
    for section in &sections {
        match (&section.header, &section.header_kind) {
            (Some(_), Some(HeaderKind::Keyword | HeaderKind::Flag)) => {
                child_count += 1;
            }
            _ => {
                // Positional: each argument is a direct child
                child_count += section.arguments.len();
            }
        }
    }

    let mut child_idx = 0;

    for section in &sections {
        match (&section.header, &section.header_kind) {
            (Some(name), Some(HeaderKind::Keyword)) => {
                child_idx += 1;
                let conn = connector_for(child_idx - 1, child_count);
                let cp = child_prefix_for(child_idx - 1, child_count);
                write_line(
                    out,
                    child_prefix,
                    conn,
                    color,
                    &format!("{}  {}", bold_cyan("KEYWORD", color), name),
                );
                let arg_total = section.arguments.len();
                for (ai, arg) in section.arguments.iter().enumerate() {
                    let a_conn = connector_for(ai, arg_total);
                    let nested_prefix = format!("{child_prefix}{cp}");
                    if arg.is_comment() {
                        write_line(
                            out,
                            &nested_prefix,
                            a_conn,
                            color,
                            &format!(
                                "{}  {}",
                                bold_cyan("INLINE_COMMENT", color),
                                dim_green(arg.as_str(), color),
                            ),
                        );
                    } else {
                        write_line(
                            out,
                            &nested_prefix,
                            a_conn,
                            color,
                            &format!("{}  {}", bold_cyan("ARG", color), arg.as_str()),
                        );
                    }
                }
            }
            (Some(name), Some(HeaderKind::Flag)) => {
                child_idx += 1;
                let conn = connector_for(child_idx - 1, child_count);
                let cp = child_prefix_for(child_idx - 1, child_count);
                write_line(
                    out,
                    child_prefix,
                    conn,
                    color,
                    &format!("{}  {}", bold_cyan("FLAG", color), name),
                );
                // Flags can have trailing arguments attached by split_sections
                let arg_total = section.arguments.len();
                for (ai, arg) in section.arguments.iter().enumerate() {
                    let a_conn = connector_for(ai, arg_total);
                    let nested_prefix = format!("{child_prefix}{cp}");
                    if arg.is_comment() {
                        write_line(
                            out,
                            &nested_prefix,
                            a_conn,
                            color,
                            &format!(
                                "{}  {}",
                                bold_cyan("INLINE_COMMENT", color),
                                dim_green(arg.as_str(), color),
                            ),
                        );
                    } else {
                        write_line(
                            out,
                            &nested_prefix,
                            a_conn,
                            color,
                            &format!("{}  {}", bold_cyan("ARG", color), arg.as_str()),
                        );
                    }
                }
            }
            _ => {
                // Positional (headerless) section
                for arg in &section.arguments {
                    child_idx += 1;
                    let conn = connector_for(child_idx - 1, child_count);
                    if arg.is_comment() {
                        write_line(
                            out,
                            child_prefix,
                            conn,
                            color,
                            &format!(
                                "{}  {}",
                                bold_cyan("INLINE_COMMENT", color),
                                dim_green(arg.as_str(), color),
                            ),
                        );
                    } else {
                        write_line(
                            out,
                            child_prefix,
                            conn,
                            color,
                            &format!("{}  {}", bold_cyan("POSITIONAL", color), arg.as_str()),
                        );
                    }
                }
            }
        }
    }

    if let Some(tc) = &cmd.trailing_comment {
        write_line(
            out,
            child_prefix,
            "└─",
            color,
            &format!(
                "{}  {}",
                bold_cyan("TRAILING", color),
                dim_green(tc.as_str(), color),
            ),
        );
    }
}

/// Render a flow-control block (`if`/`endif`, etc.).
fn render_flow_block(
    out: &mut String,
    opener: &ast::CommandInvocation,
    bodies: &[FlowBody<'_>],
    closer: &Option<&ast::CommandInvocation>,
    registry: &CommandRegistry,
    color: bool,
    prefix: &str,
    connector: &str,
    child_prefix: &str,
) {
    let opener_name = opener.name.to_lowercase();
    let closer_name = match closer {
        Some(c) => c.name.to_lowercase(),
        None => format!("end{opener_name}"),
    };
    write_line(
        out,
        prefix,
        connector,
        color,
        &format!(
            "{}  {} ... {}",
            bold_yellow("FLOW", color),
            opener_name,
            closer_name,
        ),
    );

    // Children of the FLOW node: opener, bodies, closer
    let child_count = 1 + bodies.len() + usize::from(closer.is_some());
    let mut idx = 0;

    // Opener
    let conn = connector_for(idx, child_count);
    let cp = child_prefix_for(idx, child_count);
    let nested = format!("{child_prefix}{cp}");
    render_parse_command(out, opener, registry, color, child_prefix, conn, &nested);
    idx += 1;

    // Bodies
    for body in bodies {
        let conn = connector_for(idx, child_count);
        let cp = child_prefix_for(idx, child_count);
        let body_prefix = format!("{child_prefix}{cp}");
        idx += 1;

        if let Some(intermediate) = &body.intermediate {
            write_line(
                out,
                child_prefix,
                conn,
                color,
                &format!(
                    "{}  {}",
                    bold_cyan("BODY", color),
                    intermediate.name.to_lowercase()
                ),
            );
            // Children of this BODY: intermediate command + nested statements
            let sub_groups = group_flow_control(&body.statements);
            let body_child_count = 1 + sub_groups.len();

            let ic = connector_for(0, body_child_count);
            let icp = child_prefix_for(0, body_child_count);
            let inter_child = format!("{body_prefix}{icp}");
            render_parse_command(
                out,
                intermediate,
                registry,
                color,
                &body_prefix,
                ic,
                &inter_child,
            );

            // Render nested groups starting at index 1
            for (si, sg) in sub_groups.iter().enumerate() {
                let sc = connector_for(si + 1, body_child_count);
                let scp = child_prefix_for(si + 1, body_child_count);
                let sub_child = format!("{body_prefix}{scp}");
                match sg {
                    FlowGroup::Single(stmt) => {
                        render_parse_statement(
                            out,
                            stmt,
                            registry,
                            color,
                            &body_prefix,
                            sc,
                            &sub_child,
                        );
                    }
                    FlowGroup::Block {
                        opener: o,
                        bodies: b,
                        closer: c,
                    } => {
                        render_flow_block(
                            out,
                            o,
                            b,
                            c,
                            registry,
                            color,
                            &body_prefix,
                            sc,
                            &sub_child,
                        );
                    }
                }
            }
        } else {
            write_line(
                out,
                child_prefix,
                conn,
                color,
                &format!("{}", bold_cyan("BODY", color)),
            );
            let sub_groups = group_flow_control(&body.statements);
            render_groups(out, &sub_groups, registry, color, &body_prefix);
        }
    }

    // Closer
    if let Some(closer_cmd) = closer {
        let conn = connector_for(idx, child_count);
        let cp = child_prefix_for(idx, child_count);
        let nested = format!("{child_prefix}{cp}");
        render_parse_command(
            out,
            closer_cmd,
            registry,
            color,
            child_prefix,
            conn,
            &nested,
        );
    }
}

// ── Flow-control grouping ──────────────────────────────────────────────────

/// A body within a flow-control block.
struct FlowBody<'a> {
    /// The intermediate command (`elseif`, `else`), or `None` for the first body.
    intermediate: Option<&'a ast::CommandInvocation>,
    /// Statements within this body.
    statements: Vec<&'a ast::Statement>,
}

/// A single statement or a complete flow-control block.
enum FlowGroup<'a> {
    Single(&'a ast::Statement),
    Block {
        opener: &'a ast::CommandInvocation,
        bodies: Vec<FlowBody<'a>>,
        closer: Option<&'a ast::CommandInvocation>,
    },
}

fn is_block_opener(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "if" | "foreach" | "while" | "function" | "macro" | "block"
    )
}

fn is_block_intermediate(name: &str) -> bool {
    matches!(name.to_ascii_lowercase().as_str(), "elseif" | "else")
}

fn is_block_closer(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "endif" | "endforeach" | "endwhile" | "endfunction" | "endmacro" | "endblock"
    )
}

/// Walk a flat list of statements and group flow-control blocks.
fn group_flow_control<'a>(statements: &'a [&'a ast::Statement]) -> Vec<FlowGroup<'a>> {
    let mut groups: Vec<FlowGroup<'a>> = Vec::new();
    let mut i = 0;

    while i < statements.len() {
        match statements[i] {
            ast::Statement::Command(cmd) if is_block_opener(&cmd.name) => {
                let (block, consumed) = collect_flow_block(statements, i);
                groups.push(block);
                i += consumed;
            }
            stmt => {
                groups.push(FlowGroup::Single(stmt));
                i += 1;
            }
        }
    }

    groups
}

/// Collect a complete flow-control block starting at `start`.
fn collect_flow_block<'a>(
    statements: &'a [&'a ast::Statement],
    start: usize,
) -> (FlowGroup<'a>, usize) {
    let opener = match statements[start] {
        ast::Statement::Command(cmd) => cmd,
        _ => unreachable!("called on non-command"),
    };

    let mut bodies: Vec<FlowBody<'a>> = Vec::new();
    let mut current_body = FlowBody {
        intermediate: None,
        statements: Vec::new(),
    };
    let mut depth = 1usize;
    let mut i = start + 1;

    while i < statements.len() {
        match statements[i] {
            ast::Statement::Command(cmd) if is_block_opener(&cmd.name) => {
                depth += 1;
                current_body.statements.push(statements[i]);
                i += 1;
                while i < statements.len() && depth > 1 {
                    match statements[i] {
                        ast::Statement::Command(c) if is_block_opener(&c.name) => {
                            depth += 1;
                        }
                        ast::Statement::Command(c) if is_block_closer(&c.name) => {
                            depth -= 1;
                        }
                        _ => {}
                    }
                    current_body.statements.push(statements[i]);
                    i += 1;
                }
            }
            ast::Statement::Command(cmd) if depth == 1 && is_block_intermediate(&cmd.name) => {
                bodies.push(current_body);
                current_body = FlowBody {
                    intermediate: Some(cmd),
                    statements: Vec::new(),
                };
                i += 1;
            }
            ast::Statement::Command(cmd) if depth == 1 && is_block_closer(&cmd.name) => {
                bodies.push(current_body);
                let consumed = i - start + 1;
                return (
                    FlowGroup::Block {
                        opener,
                        bodies,
                        closer: Some(cmd),
                    },
                    consumed,
                );
            }
            _ => {
                current_body.statements.push(statements[i]);
                i += 1;
            }
        }
    }

    // Unterminated block
    bodies.push(current_body);
    let consumed = i - start;
    (
        FlowGroup::Block {
            opener,
            bodies,
            closer: None,
        },
        consumed,
    )
}
