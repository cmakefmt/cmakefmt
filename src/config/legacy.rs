// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Conversion support for legacy `cmake-format` configuration files.
//!
//! `cmake-format` historically supported Python, JSON, and YAML config files.
//! `cmakefmt` now accepts YAML or TOML user config, and provides a converter
//! that can render either format so users can start from a structured baseline
//! and then adapt it as needed.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::config::{
    file::DumpConfigFormat, CaseStyle, DangleAlign, FractionalTabPolicy, LineEnding,
    PerCommandConfig,
};
use crate::error::{Error, Result};
use crate::spec::{
    CommandFormOverride, CommandSpecOverride, KwargSpecOverride, LayoutOverridesOverride, NArgs,
};

/// Convert one or more legacy `cmake-format` config files into `cmakefmt`
/// config text in the requested output format.
///
/// Files are merged in the order provided, with later files overriding earlier
/// ones. The returned string is valid YAML or TOML prefixed by explanatory
/// comments.
pub fn convert_legacy_config_files(paths: &[PathBuf], format: DumpConfigFormat) -> Result<String> {
    if paths.is_empty() {
        return Err(Error::Formatter(
            "cmakefmt config convert requires at least one input path".to_owned(),
        ));
    }

    let mut converted = ConvertedConfig::default();
    for path in paths {
        let source = std::fs::read_to_string(path).map_err(Error::Io)?;
        let root = parse_legacy_config(path, &source)?;
        merge_legacy_root(&mut converted, &root, path);
    }

    let rendered = match format {
        DumpConfigFormat::Toml => {
            toml::to_string_pretty(&converted.as_config_file()).map_err(|err| {
                Error::Formatter(format!("failed to render converted config as TOML: {err}"))
            })?
        }
        DumpConfigFormat::Yaml => {
            serde_yaml::to_string(&converted.as_config_file()).map_err(|err| {
                Error::Formatter(format!("failed to render converted config as YAML: {err}"))
            })?
        }
    };

    let mut output = String::new();
    output.push_str("# Converted from legacy cmake-format configuration.\n");
    output.push_str(
        "# Review this file before committing it; unsupported options are listed below.\n",
    );

    if !converted.notes.is_empty() {
        output.push_str("#\n");
        output.push_str("# Conversion notes:\n");
        for note in &converted.notes {
            output.push_str("# - ");
            output.push_str(note);
            output.push('\n');
        }
    }

    output.push('\n');
    output.push_str(&rendered);
    Ok(output)
}

#[derive(Debug, Clone, PartialEq)]
enum LegacyValue {
    Null,
    Bool(bool),
    Integer(i64),
    String(String),
    Array(Vec<LegacyValue>),
    Table(BTreeMap<String, LegacyValue>),
}

impl LegacyValue {
    fn as_table(&self) -> Option<&BTreeMap<String, LegacyValue>> {
        match self {
            LegacyValue::Table(table) => Some(table),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            LegacyValue::String(value) => Some(value),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LegacyFormat {
    Json,
    Yaml,
    Python,
}

fn parse_legacy_config(path: &Path, source: &str) -> Result<BTreeMap<String, LegacyValue>> {
    let root = match detect_legacy_format(path)? {
        LegacyFormat::Json => legacy_from_json(serde_json::from_str(source).map_err(|err| {
            Error::Formatter(format!(
                "{}: invalid JSON legacy config: {err}",
                path.display()
            ))
        })?),
        LegacyFormat::Yaml => legacy_from_yaml(serde_yaml::from_str(source).map_err(|err| {
            Error::Formatter(format!(
                "{}: invalid YAML legacy config: {err}",
                path.display()
            ))
        })?),
        LegacyFormat::Python => parse_python_legacy_config(path, source)?,
    };

    match root {
        LegacyValue::Table(table) => Ok(table),
        _ => Err(Error::Formatter(format!(
            "{}: legacy config root must be a mapping/object",
            path.display()
        ))),
    }
}

fn detect_legacy_format(path: &Path) -> Result<LegacyFormat> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if file_name.ends_with(".json") {
        return Ok(LegacyFormat::Json);
    }
    if file_name.ends_with(".yaml") || file_name.ends_with(".yml") {
        return Ok(LegacyFormat::Yaml);
    }
    if file_name.ends_with(".py") {
        return Ok(LegacyFormat::Python);
    }

    Err(Error::Formatter(format!(
        "{}: unsupported legacy config format; expected .json, .yaml, .yml, or .py",
        path.display()
    )))
}

fn legacy_from_json(value: serde_json::Value) -> LegacyValue {
    match value {
        serde_json::Value::Null => LegacyValue::Null,
        serde_json::Value::Bool(value) => LegacyValue::Bool(value),
        serde_json::Value::Number(value) => {
            LegacyValue::Integer(value.as_i64().unwrap_or_default())
        }
        serde_json::Value::String(value) => LegacyValue::String(value),
        serde_json::Value::Array(values) => {
            LegacyValue::Array(values.into_iter().map(legacy_from_json).collect())
        }
        serde_json::Value::Object(values) => LegacyValue::Table(
            values
                .into_iter()
                .map(|(key, value)| (key, legacy_from_json(value)))
                .collect(),
        ),
    }
}

fn legacy_from_yaml(value: serde_yaml::Value) -> LegacyValue {
    match value {
        serde_yaml::Value::Null => LegacyValue::Null,
        serde_yaml::Value::Bool(value) => LegacyValue::Bool(value),
        serde_yaml::Value::Number(value) => {
            LegacyValue::Integer(value.as_i64().unwrap_or_default())
        }
        serde_yaml::Value::String(value) => LegacyValue::String(value),
        serde_yaml::Value::Sequence(values) => {
            LegacyValue::Array(values.into_iter().map(legacy_from_yaml).collect())
        }
        serde_yaml::Value::Mapping(values) => LegacyValue::Table(
            values
                .into_iter()
                .filter_map(|(key, value)| {
                    let key = match key {
                        serde_yaml::Value::String(value) => value,
                        other => legacy_from_yaml(other).as_str()?.to_owned(),
                    };
                    Some((key, legacy_from_yaml(value)))
                })
                .collect(),
        ),
        serde_yaml::Value::Tagged(tagged) => legacy_from_yaml(tagged.value),
    }
}

fn parse_python_legacy_config(path: &Path, source: &str) -> Result<LegacyValue> {
    let mut parser = PythonLegacyParser::new(path, source);
    parser.parse()
}

struct PythonLegacyParser<'a> {
    path: &'a Path,
    lines: Vec<&'a str>,
    index: usize,
}

impl<'a> PythonLegacyParser<'a> {
    fn new(path: &'a Path, source: &'a str) -> Self {
        Self {
            path,
            lines: source.lines().collect(),
            index: 0,
        }
    }

