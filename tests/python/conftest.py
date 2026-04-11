# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Shared fixtures for cmakefmt Python tests."""

import pytest


@pytest.fixture
def cmake_source():
    return "cmake_minimum_required(VERSION 3.20)\nproject(test)\nset(FOO bar)\n"


@pytest.fixture
def unformatted_source():
    return "CMAKE_MINIMUM_REQUIRED(  VERSION   3.20 )\nproject(  test )\nset(  FOO  bar )\n"


@pytest.fixture
def config_dir(tmp_path):
    """Create a temp directory with a .cmakefmt.yaml config."""
    config = tmp_path / ".cmakefmt.yaml"
    config.write_text("format:\n  command_case: upper\n")
    return tmp_path
