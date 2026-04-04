# CLI Reference

## Synopsis

```text
cmakefmt [OPTIONS] [FILES]...
```

## Inputs

| Input | Behavior |
| --- | --- |
| `cmakefmt file.cmake` | Format one file to stdout. |
| `cmakefmt dir/` | Recursively discover CMake files under that directory. |
| `cmakefmt` | Recursively discover CMake files under the current working directory. |
| `cmakefmt -` | Read from stdin and write formatted output to stdout. |

## Operational Flags

| Flag | Meaning |
| --- | --- |
| `-i`, `--in-place` | Rewrite files on disk. |
| `--check` | Exit with code 1 when any file would change. |
| `--list-files` | List files that would change without modifying them. |
| `--path-regex <REGEX>` | Filter recursively discovered CMake file paths. |
| `--ignore-path <PATH>` | Add one or more extra ignore files during recursive discovery. |
| `--no-gitignore` | Ignore `.gitignore` files and only use built-in / explicit ignore rules. |
| `--files-from <PATH>` | Read newline-delimited or NUL-delimited input paths from a file, or `-` for stdin. |
| `--debug` | Emit diagnostics about discovery, config resolution, barriers, and formatting decisions. |
| `--diff` | Print a unified diff instead of the full formatted output. |
| `--staged` | Select only staged Git-tracked files. |
| `--changed` | Select only changed Git-tracked files. |
| `--since <REF>` | Base Git ref used together with `--changed`. |
| `--stdin-path <PATH>` | Virtual on-disk path used for stdin config discovery and diagnostics. |
| `--lines <START:END>` | Restrict formatting to one or more inclusive 1-based line ranges. |
| `--report-format <human\|json>` | Switch between human terminal output and machine-readable JSON. |
| `--colour <auto\|always\|never>` | Highlight changed formatted output lines in cyan. `auto` only colors terminal output. |
| `-j`, `--parallel [JOBS]` | Enable parallel file processing when explicitly requested. If no value is given, use the available CPU count. If omitted, formatting stays single-threaded. |
| `--progress-bar` | Show a progress bar on stderr during `--in-place` multi-file runs. |
| `--dump-config [FORMAT]` | Print a starter config template and exit. Defaults to YAML; pass `toml` for TOML. |
| `--convert-legacy-config <PATH>` | Convert a legacy `cmake-format` JSON/YAML/Python config file to `.cmakefmt.toml` on stdout. |

## Config-backed Override Flags

| Flag | Meaning |
| --- | --- |
| `--config-file <PATH>` | Use one or more specific config files instead of config discovery. Later files override earlier ones. `--config` remains as a compatibility alias. |
| `--line-width <N>` | Override `[format].line_width`. |
| `--tab-size <N>` | Override `[format].tab_size`. |
| `--command-case <lower\|upper\|unchanged>` | Override `[style].command_case`. |
| `--keyword-case <lower\|upper\|unchanged>` | Override `[style].keyword_case`. |
| `--dangle-parens <true\|false>` | Override `[format].dangle_parens`. |

## Exit Codes

- `0`: success
- `1`: `--check` or `--list-files` found files that would change
- `2`: parse, config, regex, or I/O error

## Common Examples

```bash
cmakefmt CMakeLists.txt
cmakefmt -i .
cmakefmt --check .
cmakefmt --list-files --path-regex 'cmake|toolchain' .
cmakefmt --ignore-path ci/cmakefmt.ignore --list-files .
cmakefmt --diff CMakeLists.txt
cmakefmt --report-format json --check .
cmakefmt --staged --check
cmakefmt --changed --since origin/main --check
git diff --name-only --diff-filter=ACMR origin/main...HEAD | cmakefmt --files-from - --check
cat CMakeLists.txt | cmakefmt - --stdin-path subdir/CMakeLists.txt
cmakefmt --stdin-path src/CMakeLists.txt --lines 10:25 -
cmakefmt --colour never CMakeLists.txt
cmakefmt --progress-bar --in-place .
cmakefmt --config-file base.yaml --config-file team.yaml CMakeLists.txt
cmakefmt --convert-legacy-config .cmake-format.py > .cmakefmt.toml
cmakefmt --debug --check tests/fixtures/real_world
```

## Discovery Precedence

- Direct file arguments are always processed, even if an ignore rule would skip them.
- Recursive discovery honors `.cmakefmtignore` and, by default, `.gitignore`.
- `--ignore-path` adds more ignore files for discovered directories only.

## Diagnostics

For parse and config failures, `cmakefmt` prints a file path, line/column,
source snippet, likely-cause hint when possible, and a repro command using
`--debug --check`.

For issue reports, capture:

- the exact command you ran
- the full stderr output
- the relevant `.cmakefmt.yaml`, `.cmakefmt.yml`, or `.cmakefmt.toml` files
- `--debug` output if the problem is formatting-related rather than a hard failure
