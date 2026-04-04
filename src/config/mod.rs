// SPDX-FileCopyrightText: 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Runtime formatter configuration.
//!
//! [`Config`] is the fully resolved in-memory configuration used by the
//! formatter. It is built from defaults, user config files
//! (`.cmakefmt.yaml`, `.cmakefmt.yml`, or `.cmakefmt.toml`), and CLI
//! overrides.

pub mod file;
mod legacy;
/// Render a commented starter config template.
pub use file::{
    default_config_template, default_config_template_for, render_effective_config, DumpConfigFormat,
};
pub use legacy::convert_legacy_config_files;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// How to normalise command/keyword casing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum CaseStyle {
    /// Force lowercase output.
    Lower,
    /// Force uppercase output.
    #[default]
    Upper,
    /// Preserve the original source casing.
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
/// This struct is used at runtime. It is populated from defaults, supported
/// user config files (YAML or TOML), and CLI flag overrides (highest wins).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    // ── Layout ──────────────────────────────────────────────────────────
    /// Maximum rendered line width before wrapping is attempted.
    pub line_width: usize,
    /// Number of spaces that make up one indentation level when
    /// [`Self::use_tabchars`] is `false`.
    pub tab_size: usize,
    /// Emit tab characters for indentation instead of spaces.
    pub use_tabchars: bool,
    /// Maximum number of consecutive empty lines to preserve.
    pub max_empty_lines: usize,
    /// Maximum number of wrapped lines tolerated before switching to a more
    /// vertical layout.
    pub max_lines_hwrap: usize,
    /// Maximum number of positional arguments to keep in a hanging-wrap layout
    /// before going vertical.
    pub max_pargs_hwrap: usize,
    /// Maximum number of keyword/flag subgroups to keep in a horizontal wrap.
    pub max_subgroups_hwrap: usize,

    // ── Parenthesis style ───────────────────────────────────────────────
    /// Place the closing `)` on its own line when a call wraps.
    pub dangle_parens: bool,
    /// Alignment strategy for a dangling closing `)`.
    pub dangle_align: DangleAlign,
    /// Lower bound used by layout heuristics when deciding whether a command
    /// name is short enough to prefer one style over another.
    pub min_prefix_chars: usize,
    /// Upper bound used by layout heuristics when deciding whether a command
    /// name is long enough to prefer one style over another.
    pub max_prefix_chars: usize,
    /// Insert a space before `(` for control-flow commands such as `if`.
    pub separate_ctrl_name_with_space: bool,
    /// Insert a space before `(` for `function`/`macro` definitions.
    pub separate_fn_name_with_space: bool,

    // ── Casing ──────────────────────────────────────────────────────────
    /// Output casing policy for command names.
    pub command_case: CaseStyle,
    /// Output casing policy for recognized keywords and flags.
    pub keyword_case: CaseStyle,

    // ── Comment markup ──────────────────────────────────────────────────
    /// Enable markup-aware comment handling.
    pub enable_markup: bool,
    /// Reflow plain line comments to fit within the configured width.
    pub reflow_comments: bool,
    /// Preserve the first comment block in a file literally.
    pub first_comment_is_literal: bool,
    /// Regex for comments that should never be reflowed.
    pub literal_comment_pattern: String,
    /// Preferred bullet character when normalizing list markup.
    pub bullet_char: String,
    /// Preferred enumeration punctuation when normalizing numbered list markup.
    pub enum_char: String,
    /// Regex describing fenced literal comment blocks.
    pub fence_pattern: String,
    /// Regex describing ruler-style comments.
    pub ruler_pattern: String,
    /// Minimum ruler length before a `#-----` style line is treated as a ruler.
    pub hashruler_min_length: usize,
    /// Normalize ruler comments when markup handling is enabled.
    pub canonicalize_hashrulers: bool,

    // ── Per-command overrides ────────────────────────────────────────────
    /// Per-command configuration overrides keyed by lowercase command name.
    pub per_command_overrides: HashMap<String, PerCommandConfig>,
}

