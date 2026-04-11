# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Tests for cmakefmt exception handling."""

import cmakefmt
import pytest


class TestParseError:
    def test_unclosed_paren(self):
        with pytest.raises(cmakefmt.ParseError):
            cmakefmt.format_source("set(FOO\n")

    def test_invalid_syntax(self):
        with pytest.raises(cmakefmt.ParseError):
            cmakefmt.format_source(")()()(")

    def test_error_message_contains_detail(self):
        with pytest.raises(cmakefmt.ParseError, match="parse|Parse"):
            cmakefmt.format_source("if(\n")

    def test_is_exception_subclass(self):
        assert issubclass(cmakefmt.ParseError, Exception)


class TestConfigError:
    def test_invalid_yaml(self):
        with pytest.raises(cmakefmt.ConfigError):
            cmakefmt.format_source("set(X 1)\n", config="{{invalid")

    def test_unknown_top_level_section(self):
        with pytest.raises(cmakefmt.ConfigError):
            cmakefmt.format_source(
                "set(X 1)\n",
                config="bogus_section:\n  foo: bar",
            )

    def test_unknown_field_in_format_section(self):
        with pytest.raises(cmakefmt.ConfigError):
            cmakefmt.format_source(
                "set(X 1)\n",
                config="format:\n  nonexistent_field: 42",
            )

    def test_wrong_type_for_line_width(self):
        with pytest.raises(cmakefmt.ConfigError):
            cmakefmt.format_source(
                "set(X 1)\n",
                config="format:\n  line_width: not_a_number",
            )

    def test_is_exception_subclass(self):
        assert issubclass(cmakefmt.ConfigError, Exception)


class TestFormatterError:
    def test_is_exception_subclass(self):
        assert issubclass(cmakefmt.FormatterError, Exception)


class TestLayoutError:
    def test_is_exception_subclass(self):
        assert issubclass(cmakefmt.LayoutError, Exception)


class TestExceptionHierarchy:
    """All custom exceptions are independent -- not subclasses of each other."""

    def test_parse_error_not_config_error(self):
        assert not issubclass(cmakefmt.ParseError, cmakefmt.ConfigError)

    def test_config_error_not_parse_error(self):
        assert not issubclass(cmakefmt.ConfigError, cmakefmt.ParseError)

    def test_all_are_exceptions(self):
        for exc in [
            cmakefmt.ParseError,
            cmakefmt.ConfigError,
            cmakefmt.FormatterError,
            cmakefmt.LayoutError,
        ]:
            assert issubclass(exc, Exception)
