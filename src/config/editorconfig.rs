// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `.editorconfig` fallback for formatting options.
//!
//! When no `.cmakefmt.yaml`/`.yml`/`.toml` config file is found, cmakefmt
//! reads a small subset of `.editorconfig` properties as a fallback:
//! `indent_style`/`indent_size` (indentation), `max_line_length` (line width),
//! and `end_of_line` (line endings). Other properties have no cmakefmt
//! equivalent and are ignored. A cmakefmt config file always takes precedence.

use std::path::Path;

use crate::config::LineEnding;

/// Properties extracted from `.editorconfig` that cmakefmt can use.
#[derive(Debug, Default)]
pub struct EditorConfigOverrides {
    pub tab_size: Option<usize>,
    pub use_tabs: Option<bool>,
    pub line_width: Option<usize>,
    pub line_ending: Option<LineEnding>,
}

impl EditorConfigOverrides {
    pub fn has_any(&self) -> bool {
        self.tab_size.is_some()
            || self.use_tabs.is_some()
            || self.line_width.is_some()
            || self.line_ending.is_some()
    }
}

/// Read `.editorconfig` properties for the given file path.
///
/// Returns `EditorConfigOverrides` with whichever values were found.
/// Silently returns empty overrides on any error — `.editorconfig` failures
/// should never block formatting.
pub fn read_editorconfig(file_path: &Path) -> EditorConfigOverrides {
    let properties = match ec4rs::properties_of(file_path) {
        Ok(props) => props,
        Err(_) => return EditorConfigOverrides::default(),
    };

    let use_tabs = properties
        .get::<ec4rs::property::IndentStyle>()
        .ok()
        .map(|style| matches!(style, ec4rs::property::IndentStyle::Tabs));

    let tab_size = properties
        .get::<ec4rs::property::IndentSize>()
        .ok()
        .and_then(|size| match size {
            ec4rs::property::IndentSize::Value(n) => Some(n),
            ec4rs::property::IndentSize::UseTabWidth => properties
                .get::<ec4rs::property::TabWidth>()
                .ok()
                .map(|tw| match tw {
                    ec4rs::property::TabWidth::Value(n) => n,
                }),
        });

    // `max_line_length` → `line_width`. Only numeric values map; `off`
    // (disable the limit) has no cmakefmt equivalent and is skipped.
    let line_width = properties
        .get_raw_for_key("max_line_length")
        .into_option()
        .and_then(|v| v.parse::<usize>().ok());

    // `end_of_line` → `line_ending`. `lf`/`crlf` map; `cr` (classic Mac) has
    // no cmakefmt equivalent and is skipped.
    let line_ending = properties
        .get_raw_for_key("end_of_line")
        .into_option()
        .and_then(|v| {
            if v.eq_ignore_ascii_case("lf") {
                Some(LineEnding::Unix)
            } else if v.eq_ignore_ascii_case("crlf") {
                Some(LineEnding::Windows)
            } else {
                None
            }
        });

    EditorConfigOverrides {
        tab_size,
        use_tabs,
        line_width,
        line_ending,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn reads_indent_style_spaces() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(".editorconfig"),
            "[*]\nroot = true\nindent_style = space\nindent_size = 4\n",
        )
        .unwrap();
        let file = dir.path().join("CMakeLists.txt");
        fs::write(&file, "").unwrap();

        let overrides = read_editorconfig(&file);
        assert_eq!(overrides.use_tabs, Some(false));
        assert_eq!(overrides.tab_size, Some(4));
    }

    #[test]
    fn reads_indent_style_tabs() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(".editorconfig"),
            "[*]\nroot = true\nindent_style = tab\n",
        )
        .unwrap();
        let file = dir.path().join("CMakeLists.txt");
        fs::write(&file, "").unwrap();

        let overrides = read_editorconfig(&file);
        assert_eq!(overrides.use_tabs, Some(true));
    }

    #[test]
    fn reads_max_line_length() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(".editorconfig"),
            "[*]\nroot = true\nmax_line_length = 100\n",
        )
        .unwrap();
        let file = dir.path().join("CMakeLists.txt");
        fs::write(&file, "").unwrap();

        let overrides = read_editorconfig(&file);
        assert_eq!(overrides.line_width, Some(100));
    }

    #[test]
    fn ignores_max_line_length_off() {
        // `max_line_length = off` disables the limit in editorconfig; cmakefmt
        // has no equivalent, so it is skipped rather than mapped.
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(".editorconfig"),
            "[*]\nroot = true\nmax_line_length = off\n",
        )
        .unwrap();
        let file = dir.path().join("CMakeLists.txt");
        fs::write(&file, "").unwrap();

        let overrides = read_editorconfig(&file);
        assert_eq!(overrides.line_width, None);
    }

    #[test]
    fn reads_end_of_line_lf() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(".editorconfig"),
            "[*]\nroot = true\nend_of_line = lf\n",
        )
        .unwrap();
        let file = dir.path().join("CMakeLists.txt");
        fs::write(&file, "").unwrap();

        let overrides = read_editorconfig(&file);
        assert_eq!(overrides.line_ending, Some(LineEnding::Unix));
    }

    #[test]
    fn reads_end_of_line_crlf() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(".editorconfig"),
            "[*]\nroot = true\nend_of_line = crlf\n",
        )
        .unwrap();
        let file = dir.path().join("CMakeLists.txt");
        fs::write(&file, "").unwrap();

        let overrides = read_editorconfig(&file);
        assert_eq!(overrides.line_ending, Some(LineEnding::Windows));
    }

    #[test]
    fn ignores_end_of_line_cr() {
        // Classic-Mac lone-CR endings have no cmakefmt equivalent.
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(".editorconfig"),
            "[*]\nroot = true\nend_of_line = cr\n",
        )
        .unwrap();
        let file = dir.path().join("CMakeLists.txt");
        fs::write(&file, "").unwrap();

        let overrides = read_editorconfig(&file);
        assert_eq!(overrides.line_ending, None);
    }

    #[test]
    fn returns_empty_when_no_editorconfig() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("CMakeLists.txt");
        fs::write(&file, "").unwrap();

        let overrides = read_editorconfig(&file);
        assert!(!overrides.has_any());
    }

    #[test]
    fn returns_empty_on_malformed_editorconfig() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".editorconfig"), "not valid [[[").unwrap();
        let file = dir.path().join("CMakeLists.txt");
        fs::write(&file, "").unwrap();

        let overrides = read_editorconfig(&file);
        // Should not panic or error — just returns empty.
        let _ = overrides;
    }
}
