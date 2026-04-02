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
| `-f`, `--file-regex <REGEX>` | Filter recursively discovered CMake file paths. |
| `--debug` | Emit diagnostics about discovery, config resolution, barriers, and formatting decisions. |
| `--color <auto\|always\|never>` | Highlight changed formatted output lines in cyan. `auto` only colors terminal output. |
| `-j`, `--parallel [JOBS]` | Enable parallel file processing. If no value is given, use the available CPU count. Default behavior remains single-threaded. |
| `--dump-config` | Print a starter config template and exit. |

## Config-backed Override Flags

| Flag | Meaning |
| --- | --- |
| `--config <PATH>` | Use a specific config file instead of config discovery. |
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
cmakefmt --list-files --file-regex 'cmake|toolchain' .
cmakefmt --color never CMakeLists.txt
cat CMakeLists.txt | cmakefmt -
cmakefmt --debug --check tests/fixtures/real_world
```
