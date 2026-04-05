# `docs/`

This directory contains the `mdBook` source for the GitHub Pages
documentation site for `cmakefmt`.

## Local Preview

Use `mdbook` to preview or build the docs locally:

```bash
mdbook serve docs
```

Then open the local URL printed by `mdbook`, usually
<http://localhost:3000>.

To build the static site without serving it:

```bash
mdbook build docs
```

## Rules

- keep `docs/src/SUMMARY.md` in sync with the available chapters
- if you add a new primary docs page, add it to `docs/src/SUMMARY.md`
- keep the site content aligned with `README.md`, `CHANGELOG.md`, and
  `CONTRIBUTING.md`
