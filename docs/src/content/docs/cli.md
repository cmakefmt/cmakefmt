---
title: CLI Reference
description: Complete reference for cmakefmt's command-line flags, subcommands, and input modes.
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

The complete reference for everything `cmakefmt` can do from the command line.
If you just want to get up and running, start with [Installation](/installation/) first
and come back here when you want the full picture.

## Synopsis

```text
cmakefmt [OPTIONS] [FILES]...
```

## The Four Main Ways To Run `cmakefmt`

| Pattern | What it does |
| --- | --- |
| `cmakefmt CMakeLists.txt` | Format one file to stdout. |
| `cmakefmt dir/` | Recursively discover CMake files under that directory. |
| `cmakefmt` | Recursively discover CMake files under the current working directory. |
| `cmakefmt -` | Read one file from stdin and write formatted output to stdout. |

## How Input Selection Works

One rule governs everything:

- **direct file arguments always win**

If you pass a file path explicitly, `cmakefmt` processes it even if an ignore
file or regex would have excluded it during discovery.

Ignore rules only affect:

- directory discovery
- `--files-from`
- `--staged`
- `--changed`

## Input Selection Flags

| Flag | Meaning |
| --- | --- |
| `--files-from <PATH>` | Read more input paths from a file, or `-` for stdin. Accepts newline-delimited or NUL-delimited path lists. |
| `--path-regex <REGEX>` | Filter discovered CMake paths. Direct file arguments are not filtered out. |
| `--ignore-path <PATH>` | Add extra ignore files during recursive discovery. Direct file arguments still win. |
| `--no-gitignore` | Stop honoring `.gitignore` during recursive discovery. |
| `--sorted` | Sort discovered files by path before processing. |
| `--staged` | Use staged Git-tracked files instead of explicit input paths. |
| `--changed` | Use modified Git-tracked files instead of explicit input paths. |
| `--since <REF>` | Choose the Git base ref used by `--changed`. Without it, `HEAD` is the base. |
| `--stdin-path <PATH>` | Give stdin formatting a virtual on-disk path for config discovery and diagnostics. |
| `--lines <START:END>` | Restrict formatting to one or more inclusive 1-based line ranges within a single file. |

## Output Mode Flags

| Flag | Meaning |
| --- | --- |
| `-i`, `--in-place` | Rewrite files on disk instead of printing formatted output. |
| `--check` | Exit with code `1` when any selected file would change. |
| `--list-changed-files` | Print only the files that would change after formatting. |
| `--list-input-files` | Print the selected input files after discovery and filtering, without formatting them. |
| `--list-unknown-commands` | Parse files and report commands that don't match any built-in or user-defined spec, with file:line locations. |
| `-d`, `--diff` | Print a unified diff instead of the full formatted output. |
| `--explain` | Show per-command formatting decisions (layout choice, config values, thresholds) for a single file. |
| `--report-format <human\|json\|github\|checkstyle\|junit\|sarif\|edit>` | Switch between human output and CI/editor-friendly machine reporters. `edit` outputs JSON with full-file replacements. |
| `-s`, `--summary` | Show a per-file status summary instead of formatted output. In stdout mode, formatted output is suppressed. |
| `--color <auto\|always\|never>` | Control ANSI color output. `auto` only colors terminal output. `--colour` is an alias. |

## Execution Flags

| Flag | Meaning |
| --- | --- |
| `--debug` | Emit discovery, config, barrier, and formatter diagnostics to stderr. |
| `-q`, `--quiet` | Suppress per-file human output and keep only summaries plus actual errors. |
| `--keep-going` | Continue processing later files after a file-level parse/format error. |
| `--required-version <VERSION>` | Refuse to run unless the current `cmakefmt` version matches exactly. Useful for pinned CI and editor wrappers. |
| `--verify` | Parse the original and formatted output and reject the result if the CMake semantics change. |
| `--no-verify` | Skip semantic verification, including the default rewrite-time verification used by `--in-place`. `--fast` is a deprecated alias. |
| `--cache` | Cache formatted file results for repeated runs on the same files. |
| `--cache-location <PATH>` | Override the cache directory. Supplying it also enables caching. |
| `--cache-strategy <metadata\|content>` | Choose whether cache invalidation tracks file metadata or file contents. |
| `--require-pragma` | Format only files that opt in with a `# cmakefmt: enable` style pragma. |
| `-j`, `--parallel [JOBS]` | Set the number of parallel formatting jobs. Defaults to the available CPU count minus one. Pass `--parallel 1` to force serial. |
| `-p`, `--progress-bar` | Show a progress bar on stderr during multi-file runs. Automatically suppressed when stdout streams to the terminal. |
| `--watch` | Watch directories for changes and reformat in-place automatically. Press Ctrl+C to stop. |