    fn parse(&mut self) -> Result<LegacyValue> {
        let mut root = BTreeMap::new();
        let mut current_section: Option<String> = None;

        while self.index < self.lines.len() {
            let raw_line = self.lines[self.index];
            let trimmed = raw_line.trim();
            self.index += 1;

            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if !raw_line.starts_with(' ') && !raw_line.starts_with('\t') {
                current_section = None;
            }

            if let Some(section_name) = parse_python_section_header(trimmed) {
                root.entry(section_name.clone())
                    .or_insert_with(|| LegacyValue::Table(BTreeMap::new()));
                current_section = Some(section_name);
                continue;
            }

            let Some((key, initial_expr)) = parse_python_assignment(trimmed) else {
                continue;
            };

            let expression = self.collect_python_expression(initial_expr);
            let value = parse_python_literal(self.path, &expression)?;
            if let Some(section_name) = &current_section {
                let section = root
                    .entry(section_name.clone())
                    .or_insert_with(|| LegacyValue::Table(BTreeMap::new()));
                let Some(table) = section.as_table_mut() else {
                    return Err(Error::Formatter(format!(
                        "{}: section {section_name:?} is not a table",
                        self.path.display()
                    )));
                };
                table.insert(key.to_owned(), value);
            } else {
                root.insert(key.to_owned(), value);
            }
        }

        Ok(LegacyValue::Table(root))
    }

    fn collect_python_expression(&mut self, initial_expr: &str) -> String {
        let mut expression = strip_python_comment(initial_expr).trim().to_owned();
        while !python_literal_complete(&expression) && self.index < self.lines.len() {
            let next = self.lines[self.index];
            self.index += 1;
            if !expression.is_empty() {
                expression.push('\n');
            }
            expression.push_str(strip_python_comment(next));
        }
        expression
    }
}

impl LegacyValue {
    fn as_table_mut(&mut self) -> Option<&mut BTreeMap<String, LegacyValue>> {
        match self {
            LegacyValue::Table(table) => Some(table),
            _ => None,
        }
    }
}

fn parse_python_section_header(line: &str) -> Option<String> {
    let rest = line.strip_prefix("with section(")?;
    let rest = rest.strip_suffix("):")?;
    parse_python_quoted_name(rest.trim())
}

fn parse_python_quoted_name(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    if bytes.len() < 2 {
        return None;
    }
    let quote = bytes[0];
    if quote != b'\'' && quote != b'"' {
        return None;
    }
    if *bytes.last()? != quote {
        return None;
    }
    Some(input[1..input.len() - 1].to_owned())
}

fn parse_python_assignment(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once('=')?;
    let key = key.trim();
    if key.is_empty()
        || !key
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    {
        return None;
    }
    Some((key, value.trim()))
}

fn strip_python_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut raw_string = false;
    let mut quote = '\0';
    let mut prev = '\0';

    for (index, ch) in line.char_indices() {
        if !in_string {
            if ch == '#' {
                return &line[..index];
            }

            if ch == '\'' || ch == '"' {
                in_string = true;
                quote = ch;
                raw_string = prev == 'r' || prev == 'R';
            }
        } else if ch == quote && (raw_string || prev != '\\') {
            in_string = false;
            raw_string = false;
            quote = '\0';
        }

        prev = ch;
    }

    line
}

fn python_literal_complete(input: &str) -> bool {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut raw_string = false;
    let mut quote = '\0';
    let mut prev = '\0';

    for ch in input.chars() {
        if !in_string {
            if ch == '\'' || ch == '"' {
                in_string = true;
                quote = ch;
                raw_string = prev == 'r' || prev == 'R';
            } else if matches!(ch, '[' | '{' | '(') {
                depth += 1;
            } else if matches!(ch, ']' | '}' | ')') {
                depth = depth.saturating_sub(1);
            }
        } else if ch == quote && (raw_string || prev != '\\') {
            in_string = false;
            raw_string = false;
            quote = '\0';
        }

        prev = ch;
    }

    !in_string && depth == 0
}

fn parse_python_literal(path: &Path, input: &str) -> Result<LegacyValue> {
    let mut parser = PythonLiteralParser {
        path,
        chars: input.chars().collect(),
        index: 0,
    };
    let value = parser.parse_value()?;
    parser.skip_ws();
    if parser.index != parser.chars.len() {
        return Err(Error::Formatter(format!(
            "{}: unsupported trailing content in python config literal: {}",
            path.display(),
            input.trim()
        )));
    }
    Ok(value)
}

struct PythonLiteralParser<'a> {
    path: &'a Path,
    chars: Vec<char>,
    index: usize,
}

