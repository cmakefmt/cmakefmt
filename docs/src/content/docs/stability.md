---
title: Stability Contract
description: What `cmakefmt` guarantees across releases, and how changes are introduced.
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

This page defines what constitutes a breaking change in `cmakefmt`, how
new formatting behaviors are introduced, and what you can rely on across
releases.

## What is stable

After 1.0, the following are part of the stability contract:

| Surface | Guarantee |
|---------|-----------|
| **Formatted output** | The same source + config produces the same output across patch and minor releases. Formatting changes are introduced via `--preview` first. |
| **Config schema** | Existing config keys keep their names and semantics. New keys may be added with backward-compatible defaults. |
| **CLI flags and exit codes** | Existing flags keep their names and behavior. New flags may be added. Exit codes are stable. |
| **Rust library API** | Public types, traits, and function signatures in `cmakefmt::*` follow standard Rust semver. |

## What can change

| Change | When allowed |
|--------|-------------|
| **New config options** | Any minor release. New options always have defaults that preserve existing behavior. |
| **New CLI flags** | Any minor release. |
| **Bug fixes that change output** | Any release. If `cmakefmt` produces incorrect output (violates its own layout rules), the fix ships as a bug fix, not a formatting change. |
| **Formatting improvements** | Introduced behind `--preview` / `[experimental]` first, promoted to stable in a subsequent minor release. |
| **Removing a deprecated flag** | At least one minor release with a deprecation warning before removal. |

## The preview mechanism

New formatting behaviors that change output are gated behind the
`[experimental]` config section or the `--preview` CLI flag:

1. **Introduced** — the behavior ships behind `[experimental]` with a
   default of `false` (off). Users opt in explicitly.
2. **Feedback** — at least one release cycle with the option available.
   If no issues are reported, it moves forward.
3. **Promoted** — the option moves from `[experimental]` to a stable
   config key (or becomes the new default). This is documented in the
   changelog as a formatting output change.

Users who pin a formatter version and don't use `--preview` will see no
output changes across minor releases.

## Versioning policy

`cmakefmt` follows [Semantic Versioning](https://semver.org/):

- **Patch** (`0.10.1`): bug fixes, documentation, CI changes. No
  formatting output changes.
- **Minor** (`0.11.0`): new features, new config options, promoted
  experimental options. May change formatting output only through the
  preview promotion path.
- **Major** (`2.0.0`): breaking changes to the config schema, CLI
  flags, or library API. Reserved for rare, well-justified cases.

## For CI users

Pin an explicit version to avoid surprises:

```bash
cmakefmt --required-version 1.0.0 --check .
```

Or pin the action version:

```yaml
- uses: cmakefmt/cmakefmt-action@v2
  with:
    version: '1.0.0'
```

## Reporting stability issues

If you believe a release broke the stability contract, please
[open an issue](https://github.com/cmakefmt/cmakefmt/issues/new)
with:

- The `cmakefmt` version that changed behavior
- The input file and config
- The expected vs actual output