## Subcommands

| Subcommand | Meaning |
| --- | --- |
| `cmakefmt lsp` | Start the LSP server (JSON-RPC on stdio). |
| `cmakefmt completions <SHELL>` | Print shell completions for bash, zsh, or fish. |
| `cmakefmt install-hook` | Install a git pre-commit hook that runs `cmakefmt --check --staged`. |
| `cmakefmt config dump [FORMAT]` | Print a starter config template. Defaults to YAML; pass `toml` for TOML. |
| `cmakefmt config schema` | Print the JSON Schema for the config file. |
| `cmakefmt config check [PATH]` | Validate a config file without formatting. |
| `cmakefmt config show [FORMAT]` | Print the effective config for a target. |
| `cmakefmt config path` | Print the selected config file path for a target. |
| `cmakefmt config explain` | Explain config resolution for a target or the current directory. |
| `cmakefmt config convert <PATH>...` | Convert legacy cmake-format config files. |
| `cmakefmt config init` | Write a starter `.cmakefmt.yaml` to the current directory. |
| `cmakefmt dump ast <FILE>` | Print the raw parser AST as a tree. |
| `cmakefmt dump parse <FILE>` | Print the spec-resolved parse tree with keyword/flag grouping and flow-control nesting. |

## Other Flags

| Flag | Meaning |
| --- | --- |
| `--generate-man-page` | Print a roff man page for packagers and Unix-like installs. |

## Config Override Flags

| Flag | Meaning |
| --- | --- |
| `-c`, `--config-file <PATH>` | Use one or more specific config files instead of config discovery. Later files override earlier ones. `--config` remains a compatibility alias. |
| `--no-config` | Ignore discovered config files and explicit `--config-file` entries. Only built-in defaults plus CLI overrides remain. |
| `-l`, `--line-width <N>` | Override `format.line_width`. |
| `--tab-size <N>` | Override `format.tab_size`. |
| `--command-case <lower\|upper\|unchanged>` | Override `format.command_case`. |
| `--keyword-case <lower\|upper\|unchanged>` | Override `format.keyword_case`. |
| `--dangle-parens <true\|false>` | Override `format.dangle_parens`. |

## Exit Codes

- `0`: success
- `1`: `--check` or `--list-changed-files` found files that would change
- `2`: parse, config, regex, or I/O error

## Common Examples

### Format One File To Stdout

```bash
cmakefmt CMakeLists.txt
```

Prints the formatted file to stdout. The file on disk is untouched.

### Rewrite Files In Place

```bash
cmakefmt --in-place .
```

The "apply formatting now" mode. Every discovered CMake file gets rewritten.
In-place rewrites verify parse-tree stability by default. Use `--no-verify` to skip
this verification and improve throughput on trusted inputs.

### Verify A Dry Run Semantically

```bash
cmakefmt --verify CMakeLists.txt
```

This keeps stdout output, but also reparses both the original and formatted
source and rejects the result if the parsed CMake structure changes.

### Use `--check` In CI

```bash
cmakefmt --check .
```

Typical human-mode output:

```text
would reformat src/foo/CMakeLists.txt
would reformat cmake/Toolchain.cmake

summary: selected=12 changed=2 unchanged=10 failed=0
```

Exit code `0` means nothing would change. Exit code `1` means at least one
file is out of format — exactly what CI needs.

If your CI system prefers structured annotations or standard interchange
formats, switch reporters:

```bash
cmakefmt --check --report-format json .
cmakefmt --check --report-format github .
cmakefmt --check --report-format checkstyle .
cmakefmt --check --report-format junit .
cmakefmt --check --report-format sarif .
```

### Pin The Formatter Version In Automation

```bash
cmakefmt --required-version 1.3.0 --check .
```

This makes shell scripts and editor wrappers fail fast when the installed
binary is not the exact version the workflow expects.

### List Only The Files That Would Change

```bash
cmakefmt --list-changed-files --path-regex 'cmake|toolchain' .
```

Typical output:

```text
cmake/Toolchain.cmake
cmake/Warnings.cmake
```

Useful for editor integration, scripts, and review tooling that needs a
precise list without actually reformatting anything.

### List The Selected Input Files Without Formatting Them