impl PythonLiteralParser<'_> {
    fn parse_value(&mut self) -> Result<LegacyValue> {
        self.skip_ws();
        let Some(ch) = self.peek() else {
            return Err(self.error("unexpected end of literal"));
        };

        match ch {
            '{' => self.parse_dict(),
            '[' => self.parse_sequence(']'),
            '(' => self.parse_sequence(')'),
            '\'' | '"' => self.parse_string(false).map(LegacyValue::String),
            'r' | 'R' => {
                if matches!(self.peek_next(), Some('\'' | '"')) {
                    self.index += 1;
                    self.parse_string(true).map(LegacyValue::String)
                } else {
                    self.parse_identifier_or_number()
                }
            }
            '-' | '0'..='9' => self.parse_number().map(LegacyValue::Integer),
            _ => self.parse_identifier_or_number(),
        }
    }

    fn parse_dict(&mut self) -> Result<LegacyValue> {
        self.expect('{')?;
        let mut table = BTreeMap::new();
        loop {
            self.skip_ws();
            if self.consume_if('}') {
                break;
            }

            let key = if matches!(self.peek(), Some('\'' | '"' | 'r' | 'R')) {
                match self.parse_value()? {
                    LegacyValue::String(value) => value,
                    _ => return Err(self.error("dictionary key must be a string")),
                }
            } else {
                self.parse_identifier()
            };

            self.skip_ws();
            self.expect(':')?;
            let value = self.parse_value()?;
            table.insert(key, value);

            self.skip_ws();
            if self.consume_if(',') {
                continue;
            }
            self.expect('}')?;
            break;
        }
        Ok(LegacyValue::Table(table))
    }

    fn parse_sequence(&mut self, closing: char) -> Result<LegacyValue> {
        let opening = if closing == ']' { '[' } else { '(' };
        self.expect(opening)?;
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            if self.consume_if(closing) {
                break;
            }
            items.push(self.parse_value()?);
            self.skip_ws();
            if self.consume_if(',') {
                continue;
            }
            self.expect(closing)?;
            break;
        }
        Ok(LegacyValue::Array(items))
    }

    fn parse_string(&mut self, raw: bool) -> Result<String> {
        let quote = self
            .next()
            .ok_or_else(|| self.error("expected string quote"))?;
        let mut output = String::new();

        while let Some(ch) = self.next() {
            if ch == quote {
                return Ok(output);
            }

            if raw || ch != '\\' {
                output.push(ch);
                continue;
            }

            let escaped = self
                .next()
                .ok_or_else(|| self.error("unterminated escape sequence"))?;
            output.push(match escaped {
                '\\' => '\\',
                '\'' => '\'',
                '"' => '"',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            });
        }

        Err(self.error("unterminated string literal"))
    }

    fn parse_number(&mut self) -> Result<i64> {
        let start = self.index;
        self.consume_if('-');
        while matches!(self.peek(), Some('0'..='9')) {
            self.index += 1;
        }
        self.chars[start..self.index]
            .iter()
            .collect::<String>()
            .parse::<i64>()
            .map_err(|_| self.error("invalid integer literal"))
    }

    fn parse_identifier_or_number(&mut self) -> Result<LegacyValue> {
        let ident = self.parse_identifier();
        match ident.as_str() {
            "True" => Ok(LegacyValue::Bool(true)),
            "False" => Ok(LegacyValue::Bool(false)),
            "None" => Ok(LegacyValue::Null),
            _ => Err(self.error(format!("unsupported python literal or identifier: {ident}"))),
        }
    }

    fn parse_identifier(&mut self) -> String {
        let start = self.index;
        while matches!(self.peek(), Some(ch) if ch == '_' || ch.is_ascii_alphanumeric()) {
            self.index += 1;
        }
        self.chars[start..self.index].iter().collect()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(ch) if ch.is_whitespace()) {
            self.index += 1;
        }
    }

    fn expect(&mut self, expected: char) -> Result<()> {
        match self.next() {
            Some(ch) if ch == expected => Ok(()),
            _ => Err(self.error(format!("expected {expected:?}"))),
        }
    }

    fn consume_if(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.index + 1).copied()
    }

    fn next(&mut self) -> Option<char> {
        let value = self.peek()?;
        self.index += 1;
        Some(value)
    }

    fn error(&self, message: impl AsRef<str>) -> Error {
        Error::Formatter(format!("{}: {}", self.path.display(), message.as_ref()))
    }
}

#[derive(Default)]
struct ConvertedConfig {
    format: OutputFormatSection,
    markup: OutputMarkupSection,
    per_command_overrides: BTreeMap<String, OutputPerCommandConfig>,
    commands: BTreeMap<String, CommandSpecOverride>,
    notes: Vec<String>,
}

impl ConvertedConfig {
    fn as_config_file(&self) -> OutputConfigFile {
        OutputConfigFile {
            format: self.format.has_any().then_some(self.format.clone()),
            markup: self.markup.has_any().then_some(self.markup.clone()),
            per_command_overrides: (!self.per_command_overrides.is_empty())
                .then_some(self.per_command_overrides.clone()),
            commands: (!self.commands.is_empty()).then_some(self.commands.clone()),
        }
    }

    fn note_unsupported(&mut self, path: &Path, key: &str) {
        self.notes.push(format!(
            "{}: unsupported legacy option {key}",
            path.display()
        ));
    }

    fn note_warning(&mut self, path: &Path, message: impl Into<String>) {
        self.notes
            .push(format!("{}: {}", path.display(), message.into()));
    }
}

#[derive(Debug, Clone, Default, Serialize)]
struct OutputConfigFile {
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<OutputFormatSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    markup: Option<OutputMarkupSection>,
    #[serde(
        rename = "per_command_overrides",
        skip_serializing_if = "Option::is_none"
    )]
    per_command_overrides: Option<BTreeMap<String, OutputPerCommandConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    commands: Option<BTreeMap<String, CommandSpecOverride>>,
}

#[derive(Debug, Clone, Default, Serialize)]
struct OutputFormatSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    disable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line_ending: Option<LineEnding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line_width: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tab_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    use_tabs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fractional_tab_policy: Option<FractionalTabPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_empty_lines: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_hanging_wrap_lines: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_hanging_wrap_positional_args: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_hanging_wrap_groups: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_rows_cmdline: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    always_wrap: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    require_valid_layout: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dangle_parens: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dangle_align: Option<DangleAlign>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_prefix_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_prefix_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    space_before_control_paren: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    space_before_definition_paren: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    command_case: Option<CaseStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    keyword_case: Option<CaseStyle>,
}

