pub mod file;
pub use file::default_config_template;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// How to normalise command/keyword casing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum CaseStyle {
    Lower,
    #[default]
    Upper,
    Unchanged,
}

/// How to align the dangling closing paren.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DangleAlign {
    /// Align with the start of the command name.
    #[default]
    Prefix,
    /// Align with the opening paren column.
    Open,
    /// No extra indent (flush with current indent level).
    Close,
}

/// Full formatter configuration.
///
/// This struct is used at runtime. It is populated from defaults, TOML files,
/// and CLI flag overrides (highest wins).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    // ── Layout ──────────────────────────────────────────────────────────
    pub line_width: usize,
    pub tab_size: usize,
    pub use_tabchars: bool,
    pub max_empty_lines: usize,
    pub max_lines_hwrap: usize,
    pub max_pargs_hwrap: usize,
    pub max_subgroups_hwrap: usize,

    // ── Parenthesis style ───────────────────────────────────────────────
    pub dangle_parens: bool,
    pub dangle_align: DangleAlign,
    pub min_prefix_chars: usize,
    pub max_prefix_chars: usize,
    pub separate_ctrl_name_with_space: bool,
    pub separate_fn_name_with_space: bool,

    // ── Casing ──────────────────────────────────────────────────────────
    pub command_case: CaseStyle,
    pub keyword_case: CaseStyle,

    // ── Comment markup ──────────────────────────────────────────────────
    pub enable_markup: bool,
    pub first_comment_is_literal: bool,
    pub literal_comment_pattern: String,
    pub bullet_char: String,
    pub enum_char: String,
    pub fence_pattern: String,
    pub ruler_pattern: String,
    pub hashruler_min_length: usize,
    pub canonicalize_hashrulers: bool,

    // ── Per-command overrides ────────────────────────────────────────────
    pub per_command: HashMap<String, PerCommandConfig>,
}

/// Per-command overrides. All fields are optional — only specified fields
/// override the global config for that command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PerCommandConfig {
    pub command_case: Option<CaseStyle>,
    pub keyword_case: Option<CaseStyle>,
    pub line_width: Option<usize>,
    pub tab_size: Option<usize>,
    pub dangle_parens: Option<bool>,
    pub dangle_align: Option<DangleAlign>,
    pub max_pargs_hwrap: Option<usize>,
    pub max_subgroups_hwrap: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            line_width: 80,
            tab_size: 2,
            use_tabchars: false,
            max_empty_lines: 1,
            max_lines_hwrap: 2,
            max_pargs_hwrap: 6,
            max_subgroups_hwrap: 2,
            dangle_parens: true,
            dangle_align: DangleAlign::Prefix,
            min_prefix_chars: 4,
            max_prefix_chars: 10,
            separate_ctrl_name_with_space: false,
            separate_fn_name_with_space: false,
            command_case: CaseStyle::Lower,
            keyword_case: CaseStyle::Upper,
            enable_markup: true,
            first_comment_is_literal: true,
            literal_comment_pattern: String::new(),
            bullet_char: "*".to_string(),
            enum_char: ".".to_string(),
            fence_pattern: r"^\s*[`~]{3}[^`\n]*$".to_string(),
            ruler_pattern: r"^[^\w\s]{3}.*[^\w\s]{3}$".to_string(),
            hashruler_min_length: 10,
            canonicalize_hashrulers: true,
            per_command: HashMap::new(),
        }
    }
}

/// CMake control-flow commands that get `separate_ctrl_name_with_space`.
const CONTROL_FLOW_COMMANDS: &[&str] = &[
    "if",
    "elseif",
    "else",
    "endif",
    "foreach",
    "endforeach",
    "while",
    "endwhile",
    "break",
    "continue",
    "return",
    "block",
    "endblock",
];

/// CMake function/macro definition commands that get
/// `separate_fn_name_with_space`.
const FN_DEFINITION_COMMANDS: &[&str] = &["function", "endfunction", "macro", "endmacro"];

impl Config {
    /// Returns a `Config` with any per-command overrides applied for the
    /// given command name, plus the appropriate space-before-paren setting.
    pub fn for_command(&self, command_name: &str) -> CommandConfig<'_> {
        let lower = command_name.to_ascii_lowercase();
        let per_cmd = self.per_command.get(&lower);

        let space_before_paren = if CONTROL_FLOW_COMMANDS.contains(&lower.as_str()) {
            self.separate_ctrl_name_with_space
        } else if FN_DEFINITION_COMMANDS.contains(&lower.as_str()) {
            self.separate_fn_name_with_space
        } else {
            false
        };

        CommandConfig {
            global: self,
            per_cmd,
            space_before_paren,
        }
    }

    /// Apply the command_case rule to a command name.
    pub fn apply_command_case(&self, name: &str) -> String {
        apply_case(self.command_case, name)
    }

    /// Apply the keyword_case rule to a keyword token.
    pub fn apply_keyword_case(&self, keyword: &str) -> String {
        apply_case(self.keyword_case, keyword)
    }

    /// The indentation string (spaces or tab).
    pub fn indent_str(&self) -> String {
        if self.use_tabchars {
            "\t".to_string()
        } else {
            " ".repeat(self.tab_size)
        }
    }
}

/// A resolved config for formatting a specific command, with per-command
/// overrides already applied.
#[derive(Debug)]
pub struct CommandConfig<'a> {
    pub global: &'a Config,
    per_cmd: Option<&'a PerCommandConfig>,
    pub space_before_paren: bool,
}

impl CommandConfig<'_> {
    pub fn line_width(&self) -> usize {
        self.per_cmd
            .and_then(|p| p.line_width)
            .unwrap_or(self.global.line_width)
    }

    pub fn tab_size(&self) -> usize {
        self.per_cmd
            .and_then(|p| p.tab_size)
            .unwrap_or(self.global.tab_size)
    }

    pub fn dangle_parens(&self) -> bool {
        self.per_cmd
            .and_then(|p| p.dangle_parens)
            .unwrap_or(self.global.dangle_parens)
    }

    pub fn dangle_align(&self) -> DangleAlign {
        self.per_cmd
            .and_then(|p| p.dangle_align)
            .unwrap_or(self.global.dangle_align)
    }

    pub fn command_case(&self) -> CaseStyle {
        self.per_cmd
            .and_then(|p| p.command_case)
            .unwrap_or(self.global.command_case)
    }

    pub fn keyword_case(&self) -> CaseStyle {
        self.per_cmd
            .and_then(|p| p.keyword_case)
            .unwrap_or(self.global.keyword_case)
    }

    pub fn max_pargs_hwrap(&self) -> usize {
        self.per_cmd
            .and_then(|p| p.max_pargs_hwrap)
            .unwrap_or(self.global.max_pargs_hwrap)
    }

    pub fn max_subgroups_hwrap(&self) -> usize {
        self.per_cmd
            .and_then(|p| p.max_subgroups_hwrap)
            .unwrap_or(self.global.max_subgroups_hwrap)
    }

    pub fn indent_str(&self) -> String {
        if self.global.use_tabchars {
            "\t".to_string()
        } else {
            " ".repeat(self.tab_size())
        }
    }
}

fn apply_case(style: CaseStyle, s: &str) -> String {
    match style {
        CaseStyle::Lower => s.to_ascii_lowercase(),
        CaseStyle::Upper => s.to_ascii_uppercase(),
        CaseStyle::Unchanged => s.to_string(),
    }
}
