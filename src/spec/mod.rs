// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Command-spec data model used by the formatter.
//!
//! The built-in registry describes the argument structure of known commands so
//! the formatter can recognize positional arguments, keywords, flags, and
//! command-specific layout hints.

pub mod registry;

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

// ── NArgs ────────────────────────────────────────────────────────────────────

/// How many arguments a positional slot or keyword takes.
///
/// In TOML this can be written as:
///   - integer   `nargs = 1`       → `Fixed(1)`
///   - `"*"`                      → `ZeroOrMore`
///   - `"+"`                      → `OneOrMore`
///   - `"?"`                      → `Optional`
///   - `"N+"` e.g. `"2+"`         → `AtLeast(2)`
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NArgs {
    Fixed(usize),
    #[default]
    ZeroOrMore,
    OneOrMore,
    Optional,
    AtLeast(usize),
}

impl Serialize for NArgs {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            NArgs::Fixed(value) => serializer.serialize_u64(*value as u64),
            NArgs::ZeroOrMore => serializer.serialize_str("*"),
            NArgs::OneOrMore => serializer.serialize_str("+"),
            NArgs::Optional => serializer.serialize_str("?"),
            NArgs::AtLeast(value) => serializer.serialize_str(&format!("{value}+")),
        }
    }
}

impl<'de> Deserialize<'de> for NArgs {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = NArgs;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, r#"integer or string ("*", "+", "?", "N+")"#)
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<NArgs, E> {
                Ok(NArgs::Fixed(v as usize))
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<NArgs, E> {
                Ok(NArgs::Fixed(v.max(0) as usize))
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<NArgs, E> {
                match v {
                    "*" => Ok(NArgs::ZeroOrMore),
                    "+" => Ok(NArgs::OneOrMore),
                    "?" => Ok(NArgs::Optional),
                    s if s.ends_with('+') && s.len() > 1 => {
                        let n = s[..s.len() - 1]
                            .parse::<usize>()
                            .map_err(|_| E::custom(format!("invalid NArgs pattern: {s}")))?;
                        Ok(NArgs::AtLeast(n))
                    }
                    s => {
                        let n = s
                            .parse::<usize>()
                            .map_err(|_| E::custom(format!("invalid NArgs value: {s}")))?;
                        Ok(NArgs::Fixed(n))
                    }
                }
            }
        }

        d.deserialize_any(Visitor)
    }
}

// ── Fully specified command model ────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutOverrides {
    /// Override line width for this command form.
    pub line_width: Option<usize>,
    /// Override indentation width for this command form.
    pub tab_size: Option<usize>,
    /// Override dangling-paren behavior for this command form.
    pub dangle_parens: Option<bool>,
    /// Force this command form into a wrapped layout.
    pub always_wrap: Option<bool>,
    /// Override the positional-argument hanging-wrap threshold for this form.
    pub max_pargs_hwrap: Option<usize>,
}

/// Specification for a keyword section and any nested sub-keywords it accepts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KwargSpec {
    /// Number of positional arguments accepted after the keyword itself.
    #[serde(default)]
    pub nargs: NArgs,
    /// Nested keywords that may appear after this keyword.
    #[serde(default)]
    pub kwargs: IndexMap<String, KwargSpec>,
    /// Flag tokens accepted within this keyword section.
    #[serde(default)]
    pub flags: IndexSet<String>,
}

/// One fully resolved command form.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandForm {
    /// Number of positional arguments before keyword/flag processing starts.
    #[serde(default)]
    pub pargs: NArgs,
    /// Recognized top-level keywords for this form.
    #[serde(default)]
    pub kwargs: IndexMap<String, KwargSpec>,
    /// Recognized top-level flags for this form.
    #[serde(default)]
    pub flags: IndexSet<String>,
    /// Optional layout hints for this form.
    #[serde(default)]
    pub layout: Option<LayoutOverrides>,
}

