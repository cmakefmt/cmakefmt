# Changelog

This project follows a simple changelog discipline:

- keep user-visible changes in `Unreleased` until the next cut
- group entries by impact, not by file
- call out migration-impacting changes explicitly

## Unreleased

### Added

- full CLI workflow for formatting, checking, recursive discovery, file
  filtering, debug diagnostics, parallel formatting, and config dumping
- comment preservation, fence/barrier support, and real-world regression
  corpus coverage
- performance benchmarking, profiling notes, and direct `cmake-format`
  comparisons
- expanding built-in and module-command spec coverage audited against
  CMake `4.3.1`
- contributor guidance, docs roadmap, and GitHub Pages-ready docs structure

### Changed

- formatter output is now validated against real-world fixtures and snapshots
- the built-in command registry is cached to reduce end-to-end formatter cost
- parallel formatting remains opt-in by default while large-codebase RAM/system
  impact is still being surveyed

### Compatibility Notes

- `cmakefmt` aims to be easy to migrate to from `cmake-format`, but output is
  not intended to be byte-for-byte identical
- config and CLI compatibility are still being expanded as Phase 9 and Phase 10
  continue

## Release Process

For each release:

1. move relevant `Unreleased` entries into a versioned heading
2. summarize major user-visible changes
3. note any compatibility or migration impact
4. link the release tag or GitHub release when published
