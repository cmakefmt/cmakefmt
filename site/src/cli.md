# CLI Reference

The complete reference for everything `cmakefmt` can do from the command line.
If you just want to get up and running, start with [Install](install.md) first
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
| `--staged` | Use staged Git-tracked files instead of explicit input paths. |
| `--changed` | Use modified Git-tracked files instead of explicit input paths. |
| `--since <REF>` | Choose the Git base ref used by `--changed`. Without it, `HEAD` is the base. |
| `--stdin-path <PATH>` | Give stdin formatting a virtual on-disk path for config discovery and diagnostics. |
| `--lines <START:END>` | Restrict formatting to one or more inclusive 1-based line ranges on a single target. |

## Output Mode Flags

| Flag | Meaning |
| --- | --- |
| `-i`, `--in-place` | Rewrite files on disk instead of printing formatted output. |
| `--check` | Exit with code `1` when any selected file would change. |
| `--list-files` | Print only the files that would change. |
| `--diff` | Print a unified diff instead of the full formatted output. |
| `--report-format <human\|json>` | Switch between human terminal output and machine-readable JSON. |
| `--colour <auto\|always\|never>` | Highlight changed formatted output lines in cyan. `auto` only colors terminal output. |

## Execution Flags

| Flag | Meaning |
| --- | --- |
| `--debug` | Emit discovery, config, barrier, and formatter diagnostics to stderr. |
| `--quiet` | Suppress per-file human output and keep only summaries plus actual errors. |
| `--keep-going` | Continue processing later files after a file-level parse/format error. |
| `-j`, `--parallel [JOBS]` | Enable parallel file processing when explicitly requested. If no value is given, use the available CPU count. |
| `--progress-bar` | Show a progress bar on stderr during `--in-place` multi-file runs. |

## Config And Conversion Flags

| Flag | Meaning |
| --- | --- |
| `--dump-config [FORMAT]` | Print a starter config template and exit. Defaults to YAML; pass `toml` for TOML. |
| `--show-config [FORMAT]` | Print the effective config for a single target and exit. Defaults to YAML; pass `toml` for TOML. |
| `--show-config-path` | Print the selected config file path for a single target and exit. `--find-config-path` is an alias. |
| `--explain-config <PATH>` | Explain config resolution for a target path, including selected files and CLI overrides. |
| `--convert-legacy-config <PATH>` | Convert a legacy `cmake-format` JSON/YAML/Python config file to `.cmakefmt.toml` on stdout. |

## Config Override Flags

| Flag | Meaning |
| --- | --- |
| `-c`, `--config-file <PATH>` | Use one or more specific config files instead of config discovery. Later files override earlier ones. `--config` remains a compatibility alias. |
| `--no-config` | Ignore discovered config files and explicit `--config-file` entries. Only built-in defaults plus CLI overrides remain. |
| `-l`, `--line-width <N>` | Override `format.line_width`. |
| `--tab-size <N>` | Override `format.tab_size`. |
| `--command-case <lower\|upper\|unchanged>` | Override `style.command_case`. |
| `--keyword-case <lower\|upper\|unchanged>` | Override `style.keyword_case`. |
| `--dangle-parens <true\|false>` | Override `format.dangle_parens`. |

## Exit Codes

- `0`: success
- `1`: `--check` or `--list-files` found files that would change
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

### List Only The Files That Would Change

```bash
cmakefmt --list-files --path-regex 'cmake|toolchain' .
```

Typical output:

```text
cmake/Toolchain.cmake
cmake/Warnings.cmake
```

Useful for editor integration, scripts, and review tooling that needs a
precise list without actually reformatting anything.

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
cmakefmt --show-config-path src/CMakeLists.txt
```

Typical output:

```text
/path/to/project/.cmakefmt.yaml
```

### Inspect The Effective Config

```bash
cmakefmt --show-config src/CMakeLists.txt
cmakefmt --show-config=toml src/CMakeLists.txt
```

Prints the fully resolved config after discovery plus any CLI overrides.
No more guessing what the formatter is actually using.

### Explain Config Resolution

```bash
cmakefmt --explain-config src/CMakeLists.txt
```

Typical output includes:

- the target path being resolved
- config files considered
- config file selected
- CLI overrides applied

### Generate A Starter Config

```bash
cmakefmt --dump-config > .cmakefmt.yaml
cmakefmt --dump-config toml > .cmakefmt.toml
```

YAML is the default because it is easier to maintain once you start defining
larger custom command specs.

### Convert An Old `cmake-format` Config

```bash
cmakefmt --convert-legacy-config .cmake-format.py > .cmakefmt.toml
```

The fastest path through a legacy config migration.

## Discovery Precedence And Filtering Rules

- Direct file arguments are always processed, even if an ignore rule would skip them.
- Recursive discovery honors `.cmakefmtignore` and, by default, `.gitignore`.
- `--ignore-path` adds more ignore files for discovered directories only.
- `--files-from`, `--staged`, and `--changed` still pass through normal discovery filters when they produce directories or paths that need filtering.
- `--show-config-path`, `--show-config`, and `--explain-config` resolve a single target context and make the selected config path(s) visible.
- `--no-config` disables config discovery entirely.

## Diagnostic Quality

For parse and config failures, `cmakefmt` prints:

- the file path
- line and column information
- source context
- likely-cause hints when possible
- a repro hint using `--debug --check`

When formatting results surprise you rather than hard-failing, reach for
`--debug` first.

## Related Reading

- [Config Reference](config.md)
- [Formatter Behavior](behavior.md)
- [Troubleshooting](troubleshooting.md)
