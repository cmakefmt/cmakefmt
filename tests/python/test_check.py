# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Tests for cmakefmt.is_formatted()."""

import cmakefmt
import pytest


class TestIsFormatted:
    def test_returns_false_for_unformatted(self):
        assert not cmakefmt.is_formatted("CMAKE_MINIMUM_REQUIRED(VERSION 3.20)\n")

    def test_returns_true_for_formatted(self):
        assert cmakefmt.is_formatted("cmake_minimum_required(VERSION 3.20)\n")

    def test_returns_true_for_empty(self):
        assert cmakefmt.is_formatted("")

    def test_with_config_needs_formatting(self):
        # Already uppercase, but config says lower -- not formatted
        assert not cmakefmt.is_formatted(
            "CMAKE_MINIMUM_REQUIRED(VERSION 3.20)\n",
            config="format:\n  command_case: lower",
        )

    def test_with_config_already_matching(self):
        assert cmakefmt.is_formatted(
            "CMAKE_MINIMUM_REQUIRED(VERSION 3.20)\n",
            config="format:\n  command_case: upper",
        )

    def test_with_dict_config(self):
        assert not cmakefmt.is_formatted(
            "CMAKE_MINIMUM_REQUIRED(VERSION 3.20)\n",
            config={"format": {"command_case": "lower"}},
        )

    def test_returns_bool(self):
        result = cmakefmt.is_formatted("set(X 1)\n")
        assert isinstance(result, bool)

    def test_parse_error_propagates(self):
        with pytest.raises(cmakefmt.ParseError):
            cmakefmt.is_formatted("if(\n")
