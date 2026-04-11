# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Tests for cmakefmt.default_config()."""

import cmakefmt
import yaml


class TestDefaultConfig:
    def test_returns_string(self):
        config = cmakefmt.default_config()
        assert isinstance(config, str)

    def test_contains_line_width(self):
        assert "line_width" in cmakefmt.default_config()

    def test_contains_command_case(self):
        assert "command_case" in cmakefmt.default_config()

    def test_contains_format_section(self):
        config = cmakefmt.default_config()
        assert "format:" in config or "line_width" in config

    def test_is_valid_yaml(self):
        config = cmakefmt.default_config()
        parsed = yaml.safe_load(config)
        assert isinstance(parsed, dict)

    def test_roundtrip_through_format_source(self):
        """Default config used explicitly should match default behavior."""
        source = "set(  FOO  bar )\n"
        default_result = cmakefmt.format_source(source)
        config = cmakefmt.default_config()
        explicit_result = cmakefmt.format_source(source, config=config)
        assert default_result == explicit_result
