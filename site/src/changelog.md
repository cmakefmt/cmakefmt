# Changelog

The canonical changelog lives in the repository root as `CHANGELOG.md`.
This page documents the release-note policy for the published docs site.

## Policy

- keep user-visible work under `Unreleased` until the next cut
- group changes by impact — what users feel, not what files changed
- call out migration and compatibility notes explicitly
- never bury user-visible behavior changes inside implementation-only commit messages

## What Release Notes Must Cover

- new CLI or config surface
- formatter behavior changes
- compatibility differences from `cmake-format`
- performance changes that matter to users
- breaking changes and rollout advice
