// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Runtime formatter configuration.
//!
//! [`Config`] is the fully resolved in-memory configuration used by the
//! formatter. It is built from defaults, user config files
//! (`.cmakefmt.yaml`, `.cmakefmt.yml`, or `.cmakefmt.toml`), and CLI
//! overrides.

#[cfg(all(not(target_arch = "wasm32"), feature = "cli"))]
#[doc(hidden)]
pub mod editorconfig;
pub mod file;
#[cfg(all(not(target_arch = "wasm32"), feature = "cli"))]
mod legacy;
/// Render a commented starter config template.
pub use file::default_config_template;
#[cfg(feature = "cli")]
pub use file::{
    default_config_template_for, generate_json_schema, render_effective_config, DumpConfigFormat,
};
#[cfg(all(not(target_arch = "wasm32"), feature = "cli"))]
pub use legacy::convert_legacy_config_files;

use std::collections::HashMap;

use regex::Regex;
use serde::{Deserialize, Serialize};

/// How to normalise command/keyword casing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum, schemars::JsonSchema))]
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

/// Output line-ending style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum, schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum LineEnding {
    /// Unix-style LF (`\n`). The default.
    #[default]
    Unix,
    /// Windows-style CRLF (`\r\n`).
    Windows,
    /// Auto-detect the line ending from the input source.
    Auto,
}

/// How to handle fractional tab indentation when [`Config::use_tabchars`] is
/// `true`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum, schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum FractionalTabPolicy {
    /// Leave fractional spaces as-is (utf-8 0x20). The default.
    #[default]
    UseSpace,
    /// Round fractional indentation up to the next full tab stop (utf-8 0x09).
    RoundUp,
}

/// How to align the dangling closing paren.
///
/// Only takes effect when [`Config::dangle_parens`] is `true`.
/// Controls where `)` is placed when a call wraps onto multiple lines:
///
/// ```cmake
/// # Prefix / Close — `)` at the command-name column (tracks block depth):
/// target_link_libraries(
///   mylib PUBLIC dep1
/// )
///
/// # Open — `)` at the opening-paren column:
/// target_link_libraries(
///   mylib PUBLIC dep1
///                      )
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "cli", derive(schemars::JsonSchema))]
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
/// Construct [`Config::default`] and set fields as needed before passing it to
/// [`format_source`](crate::format_source) or related functions.
///
/// ```
/// use cmakefmt::{Config, CaseStyle, DangleAlign};
///
/// let config = Config {
///     line_width: 100,
///     command_case: CaseStyle::Lower,
///     dangle_parens: true,
///     dangle_align: DangleAlign::Open,
///     ..Config::default()
/// };
/// ```
///
/// # Defaults
///
/// | Field | Default |
/// |-------|---------|
/// | `line_width` | `80` |
/// | `tab_size` | `2` |
/// | `use_tabchars` | `false` |
/// | `max_empty_lines` | `1` |
/// | `command_case` | [`CaseStyle::Lower`] |
/// | `keyword_case` | [`CaseStyle::Upper`] |
/// | `dangle_parens` | `false` |
/// | `dangle_align` | [`DangleAlign::Prefix`] |
/// | `enable_markup` | `true` |
/// | `reflow_comments` | `false` |
/// | `first_comment_is_literal` | `true` |
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    // ── Kill-switch ─────────────────────────────────────────────────────
    /// When `true`, skip all formatting and return the source unchanged.
    pub disable: bool,

    // ── Line endings ─────────────────────────────────────────────────────
    /// Output line-ending style.
    pub line_ending: LineEnding,

    // ── Layout ──────────────────────────────────────────────────────────
    /// Maximum rendered line width before wrapping is attempted.
    pub line_width: usize,
    /// Number of spaces that make up one indentation level when
    /// [`Self::use_tabchars`] is `false`.
    pub tab_size: usize,
    /// Emit tab characters for indentation instead of spaces.
    pub use_tabchars: bool,
    /// How to handle fractional indentation when [`Self::use_tabchars`] is
    /// `true`.
    pub fractional_tab_policy: FractionalTabPolicy,
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
    /// Maximum rows a hanging-wrap positional group may consume before the
    /// layout is rejected and nesting is forced.
    pub max_rows_cmdline: usize,
    /// Command names (lowercase) that must always use vertical layout,
    /// regardless of line width.
    pub always_wrap: Vec<String>,
    /// Return an error when any formatted output line exceeds
    /// [`Self::line_width`].
    pub require_valid_layout: bool,
    /// When wrapping, keep the first positional argument on the command
    /// line and align continuation to the open parenthesis. Can be
    /// overridden per-command via `per_command_overrides` or the spec's
    /// `layout.wrap_after_first_arg`.
    pub wrap_after_first_arg: bool,
    /// Sort arguments in keyword sections marked `sortable` in the
    /// command spec. Sorting is lexicographic and case-insensitive.
    pub enable_sort: bool,
    /// Heuristically infer sortability for keyword sections without
    /// an explicit `sortable` annotation. When enabled, a section is
    /// considered sortable if all its arguments are simple unquoted
    /// tokens (no variables, generator expressions, or quoted strings).
    pub autosort: bool,

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
    /// Regex pattern that marks an inline comment as explicitly trailing its
    /// preceding argument. Matching comments are rendered on the same line as
    /// the preceding token rather than on their own line.
    pub explicit_trailing_pattern: String,

    // ── Per-command overrides ────────────────────────────────────────────
    /// Per-command configuration overrides keyed by lowercase command name.
    pub per_command_overrides: HashMap<String, PerCommandConfig>,

    // ── Experimental ──────────────────────────────────────────────────
    /// Opt-in experimental formatting options. These are unstable and may
    /// change or be removed between releases. Enable all at once with
    /// `--preview` on the CLI.
    #[serde(default)]
    pub experimental: Experimental,
}

