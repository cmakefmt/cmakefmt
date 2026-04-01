pub mod registry;

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Deserializer};
use std::fmt;

// ── NArgs ────────────────────────────────────────────────────────────────────

/// How many arguments a positional slot or keyword takes.
///
/// In TOML this can be written as:
///   - integer   `nargs = 1`       → `Fixed(1)`
///   - `"*"`                        → `ZeroOrMore`
///   - `"+"`                        → `OneOrMore`
///   - `"?"`                        → `Optional`
///   - `"N+"` e.g. `"2+"`           → `AtLeast(2)`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NArgs {
    Fixed(usize),
    ZeroOrMore,
    OneOrMore,
    Optional,
    AtLeast(usize),
}

impl Default for NArgs {
    fn default() -> Self {
        NArgs::ZeroOrMore
    }
}

impl<'de> Deserialize<'de> for NArgs {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = NArgs;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

// ── Spec types ───────────────────────────────────────────────────────────────

/// Per-command layout overrides (override global config for this command only).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LayoutOverrides {
    pub line_width: Option<usize>,
    pub tab_size: Option<usize>,
    pub dangle_parens: Option<bool>,
    /// Force vertical (multi-line) layout even when args fit on one line.
    pub always_wrap: Option<bool>,
    pub max_pargs_hwrap: Option<usize>,
}

/// Specification for a keyword's following argument group, plus any
/// sub-keywords that keyword introduces.
#[derive(Debug, Clone, Deserialize)]
pub struct KwargSpec {
    /// How many arguments follow this keyword.
    #[serde(default)]
    pub nargs: NArgs,
    /// Sub-keywords valid inside this keyword's argument group.
    #[serde(default)]
    pub kwargs: IndexMap<String, KwargSpec>,
    /// Flag keywords (zero args) valid inside this keyword's argument group.
    #[serde(default)]
    pub flags: IndexSet<String>,
}

impl Default for KwargSpec {
    fn default() -> Self {
        KwargSpec {
            nargs: NArgs::ZeroOrMore,
            kwargs: IndexMap::new(),
            flags: IndexSet::new(),
        }
    }
}

/// The complete argument shape of one command invocation form.
#[derive(Debug, Clone, Deserialize)]
pub struct CommandForm {
    /// Positional arguments before the first keyword.  Default: ZeroOrMore.
    #[serde(default)]
    pub pargs: NArgs,
    /// Keyword → spec for its following argument group.
    #[serde(default)]
    pub kwargs: IndexMap<String, KwargSpec>,
    /// Boolean keywords (flags) that take no arguments.
    #[serde(default)]
    pub flags: IndexSet<String>,
    /// Per-command layout overrides.
    #[serde(default)]
    pub layout: Option<LayoutOverrides>,
}

/// Full specification for a command.
///
/// Most commands have a single fixed shape (`Single`).  Commands like
/// `install`, `file`, and `string` have multiple forms selected by the
/// value of their first positional argument (`Discriminated`).
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum CommandSpec {
    /// First positional argument selects the form (e.g. `install(TARGETS ...)` vs
    /// `install(FILES ...)`).  `forms` keys are the discriminator values.
    Discriminated {
        forms: IndexMap<String, CommandForm>,
        /// Form to use when the first arg doesn't match any key.
        #[serde(default)]
        fallback: Option<CommandForm>,
    },
    /// All invocations have the same argument shape.
    Single(CommandForm),
}

impl CommandSpec {
    /// Return the `CommandForm` to use given the first argument token (if any).
    pub fn form_for(&self, first_arg: Option<&str>) -> &CommandForm {
        match self {
            CommandSpec::Single(f) => f,
            CommandSpec::Discriminated { forms, fallback } => {
                let key = first_arg.unwrap_or("").to_ascii_uppercase();
                forms
                    .get(&key)
                    .or(fallback.as_ref())
                    .unwrap_or_else(|| forms.values().next().expect("non-empty forms"))
            }
        }
    }
}

// ── TOML root structure ───────────────────────────────────────────────────────

/// The shape of `builtins.toml` (and user config `[commands]` section).
#[derive(Debug, Default, Deserialize)]
pub struct SpecFile {
    #[serde(default)]
    pub commands: IndexMap<String, CommandSpec>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nargs(s: &str) -> NArgs {
        toml::from_str::<toml::Value>(&format!("v = {s}"))
            .unwrap()
            .get("v")
            .unwrap()
            .clone()
            .try_into::<NArgs>()
            .unwrap_or_else(|_| {
                // Try with quotes stripped for string values
                let s2 = s.trim_matches('"');
                match s2 {
                    "*" => NArgs::ZeroOrMore,
                    "+" => NArgs::OneOrMore,
                    "?" => NArgs::Optional,
                    s if s.ends_with('+') => NArgs::AtLeast(s[..s.len()-1].parse().unwrap()),
                    s => NArgs::Fixed(s.parse().unwrap()),
                }
            })
    }

    fn parse_nargs(toml_val: &str) -> NArgs {
        let src = format!("v = {toml_val}");
        let val: toml::Value = toml::from_str(&src).unwrap();
        val["v"].clone().try_into::<NArgs>().unwrap_or_else(|_| {
            // fallback: deserialize via serde
            let raw = val["v"].as_str().unwrap_or_default();
            match raw {
                "*" => NArgs::ZeroOrMore,
                "+" => NArgs::OneOrMore,
                "?" => NArgs::Optional,
                s if s.ends_with('+') => NArgs::AtLeast(s[..s.len()-1].parse().unwrap()),
                s => NArgs::Fixed(s.parse().unwrap()),
            }
        })
    }

    #[test]
    fn nargs_integer() {
        let src = "[v]\nnargs = 1\n";
        let spec: KwargSpec = toml::from_str(src).unwrap();
        assert_eq!(spec.nargs, NArgs::Fixed(1));
    }

    #[test]
    fn nargs_zero_or_more() {
        let src = "[v]\nnargs = \"*\"\n";
        let spec: KwargSpec = toml::from_str(src).unwrap();
        assert_eq!(spec.nargs, NArgs::ZeroOrMore);
    }

    #[test]
    fn nargs_one_or_more() {
        let src = "[v]\nnargs = \"+\"\n";
        let spec: KwargSpec = toml::from_str(src).unwrap();
        assert_eq!(spec.nargs, NArgs::OneOrMore);
    }

    #[test]
    fn nargs_optional() {
        let src = "[v]\nnargs = \"?\"\n";
        let spec: KwargSpec = toml::from_str(src).unwrap();
        assert_eq!(spec.nargs, NArgs::Optional);
    }

    #[test]
    fn nargs_at_least() {
        let src = "[v]\nnargs = \"2+\"\n";
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
        let form = spec.form_for(Some("TARGETS"));
        assert!(form.kwargs.contains_key("DESTINATION"));
    }
}
