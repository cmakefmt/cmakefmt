---
title: Release Channels
description: How cmakefmt versions, ships, and publishes — release contract, official install channels, and what each release includes.
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

This page answers three practical questions:

1. what the first public release means
2. which install channels are official
3. what ships with a release

## Release Contract

The first public release should mean:

- the formatter is fast enough and complete enough for real project use
- Linux, macOS, and Windows are first-class supported platforms
- the CLI, config surface, and diagnostics are intentionally designed rather
  than experimental
- formatting output may still change between early releases when bugs are fixed
  or layout behavior is refined based on user feedback

In other words: usable now, but not yet promising `1.0`-level formatting
stability.

## Support Levels

`cmakefmt` distinguishes between channels that are part of the core release
contract and channels that are convenient but lower-priority in the initial
release rollout.

| Channel | Support level | What to expect |
|---------|----------------|----------------|
| GitHub Releases binaries | Officially maintained | Release artifacts and checksums published for supported platforms. |
| `cargo install cmakefmt-rust` | Officially maintained | Curated crates.io package with the same source tree used for releases. |
| Documentation site | Officially maintained | Updated as part of the tagged release. |
| Homebrew / `winget` / Scoop | Officially maintained | These are the first package-manager targets after GitHub Releases and crates.io. |
| Additional package managers / wrappers | Best effort | Useful distribution channels, but not all are blockers for the initial release. |

## Planned Release Artifacts

Each tagged release is expected to ship:

- `cmakefmt` binaries for supported platforms
- `SHA256SUMS`
- a curated source package on crates.io
- release notes with installation examples
- shell completions generated from the same CLI metadata as `--help`
- a generated man page for packagers and Unix-like installs

You can preview the packaging helper outputs from a local build:

```bash
cmakefmt completions bash > cmakefmt.bash
cmakefmt completions zsh > _cmakefmt
cmakefmt manpage > cmakefmt.1
```

## Version Output

`cmakefmt --version` reports the package version, and local development builds
also include a short Git commit when available.

That keeps local binaries identifiable without forcing Git metadata into
published release packages.

## Early-Release Stability Expectations

Before `1.0`, formatting behavior may still change between releases. The goal
is to keep those changes understandable and intentional:

- bug fixes that make output more obviously correct are expected
- formatting behavior changes should be documented in the changelog
- teams should pin an explicit released version in CI if output stability matters

Release notes and support policy updates are published with each tagged release
and reflected in the project changelog.
