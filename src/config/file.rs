use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::config::{CaseStyle, Config, DangleAlign, PerCommandConfig};
use crate::error::{Error, Result};

/// The TOML file structure for `.cmake-format.toml`.
///
/// All fields are optional — only specified values override the defaults.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct FileConfig {
    format: FormatSection,
    style: StyleSection,
    markup: MarkupSection,
    #[serde(rename = "per_command")]
    per_command: HashMap<String, PerCommandConfig>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct FormatSection {
    line_width: Option<usize>,
    tab_size: Option<usize>,
    use_tabchars: Option<bool>,
    max_empty_lines: Option<usize>,
    max_lines_hwrap: Option<usize>,
    max_pargs_hwrap: Option<usize>,
    max_subgroups_hwrap: Option<usize>,
    dangle_parens: Option<bool>,
    dangle_align: Option<DangleAlign>,
    min_prefix_chars: Option<usize>,
    max_prefix_chars: Option<usize>,
    separate_ctrl_name_with_space: Option<bool>,
    separate_fn_name_with_space: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct StyleSection {
    command_case: Option<CaseStyle>,
    keyword_case: Option<CaseStyle>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct MarkupSection {
    enable_markup: Option<bool>,
    reflow_comments: Option<bool>,
    first_comment_is_literal: Option<bool>,
    literal_comment_pattern: Option<String>,
    bullet_char: Option<String>,
    enum_char: Option<String>,
    fence_pattern: Option<String>,
    ruler_pattern: Option<String>,
    hashruler_min_length: Option<usize>,
    canonicalize_hashrulers: Option<bool>,
}

const CONFIG_FILE_NAME: &str = ".cmake-format.toml";

pub fn default_config_template() -> String {
    format!(
        concat!(
            "# Default cmakefmt configuration.\n",
            "# Copy this to .cmake-format.toml and uncomment the optional settings\n",
            "# you want to customize.\n\n",
            "[format]\n",
            "line_width = {line_width}\n",
            "tab_size = {tab_size}\n",
            "# Uncomment to indent with tabs instead of spaces.\n",
            "# use_tabchars = true\n",
            "max_empty_lines = {max_empty_lines}\n",
            "max_lines_hwrap = {max_lines_hwrap}\n",
            "max_pargs_hwrap = {max_pargs_hwrap}\n",
            "max_subgroups_hwrap = {max_subgroups_hwrap}\n",
            "dangle_parens = {dangle_parens}\n",
            "dangle_align = \"{dangle_align}\"\n",
            "min_prefix_chars = {min_prefix_chars}\n",
            "max_prefix_chars = {max_prefix_chars}\n",
            "# Uncomment to insert a space before control-flow parentheses.\n",
            "# separate_ctrl_name_with_space = true\n",
            "# Uncomment to insert a space before function or macro parentheses.\n",
            "# separate_fn_name_with_space = true\n\n",
            "[style]\n",
            "command_case = \"{command_case}\"\n",
            "keyword_case = \"{keyword_case}\"\n\n",
            "[markup]\n",
            "enable_markup = {enable_markup}\n",
            "# Uncomment to reflow line comments to fit within the configured line width.\n",
            "# reflow_comments = true\n",
            "first_comment_is_literal = {first_comment_is_literal}\n",
            "# Uncomment to preserve comments matching a custom regex literally.\n",
            "# literal_comment_pattern = \"^\\\\s*NOTE:\"\n",
            "bullet_char = \"{bullet_char}\"\n",
            "enum_char = \"{enum_char}\"\n",
            "fence_pattern = '{fence_pattern}'\n",
            "ruler_pattern = '{ruler_pattern}'\n",
            "hashruler_min_length = {hashruler_min_length}\n",
            "canonicalize_hashrulers = {canonicalize_hashrulers}\n\n",
            "# Uncomment and edit a block like this to override formatting for a\n",
            "# specific command.\n",
            "# [per_command.message]\n",
            "# line_width = 120\n",
            "# command_case = \"unchanged\"\n",
            "# keyword_case = \"upper\"\n",
            "# tab_size = 4\n",
            "# dangle_parens = false\n",
            "# dangle_align = \"prefix\"\n",
            "# max_pargs_hwrap = 8\n",
            "# max_subgroups_hwrap = 3\n",
        ),
        line_width = Config::default().line_width,
        tab_size = Config::default().tab_size,
        max_empty_lines = Config::default().max_empty_lines,
        max_lines_hwrap = Config::default().max_lines_hwrap,
        max_pargs_hwrap = Config::default().max_pargs_hwrap,
        max_subgroups_hwrap = Config::default().max_subgroups_hwrap,
        dangle_parens = Config::default().dangle_parens,
        dangle_align = "prefix",
        min_prefix_chars = Config::default().min_prefix_chars,
        max_prefix_chars = Config::default().max_prefix_chars,
        command_case = "lower",
        keyword_case = "upper",
        enable_markup = Config::default().enable_markup,
        first_comment_is_literal = Config::default().first_comment_is_literal,
        bullet_char = Config::default().bullet_char,
        enum_char = Config::default().enum_char,
        fence_pattern = Config::default().fence_pattern,
        ruler_pattern = Config::default().ruler_pattern,
        hashruler_min_length = Config::default().hashruler_min_length,
        canonicalize_hashrulers = Config::default().canonicalize_hashrulers,
    )
}

impl Config {
    /// Load configuration for a file at the given path.
    ///
    /// Searches for `.cmake-format.toml` starting from the file's directory,
    /// walking up to the repository/filesystem root, then checks `~/.cmake-format.toml`.
    /// Merges all found configs (closest wins).
    pub fn for_file(file_path: &Path) -> Result<Self> {
        let mut config = Config::default();

        let config_paths = find_config_files(file_path);
        // Apply in reverse order (most distant first) so closest wins.
        for path in config_paths.iter().rev() {
            let file_config = load_config_file(path)?;
            config.apply(file_config);
        }

        Ok(config)
    }

    /// Load configuration from a specific TOML file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let mut config = Config::default();
        let file_config = load_config_file(path)?;
        config.apply(file_config);
        Ok(config)
    }

    fn apply(&mut self, fc: FileConfig) {
        // Format section
        if let Some(v) = fc.format.line_width {
            self.line_width = v;
        }
        if let Some(v) = fc.format.tab_size {
            self.tab_size = v;
        }
        if let Some(v) = fc.format.use_tabchars {
            self.use_tabchars = v;
        }
        if let Some(v) = fc.format.max_empty_lines {
            self.max_empty_lines = v;
        }
        if let Some(v) = fc.format.max_lines_hwrap {
            self.max_lines_hwrap = v;
        }
        if let Some(v) = fc.format.max_pargs_hwrap {
            self.max_pargs_hwrap = v;
        }
        if let Some(v) = fc.format.max_subgroups_hwrap {
            self.max_subgroups_hwrap = v;
        }
        if let Some(v) = fc.format.dangle_parens {
            self.dangle_parens = v;
        }
        if let Some(v) = fc.format.dangle_align {
            self.dangle_align = v;
        }
        if let Some(v) = fc.format.min_prefix_chars {
            self.min_prefix_chars = v;
        }
        if let Some(v) = fc.format.max_prefix_chars {
            self.max_prefix_chars = v;
        }
        if let Some(v) = fc.format.separate_ctrl_name_with_space {
            self.separate_ctrl_name_with_space = v;
        }
        if let Some(v) = fc.format.separate_fn_name_with_space {
            self.separate_fn_name_with_space = v;
        }

        // Style section
        if let Some(v) = fc.style.command_case {
            self.command_case = v;
        }
        if let Some(v) = fc.style.keyword_case {
            self.keyword_case = v;
        }

        // Markup section
        if let Some(v) = fc.markup.enable_markup {
            self.enable_markup = v;
        }
        if let Some(v) = fc.markup.reflow_comments {
            self.reflow_comments = v;
        }
        if let Some(v) = fc.markup.first_comment_is_literal {
            self.first_comment_is_literal = v;
        }
        if let Some(v) = fc.markup.literal_comment_pattern {
            self.literal_comment_pattern = v;
        }
        if let Some(v) = fc.markup.bullet_char {
            self.bullet_char = v;
        }
        if let Some(v) = fc.markup.enum_char {
            self.enum_char = v;
        }
        if let Some(v) = fc.markup.fence_pattern {
            self.fence_pattern = v;
        }
        if let Some(v) = fc.markup.ruler_pattern {
            self.ruler_pattern = v;
        }
        if let Some(v) = fc.markup.hashruler_min_length {
            self.hashruler_min_length = v;
        }
        if let Some(v) = fc.markup.canonicalize_hashrulers {
            self.canonicalize_hashrulers = v;
        }

        // Per-command overrides (merge, don't replace)
        for (name, overrides) in fc.per_command {
            self.per_command.insert(name, overrides);
        }
    }
}

fn load_config_file(path: &Path) -> Result<FileConfig> {
    let contents = std::fs::read_to_string(path).map_err(Error::Io)?;
    toml::from_str(&contents).map_err(|source| Error::Config {
        path: path.to_path_buf(),
        source,
    })
}

/// Find all `.cmake-format.toml` files from the file's directory up to root,
/// plus the user home config. Returns them ordered closest-first.
fn find_config_files(file_path: &Path) -> Vec<PathBuf> {
    let mut configs = Vec::new();

    // Walk from the file's parent directory up to root.
    let start_dir = if file_path.is_dir() {
        file_path.to_path_buf()
    } else {
        file_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    };

    let mut dir = Some(start_dir.as_path());
    while let Some(d) = dir {
        let candidate = d.join(CONFIG_FILE_NAME);
        if candidate.is_file() {
            configs.push(candidate);
        }

        // Stop at git root to avoid searching unrelated parent directories.
        if d.join(".git").exists() {
            break;
        }

        dir = d.parent();
    }

    // Check user home directory.
    if let Some(home) = home_dir() {
        let home_config = home.join(CONFIG_FILE_NAME);
        if home_config.is_file() && !configs.contains(&home_config) {
            configs.push(home_config);
        }
    }

    configs
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parse_empty_config() {
        let config: FileConfig = toml::from_str("").unwrap();
        assert!(config.format.line_width.is_none());
    }

    #[test]
    fn parse_full_config() {
        let toml_str = r#"
[format]
line_width = 120
tab_size = 4
use_tabchars = true
max_empty_lines = 2
dangle_parens = true
dangle_align = "open"
separate_ctrl_name_with_space = true
separate_fn_name_with_space = true
max_pargs_hwrap = 3
max_subgroups_hwrap = 1

[style]
command_case = "upper"
keyword_case = "lower"

[markup]
enable_markup = false
hashruler_min_length = 20

[per_command.message]
dangle_parens = true
line_width = 100
"#;
        let config: FileConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.format.line_width, Some(120));
        assert_eq!(config.format.tab_size, Some(4));
        assert_eq!(config.format.use_tabchars, Some(true));
        assert_eq!(config.format.dangle_parens, Some(true));
        assert_eq!(config.format.dangle_align, Some(DangleAlign::Open));
        assert_eq!(config.style.command_case, Some(CaseStyle::Upper));
        assert_eq!(config.style.keyword_case, Some(CaseStyle::Lower));
        assert_eq!(config.markup.enable_markup, Some(false));

        let msg = config.per_command.get("message").unwrap();
        assert_eq!(msg.dangle_parens, Some(true));
        assert_eq!(msg.line_width, Some(100));
    }

    #[test]
    fn config_from_file_applies_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join(CONFIG_FILE_NAME);
        fs::write(
            &config_path,
            r#"
[format]
line_width = 100
tab_size = 4

[style]
command_case = "upper"
"#,
        )
        .unwrap();

        let config = Config::from_file(&config_path).unwrap();
        assert_eq!(config.line_width, 100);
        assert_eq!(config.tab_size, 4);
        assert_eq!(config.command_case, CaseStyle::Upper);
        // Unspecified values keep defaults
        assert!(!config.use_tabchars);
        assert_eq!(config.max_empty_lines, 1);
    }

    #[test]
    fn default_config_template_parses() {
        let template = default_config_template();
        let parsed: FileConfig = toml::from_str(&template).unwrap();
        assert_eq!(parsed.format.line_width, Some(Config::default().line_width));
        assert_eq!(
            parsed.style.command_case,
            Some(Config::default().command_case)
        );
        assert_eq!(
            parsed.markup.enable_markup,
            Some(Config::default().enable_markup)
        );
    }

    #[test]
    fn missing_config_file_uses_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let fake_file = dir.path().join("CMakeLists.txt");
        fs::write(&fake_file, "").unwrap();

        let config = Config::for_file(&fake_file).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn config_file_in_parent_is_found() {
        let dir = tempfile::tempdir().unwrap();
        // Create a .git dir to act as root
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::write(
            dir.path().join(CONFIG_FILE_NAME),
            "[format]\nline_width = 120\n",
        )
        .unwrap();

        let subdir = dir.path().join("src");
        fs::create_dir(&subdir).unwrap();
        let file = subdir.join("CMakeLists.txt");
        fs::write(&file, "").unwrap();

        let config = Config::for_file(&file).unwrap();
        assert_eq!(config.line_width, 120);
    }

    #[test]
    fn closer_config_wins() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::write(
            dir.path().join(CONFIG_FILE_NAME),
            "[format]\nline_width = 120\ntab_size = 4\n",
        )
        .unwrap();

        let subdir = dir.path().join("src");
        fs::create_dir(&subdir).unwrap();
        fs::write(
            subdir.join(CONFIG_FILE_NAME),
            "[format]\nline_width = 100\n",
        )
        .unwrap();

        let file = subdir.join("CMakeLists.txt");
        fs::write(&file, "").unwrap();

        let config = Config::for_file(&file).unwrap();
        // Closer config wins for line_width
        assert_eq!(config.line_width, 100);
        // Parent config still applies for tab_size
        assert_eq!(config.tab_size, 4);
    }

    #[test]
    fn invalid_toml_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CONFIG_FILE_NAME);
        fs::write(&path, "this is not valid toml {{{").unwrap();

        let result = Config::from_file(&path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("config error"));
    }
}
