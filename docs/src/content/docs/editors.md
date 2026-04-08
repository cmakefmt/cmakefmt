---
title: Editor Integration
description: Use cmakefmt as the formatter in VS Code, Neovim, Helix, Zed, and Emacs.
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

`cmakefmt` works with every editor in two ways:

1. **LSP server** (`cmakefmt --lsp`) — the recommended approach for editors
   with LSP client support. Provides format-on-save and range formatting with
   no extra plugins.
2. **Stdin pipe** (`cmakefmt --stdin-path <file> -`) — works with any editor
   that supports external formatters.

Install `cmakefmt` once, then drop the config snippet for your editor below.

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

### Option A: LSP (recommended)

Add `cmakefmt` as a language server via
[nvim-lspconfig](https://github.com/neovim/nvim-lspconfig):

```lua
vim.api.nvim_create_autocmd("FileType", {
  pattern = "cmake",
  callback = function()
    vim.lsp.start({
      name = "cmakefmt",
      cmd = { "cmakefmt", "--lsp" },
    })
  end,
})

-- Format on save
vim.api.nvim_create_autocmd("BufWritePre", {
  pattern = { "CMakeLists.txt", "*.cmake" },
  callback = function()
    vim.lsp.buf.format({ timeout_ms = 2000 })
  end,
})
```

### Option B: conform.nvim

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

### Option A: LSP (recommended)

Add `cmakefmt` as a language server in `~/.config/helix/languages.toml`:

```toml
[language-server.cmakefmt]
command = "cmakefmt"
args = ["--lsp"]

[[language]]
name = "cmake"
language-servers = ["cmakefmt"]
auto-format = true
```

### Option B: External formatter

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

### Option A: LSP (recommended)

Add `cmakefmt` as a language server in `~/.config/zed/settings.json`:

```json
{
  "lsp": {
    "cmakefmt": {
      "binary": { "path": "cmakefmt", "arguments": ["--lsp"] }
    }
  },
  "languages": {
    "CMake": {
      "language_servers": ["cmakefmt"],
      "format_on_save": "on"
    }
  }
}
```

### Option B: External formatter

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

If your editor has an LSP client, point it at:

```
cmakefmt --lsp
```

This provides format-on-save and range formatting over the standard Language
Server Protocol (JSON-RPC on stdio).

If your editor does not support LSP but does support external formatters, use
the stdin pipe approach:

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
