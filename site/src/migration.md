# Migration From `cmake-format`

Switching to `cmakefmt` is designed to be straightforward. The goal is easy
adoption, not a risky big-bang rewrite — roll out incrementally, compare output
at each step, and flip the switch once you are satisfied.

## Recommended Rollout

1. start with `--check` in CI on a small target directory
2. generate a starter config with `--dump-config` (YAML by default, `toml` available explicitly if needed)
3. if you already have a `cmake-format` config file, convert it automatically with `--convert-legacy-config`
4. compare output on a representative corpus
5. switch pre-commit and CI once the output looks good

## CLI Mapping

| `cmake-format` intent | `cmakefmt` equivalent |
| --- | --- |
| format file to stdout | `cmakefmt FILE` |
| in-place format | `cmakefmt -i FILE` |
| CI check | `cmakefmt --check PATH` |
| recursive target filtering | `cmakefmt --path-regex REGEX PATH` |
| convert old config file | `cmakefmt --convert-legacy-config OLD.py > .cmakefmt.toml` |
| disable formatting regions | supports both `cmake-format` and `cmakefmt` spellings |

## Compatibility Notes

- the goal is easy adoption, not output identity
- the built-in command registry is audited through CMake 4.3.1
- `--config` is still accepted as an alias for `--config-file`
- `--path-regex` replaces the older `--file-regex`
- any unsupported compatibility should be treated as a bug, not silently assumed

## Operational Advice

Roll out with snapshots or branch-local diffs first. Formatter migrations
become painful when the first exposure is a large repository-wide rewrite
without comparison data. Start small, build confidence, then go wide.
