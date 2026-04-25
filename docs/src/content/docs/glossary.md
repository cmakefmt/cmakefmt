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
- **artifact kind** — In `install(TARGETS ...)`, one of `RUNTIME` / `LIBRARY` / `ARCHIVE` / `OBJECTS` / `FRAMEWORK` / `BUNDLE` / `PRIVATE_HEADER` / `PUBLIC_HEADER` / `RESOURCE` / `FILE_SET` / `CXX_MODULES_BMI`. Each kind opens its own subgroup of artifact-options (`DESTINATION`, `PERMISSIONS`, `CONFIGURATIONS`, `COMPONENT`, `NAMELINK_COMPONENT`) plus subflags (`OPTIONAL`, `EXCLUDE_FROM_ALL`, `NAMELINK_ONLY`, `NAMELINK_SKIP`).
- **bracket argument** — An argument delimited by `[=[` and `]=]` (with any number of `=`). Content is preserved verbatim — no variable expansion, no escape sequences.
- **bracket comment** — A comment delimited by `#[=[` and `]=]`. Can span multiple lines. Content is verbatim.
- **command form** — For a [discriminated command](#cmake-concepts), the argument structure of one specific shape. `install(TARGETS ...)` and `install(FILES ...)` are two distinct forms of the same command.
- **command invocation** — A call to a CMake command: a name followed by parenthesized arguments. Example: `target_link_libraries(mylib PUBLIC dep1)`.
- **control flow** — Block-structured commands: `if`/`elseif`/`else`/`endif`, `foreach`/`endforeach`, `while`/`endwhile`, `function`/`endfunction`, `macro`/`endmacro`, `block`/`endblock`.
- **discriminated command** — A command whose argument structure depends on a discriminator token (usually the first positional). `install(TARGETS ...)` and `install(FILES ...)` parse with completely different forms; the formatter inspects the first non-comment argument to pick the right one. Other examples: `file(...)`, `export(...)`, `string(JSON ...)`.
- **flag** — A keyword-like token that takes no arguments. Example: `FORCE` in `set(VAR "val" CACHE STRING "desc" FORCE)`. Defined in the command spec.
- **kwarg (keyword)** — A recognized token that introduces a section of arguments. Example: `PUBLIC` in `target_link_libraries(mylib PUBLIC dep1 dep2)`. Defined in the command spec under the `kwargs:` table.
- **parg (positional argument)** — An argument identified by position, not by name. Example: `mylib` in `target_link_libraries(mylib PUBLIC dep1)`. The number of pargs a command or kwarg takes is set by the spec's `pargs:` field.
- **subflag (nested flag)** — A flag that only counts as a flag inside a particular kwarg's section. Example: `EXCLUDE` only acts as a flag inside an `install(DIRECTORY ... PATTERN <pat> EXCLUDE)` subgroup.
- **subkwarg (nested kwarg)** — A kwarg that only takes effect inside another kwarg's section. Example: in `install(TARGETS foo LIBRARY DESTINATION lib)`, `LIBRARY` is a top-level kwarg and `DESTINATION` is its subkwarg. The command spec models subkwargs under the parent's `kwargs:` table.
- **template placeholder** — A `@VAR@`-style token in `.cmake.in` configure-file templates (e.g. `@PACKAGE_INIT@`). cmakefmt preserves these verbatim.

## Formatter Concepts

- **autosort** — A heuristic (`format.autosort`) that infers sortability for keyword sections without an explicit `sortable: true` marker. Sections whose arguments are all simple unquoted tokens (no variables, generator expressions, or quoted strings) get sorted lexicographically. Sections whose spec declares nested subkwargs or flags are always skipped to avoid scrambling structure.
- **barrier / disabled region** — A `# cmakefmt: off` / `# cmakefmt: on` pair that prevents formatting of the enclosed block. Also accepted: `# cmake-format: off/on`, `# fmt: off/on`. The `# ~~~` fence form toggles a region as a matched pair.
- **command spec** — A description of a command's argument structure — positional args, keywords, flags, and nested sections. Defined in `builtins.yaml` for built-in commands and in `commands:` in your config for custom commands.
- **continuation_align** — A config option that controls how wrap lines indent inside a wrapped subkwarg group. `under-first-value` (default) aligns continuation under the first value column after the subkwarg (cmake-format hanging-indent style); `same-indent` wraps at the subkwarg's own indent.
- **dangle parens** — When enabled (`format.dangle_parens`), the closing `)` of a wrapped command is placed on its own line.
- **enable_markup** — Controls whether long comments are reflowed to fit within `line_width`. When `false`, comments are preserved as written.
- **hanging indent** — The default `continuation_align` style: continuation values align under the column of the first value following a subkwarg, matching the layout shown in `cmake --help-command install`.
- **hwrap (horizontal wrap / hanging wrap)** — A layout where the first argument stays on the command line and subsequent arguments wrap to the next line, indented. The formatter tries this before falling back to fully vertical layout. Distinct from "hanging indent" above, which describes only how subkwarg continuations align within a wrapped section.
- **idempotency** — The guarantee that formatting an already-formatted file produces identical output: `format(format(x)) == format(x)`.
- **layout** — The visual arrangement of a command's arguments. cmakefmt tries inline first, then hanging wrap, then vertical. Depends on `line_width`, argument count, and wrapping thresholds.
- **line_width** — The target maximum line width (default: `80`). The formatter wraps arguments to stay within this limit.
- **nargs** — In a command spec, the number of positional arguments a keyword takes. Fixed integer (`1`, `2`), unbounded (`"+"` for one-or-more, `"*"` for zero-or-more), optional (`"?"`), or minimum (`"N+"`, e.g. `"2+"`).
- **pair-aware rendering** — A formatter behaviour that keeps each subkwarg paired with its values on one logical line when wrapping a section. Example: `RUNTIME COMPONENT Runtime` stays as a unit even when the surrounding `install(TARGETS …)` section wraps to multiple lines. Activated automatically when the section header's spec declares nested subkwargs.
- **per-command override** — A config section (`per_command_overrides:`) that changes formatting options for a single command name without affecting others.
- **reflow** — Re-wrapping text (usually comments) to fit within `line_width`, breaking long lines and joining short ones.
- **section** — A group of arguments under a keyword or flag. Example: in `target_link_libraries(mylib PUBLIC dep1 dep2 PRIVATE dep3)`, there are three sections: positional (`mylib`), `PUBLIC` (`dep1 dep2`), and `PRIVATE` (`dep3`).
- **semantic verification** — A safety check that re-parses formatted output and compares it to the original AST (ignoring comments) to ensure formatting never changes the meaning of the file.
- **sortable** — A property a command spec attaches to a keyword section to opt that section in to argument sorting under `format.enable_sort`. Example: `target_sources` source lists are commonly marked sortable so they stay alphabetical.
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
