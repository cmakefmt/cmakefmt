# `tests/fixtures/`

Fixture inputs are grouped by intent.

## Categories

- `basic/`
  - small focused formatting and parser inputs
- `comments/`
  - comment placement and preservation cases
- `edge_cases/`
  - parser/formatter edge behavior that is easy to regress
- `real_world/`
  - manifest and helper files for the fetched real-world validation corpus

## Rules

- keep fixtures minimal unless they are intentionally real-world
- keep upstream provenance and fetch instructions in `real_world/SOURCES.md`
- use the fetch script to populate `target/real-world-corpus/` locally
- if a change updates expected formatting, update the corresponding snapshot in
  `tests/snapshots/`