/// Experimental formatting options gated behind `--preview` or the
/// `[experimental]` config section. All options default to `false`.
///
/// These may be promoted to stable defaults in a future release, changed
/// incompatibly, or removed entirely.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "cli", derive(schemars::JsonSchema))]
#[serde(default)]
#[non_exhaustive]
pub struct Experimental {
    // No experimental options yet. Add new options here as they are
    // developed. Each option should default to false.
}

/// Per-command overrides. All fields are optional — only specified fields
/// override the global config for that command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "cli", derive(schemars::JsonSchema))]
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
    /// Keep the first positional argument on the command line when wrapping.
    pub wrap_after_first_arg: Option<bool>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            disable: false,
            line_ending: LineEnding::Unix,
            line_width: 80,
            tab_size: 2,
            use_tabchars: false,
            fractional_tab_policy: FractionalTabPolicy::UseSpace,
            max_empty_lines: 1,
            max_lines_hwrap: 2,
            max_pargs_hwrap: 6,
            max_subgroups_hwrap: 2,
            max_rows_cmdline: 2,
            always_wrap: Vec::new(),
            require_valid_layout: false,
            wrap_after_first_arg: false,
            enable_sort: false,
            autosort: false,
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
            explicit_trailing_pattern: "#<".to_string(),
            per_command_overrides: HashMap::new(),
            experimental: Experimental::default(),
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

    /// Validate that all regex patterns in the config are valid.
    ///
    /// Returns `Ok(())` if all patterns compile, or an error message
    /// identifying the first invalid pattern.
    pub fn validate_patterns(&self) -> Result<(), String> {
        let patterns = [
            ("literal_comment_pattern", &self.literal_comment_pattern),
            ("explicit_trailing_pattern", &self.explicit_trailing_pattern),
            ("fence_pattern", &self.fence_pattern),
            ("ruler_pattern", &self.ruler_pattern),
        ];
        for (name, pattern) in &patterns {
            if !pattern.is_empty() {
                if let Err(err) = Regex::new(pattern) {
                    return Err(format!("invalid regex in {name}: {err}"));
                }
            }
        }
        Ok(())
    }

    /// Compile all regex patterns into a cache for internal formatting use.
    ///
    /// Callers that build [`Config`] programmatically should use
    /// [`Config::validate_patterns`] to validate regexes up front.
    pub(crate) fn compiled_patterns(&self) -> Result<CompiledPatterns, String> {
        Ok(CompiledPatterns {
            literal_comment: compile_optional(
                "literal_comment_pattern",
                &self.literal_comment_pattern,
            )?,
            explicit_trailing: compile_optional(
                "explicit_trailing_pattern",
                &self.explicit_trailing_pattern,
            )?,
        })
    }
}

