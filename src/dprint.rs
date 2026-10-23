// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! dprint WebAssembly plugin adapter.

//! The adapter deliberately calls the crate's public formatting API instead of
//! maintaining a second parser or formatter for the dprint protocol.

use dprint_core::configuration::{
    get_nullable_value, get_unknown_property_diagnostics, ConfigKeyMap, ConfigurationDiagnostic,
    GlobalConfiguration, NewLineKind,
};
use dprint_core::plugins::{
    CheckConfigUpdatesMessage, ConfigChange, FileMatchingInfo, FormatError, FormatResult,
    PluginInfo, PluginResolveConfigurationResult, SyncFormatRequest, SyncHostFormatRequest,
    SyncPluginHandler,
};
use serde::{Deserialize, Serialize};

use crate::{
    ArgumentCommentStyle, CaseStyle, Config, ContinuationAlign, DangleAlign, LineEnding,
    WrapAfterFirstArgAlign,
};

/// dprint protocol handler for CMake source files.
#[derive(Debug, Clone, Default)]
pub struct CmakefmtPlugin;

/// Validated plugin configuration, including inherited dprint settings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedConfig {
    line_width: usize,
    indent_width: usize,
    use_tabs: bool,
    new_line_kind: NewLineKind,
    command_case: CaseStyle,
    keyword_case: CaseStyle,
    max_empty_lines: usize,
    max_lines_hwrap: usize,
    max_pargs_hwrap: usize,
    max_subgroups_hwrap: usize,
    max_rows_cmdline: usize,
    require_valid_layout: bool,
    space_before_control_paren: bool,
    space_before_function_paren: bool,
    wrap_after_first_arg: bool,
    wrap_after_first_arg_align: WrapAfterFirstArgAlign,
    continuation_align: ContinuationAlign,
    enable_sort: bool,
    autosort: bool,
    argument_comment_style: ArgumentCommentStyle,
    dangle_parens: bool,
    dangle_align: DangleAlign,
    enable_markup: bool,
    first_comment_is_literal: bool,
}

impl Default for ResolvedConfig {
    fn default() -> Self {
        let config = Config::default();
        Self {
            line_width: config.line_width,
            indent_width: config.tab_size,
            use_tabs: config.use_tabchars,
            new_line_kind: NewLineKind::LineFeed,
            command_case: config.command_case,
            keyword_case: config.keyword_case,
            max_empty_lines: config.max_empty_lines,
            max_lines_hwrap: config.max_lines_hwrap,
            max_pargs_hwrap: config.max_pargs_hwrap,
            max_subgroups_hwrap: config.max_subgroups_hwrap,
            max_rows_cmdline: config.max_rows_cmdline,
            require_valid_layout: config.require_valid_layout,
            space_before_control_paren: config.separate_ctrl_name_with_space,
            space_before_function_paren: config.separate_fn_name_with_space,
            wrap_after_first_arg: config.wrap_after_first_arg,
            wrap_after_first_arg_align: config.wrap_after_first_arg_align,
            continuation_align: config.continuation_align,
            enable_sort: config.enable_sort,
            autosort: config.autosort,
            argument_comment_style: config.argument_comment_style,
            dangle_parens: config.dangle_parens,
            dangle_align: config.dangle_align,
            enable_markup: config.enable_markup,
            first_comment_is_literal: config.first_comment_is_literal,
        }
    }
}

impl ResolvedConfig {
    fn to_cmakefmt_config(&self) -> Config {
        Config {
            line_width: self.line_width,
            tab_size: self.indent_width,
            use_tabchars: self.use_tabs,
            line_ending: match self.new_line_kind {
                NewLineKind::LineFeed => LineEnding::Unix,
                NewLineKind::CarriageReturnLineFeed => LineEnding::Windows,
                NewLineKind::Auto => LineEnding::Auto,
            },
            command_case: self.command_case,
            keyword_case: self.keyword_case,
            max_empty_lines: self.max_empty_lines,
            max_lines_hwrap: self.max_lines_hwrap,
            max_pargs_hwrap: self.max_pargs_hwrap,
            max_subgroups_hwrap: self.max_subgroups_hwrap,
            max_rows_cmdline: self.max_rows_cmdline,
            require_valid_layout: self.require_valid_layout,
            separate_ctrl_name_with_space: self.space_before_control_paren,
            separate_fn_name_with_space: self.space_before_function_paren,
            wrap_after_first_arg: self.wrap_after_first_arg,
            wrap_after_first_arg_align: self.wrap_after_first_arg_align,
            continuation_align: self.continuation_align,
            enable_sort: self.enable_sort,
            autosort: self.autosort,
            argument_comment_style: self.argument_comment_style,
            dangle_parens: self.dangle_parens,
            dangle_align: self.dangle_align,
            enable_markup: self.enable_markup,
            first_comment_is_literal: self.first_comment_is_literal,
            ..Config::default()
        }
    }
}

