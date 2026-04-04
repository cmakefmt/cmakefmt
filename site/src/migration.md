# Migration From `cmake-format`

## Recommended Rollout

1. start with `--check` in CI on a small target directory
2. generate a starter config with `--dump-config`
3. if you already have a `cmake-format` config file, convert it with `--convert-legacy-config`
4. compare output on a representative corpus
5. switch pre-commit and CI once the output is acceptable

## CLI Mapping

| `cmake-format` intent | `cmakefmt` |
| --- | --- |
| format file to stdout | `cmakefmt FILE` |
| in-place format | `cmakefmt -i FILE` |
| CI check | `cmakefmt --check PATH` |
| recursive target filtering | `cmakefmt --path-regex REGEX PATH` |
| convert old config file | `cmakefmt --convert-legacy-config OLD.py > .cmakefmt.toml` |
| disable formatting regions | supports both `cmake-format` and `cmakefmt` spellings |

## Compatibility Notes

- the goal is easy adoption, not output identity
- the built-in and supported utility-module command surface is audited through CMake 4.3.1
- `--config` is still accepted as an alias for `--config-file`, and `--path-regex` replaces the older `--file-regex`
- unsupported compatibility should be treated as a bug or backlog item, not silently assumed

## Operational Advice

Roll out with snapshots or branch-local diffs first. Formatter migrations
become painful when the first exposure is a large repository-wide rewrite
without comparison data.
