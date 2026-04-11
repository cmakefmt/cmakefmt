---
title: Python API
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

Use `cmakefmt` as a native Python library — no subprocess, no binary on
`PATH`, just `import cmakefmt`.

## Install

```bash
pip install cmakefmt
```

Requires Python 3.11+. Pre-built wheels are available for Linux (x86_64,
aarch64), macOS (x86_64, aarch64), and Windows (x64).

## Quick Start

```python
import cmakefmt

# Format CMake source code
source = "CMAKE_MINIMUM_REQUIRED(  VERSION   3.20 )\n"
formatted = cmakefmt.format_source(source)
print(formatted)
# cmake_minimum_required(VERSION 3.20)

# Check if already formatted
print(cmakefmt.is_formatted(source))  # False
```

## API Reference

### `cmakefmt.format_source(source, *, config=None)`

Format CMake source code and return the formatted string.

**Parameters:**

- `source` (str) — CMake source code to format.
- `config` (str, optional) — YAML config string using the same format
  as `.cmakefmt.yaml` files. Supports `format:`, `markup:`,
  `per_command_overrides:`, and `commands:` sections.

**Returns:** Formatted source code as a string.

**Raises:** `ParseError`, `ConfigError`, `FormatterError`, or `LayoutError`.

```python
# With default config
formatted = cmakefmt.format_source("set(  FOO  bar )\n")

# With explicit config
formatted = cmakefmt.format_source(
    "cmake_minimum_required(VERSION 3.20)\n",
    config="format:\n  command_case: upper",
)

# With custom command specs
formatted = cmakefmt.format_source(
    "my_command(target SOURCES a.cpp)\n",
    config="""
commands:
  my_command:
    pargs: 1
    kwargs:
      SOURCES:
        nargs: '+'
""",
)
```

### `cmakefmt.is_formatted(source, *, config=None)`

Check whether source is already correctly formatted.

**Parameters:** Same as `format_source`.

**Returns:** `True` if the source is already formatted, `False` if it would change.

```python
if not cmakefmt.is_formatted(source):
    source = cmakefmt.format_source(source)
```

### `cmakefmt.default_config()`

Return the default configuration as a YAML string. The returned string uses
the same format as `.cmakefmt.yaml` files and can be passed directly to
`format_source(config=...)`.

```python
config = cmakefmt.default_config()
print(config)  # Full YAML config template
```

### `cmakefmt.__version__`

The version of the cmakefmt library (e.g. `"0.8.0"`).

## Error Handling

All exceptions inherit from Python's `Exception`. They are independent — not
subclasses of each other.

| Exception | When raised |
| --- | --- |
| `cmakefmt.ParseError` | Source cannot be parsed as valid CMake |
| `cmakefmt.ConfigError` | Invalid YAML config or unknown config fields |
| `cmakefmt.FormatterError` | Internal formatting failure |
| `cmakefmt.LayoutError` | Line exceeds `line_width` with `require_valid_layout` enabled |

```python
try:
    formatted = cmakefmt.format_source(source)
except cmakefmt.ParseError as e:
    print(f"Parse error: {e}")
except cmakefmt.ConfigError as e:
    print(f"Config error: {e}")
```

## Config Format

The `config` parameter uses the same YAML schema as `.cmakefmt.yaml`
files:

```yaml
format:
  line_width: 100
  tab_size: 4
  dangle_parens: true
  command_case: lower
  keyword_case: upper

markup:
  enable_markup: true
  reflow_comments: true
```

Unknown fields are rejected with a `ConfigError`, matching the CLI's
config validation behavior.

## Thread Safety

`cmakefmt.format_source()` and `cmakefmt.is_formatted()` are safe to call from
multiple threads concurrently. Each call creates its own formatter state —
no shared mutable state.

## Comparison With Shelling Out

| | `import cmakefmt` | `subprocess.run(["cmakefmt", ...])` |
| --- | --- | --- |
| Startup overhead | None (native function call) | Process spawn + binary load |
| Config discovery | Pass `config=` explicitly | Automatic from file path |
| Error handling | Python exceptions | Exit codes + stderr parsing |
| Dependencies | `pip install cmakefmt` | Binary on `PATH` |
| Thread safety | Yes | Yes (separate processes) |

For formatting a single string, the native binding is ~100x faster due to
avoiding process spawn overhead.