impl SyncPluginHandler<ResolvedConfig> for CmakefmtPlugin {
    fn resolve_config(
        &mut self,
        mut config: ConfigKeyMap,
        global_config: &GlobalConfiguration,
    ) -> PluginResolveConfigurationResult<ResolvedConfig> {
        let mut diagnostics = Vec::new();
        let mut resolved = ResolvedConfig::default();

        if let Some(value) = global_config.line_width {
            resolved.line_width = value as usize;
        }
        if let Some(value) = global_config.indent_width {
            resolved.indent_width = value as usize;
        }
        if let Some(value) = global_config.use_tabs {
            resolved.use_tabs = value;
        }
        if let Some(value) = global_config.new_line_kind {
            resolved.new_line_kind = value;
        }

        apply_value(
            &mut config,
            "lineWidth",
            &mut resolved.line_width,
            &mut diagnostics,
        );
        apply_value(
            &mut config,
            "indentWidth",
            &mut resolved.indent_width,
            &mut diagnostics,
        );
        apply_value(
            &mut config,
            "useTabs",
            &mut resolved.use_tabs,
            &mut diagnostics,
        );
        apply_value(
            &mut config,
            "newLineKind",
            &mut resolved.new_line_kind,
            &mut diagnostics,
        );
        apply_enum(
            &mut config,
            "commandCase",
            &mut resolved.command_case,
            parse_case_style,
            &mut diagnostics,
        );
        apply_enum(
            &mut config,
            "keywordCase",
            &mut resolved.keyword_case,
            parse_case_style,
            &mut diagnostics,
        );
        apply_value(
            &mut config,
            "maxEmptyLines",
            &mut resolved.max_empty_lines,
            &mut diagnostics,
        );
        apply_value(
            &mut config,
            "maxLinesHwrap",
            &mut resolved.max_lines_hwrap,
            &mut diagnostics,
        );
        apply_value(
            &mut config,
            "maxHangingWrapPositionalArgs",
            &mut resolved.max_pargs_hwrap,
            &mut diagnostics,
        );
        apply_value(
            &mut config,
            "maxHangingWrapGroups",
            &mut resolved.max_subgroups_hwrap,
            &mut diagnostics,
        );
        apply_value(
            &mut config,
            "maxRowsCmdline",
            &mut resolved.max_rows_cmdline,
            &mut diagnostics,
        );
        apply_value(
            &mut config,
            "requireValidLayout",
            &mut resolved.require_valid_layout,
            &mut diagnostics,
        );
        apply_value(
            &mut config,
            "spaceBeforeControlParen",
            &mut resolved.space_before_control_paren,
            &mut diagnostics,
        );
        apply_value(
            &mut config,
            "spaceBeforeFunctionParen",
            &mut resolved.space_before_function_paren,
            &mut diagnostics,
        );
        apply_value(
            &mut config,
            "wrapAfterFirstArg",
            &mut resolved.wrap_after_first_arg,
            &mut diagnostics,
        );
        apply_enum(
            &mut config,
            "wrapAfterFirstArgAlign",
            &mut resolved.wrap_after_first_arg_align,
            parse_wrap_after_first_arg_align,
            &mut diagnostics,
        );
        apply_enum(
            &mut config,
            "continuationAlign",
            &mut resolved.continuation_align,
            parse_continuation_align,
            &mut diagnostics,
        );
        apply_value(
            &mut config,
            "enableSort",
            &mut resolved.enable_sort,
            &mut diagnostics,
        );
        apply_value(
            &mut config,
            "autosort",
            &mut resolved.autosort,
            &mut diagnostics,
        );
        apply_enum(
            &mut config,
            "argumentCommentStyle",
            &mut resolved.argument_comment_style,
            parse_argument_comment_style,
            &mut diagnostics,
        );
        apply_value(
            &mut config,
            "dangleParens",
            &mut resolved.dangle_parens,
            &mut diagnostics,
        );
        apply_enum(
            &mut config,
            "dangleAlign",
            &mut resolved.dangle_align,
            parse_dangle_align,
            &mut diagnostics,
        );
        apply_value(
            &mut config,
            "enableMarkup",
            &mut resolved.enable_markup,
            &mut diagnostics,
        );
        apply_value(
            &mut config,
            "firstCommentIsLiteral",
            &mut resolved.first_comment_is_literal,
            &mut diagnostics,
        );

        diagnostics.extend(get_unknown_property_diagnostics(config));

        for (property_name, value) in [
            ("lineWidth", &mut resolved.line_width),
            ("indentWidth", &mut resolved.indent_width),
        ] {
            if *value == 0 {
                diagnostics.push(ConfigurationDiagnostic {
                    property_name: property_name.to_owned(),
                    message: "expected a positive integer".to_owned(),
                });
                *value = match property_name {
                    "lineWidth" => Config::default().line_width,
                    _ => Config::default().tab_size,
                };
            }
        }

        PluginResolveConfigurationResult {
            file_matching: FileMatchingInfo {
                file_extensions: vec!["cmake".to_owned()],
                file_names: vec!["CMakeLists.txt".to_owned(), "CMakeLists.txt.in".to_owned()],
                additive: false,
            },
            diagnostics,
            config: resolved,
        }
    }