impl OutputFormatSection {
    fn has_any(&self) -> bool {
        self.disable.is_some()
            || self.line_ending.is_some()
            || self.line_width.is_some()
            || self.tab_size.is_some()
            || self.use_tabs.is_some()
            || self.fractional_tab_policy.is_some()
            || self.max_empty_lines.is_some()
            || self.max_hanging_wrap_lines.is_some()
            || self.max_hanging_wrap_positional_args.is_some()
            || self.max_hanging_wrap_groups.is_some()
            || self.max_rows_cmdline.is_some()
            || self.always_wrap.is_some()
            || self.require_valid_layout.is_some()
            || self.dangle_parens.is_some()
            || self.dangle_align.is_some()
            || self.min_prefix_length.is_some()
            || self.max_prefix_length.is_some()
            || self.space_before_control_paren.is_some()
            || self.space_before_definition_paren.is_some()
            || self.command_case.is_some()
            || self.keyword_case.is_some()
    }
}

#[derive(Debug, Clone, Default, Serialize)]
struct OutputMarkupSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_markup: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reflow_comments: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    first_comment_is_literal: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    literal_comment_pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bullet_char: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enum_char: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fence_pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ruler_pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hashruler_min_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    canonicalize_hashrulers: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    explicit_trailing_pattern: Option<String>,
}

impl OutputMarkupSection {
    fn has_any(&self) -> bool {
        self.enable_markup.is_some()
            || self.reflow_comments.is_some()
            || self.first_comment_is_literal.is_some()
            || self.literal_comment_pattern.is_some()
            || self.bullet_char.is_some()
            || self.enum_char.is_some()
            || self.fence_pattern.is_some()
            || self.ruler_pattern.is_some()
            || self.hashruler_min_length.is_some()
            || self.canonicalize_hashrulers.is_some()
            || self.explicit_trailing_pattern.is_some()
    }
}

#[derive(Debug, Clone, Default, Serialize)]
struct OutputPerCommandConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    command_case: Option<CaseStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    keyword_case: Option<CaseStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line_width: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tab_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dangle_parens: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dangle_align: Option<DangleAlign>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "max_hanging_wrap_positional_args")]
    max_pargs_hwrap: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "max_hanging_wrap_groups")]
    max_subgroups_hwrap: Option<usize>,
}

impl From<PerCommandConfig> for OutputPerCommandConfig {
    fn from(value: PerCommandConfig) -> Self {
        Self {
            command_case: value.command_case,
            keyword_case: value.keyword_case,
            line_width: value.line_width,
            tab_size: value.tab_size,
            dangle_parens: value.dangle_parens,
            dangle_align: value.dangle_align,
            max_pargs_hwrap: value.max_pargs_hwrap,
            max_subgroups_hwrap: value.max_subgroups_hwrap,
        }
    }
}

fn merge_legacy_root(
    converted: &mut ConvertedConfig,
    root: &BTreeMap<String, LegacyValue>,
    path: &Path,
) {
    for (key, value) in root {
        match key.as_str() {
            "format" => merge_format_section(converted, path, value),
            "markup" => merge_markup_section(converted, path, value),
            "misc" => merge_misc_section(converted, path, value),
            "parse" => merge_parse_section(converted, path, value),
            unsupported => converted.note_unsupported(path, &format!("[{unsupported}]")),
        }
    }
}

fn merge_format_section(converted: &mut ConvertedConfig, path: &Path, value: &LegacyValue) {
    let Some(table) = value.as_table() else {
        converted.note_unsupported(path, "[format] (expected a table)");
        return;
    };

    for (key, value) in table {
        match key.as_str() {
            "disable" => converted.format.disable = as_bool(value),
            "line_ending" => converted.format.line_ending = as_line_ending(value),
            "line_width" => converted.format.line_width = as_usize(value),
            "tab_size" => converted.format.tab_size = as_usize(value),
            "use_tabchars" | "use_tabs" => converted.format.use_tabs = as_bool(value),
            "fractional_tab_policy" => {
                converted.format.fractional_tab_policy = as_fractional_tab_policy(value)
            }
            "max_pargs_hwrap" | "max_hanging_wrap_positional_args" => {
                converted.format.max_hanging_wrap_positional_args = as_usize(value)
            }
            "max_subgroups_hwrap" | "max_hanging_wrap_groups" => {
                converted.format.max_hanging_wrap_groups = as_usize(value)
            }
            "max_lines_hwrap" | "max_hanging_wrap_lines" => {
                converted.format.max_hanging_wrap_lines = as_usize(value)
            }
            "max_empty_lines" => converted.format.max_empty_lines = as_usize(value),
            "max_rows_cmdline" => converted.format.max_rows_cmdline = as_usize(value),
            "always_wrap" => converted.format.always_wrap = as_string_list(value),
            "require_valid_layout" => converted.format.require_valid_layout = as_bool(value),
            "dangle_parens" => converted.format.dangle_parens = as_bool(value),
            "dangle_align" => converted.format.dangle_align = as_dangle_align(value),
            "min_prefix_chars" | "min_prefix_length" => {
                converted.format.min_prefix_length = as_usize(value)
            }
            "max_prefix_chars" | "max_prefix_length" => {
                converted.format.max_prefix_length = as_usize(value)
            }
            "separate_ctrl_name_with_space" | "space_before_control_paren" => {
                converted.format.space_before_control_paren = as_bool(value)
            }
            "separate_fn_name_with_space" | "space_before_definition_paren" => {
                converted.format.space_before_definition_paren = as_bool(value)
            }
            "command_case" => {
                converted.format.command_case = convert_case_style(path, converted, key, value)
            }
            "keyword_case" => {
                converted.format.keyword_case = convert_case_style(path, converted, key, value)
            }
            unsupported => converted.note_unsupported(path, &format!("[format].{unsupported}")),
        }
    }
}