impl Default for CommandForm {
    fn default() -> Self {
        Self {
            pargs: NArgs::ZeroOrMore,
            kwargs: IndexMap::new(),
            flags: IndexSet::new(),
            layout: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum CommandSpec {
    /// A command whose structure depends on a discriminator token, usually the
    /// first positional argument.
    Discriminated {
        /// Known forms keyed by normalized discriminator token.
        forms: IndexMap<String, CommandForm>,
        /// Fallback form to use when no discriminator matches.
        #[serde(default)]
        fallback: Option<CommandForm>,
    },
    /// A command with a single argument structure.
    Single(CommandForm),
}

impl CommandSpec {
    /// Resolve the command form for a specific invocation.
    ///
    /// `first_arg` is typically the first non-comment argument in the call and
    /// is used for discriminated commands such as `file(...)` or `install(...)`.
    pub fn form_for(&self, first_arg: Option<&str>) -> &CommandForm {
        match self {
            CommandSpec::Single(form) => form,
            CommandSpec::Discriminated { forms, fallback } => {
                let key = first_arg.unwrap_or_default();
                forms
                    .get(key)
                    .or_else(|| {
                        has_ascii_lowercase(key)
                            .then(|| key.to_ascii_uppercase())
                            .and_then(|normalized| forms.get(&normalized))
                    })
                    .or(fallback.as_ref())
                    .unwrap_or_else(|| {
                        forms
                            .values()
                            .next()
                            .expect("discriminated spec has a form")
                    })
            }
        }
    }
}

fn has_ascii_lowercase(s: &str) -> bool {
    s.bytes().any(|byte| byte.is_ascii_lowercase())
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct SpecMetadata {
    /// Upstream CMake version the built-in spec was last audited against.
    #[serde(default)]
    pub cmake_version: String,
    /// Date of the most recent audit.
    #[serde(default)]
    pub audited_at: String,
    /// Free-form notes about the current audit state.
    #[serde(default)]
    pub notes: String,
}

/// Top-level spec file containing metadata plus command entries.
#[derive(Debug, Default, Deserialize)]
pub struct SpecFile {
    /// Version and audit metadata for the built-in spec surface.
    #[serde(default)]
    pub metadata: SpecMetadata,
    /// Built-in command specifications keyed by command name.
    #[serde(default)]
    pub commands: IndexMap<String, CommandSpec>,
}

// ── Mergeable override model ─────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutOverridesOverride {
    /// Override line width for this command form.
    pub line_width: Option<usize>,
    /// Override indentation width for this command form.
    pub tab_size: Option<usize>,
    /// Override dangling-paren behavior for this command form.
    pub dangle_parens: Option<bool>,
    /// Force this command form into a wrapped layout.
    pub always_wrap: Option<bool>,
    /// Override the positional-argument hanging-wrap threshold for this form.
    pub max_pargs_hwrap: Option<usize>,
}

/// Partial override for a keyword specification.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KwargSpecOverride {
    /// Override the number of positional arguments accepted after the keyword.
    pub nargs: Option<NArgs>,
    /// Nested keyword overrides.
    #[serde(default)]
    pub kwargs: IndexMap<String, KwargSpecOverride>,
    /// Additional supported flags.
    #[serde(default)]
    pub flags: IndexSet<String>,
}

/// Partial override for a command form.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommandFormOverride {
    /// Override the positional argument count for the form.
    pub pargs: Option<NArgs>,
    /// Keyword overrides to merge into the form.
    #[serde(default)]
    pub kwargs: IndexMap<String, KwargSpecOverride>,
    /// Additional supported flags.
    #[serde(default)]
    pub flags: IndexSet<String>,
    /// Optional layout overrides for the form.
    pub layout: Option<LayoutOverridesOverride>,
}

/// Partial override for a full command spec.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CommandSpecOverride {
    /// Override a single-form command.
    Single(CommandFormOverride),
    /// Override one or more discriminated forms.
    Discriminated {
        /// Per-discriminator form overrides.
        #[serde(default)]
        forms: IndexMap<String, CommandFormOverride>,
        /// Optional fallback form override.
        #[serde(default)]
        fallback: Option<CommandFormOverride>,
    },
}

/// Top-level user override file containing command overrides only.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct SpecOverrideFile {
    /// Override specs keyed by command name.
    #[serde(default)]
    pub commands: IndexMap<String, CommandSpecOverride>,
}

impl CommandSpecOverride {
    /// Convert a partial override into a fully specified standalone command
    /// spec.
    pub fn into_full_spec(self) -> CommandSpec {
        match self {
            CommandSpecOverride::Single(form) => CommandSpec::Single(form.into_full_form()),
            CommandSpecOverride::Discriminated { forms, fallback } => CommandSpec::Discriminated {
                forms: forms
                    .into_iter()
                    .map(|(name, form)| (name.to_ascii_uppercase(), form.into_full_form()))
                    .collect(),
                fallback: fallback.map(CommandFormOverride::into_full_form),
            },
        }
    }
}