    fn plugin_info(&mut self) -> PluginInfo {
        let version = env!("CARGO_PKG_VERSION");
        PluginInfo {
            name: "cmakefmt".to_owned(),
            version: version.to_owned(),
            config_key: "cmakefmt".to_owned(),
            help_url: concat!(
                env!("CARGO_PKG_REPOSITORY"),
                "/blob/main/docs/src/content/docs/editors.md"
            )
            .to_owned(),
            config_schema_url: format!(
                "https://plugins.dprint.dev/cmakefmt/cmakefmt/v{version}/schema.json"
            ),
            update_url: Some(
                "https://plugins.dprint.dev/cmakefmt/cmakefmt/latest.json".to_string(),
            ),
        }
    }

    fn license_text(&mut self) -> String {
        include_str!("../LICENSE").to_owned()
    }

    fn check_config_updates(
        &self,
        _message: CheckConfigUpdatesMessage,
    ) -> Result<Vec<ConfigChange>, FormatError> {
        Ok(Vec::new())
    }

    fn format(
        &mut self,
        request: SyncFormatRequest<ResolvedConfig>,
        _format_with_host: impl FnMut(SyncHostFormatRequest) -> FormatResult,
    ) -> FormatResult {
        if request.token.is_cancelled() {
            return Err("Formatting cancelled.".into());
        }
        if request.range.is_some() {
            return Err("Range formatting is not supported by cmakefmt.".into());
        }

        let source = std::str::from_utf8(&request.file_bytes).map_err(|err| {
            FormatError::new(format!(
                "Could not format {} because it is not valid UTF-8: {err}",
                request.file_path.display()
            ))
        })?;
        let formatted = crate::format_source(source, &request.config.to_cmakefmt_config())
            .map_err(|err| {
                FormatError::new(format!(
                    "Could not format {} with cmakefmt: {err}",
                    request.file_path.display()
                ))
            })?;

        if request.token.is_cancelled() {
            return Err("Formatting cancelled.".into());
        }
        Ok((formatted != source).then(|| formatted.into_bytes()))
    }
}

fn apply_value<T>(
    config: &mut ConfigKeyMap,
    key: &str,
    target: &mut T,
    diagnostics: &mut Vec<ConfigurationDiagnostic>,
) where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    if let Some(value) = get_nullable_value(config, key, diagnostics) {
        *target = value;
    }
}

fn apply_enum<T>(
    config: &mut ConfigKeyMap,
    key: &str,
    target: &mut T,
    parse: impl FnOnce(&str) -> Result<T, String>,
    diagnostics: &mut Vec<ConfigurationDiagnostic>,
) {
    if let Some(value) = get_nullable_value::<String>(config, key, diagnostics) {
        match parse(&value) {
            Ok(value) => *target = value,
            Err(message) => diagnostics.push(ConfigurationDiagnostic {
                property_name: key.to_owned(),
                message,
            }),
        }
    }
}

fn parse_case_style(value: &str) -> Result<CaseStyle, String> {
    match value {
        "lower" => Ok(CaseStyle::Lower),
        "upper" => Ok(CaseStyle::Upper),
        "unchanged" => Ok(CaseStyle::Unchanged),
        _ => Err(format!(
            "expected lower, upper, or unchanged; found {value:?}"
        )),
    }
}

