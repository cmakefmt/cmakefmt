# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Tests for cmakefmt.format_source()."""

import cmakefmt
import pytest


class TestFormatSourceBasic:
    """Basic formatting with default config."""

    def test_formats_simple_command(self):
        result = cmakefmt.format_source("set(  FOO  bar )\n")
        assert result == "set(FOO bar)\n"

    def test_formats_cmake_minimum_required(self):
        result = cmakefmt.format_source("CMAKE_MINIMUM_REQUIRED(VERSION 3.20)\n")
        assert result == "cmake_minimum_required(VERSION 3.20)\n"

    def test_preserves_already_formatted(self):
        source = "set(FOO bar)\n"
        assert cmakefmt.format_source(source) == source

    def test_empty_input(self):
        assert cmakefmt.format_source("") == ""

    def test_whitespace_only(self):
        result = cmakefmt.format_source("   \n\n  \n")
        assert result.strip() == ""

    def test_single_newline(self):
        assert cmakefmt.format_source("\n") == "\n"

    def test_multiple_commands(self):
        source = "set(  A  1 )\nset(  B  2 )\n"
        result = cmakefmt.format_source(source)
        assert "set(A 1)" in result
        assert "set(B 2)" in result

    def test_preserves_comments(self):
        source = "# This is a comment\nset(FOO bar)\n"
        result = cmakefmt.format_source(source)
        assert "# This is a comment" in result

    def test_preserves_blank_lines(self):
        source = "set(A 1)\n\nset(B 2)\n"
        result = cmakefmt.format_source(source)
        assert "\n\n" in result

    def test_handles_nested_control_flow(self):
        source = "if(TRUE)\n  set(  X  1 )\nendif()\n"
        result = cmakefmt.format_source(source)
        assert "if(TRUE)" in result
        assert "set(X 1)" in result
        assert "endif()" in result

    def test_handles_quoted_arguments(self):
        source = 'message(STATUS "hello world")\n'
        result = cmakefmt.format_source(source)
        assert '"hello world"' in result

    def test_handles_bracket_arguments(self):
        source = "message([==[hello]==])\n"
        result = cmakefmt.format_source(source)
        assert "[==[hello]==]" in result

    def test_handles_variable_references(self):
        source = "set(PATH ${CMAKE_SOURCE_DIR}/src)\n"
        result = cmakefmt.format_source(source)
        assert "${CMAKE_SOURCE_DIR}" in result

    def test_handles_generator_expressions(self):
        source = "target_link_libraries(foo $<$<BOOL:${X}>:bar>)\n"
        result = cmakefmt.format_source(source)
        assert "$<$<BOOL:${X}>:bar>" in result

    def test_long_argument_list_wraps(self):
        args = " ".join(f"arg{i}" for i in range(20))
        source = f"set({args})\n"
        result = cmakefmt.format_source(source)
        assert "\n" in result  # should wrap

    def test_idempotent(self):
        """Formatting twice produces same result."""
        source = 'set(  FOO  bar  baz )\nif(TRUE)\nmessage(STATUS  "hi")\nendif()\n'
        first = cmakefmt.format_source(source)
        second = cmakefmt.format_source(first)
        assert first == second

    def test_return_type_is_str(self):
        result = cmakefmt.format_source("set(X 1)\n")
        assert isinstance(result, str)

    def test_unicode_in_strings(self):
        source = 'message("hello world")\n'
        result = cmakefmt.format_source(source)
        assert "hello world" in result

    def test_utf8_bom_handled(self):
        source = "\ufeffcmake_minimum_required(VERSION 3.20)\n"
        result = cmakefmt.format_source(source)
        assert "cmake_minimum_required" in result


class TestFormatSourceConfig:
    """Formatting with config parameter."""

    def test_command_case_upper(self):
        result = cmakefmt.format_source(
            "cmake_minimum_required(VERSION 3.20)\n",
            config="format:\n  command_case: upper",
        )
        assert "CMAKE_MINIMUM_REQUIRED" in result

    def test_command_case_lower(self):
        result = cmakefmt.format_source(
            "CMAKE_MINIMUM_REQUIRED(VERSION 3.20)\n",
            config="format:\n  command_case: lower",
        )
        assert "cmake_minimum_required" in result

    def test_command_case_unchanged(self):
        result = cmakefmt.format_source(
            "CMAKE_MINIMUM_REQUIRED(VERSION 3.20)\n",
            config="format:\n  command_case: unchanged",
        )
        assert "CMAKE_MINIMUM_REQUIRED" in result

    def test_keyword_case_upper(self):
        result = cmakefmt.format_source(
            'set(FOO bar CACHE STRING "")\n',
            config="format:\n  keyword_case: upper",
        )
        assert "CACHE" in result

    def test_line_width(self):
        source = "set(FOO bar baz qux quux corge)\n"
        narrow = cmakefmt.format_source(
            source, config="format:\n  line_width: 20"
        )
        wide = cmakefmt.format_source(
            source, config="format:\n  line_width: 200"
        )
        # Narrow should wrap onto more lines than wide
        assert narrow.count("\n") > wide.count("\n")

    def test_tab_size(self):
        source = "if(TRUE)\nset(X 1)\nendif()\n"
        result = cmakefmt.format_source(
            source, config="format:\n  tab_size: 4"
        )
        lines = result.split("\n")
        indented = [line for line in lines if line.startswith("    ")]
        assert len(indented) > 0

    def test_dangle_parens_true(self):
        args = " ".join(f"arg{i}" for i in range(15))
        source = f"set({args})\n"
        result = cmakefmt.format_source(
            source,
            config="format:\n  line_width: 40\n  dangle_parens: true",
        )
        lines = result.strip().split("\n")
        assert lines[-1].strip() == ")"

    def test_multiple_format_options(self):
        result = cmakefmt.format_source(
            "CMAKE_MINIMUM_REQUIRED(VERSION 3.20)\n",
            config="format:\n  line_width: 120\n  command_case: upper",
        )
        assert "CMAKE_MINIMUM_REQUIRED" in result

    def test_config_with_commands_section(self):
        source = "my_custom(target SOURCES a.cpp b.cpp)\n"
        result = cmakefmt.format_source(
            source,
            config=(
                "commands:\n"
                "  my_custom:\n"
                "    pargs: 1\n"
                "    kwargs:\n"
                "      SOURCES:\n"
                "        nargs: '+'"
            ),
        )
        assert "SOURCES" in result

    def test_config_none_uses_defaults(self):
        result = cmakefmt.format_source("set(  X  1 )\n", config=None)
        assert result == "set(X 1)\n"

    def test_empty_config_uses_defaults(self):
        result = cmakefmt.format_source("set(  X  1 )\n", config="")
        assert result == "set(X 1)\n"
