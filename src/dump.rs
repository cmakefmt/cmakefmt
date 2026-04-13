// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::parser::ast;

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

// ── Tree rendering ──────────────────────────────────────────────────────────

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

fn format_annotation(kind: &str, color: bool) -> String {
    let text = format!("({kind})");
    if color {
        format!("  {}", dim(&text, true))
    } else {
        format!("  {text}")
    }
}
