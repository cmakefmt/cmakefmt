pub mod registry;

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Deserializer};
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
    pub line_width: Option<usize>,
    pub tab_size: Option<usize>,
    pub dangle_parens: Option<bool>,
    pub always_wrap: Option<bool>,
    pub max_pargs_hwrap: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KwargSpec {
    #[serde(default)]
    pub nargs: NArgs,
    #[serde(default)]
    pub kwargs: IndexMap<String, KwargSpec>,
    #[serde(default)]
    pub flags: IndexSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandForm {
    #[serde(default)]
    pub pargs: NArgs,
    #[serde(default)]
    pub kwargs: IndexMap<String, KwargSpec>,
    #[serde(default)]
    pub flags: IndexSet<String>,
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
    Discriminated {
        forms: IndexMap<String, CommandForm>,
        #[serde(default)]
        fallback: Option<CommandForm>,
    },
    Single(CommandForm),
}

impl CommandSpec {
    pub fn form_for(&self, first_arg: Option<&str>) -> &CommandForm {
        match self {
            CommandSpec::Single(form) => form,
            CommandSpec::Discriminated { forms, fallback } => {
                let key = first_arg.unwrap_or_default().to_ascii_uppercase();
                forms.get(&key).or(fallback.as_ref()).unwrap_or_else(|| {
                    forms
                        .values()
                        .next()
                        .expect("discriminated spec has a form")
                })
            }
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct SpecFile {
    #[serde(default)]
    pub commands: IndexMap<String, CommandSpec>,
}

// ── Mergeable override model ─────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutOverridesOverride {
    pub line_width: Option<usize>,
    pub tab_size: Option<usize>,
    pub dangle_parens: Option<bool>,
    pub always_wrap: Option<bool>,
    pub max_pargs_hwrap: Option<usize>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KwargSpecOverride {
    pub nargs: Option<NArgs>,
    #[serde(default)]
    pub kwargs: IndexMap<String, KwargSpecOverride>,
    #[serde(default)]
    pub flags: IndexSet<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandFormOverride {
    pub pargs: Option<NArgs>,
    #[serde(default)]
    pub kwargs: IndexMap<String, KwargSpecOverride>,
    #[serde(default)]
    pub flags: IndexSet<String>,
    pub layout: Option<LayoutOverridesOverride>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum CommandSpecOverride {
    Single(CommandFormOverride),
    Discriminated {
        #[serde(default)]
        forms: IndexMap<String, CommandFormOverride>,
        #[serde(default)]
        fallback: Option<CommandFormOverride>,
    },
}

#[derive(Debug, Default, Deserialize)]
pub struct SpecOverrideFile {
    #[serde(default)]
    pub commands: IndexMap<String, CommandSpecOverride>,
}

impl CommandSpecOverride {
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
