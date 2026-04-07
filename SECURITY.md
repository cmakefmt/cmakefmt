<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Security Policy

## Supported Versions

Security fixes are applied to the latest release only. There are no
long-term-support branches at this stage of the project.

| Version | Supported |
|---------|-----------|
| latest  | yes       |
| older   | no        |

## Reporting a Vulnerability

Please **do not** open a public GitHub issue for security vulnerabilities.

Report privately via [GitHub Security Advisories](https://github.com/cmakefmt/cmakefmt/security/advisories/new).
You will receive a response within **5 business days** acknowledging receipt.
Fixes are typically released within **30 days** of a confirmed vulnerability,
depending on complexity.

Once a fix is released, a public CVE will be requested if appropriate and the
advisory will be published.

## Scope

`cmakefmt` is a source-code formatter. It reads CMake files and writes
formatted output. It does not:

- execute CMake code
- make network requests
- access credentials or secrets
- write to any path other than the input file (with `--in-place`)

The primary risk surface is processing untrusted CMake files (e.g. from
third-party repositories). Crashes, panics, or incorrect output on malformed
input are treated as bugs. Infinite loops or excessive memory consumption on
crafted inputs are treated as security-relevant.

## Dependency Auditing

Dependencies are scanned for known vulnerabilities on every CI run via
`cargo audit` and `cargo deny`. The dependency tree is intentionally minimal
and contains only permissive-licensed crates.
