// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use regex::Regex;

/// User-facing custom ignore filename honored during recursive discovery.
pub const CUSTOM_IGNORE_FILE_NAME: &str = ".cmakefmtignore";

/// Options controlling recursive CMake file discovery.
#[derive(Debug, Clone, Default)]
pub struct DiscoveryOptions<'a> {
    /// Optional regex filter applied after filename/ignore filtering.
    pub file_filter: Option<&'a Regex>,
    /// Honor Git ignore files while walking directories.
    pub honor_gitignore: bool,
    /// Additional ignore files loaded explicitly by the user.
    pub explicit_ignore_paths: &'a [PathBuf],
}

/// Recursively discover CMake files below `root`, optionally filtering the
/// discovered paths with `file_filter`.
///
/// Returned paths are sorted to keep CLI output and batch formatting stable.
pub fn discover_cmake_files(root: &Path, file_filter: Option<&Regex>) -> Vec<PathBuf> {
    discover_cmake_files_with_options(
        root,
        DiscoveryOptions {
            file_filter,
            honor_gitignore: false,
            explicit_ignore_paths: &[],
        },
    )
}

/// Recursively discover CMake files below `root` using the provided workflow
/// options, including ignore-file handling.
pub fn discover_cmake_files_with_options(
    root: &Path,
    options: DiscoveryOptions<'_>,
) -> Vec<PathBuf> {
    let mut builder = WalkBuilder::new(root);
    builder.hidden(false);
    builder.git_ignore(options.honor_gitignore);
    builder.git_global(options.honor_gitignore);
    builder.git_exclude(options.honor_gitignore);
    builder.require_git(false);
    builder.add_custom_ignore_filename(CUSTOM_IGNORE_FILE_NAME);

    for ignore_path in options.explicit_ignore_paths {
        builder.add_ignore(ignore_path);
    }

    let mut files: Vec<_> = builder
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
        .map(|entry| entry.into_path())
        .filter(|path| is_cmake_file(path))
        .filter(|path| matches_filter(path, options.file_filter))
        .collect();
    files.sort();
    files
}

/// Returns `true` when the path matches one of the built-in CMake filename
/// patterns understood by `cmakefmt`.
///
/// Supported patterns are:
///
/// - `CMakeLists.txt`
/// - `*.cmake`
/// - `*.cmake.in`
pub fn is_cmake_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(OsStr::to_str) else {
        return false;
    };

    if file_name == "CMakeLists.txt" {
        return true;
    }

    file_name.ends_with(".cmake") || file_name.ends_with(".cmake.in")
}

/// Returns `true` when `path` matches the optional user-supplied discovery
/// regex.
///
/// When no regex is supplied, every discovered CMake file matches.
pub fn matches_filter(path: &Path, file_filter: Option<&Regex>) -> bool {
    let Some(file_filter) = file_filter else {
        return true;
    };

    file_filter.is_match(&path.to_string_lossy())
}
