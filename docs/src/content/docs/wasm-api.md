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
wasm-pack build --target web
```

This produces a `pkg/` directory with the WASM module and TypeScript
bindings.

## API

The WASM module exports a single function:

### `format(source: string, config_yaml: string): string`

Formats a CMake source string using the given YAML config.

**Parameters:**

- `source` — the CMake source code to format
- `config_yaml` — a YAML string with config options (same format as
  `.cmakefmt.yaml`); pass `""` for defaults

**Returns:** the formatted source code as a string.

**Throws:** if parsing fails or the config is invalid.

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

```javascript
const { format } = require('./pkg/cmakefmt.js');

const formatted = format(
  'SET(FOO bar baz)',
  ''  // use defaults
);
console.log(formatted);
// → set(FOO bar baz)
```

## Limitations

- The WASM build does not include the CLI, LSP server, or file discovery
- Custom command specs must be passed as part of the config YAML
- Performance is comparable to native for small files; large files may be
  slower due to WASM overhead
