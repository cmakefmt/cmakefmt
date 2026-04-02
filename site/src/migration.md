# Migration From `cmake-format`

## Recommended Rollout

1. start with `--check` in CI on a small target directory
2. generate a starter config with `--dump-config`
3. compare output on a representative corpus
4. switch pre-commit and CI once the output is acceptable

## CLI Mapping

| `cmake-format` intent | `cmakefmt` |
| --- | --- |
| format file to stdout | `cmakefmt FILE` |
| in-place format | `cmakefmt -i FILE` |
| CI check | `cmakefmt --check PATH` |
| recursive target filtering | `cmakefmt --file-regex REGEX PATH` |
| disable formatting regions | supports both `cmake-format` and `cmakefmt` spellings |

## Compatibility Notes

- the goal is easy adoption, not output identity
- some config and module-command coverage is still expanding in Phase 9
- unsupported compatibility should be treated as a bug or backlog item, not silently assumed

## Operational Advice

Roll out with snapshots or branch-local diffs first. Formatter migrations
become painful when the first exposure is a large repository-wide rewrite
without comparison data.
