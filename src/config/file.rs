// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Config-file loading and starter template generation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
#[cfg(feature = "cli")]
use serde::Serialize;

use crate::config::{
    CaseStyle, Config, ContinuationAlign, DangleAlign, Experimental, FractionalTabPolicy,
    LineEnding, PerCommandConfig,
};
use crate::error::{Error, FileParseError, Result};

/// The user-config file structure for `.cmakefmt.yaml`, `.cmakefmt.yml`, and
/// `.cmakefmt.toml`.
///
/// All fields are optional — only specified values override the defaults.
#[derive(Debug, Clone, Deserialize, Default)]
#[cfg_attr(feature = "cli", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "cli", schemars(title = "cmakefmt configuration"))]
#[serde(default, deny_unknown_fields)]
struct FileConfig {
    /// JSON Schema reference (ignored at runtime, used by editors for
    /// autocomplete and validation).
    #[serde(rename = "$schema")]
    #[cfg_attr(feature = "cli", schemars(skip))]
    _schema: Option<String>,
    /// Command spec overrides (parsed separately by the spec registry).
    #[cfg_attr(feature = "cli", schemars(skip))]
    commands: Option<serde_yaml::Value>,
    /// Formatting options controlling line width, indentation, casing, and layout.
    format: FormatSection,
    /// Comment markup processing options.
    markup: MarkupSection,
    /// Per-command configuration overrides keyed by lowercase command name.
    #[serde(rename = "per_command_overrides")]
    per_command_overrides: HashMap<String, PerCommandConfig>,
    #[serde(rename = "per_command")]
    #[cfg_attr(feature = "cli", schemars(skip))]
    legacy_per_command: HashMap<String, PerCommandConfig>,
    /// Experimental formatting options (unstable, may change between releases).
    #[serde(default)]
    experimental: Experimental,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[cfg_attr(feature = "cli", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(deny_unknown_fields)]
struct FormatSection {
    /// Disable formatting entirely and return the source unchanged.
    disable: Option<bool>,
    /// Output line-ending style: `unix` (LF), `windows` (CRLF), or `auto` (detect from input).
    line_ending: Option<LineEnding>,
    /// Maximum rendered line width before cmakefmt wraps a call. Default: `80`.
    line_width: Option<usize>,
    /// Number of spaces per indentation level when `use_tabs` is `false`. Default: `2`.
    tab_size: Option<usize>,
    /// Indent with tab characters instead of spaces.
    use_tabs: Option<bool>,
    /// How to handle fractional indentation when `use_tabs` is `true`: `use-space` or `round-up`.
    fractional_tab_policy: Option<FractionalTabPolicy>,
    /// Maximum number of consecutive blank lines to preserve. Default: `1`.
    max_empty_lines: Option<usize>,
    /// Maximum wrapped lines to tolerate before switching to a more vertical layout. Default: `2`.
    max_hanging_wrap_lines: Option<usize>,
    /// Maximum positional arguments to keep in a hanging-wrap layout. Default: `6`.
    max_hanging_wrap_positional_args: Option<usize>,
    /// Maximum keyword/flag subgroups to keep in a hanging-wrap layout. Default: `2`.
    max_hanging_wrap_groups: Option<usize>,
    /// Maximum rows a hanging-wrap positional group may consume before nesting is forced. Default: `2`.
    max_rows_cmdline: Option<usize>,
    /// Command names (lowercase) that must always use vertical layout regardless of line width.
    always_wrap: Option<Vec<String>>,
    /// Return an error if any formatted output line exceeds `line_width`.
    require_valid_layout: Option<bool>,
    /// Keep the first positional argument on the command line when wrapping.
    wrap_after_first_arg: Option<bool>,
    /// How to indent continuation lines: `same-indent` or `under-first-value`.
    continuation_align: Option<ContinuationAlign>,
    /// Sort arguments in keyword sections marked `sortable` in the command spec.
    enable_sort: Option<bool>,
    /// Heuristically infer sortability for keyword sections without explicit annotation.
    autosort: Option<bool>,
    /// Place the closing `)` on its own line when a call wraps.
    dangle_parens: Option<bool>,
    /// Alignment strategy for a dangling `)`: `prefix`, `open`, or `close`.
    dangle_align: Option<DangleAlign>,
    /// Lower heuristic bound used when deciding between compact and wrapped layouts. Default: `4`.
    min_prefix_length: Option<usize>,
    /// Upper heuristic bound used when deciding between compact and wrapped layouts. Default: `10`.
    max_prefix_length: Option<usize>,
    /// Insert a space before `(` for control-flow commands such as `if`, `foreach`, `while`.
    space_before_control_paren: Option<bool>,
    /// Insert a space before `(` for `function()` and `macro()` definitions.
    space_before_definition_paren: Option<bool>,
    /// Output casing for command names: `lower`, `upper`, or `unchanged`.
    command_case: Option<CaseStyle>,
    /// Output casing for recognized keywords and flags: `lower`, `upper`, or `unchanged`.
    keyword_case: Option<CaseStyle>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[cfg_attr(feature = "cli", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(deny_unknown_fields)]
struct MarkupSection {
    /// Enable markup-aware comment handling.
    enable_markup: Option<bool>,
    /// Preserve the first comment block in a file literally.
    first_comment_is_literal: Option<bool>,
    /// Regex for comments that should never be reflowed.
    literal_comment_pattern: Option<String>,
    /// Preferred bullet character when normalizing markup lists. Default: `*`.
    bullet_char: Option<String>,
    /// Preferred punctuation for numbered lists when normalizing markup. Default: `.`.
    enum_char: Option<String>,
    /// Regex describing fenced literal comment blocks.
    fence_pattern: Option<String>,
    /// Regex describing ruler-style comments that should be treated specially.
    ruler_pattern: Option<String>,
    /// Minimum ruler length before a hash-only line is treated as a ruler. Default: `10`.
    hashruler_min_length: Option<usize>,
    /// Normalize ruler comments when markup handling is enabled.
    canonicalize_hashrulers: Option<bool>,
    /// Regex pattern for inline comments explicitly trailing their preceding argument. Default: `#<`.
    explicit_trailing_pattern: Option<String>,
}

const CONFIG_FILE_NAME_TOML: &str = ".cmakefmt.toml";
const CONFIG_FILE_NAME_YAML: &str = ".cmakefmt.yaml";
const CONFIG_FILE_NAME_YML: &str = ".cmakefmt.yml";
const CONFIG_FILE_NAMES: &[&str] = &[
    CONFIG_FILE_NAME_YAML,
    CONFIG_FILE_NAME_YML,
    CONFIG_FILE_NAME_TOML,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConfigFileFormat {
    Toml,
    Yaml,
}

impl ConfigFileFormat {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Toml => "TOML",
            Self::Yaml => "YAML",
        }
    }
}

/// Supported `cmakefmt config dump` output formats.
#[cfg(feature = "cli")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum DumpConfigFormat {
    /// Emit YAML.
    Yaml,
    /// Emit TOML.
    Toml,
}

#[cfg(feature = "cli")]
/// Render a commented starter config in the requested format.
///
/// The template is intentionally verbose: every option is introduced by a
/// short explanatory comment so users can understand the surface without
/// needing to cross-reference the docs immediately.
pub fn default_config_template_for(format: DumpConfigFormat) -> String {
    match format {
        DumpConfigFormat::Yaml => default_config_template_yaml(),
        DumpConfigFormat::Toml => default_config_template_toml(),
    }
}

#[cfg(feature = "cli")]
/// Render a resolved runtime config back into the user-facing config schema.
///
/// This is primarily used by CLI introspection commands such as
/// `cmakefmt config show`.
pub fn render_effective_config(config: &Config, format: DumpConfigFormat) -> Result<String> {
    let view = EffectiveConfigFile::from(config);
    match format {
        DumpConfigFormat::Yaml => serde_yaml::to_string(&view).map_err(|err| {
            Error::Formatter(format!("failed to render effective config as YAML: {err}"))
        }),
        DumpConfigFormat::Toml => toml::to_string_pretty(&view).map_err(|err| {
            Error::Formatter(format!("failed to render effective config as TOML: {err}"))
        }),
    }
}

/// Render a commented starter config using the default user-facing dump format.
pub fn default_config_template() -> String {
    default_config_template_yaml()
}

#[cfg(feature = "cli")]
/// Generate a JSON Schema for the cmakefmt config file format.
///
/// The output is a pretty-printed JSON string suitable for publishing to
/// `cmakefmt.dev/schemas/v{version}/schema.json`.
pub fn generate_json_schema() -> String {
    let schema = schemars::schema_for!(FileConfig);
    serde_json::to_string_pretty(&schema).expect("JSON schema serialization failed")
}

#[cfg(feature = "cli")]
fn default_config_template_toml() -> String {
    format!(
        concat!(
            "# Default cmakefmt configuration.\n",
            "# Copy this to .cmakefmt.toml and uncomment the optional settings\n",
            "# you want to customize.\n\n",
            "[format]\n",
            "# Disable formatting entirely (return source unchanged).\n",
            "# disable = true\n\n",
            "# Output line-ending style: unix (LF), windows (CRLF), or auto (detect from input).\n",
            "# line_ending = \"windows\"\n\n",
            "# Maximum rendered line width before cmakefmt wraps a call.\n",
            "line_width = {line_width}\n\n",
            "# Number of spaces per indentation level when use_tabs is false.\n",
            "tab_size = {tab_size}\n\n",
            "# Indent with tab characters instead of spaces.\n",
            "# use_tabs = true\n\n",
            "# How to handle fractional indentation when use_tabs is true: use-space or round-up.\n",
            "# fractional_tab_policy = \"round-up\"\n\n",
            "# Maximum number of consecutive blank lines to preserve.\n",
            "max_empty_lines = {max_empty_lines}\n\n",
            "# Maximum wrapped lines to tolerate before switching to a more vertical layout.\n",
            "max_hanging_wrap_lines = {max_lines_hwrap}\n\n",
            "# Maximum positional arguments to keep in a hanging-wrap layout.\n",
            "max_hanging_wrap_positional_args = {max_pargs_hwrap}\n\n",
            "# Maximum keyword/flag subgroups to keep in a hanging-wrap layout.\n",
            "max_hanging_wrap_groups = {max_subgroups_hwrap}\n\n",
            "# Maximum rows a hanging-wrap positional group may consume before nesting is forced.\n",
            "max_rows_cmdline = {max_rows_cmdline}\n\n",
            "# Commands that must always use vertical (wrapped) layout.\n",
            "# always_wrap = [\"target_link_libraries\"]\n\n",
            "# Return an error if any formatted line exceeds line_width.\n",
            "# require_valid_layout = true\n\n",
            "# Keep the first positional argument on the command line when wrapping.\n",
            "# wrap_after_first_arg = true\n\n",
            "# Continuation-line alignment when a wrapped keyword section overflows\n",
            "# line_width: same-indent (wrap at the keyword's indent) or under-first-value\n",
            "# (cmake-format hanging-indent style, aligned under the first value).\n",
            "# continuation_align = \"under-first-value\"\n\n",
            "# Sort arguments in keyword sections marked sortable in the command spec.\n",
            "# enable_sort = true\n\n",
            "# Heuristically sort keyword sections where all arguments are simple unquoted tokens.\n",
            "# autosort = true\n\n",
            "# Put the closing ')' on its own line when a call wraps.\n",
            "dangle_parens = {dangle_parens}\n\n",
            "# Alignment strategy for a dangling ')': prefix, open, or close.\n",
            "dangle_align = \"{dangle_align}\"\n\n",
            "# Lower heuristic bound used when deciding between compact and wrapped layouts.\n",
            "min_prefix_length = {min_prefix_chars}\n\n",
            "# Upper heuristic bound used when deciding between compact and wrapped layouts.\n",
            "max_prefix_length = {max_prefix_chars}\n\n",
            "# Insert a space before '(' for control-flow commands like if/foreach.\n",
            "# space_before_control_paren = true\n\n",
            "# Insert a space before '(' for function() and macro() definitions.\n",
            "# space_before_definition_paren = true\n\n",
            "# Output casing for command names: lower, upper, or unchanged.\n",
            "command_case = \"{command_case}\"\n\n",
            "# Output casing for recognized keywords and flags: lower, upper, or unchanged.\n",
            "keyword_case = \"{keyword_case}\"\n\n",
            "[markup]\n",
            "# Enable markup-aware comment handling.\n",
            "enable_markup = {enable_markup}\n\n",
            "# Preserve the first comment block in a file literally.\n",
            "first_comment_is_literal = {first_comment_is_literal}\n\n",
            "# Preserve comments matching a custom regex literally.\n",
            "# literal_comment_pattern = \"^\\\\s*NOTE:\"\n\n",
            "# Preferred bullet character when normalizing markup lists.\n",
            "bullet_char = \"{bullet_char}\"\n\n",
            "# Preferred punctuation for numbered lists when normalizing markup.\n",
            "enum_char = \"{enum_char}\"\n\n",
            "# Regex describing fenced literal comment blocks.\n",
            "fence_pattern = '{fence_pattern}'\n\n",
            "# Regex describing ruler-style comments that should be treated specially.\n",
            "ruler_pattern = '{ruler_pattern}'\n\n",
            "# Minimum ruler length before a hash-only line is treated as a ruler.\n",
            "hashruler_min_length = {hashruler_min_length}\n\n",
            "# Normalize ruler comments when markup handling is enabled.\n",
            "canonicalize_hashrulers = {canonicalize_hashrulers}\n\n",
            "# Regex pattern for inline comments that are explicitly trailing their preceding\n",
            "# argument (rendered on the same line). Default: '#<'.\n",
            "# explicit_trailing_pattern = \"#<\"\n\n",
            "# Uncomment and edit a block like this to override formatting knobs\n",
            "# for a specific command. This changes layout behavior for that\n",
            "# command name only; it does not define new command syntax.\n",
            "#\n",
            "# [per_command_overrides.my_add_test]\n",
            "# Override the line width just for this command.\n",
            "# line_width = 120\n\n",
            "# Override command casing just for this command.\n",
            "# command_case = \"unchanged\"\n\n",
            "# Override keyword casing just for this command.\n",
            "# keyword_case = \"upper\"\n\n",
            "# Override indentation width just for this command.\n",
            "# tab_size = 4\n\n",
            "# Override dangling-paren placement just for this command.\n",
            "# dangle_parens = false\n\n",
            "# Override dangling-paren alignment just for this command.\n",
            "# dangle_align = \"prefix\"\n\n",
            "# Override the positional-argument hanging-wrap threshold just for this command.\n",
            "# max_hanging_wrap_positional_args = 8\n\n",
            "# Override the subgroup hanging-wrap threshold just for this command.\n",
            "# max_hanging_wrap_groups = 3\n\n",
            "# TOML custom-command specs live under [commands.<name>]. For\n",
            "# user config, prefer YAML once these specs grow beyond a couple\n",
            "# of simple kwargs.\n",
            "# Command specs tell the formatter which tokens are positional\n",
            "# arguments, standalone flags, and keyword sections.\n",
            "#\n",
            "# Example: a custom test command with a flag and four keyword sections.\n",
            "# Uncomment this block to teach cmakefmt the argument structure.\n",
            "#\n",
            "# [commands.my_add_test]\n",
            "# pargs = 0\n",
            "# flags = [\"VERBOSE\"]\n",
            "# kwargs = {{ NAME = {{ nargs = 1 }}, SOURCES = {{ nargs = \"+\" }}, LIBRARIES = {{ nargs = \"+\" }}, TIMEOUT = {{ nargs = 1 }} }}\n",
        ),
        line_width = Config::default().line_width,
        tab_size = Config::default().tab_size,
        max_empty_lines = Config::default().max_empty_lines,
        max_lines_hwrap = Config::default().max_lines_hwrap,
        max_pargs_hwrap = Config::default().max_pargs_hwrap,
        max_subgroups_hwrap = Config::default().max_subgroups_hwrap,
        max_rows_cmdline = Config::default().max_rows_cmdline,
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

#[cfg(feature = "cli")]
#[derive(Debug, Clone, Serialize)]
struct EffectiveConfigFile {
    format: EffectiveFormatSection,
    markup: EffectiveMarkupSection,
    per_command_overrides: HashMap<String, PerCommandConfig>,
    experimental: Experimental,
}

#[derive(Debug, Clone, Serialize)]
#[cfg(feature = "cli")]
struct EffectiveFormatSection {
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    disable: bool,
    line_ending: LineEnding,
    line_width: usize,
    tab_size: usize,
    use_tabs: bool,
    fractional_tab_policy: FractionalTabPolicy,
    max_empty_lines: usize,
    max_hanging_wrap_lines: usize,
    max_hanging_wrap_positional_args: usize,
    max_hanging_wrap_groups: usize,
    max_rows_cmdline: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    always_wrap: Vec<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    require_valid_layout: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    wrap_after_first_arg: bool,
    continuation_align: ContinuationAlign,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    enable_sort: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    autosort: bool,
    dangle_parens: bool,
    dangle_align: DangleAlign,
    min_prefix_length: usize,
    max_prefix_length: usize,
    space_before_control_paren: bool,
    space_before_definition_paren: bool,
    command_case: CaseStyle,
    keyword_case: CaseStyle,
}

#[derive(Debug, Clone, Serialize)]
#[cfg(feature = "cli")]
struct EffectiveMarkupSection {
    enable_markup: bool,
    first_comment_is_literal: bool,
    literal_comment_pattern: String,
    bullet_char: String,
    enum_char: String,
    fence_pattern: String,
    ruler_pattern: String,
    hashruler_min_length: usize,
    canonicalize_hashrulers: bool,
    explicit_trailing_pattern: String,
}

#[cfg(feature = "cli")]
impl From<&Config> for EffectiveConfigFile {
    fn from(config: &Config) -> Self {
        Self {
            format: EffectiveFormatSection {
                disable: config.disable,
                line_ending: config.line_ending,
                line_width: config.line_width,
                tab_size: config.tab_size,
                use_tabs: config.use_tabchars,
                fractional_tab_policy: config.fractional_tab_policy,
                max_empty_lines: config.max_empty_lines,
                max_hanging_wrap_lines: config.max_lines_hwrap,
                max_hanging_wrap_positional_args: config.max_pargs_hwrap,
                max_hanging_wrap_groups: config.max_subgroups_hwrap,
                max_rows_cmdline: config.max_rows_cmdline,
                always_wrap: config.always_wrap.clone(),
                require_valid_layout: config.require_valid_layout,
                wrap_after_first_arg: config.wrap_after_first_arg,
                continuation_align: config.continuation_align,
                enable_sort: config.enable_sort,
                autosort: config.autosort,
                dangle_parens: config.dangle_parens,
                dangle_align: config.dangle_align,
                min_prefix_length: config.min_prefix_chars,
                max_prefix_length: config.max_prefix_chars,
                space_before_control_paren: config.separate_ctrl_name_with_space,
                space_before_definition_paren: config.separate_fn_name_with_space,
                command_case: config.command_case,
                keyword_case: config.keyword_case,
            },
            markup: EffectiveMarkupSection {
                enable_markup: config.enable_markup,
                first_comment_is_literal: config.first_comment_is_literal,
                literal_comment_pattern: config.literal_comment_pattern.clone(),
                bullet_char: config.bullet_char.clone(),
                enum_char: config.enum_char.clone(),
                fence_pattern: config.fence_pattern.clone(),
                ruler_pattern: config.ruler_pattern.clone(),
                hashruler_min_length: config.hashruler_min_length,
                canonicalize_hashrulers: config.canonicalize_hashrulers,
                explicit_trailing_pattern: config.explicit_trailing_pattern.clone(),
            },
            per_command_overrides: config.per_command_overrides.clone(),
            experimental: config.experimental.clone(),
        }
    }
}

fn default_config_template_yaml() -> String {
    format!(
        concat!(
            "# yaml-language-server: $schema=https://cmakefmt.dev/schemas/latest/schema.json\n",
            "# Default cmakefmt configuration.\n",
            "# Copy this to .cmakefmt.yaml and uncomment the optional settings\n",
            "# you want to customize.\n\n",
            "format:\n",
            "  # Disable formatting entirely (return source unchanged).\n",
            "  # disable: true\n\n",
            "  # Output line-ending style: unix (LF), windows (CRLF), or auto (detect from input).\n",
            "  # line_ending: windows\n\n",
            "  # Maximum rendered line width before cmakefmt wraps a call.\n",
            "  line_width: {line_width}\n\n",
            "  # Number of spaces per indentation level when use_tabs is false.\n",
            "  tab_size: {tab_size}\n\n",
            "  # Indent with tab characters instead of spaces.\n",
            "  # use_tabs: true\n\n",
            "  # How to handle fractional indentation when use_tabs is true: use-space or round-up.\n",
            "  # fractional_tab_policy: round-up\n\n",
            "  # Maximum number of consecutive blank lines to preserve.\n",
            "  max_empty_lines: {max_empty_lines}\n\n",
            "  # Maximum wrapped lines to tolerate before switching to a more vertical layout.\n",
            "  max_hanging_wrap_lines: {max_lines_hwrap}\n\n",
            "  # Maximum positional arguments to keep in a hanging-wrap layout.\n",
            "  max_hanging_wrap_positional_args: {max_pargs_hwrap}\n\n",
            "  # Maximum keyword/flag subgroups to keep in a hanging-wrap layout.\n",
            "  max_hanging_wrap_groups: {max_subgroups_hwrap}\n\n",
            "  # Maximum rows a hanging-wrap positional group may consume before nesting is forced.\n",
            "  max_rows_cmdline: {max_rows_cmdline}\n\n",
            "  # Commands that must always use vertical (wrapped) layout.\n",
            "  # always_wrap:\n",
            "  #   - target_link_libraries\n\n",
            "  # Return an error if any formatted line exceeds line_width.\n",
            "  # require_valid_layout: true\n\n",
            "  # Keep the first positional argument on the command line when wrapping.\n",
            "  # wrap_after_first_arg: true\n\n",
            "  # Continuation-line alignment when a wrapped keyword section overflows\n",
            "  # line_width: same-indent (wrap at the keyword's indent) or under-first-value\n",
            "  # (cmake-format hanging-indent style, aligned under the first value).\n",
            "  # continuation_align: under-first-value\n\n",
            "  # Sort arguments in keyword sections marked sortable in the command spec.\n",
            "  # enable_sort: true\n\n",
            "  # Heuristically sort keyword sections where all arguments are simple unquoted tokens.\n",
            "  # autosort: true\n\n",
            "  # Put the closing ')' on its own line when a call wraps.\n",
            "  dangle_parens: {dangle_parens}\n\n",
            "  # Alignment strategy for a dangling ')': prefix, open, or close.\n",
            "  dangle_align: {dangle_align}\n\n",
            "  # Lower heuristic bound used when deciding between compact and wrapped layouts.\n",
            "  min_prefix_length: {min_prefix_chars}\n\n",
            "  # Upper heuristic bound used when deciding between compact and wrapped layouts.\n",
            "  max_prefix_length: {max_prefix_chars}\n\n",
            "  # Insert a space before '(' for control-flow commands like if/foreach.\n",
            "  # space_before_control_paren: true\n\n",
            "  # Insert a space before '(' for function() and macro() definitions.\n",
            "  # space_before_definition_paren: true\n\n",
            "  # Output casing for command names: lower, upper, or unchanged.\n",
            "  command_case: {command_case}\n\n",
            "  # Output casing for recognized keywords and flags: lower, upper, or unchanged.\n",
            "  keyword_case: {keyword_case}\n\n",
            "markup:\n",
            "  # Enable markup-aware comment handling.\n",
            "  enable_markup: {enable_markup}\n\n",
            "  # Preserve the first comment block in a file literally.\n",
            "  first_comment_is_literal: {first_comment_is_literal}\n\n",
            "  # Preserve comments matching a custom regex literally.\n",
            "  # literal_comment_pattern: '^\\s*NOTE:'\n\n",
            "  # Preferred bullet character when normalizing markup lists.\n",
            "  bullet_char: '{bullet_char}'\n\n",
            "  # Preferred punctuation for numbered lists when normalizing markup.\n",
            "  enum_char: '{enum_char}'\n\n",
            "  # Regex describing fenced literal comment blocks.\n",
            "  fence_pattern: '{fence_pattern}'\n\n",
            "  # Regex describing ruler-style comments that should be treated specially.\n",
            "  ruler_pattern: '{ruler_pattern}'\n\n",
            "  # Minimum ruler length before a hash-only line is treated as a ruler.\n",
            "  hashruler_min_length: {hashruler_min_length}\n\n",
            "  # Normalize ruler comments when markup handling is enabled.\n",
            "  canonicalize_hashrulers: {canonicalize_hashrulers}\n\n",
            "  # Regex pattern for inline comments that are explicitly trailing their preceding\n",
            "  # argument (rendered on the same line). Default: '#<'.\n",
            "  # explicit_trailing_pattern: '#<'\n\n",
            "# Uncomment and edit a block like this to override formatting knobs\n",
            "# for a specific command. This changes layout behavior for that\n",
            "# command name only; it does not define new command syntax.\n",
            "#\n",
            "# per_command_overrides:\n",
            "#   my_add_test:\n",
            "#     # Override the line width just for this command.\n",
            "#     line_width: 120\n",
            "#\n",
            "#     # Override command casing just for this command.\n",
            "#     command_case: unchanged\n",
            "#\n",
            "#     # Override keyword casing just for this command.\n",
            "#     keyword_case: upper\n",
            "#\n",
            "#     # Override indentation width just for this command.\n",
            "#     tab_size: 4\n",
            "#\n",
            "#     # Override dangling-paren placement just for this command.\n",
            "#     dangle_parens: false\n",
            "#\n",
            "#     # Override dangling-paren alignment just for this command.\n",
            "#     dangle_align: prefix\n",
            "#\n",
            "#     # Override the positional-argument hanging-wrap threshold just for this command.\n",
            "#     max_hanging_wrap_positional_args: 8\n",
            "#\n",
            "#     # Override the subgroup hanging-wrap threshold just for this command.\n",
            "#     max_hanging_wrap_groups: 3\n\n",
            "# YAML custom-command specs live under commands:<name>. Command\n",
            "# specs tell the formatter which tokens are positional arguments,\n",
            "# standalone flags, and keyword sections.\n",
            "#\n",
            "# Example: a custom test command with a flag and four keyword sections.\n",
            "# Uncomment this block to teach cmakefmt the argument structure.\n",
            "#\n",
            "# commands:\n",
            "#   my_add_test:\n",
            "#     pargs: 0\n",
            "#     flags:\n",
            "#       - VERBOSE\n",
            "#     kwargs:\n",
            "#       NAME:\n",
            "#         nargs: 1\n",
            "#       SOURCES:\n",
            "#         nargs: \"+\"\n",
            "#       LIBRARIES:\n",
            "#         nargs: \"+\"\n",
            "#       TIMEOUT:\n",
            "#         nargs: 1\n",
        ),
        line_width = Config::default().line_width,
        tab_size = Config::default().tab_size,
        max_empty_lines = Config::default().max_empty_lines,
        max_lines_hwrap = Config::default().max_lines_hwrap,
        max_pargs_hwrap = Config::default().max_pargs_hwrap,
        max_subgroups_hwrap = Config::default().max_subgroups_hwrap,
        max_rows_cmdline = Config::default().max_rows_cmdline,
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
    /// Searches for the nearest supported user config (`.cmakefmt.yaml`,
    /// `.cmakefmt.yml`, then `.cmakefmt.toml`) starting from the file's
    /// directory and walking up to the repository/filesystem root. If none is
    /// found, falls back to the same filenames in the home directory.
    pub fn for_file(file_path: &Path) -> Result<Self> {
        let config_paths = find_config_files(file_path);
        Self::from_files(&config_paths)
    }

    /// Load configuration from a specific supported config file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let paths = [path.to_path_buf()];
        Self::from_files(&paths)
    }

    /// Load configuration by merging several supported config files in order.
    ///
    /// Later files override earlier files.
    pub fn from_files(paths: &[PathBuf]) -> Result<Self> {
        let mut config = Config::default();
        // Default-only path: default pattern strings are known-valid, skip the
        // regex compilation that validate_patterns() performs. This matters
        // on whole-tree runs where from_files is called per file.
        if paths.is_empty() {
            return Ok(config);
        }
        for path in paths {
            let file_config = load_config_file(path)?;
            config.apply(file_config);
        }
        config.validate_patterns().map_err(Error::Formatter)?;
        Ok(config)
    }

    /// Parse a YAML config string through the same `FileConfig` schema used by
    /// config files and return the resolved runtime [`Config`].
    ///
    /// This validates sections (`format:` and `markup:`) and rejects unknown
    /// fields, matching the behavior of file-based config loading.
    pub fn from_yaml_str(yaml: &str) -> Result<Self> {
        Ok(parse_yaml_config(yaml)?.config)
    }

    /// Parse a YAML config string and also return a serialized `commands:`
    /// override block for registry merging when present.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn from_yaml_str_with_commands(yaml: &str) -> Result<(Self, Option<Box<str>>)> {
        let parsed = parse_yaml_config(yaml)?;
        Ok((parsed.config, parsed.commands_yaml))
    }

    /// Return the config files that would be applied for the given file.
    ///
    /// When config discovery is used, this is either the nearest
    /// supported config file found by walking upward from the file, or a home
    /// directory config if no nearer config exists.
    pub fn config_sources_for(file_path: &Path) -> Vec<PathBuf> {
        find_config_files(file_path)
    }

    fn apply(&mut self, fc: FileConfig) {
        // Format section
        if let Some(v) = fc.format.disable {
            self.disable = v;
        }
        if let Some(v) = fc.format.line_ending {
            self.line_ending = v;
        }
        if let Some(v) = fc.format.line_width {
            self.line_width = v;
        }
        if let Some(v) = fc.format.tab_size {
            self.tab_size = v;
        }
        if let Some(v) = fc.format.use_tabs {
            self.use_tabchars = v;
        }
        if let Some(v) = fc.format.fractional_tab_policy {
            self.fractional_tab_policy = v;
        }
        if let Some(v) = fc.format.max_empty_lines {
            self.max_empty_lines = v;
        }
        if let Some(v) = fc.format.max_hanging_wrap_lines {
            self.max_lines_hwrap = v;
        }
        if let Some(v) = fc.format.max_hanging_wrap_positional_args {
            self.max_pargs_hwrap = v;
        }
        if let Some(v) = fc.format.max_hanging_wrap_groups {
            self.max_subgroups_hwrap = v;
        }
        if let Some(v) = fc.format.max_rows_cmdline {
            self.max_rows_cmdline = v;
        }
        if let Some(v) = fc.format.always_wrap {
            self.always_wrap = v.into_iter().map(|s| s.to_ascii_lowercase()).collect();
        }
        if let Some(v) = fc.format.require_valid_layout {
            self.require_valid_layout = v;
        }
        if let Some(v) = fc.format.wrap_after_first_arg {
            self.wrap_after_first_arg = v;
        }
        if let Some(v) = fc.format.continuation_align {
            self.continuation_align = v;
        }
        if let Some(v) = fc.format.enable_sort {
            self.enable_sort = v;
        }
        if let Some(v) = fc.format.autosort {
            self.autosort = v;
        }
        if let Some(v) = fc.format.dangle_parens {
            self.dangle_parens = v;
        }
        if let Some(v) = fc.format.dangle_align {
            self.dangle_align = v;
        }
        if let Some(v) = fc.format.min_prefix_length {
            self.min_prefix_chars = v;
        }
        if let Some(v) = fc.format.max_prefix_length {
            self.max_prefix_chars = v;
        }
        if let Some(v) = fc.format.space_before_control_paren {
            self.separate_ctrl_name_with_space = v;
        }
        if let Some(v) = fc.format.space_before_definition_paren {
            self.separate_fn_name_with_space = v;
        }
        if let Some(v) = fc.format.command_case {
            self.command_case = v;
        }
        if let Some(v) = fc.format.keyword_case {
            self.keyword_case = v;
        }

        // Markup section
        if let Some(v) = fc.markup.enable_markup {
            self.enable_markup = v;
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
        if let Some(v) = fc.markup.explicit_trailing_pattern {
            self.explicit_trailing_pattern = v;
        }

        // Per-command overrides (merge, don't replace)
        for (name, overrides) in fc.per_command_overrides {
            self.per_command_overrides.insert(name, overrides);
        }

        self.experimental = fc.experimental;
    }
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Debug, Clone)]
pub(crate) struct ParsedYamlConfig {
    pub(crate) config: Config,
    pub(crate) commands_yaml: Option<Box<str>>,
}

fn parse_yaml_config(yaml: &str) -> Result<ParsedYamlConfig> {
    let file_config: FileConfig = serde_yaml::from_str(yaml).map_err(|source| {
        Error::Config(crate::error::ConfigError {
            path: std::path::PathBuf::from("<yaml-string>"),
            details: FileParseError {
                format: "yaml",
                message: source.to_string().into_boxed_str(),
                line: source.location().map(|loc| loc.line()),
                column: source.location().map(|loc| loc.column()),
            },
        })
    })?;
    if !file_config.legacy_per_command.is_empty() {
        return Err(Error::Config(crate::error::ConfigError {
            path: std::path::PathBuf::from("<yaml-string>"),
            details: FileParseError {
                format: "yaml",
                message: "`per_command` has been renamed to `per_command_overrides`"
                    .to_owned()
                    .into_boxed_str(),
                line: None,
                column: None,
            },
        }));
    }
    let commands_yaml = file_config
        .commands
        .as_ref()
        .filter(|commands| !commands.is_null())
        .map(serialize_commands_yaml)
        .transpose()?;
    let mut config = Config::default();
    config.apply(file_config);
    config.validate_patterns().map_err(|msg| {
        Error::Config(crate::error::ConfigError {
            path: std::path::PathBuf::from("<yaml-string>"),
            details: FileParseError {
                format: "yaml",
                message: msg.into_boxed_str(),
                line: None,
                column: None,
            },
        })
    })?;
    Ok(ParsedYamlConfig {
        config,
        commands_yaml,
    })
}

fn serialize_commands_yaml(commands: &serde_yaml::Value) -> Result<Box<str>> {
    let key = serde_yaml::Value::String("commands".into());
    let mut wrapper = serde_yaml::Mapping::new();
    wrapper.insert(key, commands.clone());
    serde_yaml::to_string(&wrapper)
        .map(|yaml| yaml.into_boxed_str())
        .map_err(|source| {
            Error::Config(crate::error::ConfigError {
                path: std::path::PathBuf::from("<yaml-string>"),
                details: FileParseError {
                    format: "yaml",
                    message: format!("failed to serialize commands overrides: {source}")
                        .into_boxed_str(),
                    line: None,
                    column: None,
                },
            })
        })
}

fn load_config_file(path: &Path) -> Result<FileConfig> {
    let contents = std::fs::read_to_string(path).map_err(Error::Io)?;
    let config: FileConfig = match detect_config_format(path)? {
        ConfigFileFormat::Toml => toml::from_str(&contents).map_err(|source| {
            let (line, column) = toml_line_col(&contents, source.span().map(|span| span.start));
            Error::Config(crate::error::ConfigError {
                path: path.to_path_buf(),
                details: FileParseError {
                    format: ConfigFileFormat::Toml.as_str(),
                    message: source.to_string().into_boxed_str(),
                    line,
                    column,
                },
            })
        }),
        ConfigFileFormat::Yaml => serde_yaml::from_str(&contents).map_err(|source| {
            let location = source.location();
            let line = location.as_ref().map(|loc| loc.line());
            let column = location.as_ref().map(|loc| loc.column());
            Error::Config(crate::error::ConfigError {
                path: path.to_path_buf(),
                details: FileParseError {
                    format: ConfigFileFormat::Yaml.as_str(),
                    message: source.to_string().into_boxed_str(),
                    line,
                    column,
                },
            })
        }),
    }?;

    if !config.legacy_per_command.is_empty() {
        return Err(Error::Formatter(format!(
            "{}: `per_command` has been renamed to `per_command_overrides`",
            path.display()
        )));
    }

    Ok(config)
}

/// Find the config files that apply to `file_path`.
///
/// The nearest supported config discovered while walking upward wins. If
/// multiple supported config filenames exist in the same directory, YAML is
/// preferred over TOML. If no project-local config is found, the user home
/// config is returned when present.
fn find_config_files(file_path: &Path) -> Vec<PathBuf> {
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
        if let Some(candidate) = preferred_config_in_dir(d) {
            return vec![candidate];
        }

        if d.join(".git").exists() {
            break;
        }

        dir = d.parent();
    }

    if let Some(home) = home_dir() {
        if let Some(home_config) = preferred_config_in_dir(&home) {
            return vec![home_config];
        }
    }

    Vec::new()
}

pub(crate) fn detect_config_format(path: &Path) -> Result<ConfigFileFormat> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if file_name == CONFIG_FILE_NAME_TOML
        || path.extension().and_then(|ext| ext.to_str()) == Some("toml")
    {
        return Ok(ConfigFileFormat::Toml);
    }
    if matches!(file_name, CONFIG_FILE_NAME_YAML | CONFIG_FILE_NAME_YML)
        || matches!(
            path.extension().and_then(|ext| ext.to_str()),
            Some("yaml" | "yml")
        )
    {
        return Ok(ConfigFileFormat::Yaml);
    }

    Err(Error::Formatter(format!(
        "{}: unsupported config format; use .cmakefmt.yaml, .cmakefmt.yml, or .cmakefmt.toml",
        path.display()
    )))
}

fn preferred_config_in_dir(dir: &Path) -> Option<PathBuf> {
    CONFIG_FILE_NAMES
        .iter()
        .map(|name| dir.join(name))
        .find(|candidate| candidate.is_file())
}

pub(crate) fn toml_line_col(
    contents: &str,
    offset: Option<usize>,
) -> (Option<usize>, Option<usize>) {
    let Some(offset) = offset else {
        return (None, None);
    };
    let mut line = 1usize;
    let mut column = 1usize;
    for (index, ch) in contents.char_indices() {
        if index >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (Some(line), Some(column))
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
use_tabs = true
max_empty_lines = 2
dangle_parens = true
dangle_align = "open"
space_before_control_paren = true
space_before_definition_paren = true
max_hanging_wrap_positional_args = 3
max_hanging_wrap_groups = 1
command_case = "upper"
keyword_case = "lower"

[markup]
enable_markup = false
hashruler_min_length = 20

[per_command_overrides.message]
dangle_parens = true
line_width = 100
"#;
        let config: FileConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.format.line_width, Some(120));
        assert_eq!(config.format.tab_size, Some(4));
        assert_eq!(config.format.use_tabs, Some(true));
        assert_eq!(config.format.dangle_parens, Some(true));
        assert_eq!(config.format.dangle_align, Some(DangleAlign::Open));
        assert_eq!(config.format.command_case, Some(CaseStyle::Upper));
        assert_eq!(config.format.keyword_case, Some(CaseStyle::Lower));
        assert_eq!(config.markup.enable_markup, Some(false));

        let msg = config.per_command_overrides.get("message").unwrap();
        assert_eq!(msg.dangle_parens, Some(true));
        assert_eq!(msg.line_width, Some(100));
    }

