// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `.editorconfig` fallback for formatting options.
//!
//! When no `.cmakefmt.yaml`/`.yml`/`.toml` config file is found, cmakefmt
//! reads `indent_style` and `indent_size` from `.editorconfig` as a fallback.
//! A cmakefmt config file always takes precedence.

use std::path::Path;

/// Properties extracted from `.editorconfig` that cmakefmt can use.
#[derive(Debug, Default)]
pub struct EditorConfigOverrides {
    pub tab_size: Option<usize>,
    pub use_tabs: Option<bool>,
}

impl EditorConfigOverrides {
    pub fn has_any(&self) -> bool {
        self.tab_size.is_some() || self.use_tabs.is_some()
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

    EditorConfigOverrides { tab_size, use_tabs }
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
