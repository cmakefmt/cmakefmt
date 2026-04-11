# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Tests for cmakefmt module-level attributes."""

import cmakefmt


class TestModule:
    def test_has_version(self):
        assert hasattr(cmakefmt, "__version__")

    def test_version_is_string(self):
        assert isinstance(cmakefmt.__version__, str)

    def test_version_is_semver(self):
        parts = cmakefmt.__version__.split(".")
        assert len(parts) == 3
        assert all(p.isdigit() for p in parts)

    def test_has_format_source(self):
        assert callable(cmakefmt.format_source)

    def test_has_is_formatted(self):
        assert callable(cmakefmt.is_formatted)

    def test_has_default_config(self):
        assert callable(cmakefmt.default_config)

    def test_has_parse_error(self):
        assert cmakefmt.ParseError is not None

    def test_has_config_error(self):
        assert cmakefmt.ConfigError is not None

    def test_has_formatter_error(self):
        assert cmakefmt.FormatterError is not None

    def test_has_layout_error(self):
        assert cmakefmt.LayoutError is not None