/// Per-command overrides. All fields are optional — only specified fields
/// override the global config for that command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PerCommandConfig {
    /// Override the command casing rule for this command only.
    pub command_case: Option<CaseStyle>,
    /// Override the keyword casing rule for this command only.
    pub keyword_case: Option<CaseStyle>,
    /// Override the line width for this command only.
    pub line_width: Option<usize>,
    /// Override the indentation width for this command only.
    pub tab_size: Option<usize>,
    /// Override dangling paren placement for this command only.
    pub dangle_parens: Option<bool>,
    /// Override dangling paren alignment for this command only.
    pub dangle_align: Option<DangleAlign>,
    /// Override the hanging-wrap positional argument threshold for this
    /// command only.
    #[serde(rename = "max_hanging_wrap_positional_args")]
    pub max_pargs_hwrap: Option<usize>,
    /// Override the hanging-wrap subgroup threshold for this command only.
    #[serde(rename = "max_hanging_wrap_groups")]
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
            dangle_parens: false,
            dangle_align: DangleAlign::Prefix,
            min_prefix_chars: 4,
            max_prefix_chars: 10,
            separate_ctrl_name_with_space: false,
            separate_fn_name_with_space: false,
            command_case: CaseStyle::Lower,
            keyword_case: CaseStyle::Upper,
            enable_markup: true,
            reflow_comments: false,
            first_comment_is_literal: true,
            literal_comment_pattern: String::new(),
            bullet_char: "*".to_string(),
            enum_char: ".".to_string(),
            fence_pattern: r"^\s*[`~]{3}[^`\n]*$".to_string(),
            ruler_pattern: r"^[^\w\s]{3}.*[^\w\s]{3}$".to_string(),
            hashruler_min_length: 10,
            canonicalize_hashrulers: true,
            per_command_overrides: HashMap::new(),
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
        let per_cmd = self.per_command_overrides.get(&lower);

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
    /// The global configuration before per-command overrides are applied.
    pub global: &'a Config,
    per_cmd: Option<&'a PerCommandConfig>,
    /// Whether this command should render a space before `(`.
    pub space_before_paren: bool,
}

impl CommandConfig<'_> {
    /// Effective line width for the current command.
    pub fn line_width(&self) -> usize {
        self.per_cmd
            .and_then(|p| p.line_width)
            .unwrap_or(self.global.line_width)
    }

    /// Effective indentation width for the current command.
    pub fn tab_size(&self) -> usize {
        self.per_cmd
            .and_then(|p| p.tab_size)
            .unwrap_or(self.global.tab_size)
    }

    /// Effective dangling-paren setting for the current command.
    pub fn dangle_parens(&self) -> bool {
        self.per_cmd
            .and_then(|p| p.dangle_parens)
            .unwrap_or(self.global.dangle_parens)
    }

    /// Effective dangling-paren alignment for the current command.
    pub fn dangle_align(&self) -> DangleAlign {
        self.per_cmd
            .and_then(|p| p.dangle_align)
            .unwrap_or(self.global.dangle_align)
    }

    /// Effective command casing rule for the current command.
    pub fn command_case(&self) -> CaseStyle {
        self.per_cmd
            .and_then(|p| p.command_case)
            .unwrap_or(self.global.command_case)
    }

    /// Effective keyword casing rule for the current command.
    pub fn keyword_case(&self) -> CaseStyle {
        self.per_cmd
            .and_then(|p| p.keyword_case)
            .unwrap_or(self.global.keyword_case)
    }

    /// Effective hanging-wrap positional argument threshold for the current
    /// command.
    pub fn max_pargs_hwrap(&self) -> usize {
        self.per_cmd
            .and_then(|p| p.max_pargs_hwrap)
            .unwrap_or(self.global.max_pargs_hwrap)
    }

    /// Effective hanging-wrap subgroup threshold for the current command.
    pub fn max_subgroups_hwrap(&self) -> usize {
        self.per_cmd
            .and_then(|p| p.max_subgroups_hwrap)
            .unwrap_or(self.global.max_subgroups_hwrap)
    }

    /// Effective indentation unit for the current command.
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
