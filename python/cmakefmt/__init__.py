# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

"""A fast, correct CMake formatter."""

from cmakefmt._cmakefmt import (
    ConfigError,
    FormatterError,
    LayoutError,
    ParseError,
    __version__,
    is_formatted,
    default_config,
    format_source,
)

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