```bash
cmakefmt --list-input-files --path-regex 'cmake|toolchain' .
```

Typical output:

```text
cmake/Toolchain.cmake
cmake/Warnings.cmake
cmake/modules/CompilerOptions.cmake
```

This is the pure discovery mode. It walks the file tree, applies ignore files
and filters, then prints the selected CMake inputs without parsing or formatting
them.

### Show The Actual Patch

```bash
cmakefmt --diff CMakeLists.txt
```

Typical output:

```diff
--- CMakeLists.txt
+++ CMakeLists.txt.formatted
@@
-target_link_libraries(foo PUBLIC bar baz)
+target_link_libraries(
+  foo
+  PUBLIC
+    bar
+    baz)
```

### Quiet CI Output

```bash
cmakefmt --check --quiet .
```

Typical effect:

```text
summary: selected=48 changed=3 unchanged=45 failed=0
```

A clean log with a reliable exit code — ideal for high-volume CI pipelines.

### Per-File Summary

```bash
cmakefmt --summary .
```

Typical output:

```text
! src/CMakeLists.txt
  └─ 12 lines changed, 84 → 86 lines, 2ms
✔ tests/CMakeLists.txt
  └─ unchanged, 42 lines, 1ms
! cmake/Toolchain.cmake
  └─ 3 lines changed, 60 → 61 lines, 4ms

summary: selected=3, changed=2, unchanged=1, failed=0 in 0.01s
```

Shows at a glance what happened to each file — useful for large repositories,
first-time adoption, and understanding the scope of formatting changes.

### Cache Repeated Runs

```bash
cmakefmt --cache --check .
cmakefmt --cache-location .cache/cmakefmt --cache-strategy content --check .
```

Use metadata-based invalidation for speed or content-based invalidation when
you want the cache to ignore timestamp-only churn.

### Roll Out Formatting Gradually

```bash
cmakefmt --require-pragma --check .
```

Then opt individual files in with a short marker:

```cmake
# cmakefmt: enable
```

`cmakefmt` also accepts `# fmt: enable` and `# cmake-format: enable` as
equivalent opt-in pragmas.

### Continue Past Bad Files

```bash
cmakefmt --check --keep-going .
```

Typical effect:

```text
error: failed to parse cmake/generated.cmake:...
error: failed to read vendor/missing.cmake:...

summary: selected=48 changed=3 unchanged=43 failed=2
```

Without `--keep-going`, the run stops at the first file-level error.

### Format Only Staged Files

```bash
cmakefmt --staged --check
```

The easiest pre-commit or pre-push workflow — only touches files that are
already part of the current Git change.

### Format Only Changed Files Since A Ref

```bash
cmakefmt --changed --since origin/main --check
```

Perfect for PR workflows. Checks only "what this branch changed" rather than
the entire repository.

### Feed Paths From Another Tool

```bash
git diff --name-only --diff-filter=ACMR origin/main...HEAD | \
  cmakefmt --files-from - --check
```

`--files-from` accepts newline-delimited or NUL-delimited path lists, so it
composites cleanly with any tool that can emit file paths.

### Stdin With Correct Config Discovery

```bash
cat src/CMakeLists.txt | cmakefmt - --stdin-path src/CMakeLists.txt
```

Without `--stdin-path`, stdin formatting has no on-disk context for config
discovery or path-sensitive diagnostics.

### Partial Formatting For Editor Workflows

```bash
cmakefmt --stdin-path src/CMakeLists.txt --lines 10:25 -
```

Use this when an editor wants to format only a selected line range instead of
rewriting the whole buffer.

### See Which Config Was Selected

```bash
cmakefmt config path src/CMakeLists.txt
```

Typical output:

```text
/path/to/project/.cmakefmt.yaml
```

### Inspect The Effective Config

```bash
cmakefmt config show src/CMakeLists.txt
cmakefmt config show toml src/CMakeLists.txt
```

Prints the fully resolved config after discovery plus any CLI overrides.
No more guessing what the formatter is actually using.

### Explain Config Resolution

```bash
cmakefmt config explain
```

Typical output includes:

- the target path being resolved
- config files considered
- config file selected
- CLI overrides applied

### Generate A Starter Config

```bash
cmakefmt config init
```

Or dump the full template to stdout:

```bash
cmakefmt config dump > .cmakefmt.yaml
cmakefmt config dump toml > .cmakefmt.toml
```

YAML is the default because it is easier to maintain once you start defining
larger custom command specs.

### Convert An Old `cmake-format` Config

