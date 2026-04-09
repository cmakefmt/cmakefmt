---
title: FAQ
description: Frequently asked questions about cmakefmt.
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

## How is cmakefmt different from cmake-format?

cmakefmt is a from-scratch rewrite in Rust. It is faster (10–100x on large
codebases), actively maintained, and supports the same config format with
full conversion via `--convert-config`. It also includes an LSP server,
JSON Schema for config autocomplete, and a browser playground.

## How is cmakefmt different from gersemi?

gersemi is another CMake formatter written in Python. cmakefmt is
significantly faster (native binary), supports legacy cmake-format config
conversion, and has broader editor integration (LSP server, VS Code
extension, pre-commit hook).

## Can I use my existing cmake-format config?

Yes. Run `cmakefmt --convert-config .cmake-format.yaml` to convert it.
Most options are carried forward; lint and encode options are intentionally
excluded because cmakefmt is a formatter, not a linter.

## Which config options are not supported?

The `[lint]` section (cmakefmt is a formatter, not a linter), `[encode]`
section (UTF-8 is assumed), `[parse].vartags` / `[parse].proptags`
(linting-only), and `[format].layout_passes` (covered by `always_wrap` and
per-command overrides).

## Does cmakefmt change the semantics of my CMake files?

No. cmakefmt only changes whitespace, indentation, casing, and comment
layout. It never modifies command names, arguments, or logic.

## How do I disable formatting for a section?

Use `# cmakefmt: off` and `# cmakefmt: on` barrier comments. The aliases
`# cmake-format: off/on` and `# fmt: off/on` also work.

## Does cmakefmt support custom commands?

Yes. Define command specs in the `commands:` section of your
`.cmakefmt.yaml`, or in a separate YAML file passed via `--command-spec`.

## Can I use cmakefmt in CI?

Yes. Use `cmakefmt --check .` (exits non-zero if files need formatting) or
the official [GitHub Action](https://github.com/marketplace/actions/cmakefmt)
(`cmakefmt/cmakefmt-action@v1`).

## Is there an LSP server?

Yes. Run `cmakefmt --lsp` to start a stdio LSP server that provides
format-on-save and range formatting in any editor with LSP support. See the
[editor integration](/guide/editors/) page for setup instructions.

## Where can I get help?

Open an issue on [GitHub](https://github.com/cmakefmt/cmakefmt/issues) or
start a discussion in
[GitHub Discussions](https://github.com/cmakefmt/cmakefmt/discussions).