fn parse_argument_comment_style(value: &str) -> Result<ArgumentCommentStyle, String> {
    match value {
        "preserve" => Ok(ArgumentCommentStyle::Preserve),
        "standalone" => Ok(ArgumentCommentStyle::Standalone),
        _ => Err(format!("expected preserve or standalone; found {value:?}")),
    }
}

fn parse_wrap_after_first_arg_align(value: &str) -> Result<WrapAfterFirstArgAlign, String> {
    match value {
        "default-indent" => Ok(WrapAfterFirstArgAlign::DefaultIndent),
        "under-first-arg" => Ok(WrapAfterFirstArgAlign::UnderFirstArg),
        _ => Err(format!(
            "expected default-indent or under-first-arg; found {value:?}"
        )),
    }
}

fn parse_continuation_align(value: &str) -> Result<ContinuationAlign, String> {
    match value {
        "same-indent" => Ok(ContinuationAlign::SameIndent),
        "under-first-value" => Ok(ContinuationAlign::UnderFirstValue),
        _ => Err(format!(
            "expected same-indent or under-first-value; found {value:?}"
        )),
    }
}

fn parse_dangle_align(value: &str) -> Result<DangleAlign, String> {
    match value {
        "prefix" => Ok(DangleAlign::Prefix),
        "open" => Ok(DangleAlign::Open),
        "close" => Ok(DangleAlign::Close),
        _ => Err(format!("expected prefix, open, or close; found {value:?}")),
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
dprint_core::generate_plugin_code!(CmakefmtPlugin, CmakefmtPlugin, ResolvedConfig);

#[cfg(test)]
mod tests {
    use std::path::Path;

    use dprint_core::configuration::ConfigKeyValue;
    use dprint_core::plugins::{CancellationToken, FormatConfigId, NullCancellationToken};

    use super::*;

    #[test]
    fn resolves_supported_config_and_file_patterns() {
        let mut config = ConfigKeyMap::new();
        config.insert(
            "argumentCommentStyle".to_owned(),
            ConfigKeyValue::String("standalone".into()),
        );
        let mut plugin = CmakefmtPlugin;
        let result = plugin.resolve_config(config, &GlobalConfiguration::default());

        assert!(result.diagnostics.is_empty());
        assert_eq!(
            result.config.argument_comment_style,
            ArgumentCommentStyle::Standalone
        );
        assert_eq!(result.file_matching.file_extensions, vec!["cmake"]);
        assert!(!result.file_matching.additive);
        assert!(result
            .file_matching
            .file_names
            .contains(&"CMakeLists.txt".to_owned()));
    }

    #[test]
    fn argument_comment_style_defaults_to_preserve_and_rejects_compact() {
        let default =
            CmakefmtPlugin.resolve_config(ConfigKeyMap::new(), &GlobalConfiguration::default());
        assert_eq!(
            default.config.argument_comment_style,
            ArgumentCommentStyle::Preserve
        );
        for (key, value) in [
            (
                "argumentCommentStyle",
                ConfigKeyValue::String("compact".into()),
            ),
            ("argumentCommentStyle", ConfigKeyValue::Bool(true)),
            ("preserveArgumentComments", ConfigKeyValue::Bool(true)),
        ] {
            let mut config = ConfigKeyMap::new();
            config.insert(key.to_owned(), value);
            let result = CmakefmtPlugin.resolve_config(config, &GlobalConfiguration::default());
            assert!(!result.diagnostics.is_empty());
        }
    }

    #[test]
    fn spacing_controls_match_call_and_structural_categories() {
        let source = "project(foo)\nfunction(helper)\ncustom_call()\nendfunction()\n";
        for function_spacing in [false, true] {
            for control_spacing in [false, true] {
                let mut config = ConfigKeyMap::new();
                config.insert(
                    "spaceBeforeFunctionParen".into(),
                    ConfigKeyValue::Bool(function_spacing),
                );
                config.insert(
                    "spaceBeforeControlParen".into(),
                    ConfigKeyValue::Bool(control_spacing),
                );
                let result = CmakefmtPlugin.resolve_config(config, &GlobalConfiguration::default());
                assert!(result.diagnostics.is_empty());
                let call = if function_spacing { " " } else { "" };
                let flow = if control_spacing { " " } else { "" };
                let expected = format!("project{call}(foo)\nfunction{flow}(helper)\n  custom_call{call}()\nendfunction{flow}()\n");
                let request = SyncFormatRequest {
                    file_path: Path::new("CMakeLists.txt"),
                    file_bytes: source.as_bytes().to_vec(),
                    config_id: FormatConfigId::uninitialized(),
                    config: &result.config,
                    range: None,
                    token: &NullCancellationToken,
                };
                assert_eq!(
                    CmakefmtPlugin.format(request, |_| Ok(None)).unwrap(),
                    Some(expected.into_bytes())
                );
            }
        }
    }

    #[test]
    fn function_paren_spacing_resolves_and_formats() {
        for (value, source, expected) in [
            (None, "project (foo)\n", "project(foo)\n"),
            (Some(false), "project (foo)\n", "project(foo)\n"),
            (Some(true), "project(foo)\n", "project (foo)\n"),
        ] {
            let mut config = ConfigKeyMap::new();
            if let Some(value) = value {
                config.insert(
                    "spaceBeforeFunctionParen".into(),
                    ConfigKeyValue::Bool(value),
                );
            }
            let result = CmakefmtPlugin.resolve_config(config, &GlobalConfiguration::default());
            assert!(result.diagnostics.is_empty());
            let request = SyncFormatRequest {
                file_path: Path::new("CMakeLists.txt"),
                file_bytes: source.as_bytes().to_vec(),
                config_id: FormatConfigId::uninitialized(),
                config: &result.config,
                range: None,
                token: &NullCancellationToken,
            };
            assert_eq!(
                CmakefmtPlugin.format(request, |_| Ok(None)).unwrap(),
                Some(expected.as_bytes().to_vec())
            );
        }
        let mut config = ConfigKeyMap::new();
        config.insert(
            "spaceBeforeFunctionParen".into(),
            ConfigKeyValue::String("invalid".into()),
        );
        let result = CmakefmtPlugin.resolve_config(config, &GlobalConfiguration::default());
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(
            result.diagnostics[0].property_name,
            "spaceBeforeFunctionParen"
        );
    }

    #[test]
    fn wrap_first_alignment_resolves_formats_and_round_trips() {
        let source =
            "target_compile_definitions(mylib INTERFACE USING_DLL COMMON_USING_DLL EXTRA_DEFINE)\n";
        let under_first_arg = "target_compile_definitions(mylib\n                           INTERFACE\n                             USING_DLL\n                             COMMON_USING_DLL\n                             EXTRA_DEFINE)\n";
        let default_indent = "target_compile_definitions(mylib\n  INTERFACE\n    USING_DLL COMMON_USING_DLL EXTRA_DEFINE)\n";
        for (value, expected) in [
            (None, under_first_arg),
            (Some("under-first-arg"), under_first_arg),
            (Some("default-indent"), default_indent),
        ] {
            let mut config = ConfigKeyMap::new();
            config.insert("wrapAfterFirstArg".into(), ConfigKeyValue::Bool(true));
            config.insert("lineWidth".into(), ConfigKeyValue::Number(50));
            if let Some(value) = value {
                config.insert(
                    "wrapAfterFirstArgAlign".into(),
                    ConfigKeyValue::String(value.into()),
                );
            }
            let result = CmakefmtPlugin.resolve_config(config, &GlobalConfiguration::default());
            assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
            // The WASM protocol serializes resolved configuration between calls.
            let serialized = serde_json::to_string(&result.config).unwrap();
            let resolved: ResolvedConfig = serde_json::from_str(&serialized).unwrap();
            assert_eq!(result.config, resolved);
            let request = |source: &str| SyncFormatRequest {
                file_path: Path::new("CMakeLists.txt"),
                file_bytes: source.as_bytes().to_vec(),
                config_id: FormatConfigId::uninitialized(),
                config: &resolved,
                range: None,
                token: &NullCancellationToken,
            };
            assert_eq!(
                CmakefmtPlugin
                    .format(request(source), |_| Ok(None))
                    .unwrap(),
                Some(expected.as_bytes().to_vec())
            );
            assert_eq!(
                CmakefmtPlugin
                    .format(request(expected), |_| Ok(None))
                    .unwrap(),
                None
            );
        }
    }

    #[test]
    fn wrap_first_alignment_rejects_invalid_values() {
        for value in [
            ConfigKeyValue::String("same-indent".into()),
            ConfigKeyValue::Bool(true),
        ] {
            let mut config = ConfigKeyMap::new();
            config.insert("wrapAfterFirstArgAlign".into(), value);
            let result = CmakefmtPlugin.resolve_config(config, &GlobalConfiguration::default());
            assert_eq!(result.diagnostics.len(), 1);
            assert_eq!(
                result.diagnostics[0].property_name,
                "wrapAfterFirstArgAlign"
            );
            assert_eq!(
                result.config.wrap_after_first_arg_align,
                WrapAfterFirstArgAlign::UnderFirstArg
            );
        }
    }

    #[test]
    fn inherits_globals_and_plugin_settings_take_precedence() {
        let global = GlobalConfiguration {
            line_width: Some(90),
            indent_width: Some(4),
            use_tabs: Some(true),
            new_line_kind: Some(NewLineKind::CarriageReturnLineFeed),
        };
        let mut config = ConfigKeyMap::new();
        config.insert("indentWidth".to_owned(), ConfigKeyValue::Number(2));
        config.insert("useTabs".to_owned(), ConfigKeyValue::Bool(false));
        let result = CmakefmtPlugin.resolve_config(config, &global);
        assert!(result.diagnostics.is_empty());
        let resolved = result.config.to_cmakefmt_config();
        assert_eq!(resolved.line_width, 90);
        assert_eq!(resolved.tab_size, 2);
        assert!(!resolved.use_tabchars);
        assert_eq!(resolved.line_ending, LineEnding::Windows);
    }

    #[test]
    fn rejects_invalid_and_unknown_configuration() {
        let mut config = ConfigKeyMap::new();
        config.insert("lineWidth".to_owned(), ConfigKeyValue::Number(0));
        config.insert("indentWidth".to_owned(), ConfigKeyValue::Number(-1));
        config.insert(
            "commandCase".to_owned(),
            ConfigKeyValue::String("invalid".into()),
        );
        config.insert("unsupported".to_owned(), ConfigKeyValue::Bool(true));
        let result = CmakefmtPlugin.resolve_config(config, &GlobalConfiguration::default());
        let mut properties: Vec<_> = result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.property_name.as_str())
            .collect();
        properties.sort_unstable();
        assert_eq!(
            properties,
            ["commandCase", "indentWidth", "lineWidth", "unsupported"]
        );
        assert_eq!(result.config.line_width, Config::default().line_width);
        assert_eq!(result.config.indent_width, Config::default().tab_size);
    }

    #[test]
    fn formats_cmake_source_and_reports_stable_input() {
        let mut plugin = CmakefmtPlugin;
        let config = ResolvedConfig::default();
        let request = |source: &str| SyncFormatRequest {
            file_path: Path::new("CMakeLists.txt"),
            file_bytes: source.as_bytes().to_vec(),
            config_id: FormatConfigId::uninitialized(),
            config: &config,
            range: None,
            token: &NullCancellationToken,
        };

        assert_eq!(
            plugin
                .format(request("CMAKE_MINIMUM_REQUIRED(VERSION 3.20)\n"), |_| Ok(
                    None
                ))
                .unwrap(),
            Some(b"cmake_minimum_required(VERSION 3.20)\n".to_vec())
        );
        assert_eq!(
            plugin
                .format(request("cmake_minimum_required(VERSION 3.20)\n"), |_| Ok(
                    None
                ))
                .unwrap(),
            None
        );
    }

    #[test]
    fn rejects_ranges_and_cancellation() {
        let mut plugin = CmakefmtPlugin;
        let config = ResolvedConfig::default();
        let range_request = SyncFormatRequest {
            file_path: Path::new("CMakeLists.txt"),
            file_bytes: b"cmake_minimum_required(VERSION 3.20)\n".to_vec(),
            config_id: FormatConfigId::uninitialized(),
            config: &config,
            range: Some(0..1),
            token: &NullCancellationToken,
        };
        assert!(plugin
            .format(range_request, |_| Ok(None))
            .unwrap_err()
            .to_string()
            .contains("Range formatting"));

        #[derive(Debug)]
        struct Cancelled;
        impl CancellationToken for Cancelled {
            fn is_cancelled(&self) -> bool {
                true
            }
        }
        let token = Cancelled;
        let cancelled_request = SyncFormatRequest {
            file_path: Path::new("CMakeLists.txt"),
            file_bytes: Vec::new(),
            config_id: FormatConfigId::uninitialized(),
            config: &config,
            range: None,
            token: &token,
        };
        assert!(plugin
            .format(cancelled_request, |_| Ok(None))
            .unwrap_err()
            .to_string()
            .contains("cancelled"));
    }
}
