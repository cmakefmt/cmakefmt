---
title: Glossary
description: Definitions of terms used throughout cmakefmt's documentation, config, and tree dumps.
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

Quick reference for terminology used in cmakefmt's documentation, config
options, CLI output, and parse tree dumps.

## CMake Concepts

- **argument** — A token inside a command's parentheses. Can be unquoted (`FOO`), quoted (`"hello"`), or bracket-quoted (`[=[content]=]`).
- **bracket argument** — An argument delimited by `[=[` and `]=]` (with any number of `=`). Content is preserved verbatim — no variable expansion, no escape sequences.
- **bracket comment** — A comment delimited by `#[=[` and `]=]`. Can span multiple lines. Content is verbatim.
- **command invocation** — A call to a CMake command: a name followed by parenthesized arguments. Example: `target_link_libraries(mylib PUBLIC dep1)`.
- **control flow** — Block-structured commands: `if`/`elseif`/`else`/`endif`, `foreach`/`endforeach`, `while`/`endwhile`, `function`/`endfunction`, `macro`/`endmacro`, `block`/`endblock`.
- **flag** — A keyword-like token that takes no arguments. Example: `FORCE` in `set(VAR "val" CACHE STRING "desc" FORCE)`. Defined in the command spec.
- **keyword (kwarg)** — A recognized token that introduces a section of arguments. Example: `PUBLIC` in `target_link_libraries(mylib PUBLIC dep1 dep2)`. Defined in the command spec.
- **positional argument (parg)** — An argument identified by position, not by name. Example: `mylib` in `target_link_libraries(mylib PUBLIC dep1)`.

## Formatter Concepts

- **barrier / disabled region** — A `# cmakefmt: off` / `# cmakefmt: on` pair that prevents formatting of the enclosed block. Also called a "fence" when using `# ~~~`.
- **command spec** — A description of a command's argument structure — positional args, keywords, flags, and nested sections. Defined in `builtins.yaml` for built-in commands and in `commands:` in your config for custom commands.
- **dangle parens** — When enabled (`format.dangle_parens`), the closing `)` of a wrapped command is placed on its own line.
- **enable_markup** — Controls whether long comments are reflowed to fit within `line_width`. When `false`, comments are preserved as written.
- **hwrap (horizontal wrap / hanging wrap)** — A layout where the first argument stays on the command line and subsequent arguments wrap to the next line, indented. The formatter tries this before falling back to fully vertical layout.
- **idempotency** — The guarantee that formatting an already-formatted file produces identical output: `format(format(x)) == format(x)`.
- **layout** — The visual arrangement of a command's arguments. cmakefmt tries inline first, then hanging wrap, then vertical. Depends on `line_width`, argument count, and wrapping thresholds.
- **line_width** — The target maximum line width (default: `80`). The formatter wraps arguments to stay within this limit.
- **per-command override** — A config section (`per_command_overrides:`) that changes formatting options for a single command name without affecting others.
- **reflow** — Re-wrapping text (usually comments) to fit within `line_width`, breaking long lines and joining short ones.
- **section** — A group of arguments under a keyword or flag. Example: in `target_link_libraries(mylib PUBLIC dep1 dep2 PRIVATE dep3)`, there are three sections: positional (`mylib`), `PUBLIC` (`dep1 dep2`), and `PRIVATE` (`dep3`).
- **semantic verification** — A safety check that re-parses formatted output and compares it to the original AST (ignoring comments) to ensure formatting never changes the meaning of the file.
- **trailing comment** — A comment after the closing `)` on the same line: `set(FOO bar) # trailing`. Long trailing comments are reflowed with continuation lines aligned to the `#`.
- **wrap_after_first_arg** — A layout hint that keeps the first positional argument on the command line when wrapping. Enabled by default for `set()` so the variable name stays on the `set(` line.

## Parse Tree Node Types

These appear in the output of `cmakefmt dump ast` and
`cmakefmt dump parse`. See [Parse Tree Dump](/cli/#parse-tree-dump)
for full examples.

### `dump ast` nodes

| Node | Meaning |
|------|---------|
| `FILE` | Root of the parse tree |
| `COMMAND` | A command invocation, followed by its name |
| `ARG` | An argument, annotated with `(unquoted)`, `(quoted)`, or `(bracket)` |
| `COMMENT` | A standalone comment on its own line |
| `INLINE_COMMENT` | A comment between arguments inside a command |
| `TRAILING` | A comment after the closing `)` on the same line |
| `TEMPLATE` | A configure-file placeholder like `@PACKAGE_INIT@` |
| `───` | One or more blank lines between statements |

### `dump parse` additional nodes

| Node | Meaning |
|------|---------|
| `KEYWORD` | An argument classified as a keyword by the command spec |
| `FLAG` | An argument classified as a flag by the command spec |
| `POSITIONAL` | An argument that is not a keyword or flag |
| `FLOW` | A flow-control group (`if ... endif`, `foreach ... endforeach`, etc.) |
| `BODY` | The statements nested inside a flow-control block |