impl CommandFormOverride {
    /// Convert a partial command form override into a fully specified form.
    pub fn into_full_form(self) -> CommandForm {
        CommandForm {
            pargs: self.pargs.unwrap_or_default(),
            kwargs: self
                .kwargs
                .into_iter()
                .map(|(name, spec)| (name.to_ascii_uppercase(), spec.into_full_spec()))
                .collect(),
            flags: self
                .flags
                .into_iter()
                .map(|flag| flag.to_ascii_uppercase())
                .collect(),
            layout: self.layout.map(LayoutOverridesOverride::into_full_layout),
        }
    }
}

impl KwargSpecOverride {
    /// Convert a partial keyword override into a fully specified keyword spec.
    pub fn into_full_spec(self) -> KwargSpec {
        KwargSpec {
            nargs: self.nargs.unwrap_or_default(),
            kwargs: self
                .kwargs
                .into_iter()
                .map(|(name, spec)| (name.to_ascii_uppercase(), spec.into_full_spec()))
                .collect(),
            flags: self
                .flags
                .into_iter()
                .map(|flag| flag.to_ascii_uppercase())
                .collect(),
        }
    }
}

impl LayoutOverridesOverride {
    /// Convert a partial layout override into a fully specified layout block.
    pub fn into_full_layout(self) -> LayoutOverrides {
        LayoutOverrides {
            line_width: self.line_width,
            tab_size: self.tab_size,
            dangle_parens: self.dangle_parens,
            always_wrap: self.always_wrap,
            max_pargs_hwrap: self.max_pargs_hwrap,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nargs_integer() {
        let src = "nargs = 1\n";
        let spec: KwargSpec = toml::from_str(src).unwrap();
        assert_eq!(spec.nargs, NArgs::Fixed(1));
    }

    #[test]
    fn nargs_zero_or_more() {
        let src = "nargs = \"*\"\n";
        let spec: KwargSpec = toml::from_str(src).unwrap();
        assert_eq!(spec.nargs, NArgs::ZeroOrMore);
    }

    #[test]
    fn nargs_one_or_more() {
        let src = "nargs = \"+\"\n";
        let spec: KwargSpec = toml::from_str(src).unwrap();
        assert_eq!(spec.nargs, NArgs::OneOrMore);
    }

    #[test]
    fn nargs_optional() {
        let src = "nargs = \"?\"\n";
        let spec: KwargSpec = toml::from_str(src).unwrap();
        assert_eq!(spec.nargs, NArgs::Optional);
    }

    #[test]
    fn nargs_at_least() {
        let src = "nargs = \"2+\"\n";
        let spec: KwargSpec = toml::from_str(src).unwrap();
        assert_eq!(spec.nargs, NArgs::AtLeast(2));
    }

    #[test]
    fn single_command_form() {
        let src = r#"
pargs = 1
flags = ["REQUIRED"]

[kwargs.COMPONENTS]
nargs = "+"
"#;
        let form: CommandForm = toml::from_str(src).unwrap();
        assert_eq!(form.pargs, NArgs::Fixed(1));
        assert!(form.flags.contains("REQUIRED"));
        assert!(form.kwargs.contains_key("COMPONENTS"));
    }

    #[test]
    fn discriminated_command() {
        let src = r#"
[forms.TARGETS]
pargs = "+"

[forms.TARGETS.kwargs.DESTINATION]
nargs = 1

[forms.FILES]
pargs = "+"
"#;
        let spec: CommandSpec = toml::from_str(src).unwrap();
        assert!(matches!(spec, CommandSpec::Discriminated { .. }));
        let form = spec.form_for(Some("targets"));
        assert!(form.kwargs.contains_key("DESTINATION"));
    }

    #[test]
    fn partial_override_round_trips() {
        let src = r#"
layout.always_wrap = true

[kwargs.COMPONENTS]
nargs = "+"
"#;
        let override_form: CommandFormOverride = toml::from_str(src).unwrap();
        assert_eq!(override_form.layout.unwrap().always_wrap, Some(true));
        assert_eq!(
            override_form.kwargs["COMPONENTS"].nargs,
            Some(NArgs::OneOrMore)
        );
    }
}
