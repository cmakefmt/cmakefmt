---
title: Editor Integration
description: Use cmakefmt as the formatter in VS Code, Neovim, Helix, Zed, and Emacs.
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

`cmakefmt` reads from stdin and writes to stdout, which means every editor that
supports external formatters works out of the box. Install `cmakefmt` once,
then drop the config snippet for your editor below.

## VS Code

Install the official extension from the VS Code Marketplace:

- **Extension ID**: `cmakefmt.vscode-cmakefmt`
- **Marketplace**: [cmakefmt — VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=cmakefmt.vscode-cmakefmt)

The extension formats on save by default and uses whichever `cmakefmt` binary
is on your `PATH`. To customise, add any of the following to your
`.vscode/settings.json`:

```json
{
  "cmakefmt.executablePath": "cmakefmt",
  "cmakefmt.extraArgs": [],
  "cmakefmt.onSave": true
}
```

| Setting | Default | Description |
|---|---|---|
| `cmakefmt.executablePath` | `"cmakefmt"` | Path to the `cmakefmt` binary |
| `cmakefmt.extraArgs` | `[]` | Extra flags, e.g. `["--config", "/path/to/.cmakefmt.yaml"]` |
| `cmakefmt.onSave` | `true` | Format CMake files automatically on save |

## Neovim

Use [conform.nvim](https://github.com/stevearc/conform.nvim) — the standard
Neovim formatter plugin. Add `cmakefmt` as a custom formatter and associate it
with the `cmake` filetype:

```lua
require("conform").setup({
  formatters_by_ft = {
    cmake = { "cmakefmt" },
  },
  formatters = {
    cmakefmt = {
      command = "cmakefmt",
      args = { "--stdin-path", "$FILENAME", "-" },
      stdin = true,
    },
  },
  format_on_save = {
    timeout_ms = 2000,
    lsp_fallback = false,
  },
})
```

The `$FILENAME` argument lets `cmakefmt` discover the nearest `.cmakefmt.yaml`
config relative to the file being formatted.

## Helix

Add a `[[language]]` entry to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "cmake"
formatter = { command = "cmakefmt", args = ["-"] }
auto-format = true
```

Helix invokes the formatter from the project root, so `cmakefmt` will discover
your `.cmakefmt.yaml` automatically. The `-` argument tells `cmakefmt` to read
from stdin.

## Zed

Add the following to `~/.config/zed/settings.json`:

```json
{
  "languages": {
    "CMake": {
      "formatter": {
        "external": {
          "command": "cmakefmt",
          "arguments": ["--stdin-path", "{buffer_path}", "-"]
        }
      },
      "format_on_save": "on"
    }
  }
}
```

Zed substitutes `{buffer_path}` with the actual file path at format time, which
enables config discovery.

## Emacs

With [apheleia](https://github.com/radian-software/apheleia):

```elisp
(require 'apheleia)

(add-to-list 'apheleia-formatters
             '(cmakefmt . ("cmakefmt" "--stdin-path" filepath "-")))

(add-to-list 'apheleia-mode-alist
             '(cmake-mode . cmakefmt))

(apheleia-global-mode +1)
```

Apheleia replaces `filepath` with the path of the buffer being formatted.

## Any other editor

The general pattern is:

```
cmakefmt --stdin-path <current-file-path> -
```

Pipe the buffer contents to stdin and read the formatted output from stdout.
The `--stdin-path` argument is optional but recommended — it tells `cmakefmt`
which file the input corresponds to so it can discover the right config file and
apply the correct ignore rules.

If your editor does not support passing the file path to the formatter, omit
`--stdin-path` and `cmakefmt` will fall back to config discovery from the
current working directory.
