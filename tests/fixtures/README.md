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
  - checked-in upstream examples used for real-world validation

## Rules

- keep fixtures minimal unless they are intentionally real-world
- if a fixture is copied from upstream, keep provenance in
  `real_world/SOURCES.md`
- if a change updates expected formatting, update the corresponding snapshot in
  `tests/snapshots/`