fn merge_markup_section(converted: &mut ConvertedConfig, path: &Path, value: &LegacyValue) {
    let Some(table) = value.as_table() else {
        converted.note_unsupported(path, "[markup] (expected a table)");
        return;
    };

    for (key, value) in table {
        match key.as_str() {
            "enable_markup" => converted.markup.enable_markup = as_bool(value),
            "reflow_comments" => converted.markup.reflow_comments = as_bool(value),
            "first_comment_is_literal" => {
                converted.markup.first_comment_is_literal = as_bool(value)
            }
            "literal_comment_pattern" => {
                converted.markup.literal_comment_pattern = as_string(value)
            }
            "bullet_char" => converted.markup.bullet_char = as_string(value),
            "enum_char" => converted.markup.enum_char = as_string(value),
            "fence_pattern" => converted.markup.fence_pattern = as_string(value),
            "ruler_pattern" => converted.markup.ruler_pattern = as_string(value),
            "hashruler_min_length" => converted.markup.hashruler_min_length = as_usize(value),
            "canonicalize_hashrulers" => converted.markup.canonicalize_hashrulers = as_bool(value),
            "explicit_trailing_pattern" => {
                converted.markup.explicit_trailing_pattern = as_string(value)
            }
            unsupported => converted.note_unsupported(path, &format!("[markup].{unsupported}")),
        }
    }
}

fn merge_misc_section(converted: &mut ConvertedConfig, path: &Path, value: &LegacyValue) {
    let Some(table) = value.as_table() else {
        converted.note_unsupported(path, "[misc] (expected a table)");
        return;
    };

    for (key, value) in table {
        match key.as_str() {
            "per_command" => merge_per_command_section(converted, path, value),
            unsupported => converted.note_unsupported(path, &format!("[misc].{unsupported}")),
        }
    }
}

fn merge_parse_section(converted: &mut ConvertedConfig, path: &Path, value: &LegacyValue) {
    let Some(table) = value.as_table() else {
        converted.note_unsupported(path, "[parse] (expected a table)");
        return;
    };

    for (key, value) in table {
        match key.as_str() {
            "additional_commands" | "override_spec" => {
                merge_command_specs(converted, path, value, key)
            }
            unsupported => converted.note_unsupported(path, &format!("[parse].{unsupported}")),
        }
    }
}

fn merge_per_command_section(converted: &mut ConvertedConfig, path: &Path, value: &LegacyValue) {
    let Some(table) = value.as_table() else {
        converted.note_unsupported(path, "[misc].per_command (expected a table)");
        return;
    };

    for (command_name, overrides) in table {
        let Some(overrides) = overrides.as_table() else {
            converted.note_unsupported(
                path,
                &format!("[misc].per_command.{command_name} (expected a table)"),
            );
            continue;
        };

        let mut config = PerCommandConfig::default();
        for (key, value) in overrides {
            match key.as_str() {
                "command_case" => {
                    config.command_case = convert_case_style(path, converted, key, value)
                }
                "keyword_case" => {
                    config.keyword_case = convert_case_style(path, converted, key, value)
                }
                "line_width" => config.line_width = as_usize(value),
                "tab_size" => config.tab_size = as_usize(value),
                "dangle_parens" => config.dangle_parens = as_bool(value),
                "dangle_align" => config.dangle_align = as_dangle_align(value),
                "max_pargs_hwrap" | "max_hanging_wrap_positional_args" => {
                    config.max_pargs_hwrap = as_usize(value)
                }
                "max_subgroups_hwrap" | "max_hanging_wrap_groups" => {
                    config.max_subgroups_hwrap = as_usize(value)
                }
                unsupported => converted.note_unsupported(
                    path,
                    &format!("[misc].per_command.{command_name}.{unsupported}"),
                ),
            }
        }

        converted
            .per_command_overrides
            .insert(command_name.to_ascii_lowercase(), config.into());
    }
}

fn merge_command_specs(
    converted: &mut ConvertedConfig,
    path: &Path,
    value: &LegacyValue,
    origin_key: &str,
) {
    let Some(table) = value.as_table() else {
        converted.note_unsupported(path, &format!("[parse].{origin_key} (expected a table)"));
        return;
    };

    for (command_name, spec_value) in table {
        match convert_command_spec(spec_value) {
            Some(spec) => {
                converted
                    .commands
                    .insert(command_name.to_ascii_lowercase(), spec);
            }
            None => converted.note_warning(
                path,
                format!(
                    "could not fully convert [parse].{origin_key}.{command_name}; review this command manually"
                ),
            ),
        }
    }
}

fn convert_case_style(
    path: &Path,
    converted: &mut ConvertedConfig,
    key: &str,
    value: &LegacyValue,
) -> Option<CaseStyle> {
    let style = match as_string(value)?.to_ascii_lowercase().as_str() {
        "lower" => Some(CaseStyle::Lower),
        "upper" => Some(CaseStyle::Upper),
        "unchanged" => Some(CaseStyle::Unchanged),
        "canonical" => {
            converted.note_warning(
                path,
                format!("{key} = \"canonical\" was mapped to \"lower\"; review command casing"),
            );
            Some(CaseStyle::Lower)
        }
        _ => None,
    };
    style
}

fn convert_command_spec(value: &LegacyValue) -> Option<CommandSpecOverride> {
    let table = value.as_table()?;

    if table.contains_key("forms") || table.contains_key("fallback") {
        let forms = table
            .get("forms")
            .and_then(LegacyValue::as_table)
            .map(|forms| {
                forms
                    .iter()
                    .filter_map(|(name, value)| {
                        convert_command_form(value).map(|form| (name.clone(), form))
                    })
                    .collect::<BTreeMap<_, _>>()
            })
            .unwrap_or_default()
            .into_iter()
            .collect();

        let fallback = table.get("fallback").and_then(convert_command_form);
        return Some(CommandSpecOverride::Discriminated { forms, fallback });
    }

    convert_command_form(value).map(CommandSpecOverride::Single)
}

fn convert_command_form(value: &LegacyValue) -> Option<CommandFormOverride> {
    let table = value.as_table()?;
    let mut form = CommandFormOverride::default();

    for (key, value) in table {
        match key.as_str() {
            "pargs" => form.pargs = convert_nargs(value),
            "flags" => form.flags = convert_flag_set(value),
            "kwargs" => {
                form.kwargs = value
                    .as_table()?
                    .iter()
                    .filter_map(|(name, value)| {
                        convert_kwarg_spec(value).map(|spec| (name.to_ascii_uppercase(), spec))
                    })
                    .collect();
            }
            "layout" => form.layout = convert_layout_overrides(value),
            _ => {}
        }
    }

    Some(form)
}

