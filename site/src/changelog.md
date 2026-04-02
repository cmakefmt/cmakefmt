# Changelog

The canonical changelog also exists in the repository root as `CHANGELOG.md`.
This page summarizes the release-note policy for the published docs site.

## Policy

- keep user-visible work under `Unreleased` until the next cut
- group changes by impact
- call out migration and compatibility notes explicitly
- do not hide user-visible behavior changes inside implementation-only commit messages

## Release Notes Should Cover

- new CLI or config surface
- formatter behavior changes
- compatibility differences from `cmake-format`
- performance changes that matter to users
- breaking changes and rollout advice