    #[test]
    fn old_format_key_aliases_are_rejected() {
        let toml_str = r#"
[format]
use_tabchars = true
max_lines_hwrap = 4
max_pargs_hwrap = 3
max_subgroups_hwrap = 2
min_prefix_chars = 5
max_prefix_chars = 11
separate_ctrl_name_with_space = true
separate_fn_name_with_space = true
"#;
        let err = toml::from_str::<FileConfig>(toml_str)
            .unwrap_err()
            .to_string();
        assert!(err.contains("unknown field"));
    }

    #[test]
    fn config_from_file_applies_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join(CONFIG_FILE_NAME_TOML);
        fs::write(
            &config_path,
            r#"
[format]
line_width = 100
tab_size = 4
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
    fn default_yaml_config_template_parses() {
        let template = default_config_template();
        let parsed: FileConfig = serde_yaml::from_str(&template).unwrap();
        assert_eq!(parsed.format.line_width, Some(Config::default().line_width));
        assert_eq!(
            parsed.format.command_case,
            Some(Config::default().command_case)
        );
        assert_eq!(
            parsed.markup.enable_markup,
            Some(Config::default().enable_markup)
        );
    }

    #[test]
    fn toml_config_template_parses() {
        let template = default_config_template_for(DumpConfigFormat::Toml);
        let parsed: FileConfig = toml::from_str(&template).unwrap();
        assert_eq!(parsed.format.line_width, Some(Config::default().line_width));
        assert_eq!(
            parsed.format.command_case,
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
            dir.path().join(CONFIG_FILE_NAME_TOML),
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
            dir.path().join(CONFIG_FILE_NAME_TOML),
            "[format]\nline_width = 120\ntab_size = 4\n",
        )
        .unwrap();

        let subdir = dir.path().join("src");
        fs::create_dir(&subdir).unwrap();
        fs::write(
            subdir.join(CONFIG_FILE_NAME_TOML),
            "[format]\nline_width = 100\n",
        )
        .unwrap();

        let file = subdir.join("CMakeLists.txt");
        fs::write(&file, "").unwrap();

        let config = Config::for_file(&file).unwrap();
        // Only the nearest config is used automatically.
        assert_eq!(config.line_width, 100);
        assert_eq!(config.tab_size, Config::default().tab_size);
    }

    #[test]
    fn from_files_merges_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first.toml");
        let second = dir.path().join("second.toml");
        fs::write(&first, "[format]\nline_width = 120\ntab_size = 4\n").unwrap();
        fs::write(&second, "[format]\nline_width = 100\n").unwrap();

        let config = Config::from_files(&[first, second]).unwrap();
        assert_eq!(config.line_width, 100);
        assert_eq!(config.tab_size, 4);
    }