fn convert_kwarg_spec(value: &LegacyValue) -> Option<KwargSpecOverride> {
    match value {
        LegacyValue::Integer(_) | LegacyValue::String(_) | LegacyValue::Array(_) => {
            Some(KwargSpecOverride {
                nargs: convert_nargs(value),
                ..KwargSpecOverride::default()
            })
        }
        LegacyValue::Table(table) => {
            let mut spec = KwargSpecOverride::default();
            for (key, value) in table {
                match key.as_str() {
                    "nargs" => spec.nargs = convert_nargs(value),
                    "flags" => spec.flags = convert_flag_set(value),
                    "kwargs" => {
                        spec.kwargs = value
                            .as_table()?
                            .iter()
                            .filter_map(|(name, value)| {
                                convert_kwarg_spec(value)
                                    .map(|nested| (name.to_ascii_uppercase(), nested))
                            })
                            .collect();
                    }
                    _ => {}
                }
            }
            Some(spec)
        }
        _ => None,
    }
}

fn convert_layout_overrides(value: &LegacyValue) -> Option<LayoutOverridesOverride> {
    let table = value.as_table()?;
    let mut layout = LayoutOverridesOverride::default();
    for (key, value) in table {
        match key.as_str() {
            "line_width" => layout.line_width = as_usize(value),
            "tab_size" => layout.tab_size = as_usize(value),
            "dangle_parens" => layout.dangle_parens = as_bool(value),
            "always_wrap" => layout.always_wrap = as_bool(value),
            "max_pargs_hwrap" => layout.max_pargs_hwrap = as_usize(value),
            _ => {}
        }
    }
    Some(layout)
}

fn convert_flag_set(value: &LegacyValue) -> indexmap::IndexSet<String> {
    match value {
        LegacyValue::Array(items) => items
            .iter()
            .filter_map(as_string)
            .map(|flag| flag.to_ascii_uppercase())
            .collect(),
        _ => indexmap::IndexSet::new(),
    }
}

fn convert_nargs(value: &LegacyValue) -> Option<NArgs> {
    match value {
        LegacyValue::Integer(value) if *value >= 0 => Some(NArgs::Fixed(*value as usize)),
        LegacyValue::String(value) => parse_nargs_string(value),
        LegacyValue::Array(items) if items.len() == 1 => convert_nargs(&items[0]),
        _ => None,
    }
}

fn parse_nargs_string(value: &str) -> Option<NArgs> {
    match value {
        "*" => Some(NArgs::ZeroOrMore),
        "+" => Some(NArgs::OneOrMore),
        "?" => Some(NArgs::Optional),
        _ if value.ends_with('+') => value[..value.len() - 1]
            .parse::<usize>()
            .ok()
            .map(NArgs::AtLeast),
        _ => value.parse::<usize>().ok().map(NArgs::Fixed),
    }
}

fn as_usize(value: &LegacyValue) -> Option<usize> {
    match value {
        LegacyValue::Integer(value) if *value >= 0 => Some(*value as usize),
        _ => None,
    }
}

fn as_bool(value: &LegacyValue) -> Option<bool> {
    match value {
        LegacyValue::Bool(value) => Some(*value),
        _ => None,
    }
}

fn as_string(value: &LegacyValue) -> Option<String> {
    match value {
        LegacyValue::String(value) => Some(value.clone()),
        _ => None,
    }
}

fn as_line_ending(value: &LegacyValue) -> Option<LineEnding> {
    match value.as_str()?.to_ascii_lowercase().as_str() {
        "unix" => Some(LineEnding::Unix),
        "windows" => Some(LineEnding::Windows),
        "auto" => Some(LineEnding::Auto),
        _ => None,
    }
}

fn as_fractional_tab_policy(value: &LegacyValue) -> Option<FractionalTabPolicy> {
    match value.as_str()?.to_ascii_lowercase().as_str() {
        "use-space" => Some(FractionalTabPolicy::UseSpace),
        "round-up" => Some(FractionalTabPolicy::RoundUp),
        _ => None,
    }
}

fn as_string_list(value: &LegacyValue) -> Option<Vec<String>> {
    match value {
        LegacyValue::Array(items) => Some(
            items
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_ascii_lowercase()))
                .collect(),
        ),
        _ => None,
    }
}