fn compile_optional(name: &str, pattern: &str) -> Result<Option<Regex>, String> {
    if pattern.is_empty() {
        Ok(None)
    } else {
        Regex::new(pattern)
            .map(Some)
            .map_err(|err| format!("invalid regex in {name}: {err}"))
    }
}

/// Pre-compiled regex patterns from [`Config`] used internally while formatting.
pub(crate) struct CompiledPatterns {
    /// Compiled `literal_comment_pattern`.
    pub(crate) literal_comment: Option<Regex>,
    /// Compiled `explicit_trailing_pattern`.
    pub(crate) explicit_trailing: Option<Regex>,
}

/// A resolved config for formatting a specific command, with per-command
/// overrides already applied.
#[derive(Debug)]
pub struct CommandConfig<'a> {
    /// The global configuration before per-command overrides are applied.
    global: &'a Config,
    per_cmd: Option<&'a PerCommandConfig>,
    /// Whether this command should render a space before `(`.
    space_before_paren: bool,
}

impl CommandConfig<'_> {
    /// Whether this command should render a space before `(`.
    pub fn space_before_paren(&self) -> bool {
        self.space_before_paren
    }

    pub(crate) fn global(&self) -> &Config {
        self.global
    }

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

    /// Effective `wrap_after_first_arg` for the current command.
    ///
    /// Resolution order: per-command user override > `spec_value` (from
    /// the command spec's layout overrides) > global config default.
    pub fn wrap_after_first_arg(&self, spec_value: Option<bool>) -> bool {
        self.per_cmd
            .and_then(|p| p.wrap_after_first_arg)
            .or(spec_value)
            .unwrap_or(self.global.wrap_after_first_arg)
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── Config::for_command ───────────────────────────────────────────────

    #[test]
    fn for_command_control_flow_sets_space_before_paren() {
        let config = Config {
            separate_ctrl_name_with_space: true,
            ..Config::default()
        };
        for cmd in ["if", "elseif", "foreach", "while", "return"] {
            let cc = config.for_command(cmd);
            assert!(
                cc.space_before_paren(),
                "{cmd} should have space_before_paren=true"
            );
        }
    }

    #[test]
    fn for_command_fn_definition_sets_space_before_paren() {
        let config = Config {
            separate_fn_name_with_space: true,
            ..Config::default()
        };
        for cmd in ["function", "endfunction", "macro", "endmacro"] {
            let cc = config.for_command(cmd);
            assert!(
                cc.space_before_paren(),
                "{cmd} should have space_before_paren=true"
            );
        }
    }

    #[test]
    fn for_command_regular_command_no_space_before_paren() {
        let config = Config {
            separate_ctrl_name_with_space: true,
            separate_fn_name_with_space: true,
            ..Config::default()
        };
        let cc = config.for_command("message");
        assert!(
            !cc.space_before_paren(),
            "message should not have space_before_paren"
        );
    }

    #[test]
    fn for_command_lookup_is_case_insensitive() {
        let mut overrides = HashMap::new();
        overrides.insert(
            "message".to_string(),
            PerCommandConfig {
                line_width: Some(120),
                ..Default::default()
            },
        );
        let config = Config {
            per_command_overrides: overrides,
            ..Config::default()
        };
        // uppercase lookup should still find the "message" override
        assert_eq!(config.for_command("MESSAGE").line_width(), 120);
    }

    // ── CommandConfig accessors ───────────────────────────────────────────

    #[test]
    fn command_config_returns_global_defaults_when_no_override() {
        let config = Config::default();
        let cc = config.for_command("set");
        assert_eq!(cc.line_width(), config.line_width);
        assert_eq!(cc.tab_size(), config.tab_size);
        assert_eq!(cc.dangle_parens(), config.dangle_parens);
        assert_eq!(cc.command_case(), config.command_case);
        assert_eq!(cc.keyword_case(), config.keyword_case);
        assert_eq!(cc.max_pargs_hwrap(), config.max_pargs_hwrap);
        assert_eq!(cc.max_subgroups_hwrap(), config.max_subgroups_hwrap);
    }

    #[test]
    fn command_config_per_command_overrides_take_effect() {
        let mut overrides = HashMap::new();
        overrides.insert(
            "set".to_string(),
            PerCommandConfig {
                line_width: Some(120),
                tab_size: Some(4),
                dangle_parens: Some(true),
                dangle_align: Some(DangleAlign::Open),
                command_case: Some(CaseStyle::Upper),
                keyword_case: Some(CaseStyle::Lower),
                max_pargs_hwrap: Some(10),
                max_subgroups_hwrap: Some(5),
                wrap_after_first_arg: None,
            },
        );
        let config = Config {
            per_command_overrides: overrides,
            ..Config::default()
        };
        let cc = config.for_command("set");
        assert_eq!(cc.line_width(), 120);
        assert_eq!(cc.tab_size(), 4);
        assert!(cc.dangle_parens());
        assert_eq!(cc.dangle_align(), DangleAlign::Open);
        assert_eq!(cc.command_case(), CaseStyle::Upper);
        assert_eq!(cc.keyword_case(), CaseStyle::Lower);
        assert_eq!(cc.max_pargs_hwrap(), 10);
        assert_eq!(cc.max_subgroups_hwrap(), 5);
    }

    #[test]
    fn indent_str_spaces() {
        let config = Config {
            tab_size: 4,
            use_tabchars: false,
            ..Config::default()
        };
        assert_eq!(config.indent_str(), "    ");
        assert_eq!(config.for_command("set").indent_str(), "    ");
    }

    #[test]
    fn indent_str_tab() {
        let config = Config {
            use_tabchars: true,
            ..Config::default()
        };
        assert_eq!(config.indent_str(), "\t");
        assert_eq!(config.for_command("set").indent_str(), "\t");
    }

    // ── Case helpers ─────────────────────────────────────────────────────

    #[test]
    fn apply_command_case_lower() {
        let config = Config {
            command_case: CaseStyle::Lower,
            ..Config::default()
        };
        assert_eq!(
            config.apply_command_case("TARGET_LINK_LIBRARIES"),
            "target_link_libraries"
        );
    }

    #[test]
    fn apply_command_case_upper() {
        let config = Config {
            command_case: CaseStyle::Upper,
            ..Config::default()
        };
        assert_eq!(
            config.apply_command_case("target_link_libraries"),
            "TARGET_LINK_LIBRARIES"
        );
    }

    #[test]
    fn apply_command_case_unchanged() {
        let config = Config {
            command_case: CaseStyle::Unchanged,
            ..Config::default()
        };
        assert_eq!(
            config.apply_command_case("Target_Link_Libraries"),
            "Target_Link_Libraries"
        );
    }

    #[test]
    fn apply_keyword_case_variants() {
        let config_upper = Config {
            keyword_case: CaseStyle::Upper,
            ..Config::default()
        };
        assert_eq!(config_upper.apply_keyword_case("public"), "PUBLIC");

        let config_lower = Config {
            keyword_case: CaseStyle::Lower,
            ..Config::default()
        };
        assert_eq!(config_lower.apply_keyword_case("PUBLIC"), "public");
    }

    // ── Error Display ─────────────────────────────────────────────────────

    #[test]
    fn error_layout_too_wide_display() {
        use crate::error::Error;
        let err = Error::LayoutTooWide {
            line_no: 5,
            width: 95,
            limit: 80,
        };
        let msg = err.to_string();
        assert!(msg.contains("5"), "should mention line number");
        assert!(msg.contains("95"), "should mention actual width");
        assert!(msg.contains("80"), "should mention limit");
    }

    #[test]
    fn error_formatter_display() {
        use crate::error::Error;
        let err = Error::Formatter("something went wrong".to_string());
        assert!(err.to_string().contains("something went wrong"));
    }
}
