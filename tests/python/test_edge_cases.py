# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Edge case and stress tests for cmakefmt Python bindings."""

import concurrent.futures

import cmakefmt
import pytest


class TestEdgeCases:
    def test_very_long_line(self):
        source = f"set({'A' * 10000} 1)\n"
        result = cmakefmt.format_source(source)
        assert isinstance(result, str)

    def test_deeply_nested_control_flow(self):
        source = ""
        for i in range(20):
            source += "  " * i + "if(TRUE)\n"
        for i in range(19, -1, -1):
            source += "  " * i + "endif()\n"
        result = cmakefmt.format_source(source)
        assert result.count("if(TRUE)") == 20

    def test_many_commands(self):
        source = "".join(f"set(VAR{i} value{i})\n" for i in range(500))
        result = cmakefmt.format_source(source)
        assert result.count("set(") == 500

    def test_source_with_crlf_line_endings(self):
        source = "set(  FOO  bar )\r\nset(  BAZ  qux )\r\n"
        result = cmakefmt.format_source(source)
        assert "set(FOO bar)" in result

    def test_source_with_mixed_line_endings(self):
        source = "set(FOO bar)\nset(BAZ qux)\r\n"
        result = cmakefmt.format_source(source)
        assert isinstance(result, str)

    def test_source_without_trailing_newline(self):
        source = "set(FOO bar)"
        result = cmakefmt.format_source(source)
        assert isinstance(result, str)

    def test_only_comments(self):
        source = "# comment 1\n# comment 2\n"
        result = cmakefmt.format_source(source)
        assert "# comment 1" in result
        assert "# comment 2" in result

    def test_cmake_format_off_on(self):
        source = "# cmakefmt: off\nset(  X  1 )\n# cmakefmt: on\nset(  Y  2 )\n"
        result = cmakefmt.format_source(source)
        assert "set(  X  1 )" in result  # preserved verbatim
        assert "set(Y 2)" in result  # formatted

    def test_bracket_comment(self):
        source = "#[==[This is a bracket comment]==]\nset(X 1)\n"
        result = cmakefmt.format_source(source)
        assert "#[==[This is a bracket comment]==]" in result

    def test_concurrent_calls(self):
        """Multiple format calls should not interfere."""
        sources = [f"set(VAR{i}  value{i} )\n" for i in range(50)]
        with concurrent.futures.ThreadPoolExecutor(max_workers=4) as ex:
            results = list(ex.map(cmakefmt.format_source, sources))
        for i, result in enumerate(results):
            assert f"set(VAR{i} value{i})" in result

    def test_none_source_raises_type_error(self):
        with pytest.raises(TypeError):
            cmakefmt.format_source(None)

    def test_bytes_source_raises_type_error(self):
        with pytest.raises(TypeError):
            cmakefmt.format_source(b"set(X 1)\n")

    def test_int_source_raises_type_error(self):
        with pytest.raises(TypeError):
            cmakefmt.format_source(42)
