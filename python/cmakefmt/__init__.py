# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

"""A fast, correct CMake formatter."""

from __future__ import annotations

from typing import Union

from cmakefmt._cmakefmt import (
    ConfigError,
    FormatterError,
    LayoutError,
    ParseError,
    __version__,
    default_config,
)
from cmakefmt._cmakefmt import format_source as _format_source
from cmakefmt._cmakefmt import is_formatted as _is_formatted


def _resolve_config(config: Union[str, dict, None]) -> str | None:
    """Convert a config value to a YAML string for the Rust binding."""
    if config is None or isinstance(config, str):
        return config
    if isinstance(config, dict):
        import yaml

        return yaml.dump(config, default_flow_style=False)
    raise TypeError(
        f"config must be a str, dict, or None, not {type(config).__name__}"
    )


def format_source(source: str, *, config: Union[str, dict, None] = None) -> str:
    """Format CMake source code.

    Args:
        source: CMake source code as a string.
        config: Optional config as a YAML string or a dict (same schema
            as ``.cmakefmt.yaml``). Supports ``format:``, ``markup:``,
            ``per_command_overrides:``, and ``commands:`` sections.

    Returns:
        Formatted source code as a string.

    Raises:
        ParseError: If the source cannot be parsed as CMake.
        ConfigError: If the config is invalid.
        FormatterError: If formatting fails.
        LayoutError: If require_valid_layout is true and a line exceeds line_width.
    """
    return _format_source(source, config=_resolve_config(config))


def is_formatted(source: str, *, config: Union[str, dict, None] = None) -> bool:
    """Check if source is already correctly formatted.

    Args:
        source: CMake source code as a string.
        config: Optional config as a YAML string or a dict.

    Returns:
        True if the source is already formatted, False if it would change.
    """
    return _is_formatted(source, config=_resolve_config(config))


__all__ = [
    "format_source",
    "is_formatted",
    "default_config",
    "ParseError",
    "ConfigError",
    "FormatterError",
    "LayoutError",
    "__version__",
]
