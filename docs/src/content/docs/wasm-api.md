---
title: WASM API
description: Use cmakefmt in the browser or Node.js via WebAssembly.
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

The `cmakefmt` WASM build powers the
[browser playground](https://cmakefmt.dev/playground/) and can be used
in any JavaScript or TypeScript project.

## Building

```bash
# Install wasm-pack
cargo install wasm-pack

# Build the WASM package
wasm-pack build --target web --no-default-features --features browser-wasm
```

This produces a `pkg/` directory with the WASM module and TypeScript
bindings.

## API

The WASM module exports two formatter helpers, alongside initialization functions:

### `format(source: string, config_yaml: string): string`

Formats a CMake source string using the given YAML config.

**Parameters:**

- `source` — the CMake source code to format
- `config_yaml` — a YAML string with config options (same format as
  `.cmakefmt.yaml`); pass `""` for defaults

**Returns:** the formatted source code as a string.

**Throws:** if parsing fails or the config is invalid.

### `default_config_yaml(): string`

Returns the default YAML config template, matching `cmakefmt config dump`.
Pass it directly to `format()` or use it to initialize a config editor.

## Usage in the browser

```html
<script type="module">
  import init, { format } from './pkg/cmakefmt.js';

  await init();

  const source = 'CMAKE_MINIMUM_REQUIRED(VERSION 3.20)';
  const config = 'format:\n  line_width: 100';
  const formatted = format(source, config);
  console.log(formatted);
  // → cmake_minimum_required(VERSION 3.20)
</script>
```

## Usage in Node.js

The same `--target web` package works in Node.js when imported as an ES module.
Save this example as `format.mjs` next to `pkg/`, then run `node format.mjs`.
Read the WASM bytes from disk and pass them to `init()` before formatting:

<!-- docs-check -->

```javascript
import { readFileSync } from 'node:fs';
import init, { format, default_config_yaml } from './pkg/cmakefmt.js';

const bytes = readFileSync(new URL('./pkg/cmakefmt_bg.wasm', import.meta.url));
await init({ module_or_path: bytes });

const formatted = format(
  'SET(FOO bar baz)',
  default_config_yaml()
);
console.log(formatted);
// → set(FOO bar baz)
```

## Limitations

- The WASM build does not include the CLI, LSP server, or file discovery
- Custom command specs must be passed as part of the config YAML
- Performance is comparable to native for small files; large files may be
  slower due to WASM overhead
