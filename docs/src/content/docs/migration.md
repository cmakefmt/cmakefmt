---
title: Migration From `cmake-format`
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

Switching to `cmakefmt` is designed to be straightforward. The goal is easy
adoption, not a risky big-bang rewrite — roll out incrementally, compare output
at each step, and flip the switch once you are satisfied.

## Recommended Rollout

1. Start with `--check` on a small, low-risk directory to get a feel for the
   output before touching anything:

   ```bash
   cmakefmt --check cmake/
   ```

2. Generate a starter config. YAML is the recommended default:

   ```bash
   cmakefmt --dump-config > .cmakefmt.yaml
   ```

3. If you already have a `cmake-format` config file, convert it automatically:

   ```bash
   cmakefmt --convert-legacy-config .cmake-format.py > .cmakefmt.yaml
   ```

   YAML is the default output. If you prefer TOML instead, add
   `--convert-legacy-config-format toml`.

4. Compare output on a representative sample. Use `--diff` to see exactly what
   would change without touching any files:

   ```bash
   cmakefmt --diff path/to/CMakeLists.txt
   ```

5. When the output looks good across your sample, expand to `--check .` on the
   full repository:

   ```bash
   cmakefmt --check .
   ```

6. Switch pre-commit and CI once you are happy with the full-repository output.

If you want to roll out formatting gradually — file by file — use
`--require-pragma` to opt individual files in before going repository-wide.
See [Formatter Behavior](/behavior/) for the pragma syntax.

## CLI Mapping

| `cmake-format` intent | `cmakefmt` equivalent |
| --- | --- |
| format file to stdout | `cmakefmt FILE` |
| in-place format | `cmakefmt -i FILE` |
| CI check | `cmakefmt --check PATH` |
| recursive target filtering | `cmakefmt --path-regex REGEX PATH` |
| convert old config file | `cmakefmt --convert-legacy-config OLD.py > .cmakefmt.yaml` |
| disable formatting regions | supports both `cmake-format` and `cmakefmt` spellings |

## Compatibility Notes

- the goal is easy adoption, not output identity
- the built-in command registry is audited through CMake 4.3.1
- `--config` is still accepted as an alias for `--config-file`
- `--path-regex` replaces the older `--file-regex`
- any unsupported compatibility should be treated as a bug, not silently assumed

## Operational Advice

**Start small, build confidence, then go wide.** Formatter migrations become
painful when the first exposure is a large repository-wide rewrite with no
comparison data. The recommended pattern:

1. Run `--diff` or `--check` on a representative subset before committing to
   anything.
2. Capture a before/after snapshot on a branch so you can review the delta as
   a normal diff.
3. Use `--require-pragma` if you want to phase the rollout file-by-file rather
   than all at once.
4. Pin an explicit version in CI once you are happy:

   ```bash
   cmakefmt --required-version 0.0.1 --check .
   ```

**Output will not be identical to `cmake-format`.** The goal is a clean,
correct, stable result — not byte-for-byte reproduction. Judge the migration by
readability, idempotency, and ease of automation rather than by whether every
wrapped line matches historical output exactly. See [Formatter Behavior](/behavior/)
for a concrete summary of what `cmakefmt` preserves and what it intentionally changes.