    #[test]
    fn yaml_config_from_file_applies_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join(CONFIG_FILE_NAME_YAML);
        fs::write(
            &config_path,
            "format:\n  line_width: 100\n  tab_size: 4\n  command_case: upper\n",
        )
        .unwrap();

        let config = Config::from_file(&config_path).unwrap();
        assert_eq!(config.line_width, 100);
        assert_eq!(config.tab_size, 4);
        assert_eq!(config.command_case, CaseStyle::Upper);
    }

    #[test]
    fn yml_config_from_file_applies_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join(CONFIG_FILE_NAME_YML);
        fs::write(
            &config_path,
            "format:\n  keyword_case: lower\n  line_width: 90\n",
        )
        .unwrap();

        let config = Config::from_file(&config_path).unwrap();
        assert_eq!(config.line_width, 90);
        assert_eq!(config.keyword_case, CaseStyle::Lower);
    }

    #[test]
    fn yaml_is_preferred_over_toml_during_discovery() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::write(
            dir.path().join(CONFIG_FILE_NAME_TOML),
            "[format]\nline_width = 120\n",
        )
        .unwrap();
        fs::write(
            dir.path().join(CONFIG_FILE_NAME_YAML),
            "format:\n  line_width: 90\n",
        )
        .unwrap();

        let file = dir.path().join("CMakeLists.txt");
        fs::write(&file, "").unwrap();

        let config = Config::for_file(&file).unwrap();
        assert_eq!(config.line_width, 90);
        assert_eq!(
            Config::config_sources_for(&file),
            vec![dir.path().join(CONFIG_FILE_NAME_YAML)]
        );
    }

    #[test]
    fn invalid_config_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CONFIG_FILE_NAME_TOML);
        fs::write(&path, "this is not valid toml {{{").unwrap();

        let result = Config::from_file(&path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("config error"));
    }

    #[test]
    fn config_from_yaml_file_applies_all_sections_and_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join(CONFIG_FILE_NAME_YAML);
        fs::write(
            &config_path,
            r#"
format:
  line_width: 96
  tab_size: 3
  use_tabs: true
  max_empty_lines: 2
  max_hanging_wrap_lines: 4
  max_hanging_wrap_positional_args: 7
  max_hanging_wrap_groups: 5
  dangle_parens: true
  dangle_align: open
  min_prefix_length: 2
  max_prefix_length: 12
  space_before_control_paren: true
  space_before_definition_paren: true
  command_case: unchanged
  keyword_case: lower
markup:
  enable_markup: false
  first_comment_is_literal: false
  literal_comment_pattern: '^\\s*KEEP'
  bullet_char: '-'
  enum_char: ')'
  fence_pattern: '^\\s*(```+).*'
  ruler_pattern: '^\\s*={5,}\\s*$'
  hashruler_min_length: 42
  canonicalize_hashrulers: false
per_command_overrides:
  my_custom_command:
    line_width: 101
    tab_size: 5
    dangle_parens: false
    dangle_align: prefix
    max_hanging_wrap_positional_args: 8
    max_hanging_wrap_groups: 9
"#,
        )
        .unwrap();

        let config = Config::from_file(&config_path).unwrap();
        assert_eq!(config.line_width, 96);
        assert_eq!(config.tab_size, 3);
        assert!(config.use_tabchars);
        assert_eq!(config.max_empty_lines, 2);
        assert_eq!(config.max_lines_hwrap, 4);
        assert_eq!(config.max_pargs_hwrap, 7);
        assert_eq!(config.max_subgroups_hwrap, 5);
        assert!(config.dangle_parens);
        assert_eq!(config.dangle_align, DangleAlign::Open);
        assert_eq!(config.min_prefix_chars, 2);
        assert_eq!(config.max_prefix_chars, 12);
        assert!(config.separate_ctrl_name_with_space);
        assert!(config.separate_fn_name_with_space);
        assert_eq!(config.command_case, CaseStyle::Unchanged);
        assert_eq!(config.keyword_case, CaseStyle::Lower);
        assert!(!config.enable_markup);
        assert!(!config.first_comment_is_literal);
        assert_eq!(config.literal_comment_pattern, "^\\\\s*KEEP");
        assert_eq!(config.bullet_char, "-");
        assert_eq!(config.enum_char, ")");
        assert_eq!(config.fence_pattern, "^\\\\s*(```+).*");
        assert_eq!(config.ruler_pattern, "^\\\\s*={5,}\\\\s*$");
        assert_eq!(config.hashruler_min_length, 42);
        assert!(!config.canonicalize_hashrulers);
        let per_command = config
            .per_command_overrides
            .get("my_custom_command")
            .unwrap();
        assert_eq!(per_command.line_width, Some(101));
        assert_eq!(per_command.tab_size, Some(5));
        assert_eq!(per_command.dangle_parens, Some(false));
        assert_eq!(per_command.dangle_align, Some(DangleAlign::Prefix));
        assert_eq!(per_command.max_pargs_hwrap, Some(8));
        assert_eq!(per_command.max_subgroups_hwrap, Some(9));
    }

    #[test]
    fn detect_config_format_supports_yaml_and_rejects_unknown() {
        assert!(matches!(
            detect_config_format(Path::new(".cmakefmt.yml")).unwrap(),
            ConfigFileFormat::Yaml
        ));
        assert!(matches!(
            detect_config_format(Path::new("tooling/settings.yaml")).unwrap(),
            ConfigFileFormat::Yaml
        ));
        assert!(matches!(
            detect_config_format(Path::new("project.toml")).unwrap(),
            ConfigFileFormat::Toml
        ));
        let err = detect_config_format(Path::new("config.json")).unwrap_err();
        assert!(err.to_string().contains("unsupported config format"));
    }

    #[test]
    fn yaml_config_with_legacy_per_command_key_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join(CONFIG_FILE_NAME_YAML);
        fs::write(
            &config_path,
            "per_command:\n  message:\n    line_width: 120\n",
        )
        .unwrap();
        let err = Config::from_file(&config_path).unwrap_err();
        assert!(err
            .to_string()
            .contains("`per_command` has been renamed to `per_command_overrides`"));
    }

    #[test]
    fn invalid_yaml_reports_line_and_column() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join(CONFIG_FILE_NAME_YAML);
        fs::write(&config_path, "format:\n  line_width: [\n").unwrap();

        let err = Config::from_file(&config_path).unwrap_err();
        match err {
            Error::Config(config_err) => {
                let details = &config_err.details;
                assert_eq!(details.format, "YAML");
                assert!(details.line.is_some());
                assert!(details.column.is_some());
            }
            other => panic!("expected config parse error, got {other:?}"),
        }
    }

    #[test]
    fn toml_line_col_returns_none_when_offset_is_missing() {
        assert_eq!(toml_line_col("line = true\n", None), (None, None));
    }

    // ── from_yaml_str tests ─────────────────────────────────────────────

    #[test]
    fn from_yaml_str_parses_format_section() {
        let config = Config::from_yaml_str("format:\n  line_width: 120\n  tab_size: 4").unwrap();
        assert_eq!(config.line_width, 120);
        assert_eq!(config.tab_size, 4);
    }

    #[test]
    fn from_yaml_str_parses_casing_in_format_section() {
        let config = Config::from_yaml_str("format:\n  command_case: upper").unwrap();
        assert_eq!(config.command_case, CaseStyle::Upper);
    }

    #[test]
    fn from_yaml_str_parses_markup_section() {
        let config = Config::from_yaml_str("markup:\n  enable_markup: false").unwrap();
        assert!(!config.enable_markup);
    }

    #[test]
    fn from_yaml_str_with_commands_extracts_serialized_commands_block() {
        let (_, commands_yaml) =
            Config::from_yaml_str_with_commands("commands:\n  my_cmd:\n    pargs: 1").unwrap();
        let commands_yaml = commands_yaml.expect("expected serialized commands YAML");
        assert!(commands_yaml.contains("commands:"));
        assert!(commands_yaml.contains("my_cmd:"));
    }

    #[test]
    fn from_yaml_str_rejects_unknown_top_level_field() {
        let result = Config::from_yaml_str("bogus_section:\n  foo: bar");
        assert!(result.is_err());
    }

    #[test]
    fn from_yaml_str_rejects_unknown_format_field() {
        let result = Config::from_yaml_str("format:\n  nonexistent: 42");
        assert!(result.is_err());
    }

    #[test]
    fn from_yaml_str_rejects_invalid_yaml() {
        let result = Config::from_yaml_str("{{invalid");
        assert!(result.is_err());
    }

    #[test]
    fn from_yaml_str_empty_string_returns_defaults() {
        let config = Config::from_yaml_str("").unwrap();
        assert_eq!(config.line_width, Config::default().line_width);
    }

    #[test]
    fn from_yaml_str_multiple_sections() {
        let config =
            Config::from_yaml_str("format:\n  line_width: 100\n  command_case: upper").unwrap();
        assert_eq!(config.line_width, 100);
        assert_eq!(config.command_case, CaseStyle::Upper);
    }

    #[test]
    fn from_yaml_str_rejects_legacy_per_command() {
        let result = Config::from_yaml_str("per_command:\n  message:\n    line_width: 120");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("per_command_overrides"),
            "error should mention the new key name: {err}"
        );
    }
}
