use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use regex::Regex;
use walkdir::WalkDir;

/// Recursively discover CMake files below `root`, optionally filtering the
/// discovered paths with `file_filter`.
///
/// Returned paths are sorted to keep CLI output and batch formatting stable.
pub fn discover_cmake_files(root: &Path, file_filter: Option<&Regex>) -> Vec<PathBuf> {
    let mut files: Vec<_> = WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| is_cmake_file(path))
        .filter(|path| matches_filter(path, file_filter))
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