```bash
cmakefmt config convert .cmake-format.py
```

The fastest path through a legacy config migration.

## Discovery Precedence And Filtering Rules

- Direct file arguments are always processed, even if an ignore rule would skip them.
- Recursive discovery honors `.cmakefmtignore` and, by default, `.gitignore`.
- `--ignore-path` adds more ignore files for discovered directories only.
- `--files-from`, `--staged`, and `--changed` still pass through normal discovery filters when they produce directories or paths that need filtering.
- `cmakefmt config path`, `cmakefmt config show`, and `cmakefmt config explain` resolve a single target context and make the selected config path(s) visible.
- `--no-config` disables config discovery entirely.

## Diagnostic Quality

For parse and config failures, `cmakefmt` prints:

- the file path
- line and column information
- source context
- likely-cause hints when possible
- a repro hint using `--debug --check`

For unclosed parentheses, the error points directly to the unmatched `(`
instead of the end of the file where the parser gave up:

```text
error: failed to parse a command invocation
  --> src/CMakeLists.txt:19:6

  18 | find_package(oomphlib CONFIG REQUIRED PATHS "../install")
> 19 | endif(
     |      ^
hint: unclosed `(` — the closing `)` is missing
```

When formatting results surprise you rather than hard-failing, reach for
`--debug` first.

## Parse Tree Dump

`cmakefmt dump` provides two tree views for debugging parser and
formatter behavior.

### `dump ast` — Raw Parser AST

Prints the AST exactly as the parser produces it. No command-spec
resolution, no flow-control grouping. Useful when a parse error is
confusing or the parser's trailing-comment merging does something
unexpected.

```bash
cmakefmt dump ast CMakeLists.txt
```

```text
└─ FILE
    ├─ COMMAND  cmake_minimum_required
    │   ├─ ARG  VERSION  (unquoted)
    │   └─ ARG  3.20  (unquoted)
    ├─ ───
    ├─ COMMAND  set
    │   ├─ ARG  FOO  (unquoted)
    │   ├─ ARG  bar  (unquoted)
    │   └─ TRAILING  # my comment
    └─ COMMAND  message
        ├─ ARG  STATUS  (unquoted)
        └─ ARG  "hello"  (quoted)
```

Every argument is annotated with its type — `(unquoted)`, `(quoted)`,
or `(bracket)`. Comments show as `COMMENT`, `INLINE_COMMENT`, or
`TRAILING`. Blank lines show as `───`.

### `dump parse` — Spec-Resolved Tree

Resolves each command against the built-in spec registry (and any
user-defined specs) to show keyword/flag/positional grouping.
Flow-control blocks are grouped into `FLOW` nodes. Useful when a
layout decision is surprising — "why did `FORCE` end up under
`CACHE`?" or "why is `PUBLIC` treated as a keyword?"

```bash
cmakefmt dump parse CMakeLists.txt
```

```text
└─ FILE
    ├─ COMMAND  cmake_minimum_required
    │   └─ KEYWORD  VERSION
    │       └─ ARG  3.20
    ├─ ───
    ├─ FLOW  if ... endif
    │   ├─ COMMAND  if
    │   │   └─ POSITIONAL  WIN32
    │   ├─ BODY
    │   │   └─ COMMAND  target_link_libraries
    │   │       ├─ POSITIONAL  mylib
    │   │       └─ KEYWORD  PUBLIC
    │   │           ├─ ARG  dep1
    │   │           └─ ARG  dep2
    │   └─ COMMAND  endif
    └─ COMMAND  set
        ├─ POSITIONAL  CMAKE_BUILD_TYPE
        ├─ POSITIONAL  "Release"
        └─ KEYWORD  CACHE
            ├─ ARG  STRING
            ├─ ARG  "Build mode."
            └─ FLAG  FORCE
```

Nested keyword specs are resolved recursively — `FORCE` shows as
`FLAG` under `CACHE` because the `set()` spec defines it there.

### Reading from stdin

Both commands accept `-` to read from stdin:

```bash
echo 'set(FOO bar)' | cmakefmt dump ast -
```

### Color

Output is colored by default on terminals (node types in bold cyan,
comments in dim green, connectors in dim). Use `--color never` to
suppress ANSI codes when piping:

```bash
cmakefmt --color never dump ast CMakeLists.txt > tree.txt
```

## Related Reading

- [Config Reference](/config/)
- [Formatter Behavior](/behavior/)
- [Troubleshooting](/troubleshooting/)