fn as_dangle_align(value: &LegacyValue) -> Option<DangleAlign> {
    match value.as_str()?.to_ascii_lowercase().as_str() {
        "prefix" => Some(DangleAlign::Prefix),
        "open" => Some(DangleAlign::Open),
        "close" => Some(DangleAlign::Close),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn convert_legacy_requires_input_paths() {
        let err = convert_legacy_config_files(&[], DumpConfigFormat::Yaml).unwrap_err();
        assert!(err
            .to_string()
            .contains("cmakefmt config convert requires at least one input path"));
    }

    #[test]
    fn unsupported_legacy_extension_returns_error() {
        let err = detect_legacy_format(Path::new("legacy.txt")).unwrap_err();
        assert!(err.to_string().contains("unsupported legacy config format"));
    }

    #[test]
    fn python_helpers_handle_comments_and_assignments() {
        assert_eq!(
            strip_python_comment(r#"value = "http://example.com#frag"  # comment"#).trim_end(),
            r#"value = "http://example.com#frag""#
        );
        assert_eq!(
            parse_python_assignment("line_width = 100"),
            Some(("line_width", "100"))
        );
        assert_eq!(parse_python_assignment("not valid key = 1"), None);
        assert_eq!(
            parse_python_section_header(r#"with section("format"):"#),
            Some("format".to_owned())
        );
    }

    #[test]
    fn python_literal_parser_handles_core_types() {
        let path = Path::new("legacy.py");
        assert_eq!(
            parse_python_literal(path, "{'a': [1, 'x'], 'b': True, 'c': None}").unwrap(),
            LegacyValue::Table(BTreeMap::from([
                (
                    "a".to_owned(),
                    LegacyValue::Array(vec![
                        LegacyValue::Integer(1),
                        LegacyValue::String("x".to_owned()),
                    ]),
                ),
                ("b".to_owned(), LegacyValue::Bool(true)),
                ("c".to_owned(), LegacyValue::Null),
            ]))
        );
        assert_eq!(
            parse_python_literal(path, r#"r"c:\tmp\file""#).unwrap(),
            LegacyValue::String(r#"c:\tmp\file"#.to_owned())
        );
    }

    #[test]
    fn parse_nargs_string_supports_all_supported_forms() {
        assert_eq!(parse_nargs_string("*"), Some(NArgs::ZeroOrMore));
        assert_eq!(parse_nargs_string("+"), Some(NArgs::OneOrMore));
        assert_eq!(parse_nargs_string("?"), Some(NArgs::Optional));
        assert_eq!(parse_nargs_string("2+"), Some(NArgs::AtLeast(2)));
        assert_eq!(parse_nargs_string("3"), Some(NArgs::Fixed(3)));
        assert_eq!(parse_nargs_string("bogus"), None);
    }

    #[test]
    fn converts_canonical_case_and_records_note() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(
            &path,
            r#"
format:
  command_case: canonical
"#,
        )
        .unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("command_case = \"lower\""));
        assert!(converted.contains("mapped to \"lower\""));
    }

    #[test]
    fn unknown_options_are_reported_in_conversion_notes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.json");
        std::fs::write(
            &path,
            r#"{
  "format": {"line_width": 90, "unknown_key": true},
  "misc": {"nope": 1}
}"#,
        )
        .unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("unsupported legacy option [format].unknown_key"));
        assert!(converted.contains("unsupported legacy option [misc].nope"));
    }

    #[test]
    fn converts_legacy_json_config() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.json");
        std::fs::write(
            &path,
            r#"{
  "format": {
    "line_width": 100,
    "tab_size": 4,
    "command_case": "lower",
    "keyword_case": "upper"
  },
  "misc": {
    "per_command": {
      "message": {
        "line_width": 120
      }
    }
  }
}"#,
        )
        .unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("[format]"));
        assert!(converted.contains("line_width = 100"));
        assert!(converted.contains("command_case = \"lower\""));
        assert!(converted.contains("[per_command_overrides.message]"));
    }

    #[test]
    fn converts_legacy_python_custom_command_spec() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.py");
        std::fs::write(
            &path,
            r#"
with section("parse"):
  additional_commands = {
    "my_command": {
      "pargs": 1,
      "flags": ["QUIET"],
      "kwargs": {
        "SOURCES": "*",
        "LIBRARIES": "+"
      }
    }
  }
"#,
        )
        .unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("[commands.my_command]"));
        assert!(converted.contains("pargs = 1"));
        assert!(converted.contains("flags = [\"QUIET\"]"));
    }

    #[test]
    fn converts_legacy_yaml_config() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(
            &path,
            r#"
markup:
  enable_markup: true
  reflow_comments: true
"#,
        )
        .unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("[markup]"));
        assert!(converted.contains("reflow_comments = true"));
    }

    // ── Phase-16 option tests ─────────────────────────────────────────────

    #[test]
    fn converts_format_disable() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(&path, "format:\n  disable: true\n").unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("disable = true"));
    }

    #[test]
    fn converts_format_line_ending_unix() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(&path, "format:\n  line_ending: unix\n").unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("line_ending = \"unix\""));
    }

    #[test]
    fn converts_format_line_ending_windows() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(&path, "format:\n  line_ending: windows\n").unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("line_ending = \"windows\""));
    }

    #[test]
    fn converts_format_always_wrap() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(
            &path,
            "format:\n  always_wrap:\n    - target_link_libraries\n    - target_sources\n",
        )
        .unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("always_wrap"));
        assert!(converted.contains("target_link_libraries"));
        assert!(converted.contains("target_sources"));
    }

    #[test]
    fn converts_format_require_valid_layout() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(&path, "format:\n  require_valid_layout: true\n").unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("require_valid_layout = true"));
    }

    #[test]
    fn converts_format_fractional_tab_policy_use_space() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(&path, "format:\n  fractional_tab_policy: use-space\n").unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("fractional_tab_policy = \"use-space\""));
    }

    #[test]
    fn converts_format_fractional_tab_policy_round_up() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(&path, "format:\n  fractional_tab_policy: round-up\n").unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("fractional_tab_policy = \"round-up\""));
    }

    #[test]
    fn converts_format_max_rows_cmdline() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(&path, "format:\n  max_rows_cmdline: 3\n").unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("max_rows_cmdline = 3"));
    }

    #[test]
    fn converts_markup_explicit_trailing_pattern() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(&path, "markup:\n  explicit_trailing_pattern: '#<'\n").unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("explicit_trailing_pattern"));
        assert!(converted.contains("#<"));
    }

    // ── as_line_ending: auto variant ─────────────────────────────────────────

    #[test]
    fn converts_format_line_ending_auto() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(&path, "format:\n  line_ending: auto\n").unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("line_ending = \"auto\""));
    }

    #[test]
    fn as_line_ending_unknown_returns_none() {
        assert_eq!(
            as_line_ending(&LegacyValue::String("bogus".to_owned())),
            None
        );
        // Non-string input also returns None
        assert_eq!(as_line_ending(&LegacyValue::Integer(1)), None);
    }

    // ── as_fractional_tab_policy: non-string input ───────────────────────────

    #[test]
    fn as_fractional_tab_policy_non_string_returns_none() {
        assert_eq!(as_fractional_tab_policy(&LegacyValue::Integer(42)), None);
        assert_eq!(as_fractional_tab_policy(&LegacyValue::Bool(true)), None);
        assert_eq!(
            as_fractional_tab_policy(&LegacyValue::String("bogus".to_owned())),
            None
        );
    }

    // ── as_dangle_align variants ─────────────────────────────────────────────

    #[test]
    fn converts_dangle_align_prefix() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(&path, "format:\n  dangle_align: prefix\n").unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("dangle_align = \"prefix\""));
    }

    #[test]
    fn converts_dangle_align_open() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(&path, "format:\n  dangle_align: open\n").unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("dangle_align = \"open\""));
    }

    #[test]
    fn converts_dangle_align_close() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(&path, "format:\n  dangle_align: close\n").unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("dangle_align = \"close\""));
    }

    #[test]
    fn as_dangle_align_unknown_returns_none() {
        assert_eq!(
            as_dangle_align(&LegacyValue::String("unknown_align".to_owned())),
            None
        );
    }

    // ── as_string_list: non-array input ──────────────────────────────────────

    #[test]
    fn as_string_list_non_array_returns_none() {
        assert_eq!(as_string_list(&LegacyValue::String("foo".to_owned())), None);
        assert_eq!(as_string_list(&LegacyValue::Integer(1)), None);
        assert_eq!(as_string_list(&LegacyValue::Bool(false)), None);
    }

    // ── convert_case_style via format section ────────────────────────────────

    #[test]
    fn converts_case_style_lower() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(&path, "format:\n  command_case: lower\n").unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("command_case = \"lower\""));
    }

    #[test]
    fn converts_case_style_upper() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(&path, "format:\n  keyword_case: upper\n").unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("keyword_case = \"upper\""));
    }

    #[test]
    fn converts_case_style_unchanged() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(&path, "format:\n  command_case: unchanged\n").unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("command_case = \"unchanged\""));
    }

    #[test]
    fn converts_unknown_case_style_records_no_value() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(&path, "format:\n  command_case: totally_bogus\n").unwrap();

        // An unknown case style should not emit command_case in the output
        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(!converted.contains("command_case"));
    }

    // ── Merging error paths: non-table section values ────────────────────────

    #[test]
    fn format_section_non_table_records_note() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.json");
        std::fs::write(&path, r#"{"format": "oops"}"#).unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("[format] (expected a table)"));
    }

    #[test]
    fn markup_section_non_table_records_note() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.json");
        std::fs::write(&path, r#"{"markup": 42}"#).unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("[markup] (expected a table)"));
    }

    #[test]
    fn misc_section_non_table_records_note() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.json");
        std::fs::write(&path, r#"{"misc": true}"#).unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("[misc] (expected a table)"));
    }

    #[test]
    fn parse_section_non_table_records_note() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.json");
        std::fs::write(&path, r#"{"parse": "nope"}"#).unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("[parse] (expected a table)"));
    }

    #[test]
    fn misc_per_command_non_table_records_note() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.json");
        std::fs::write(&path, r#"{"misc": {"per_command": "not_a_table"}}"#).unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("[misc].per_command (expected a table)"));
    }

    #[test]
    fn misc_per_command_entry_non_table_records_note() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.json");
        std::fs::write(&path, r#"{"misc": {"per_command": {"my_cmd": "oops"}}}"#).unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("[misc].per_command.my_cmd (expected a table)"));
    }

    // ── convert_command_spec: forms key path ─────────────────────────────────

    #[test]
    fn convert_command_spec_with_forms_table() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(
            &path,
            r#"
parse:
  additional_commands:
    my_target:
      forms:
        default:
          pargs: 1
          flags:
            - QUIET
        extra:
          pargs: 2
      fallback:
        pargs: 0
"#,
        )
        .unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        // Discriminated commands serialize with forms sub-tables
        assert!(converted.contains("my_target"));
        assert!(converted.contains("forms"));
    }

    #[test]
    fn convert_command_spec_non_table_records_warning() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(
            &path,
            r#"
parse:
  additional_commands:
    bad_cmd: "not_a_table"
"#,
        )
        .unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("bad_cmd"));
    }

    // ── convert_layout_overrides: non-table returns None ─────────────────────

    #[test]
    fn convert_layout_overrides_non_table_returns_none() {
        let result = convert_layout_overrides(&LegacyValue::Bool(true));
        assert!(result.is_none());

        let result = convert_layout_overrides(&LegacyValue::String("oops".to_owned()));
        assert!(result.is_none());
    }

    // ── Unsupported keys in markup, misc, parse ───────────────────────────────

    #[test]
    fn markup_unknown_key_records_note() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(&path, "markup:\n  unknown_markup_key: true\n").unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("[markup].unknown_markup_key"));
    }

    #[test]
    fn misc_unknown_key_records_note() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.json");
        std::fs::write(&path, r#"{"misc": {"unknown_misc_key": 1}}"#).unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("[misc].unknown_misc_key"));
    }

    #[test]
    fn parse_unknown_key_records_note() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.json");
        std::fs::write(&path, r#"{"parse": {"unknown_parse_key": 1}}"#).unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("[parse].unknown_parse_key"));
    }

    // ── Python config edge cases ──────────────────────────────────────────────

    #[test]
    fn python_section_non_table_value_is_overwritten() {
        // In the Python parser, a section is always created as a Table, so
        // subsequent assignment will insert into that table. This test verifies
        // a simple format section in Python is parsed and converted correctly.
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.py");
        std::fs::write(
            &path,
            r#"
with section("format"):
  line_width = 88
"#,
        )
        .unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("line_width = 88"));
    }

    // ── Legacy YAML per_command with line_width override ─────────────────────

    #[test]
    fn converts_per_command_line_width_override() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(
            &path,
            r#"
misc:
  per_command:
    target_sources:
      line_width: 120
"#,
        )
        .unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("[per_command_overrides.target_sources]"));
        assert!(converted.contains("line_width = 120"));
    }

    // ── Additional format options ─────────────────────────────────────────────

    #[test]
    fn converts_min_prefix_chars() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(&path, "format:\n  min_prefix_chars: 4\n").unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("min_prefix_length = 4"));
    }

    #[test]
    fn converts_max_prefix_chars() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(&path, "format:\n  max_prefix_chars: 10\n").unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("max_prefix_length = 10"));
    }

    #[test]
    fn converts_separate_ctrl_name_with_space() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(&path, "format:\n  separate_ctrl_name_with_space: true\n").unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("space_before_control_paren = true"));
    }

    #[test]
    fn converts_separate_fn_name_with_space() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.yaml");
        std::fs::write(&path, "format:\n  separate_fn_name_with_space: false\n").unwrap();

        let converted = convert_legacy_config_files(&[path], DumpConfigFormat::Toml).unwrap();
        assert!(converted.contains("space_before_definition_paren = false"));
    }
}
