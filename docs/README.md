# Documentation

This directory holds long-form project documentation and reference material.

## What Lives Here

- `ARCHITECTURE.md`
  - formatter/parser/spec design notes
- `PERFORMANCE.md`
  - benchmark methodology, profiler notes, and current performance signal

Parser grammar source of truth is `src/parser/cmake.pest`.

## User-Facing Docs

The user-facing documentation site source lives under `site/`.

That site covers:

- installation
- CLI reference
- configuration reference
- formatter behavior
- `cmake-format` migration
- library/API usage
- release notes / changelog

## Contributor Notes

If you change user-visible behavior, keep these aligned:

- `README.md`
- `site/`
- `CHANGELOG.md`
- this directory's long-form reference docs where relevant

Use [CONTRIBUTING.md](../CONTRIBUTING.md)
as the source of truth for what needs to move together.
