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
| **Formatted output** | The same source + config produces the same output across patch and minor releases. Formatting changes are introduced as new opt-in config keys with backward-compatible defaults. |
| **Config schema** | Existing config keys keep their names and semantics. New keys may be added with backward-compatible defaults. |
| **CLI flags and exit codes** | Existing flags keep their names and behavior. New flags may be added. Exit codes are stable. |
| **Rust library API** | Public types, traits, and function signatures in `cmakefmt::*` follow standard Rust semver. |

## What can change

| Change | When allowed |
|--------|-------------|
| **New config options** | Any minor release. New options always have defaults that preserve existing behavior. |
| **New CLI flags** | Any minor release. |
| **Bug fixes that change output** | Any release. If `cmakefmt` produces incorrect output (violates its own layout rules), the fix ships as a bug fix, not a formatting change. |
| **Formatting improvements** | Introduced as new opt-in config keys with backward-compatible defaults. Existing files keep their previous output unless the project's config opts in. |
| **Removing a deprecated flag** | At least one minor release with a deprecation warning before removal. |

## How new formatting behaviors land

When a release adds a formatting option that would change existing
output:

1. **Introduced** — the option ships as a new key under `format`
   (or the relevant section) with a default value that preserves the
   previous behavior. Users who don't change their config see no
   difference.
2. **Documented** — the option appears in the config reference and
   the changelog Added section, with examples and the rationale for
   the new behavior.
3. **Promoted later** — if the option becomes the recommended setting,
   the default may flip in a subsequent minor release. Such a flip is
   called out in the changelog under a "Behaviour change" note and is
   easy to revert via the config key.

Users who pin a formatter version and don't change their config will
see no output changes across minor releases.

## Versioning policy

`cmakefmt` follows [Semantic Versioning](https://semver.org/):

- **Patch** (`0.10.1`): bug fixes, documentation, CI changes. No
  formatting output changes.
- **Minor** (`0.11.0`): new features, new config options. May change
  formatting output only through new opt-in config keys with
  backward-compatible defaults, or via documented default flips
  called out in the changelog.
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
