use std::fs;
use std::path::{Path, PathBuf};

use indexmap::{IndexMap, IndexSet};

use crate::error::{Error, Result};

use super::{
    CommandForm, CommandFormOverride, CommandSpec, CommandSpecOverride, KwargSpec,
    KwargSpecOverride, LayoutOverrides, LayoutOverridesOverride, SpecFile, SpecOverrideFile,
};

const BUILTINS_PATH: &str = "src/spec/builtins.toml";
const BUILTINS_TOML: &str = include_str!("builtins.toml");

#[derive(Debug, Clone)]
pub struct CommandRegistry {
    commands: IndexMap<String, CommandSpec>,
    fallback: CommandSpec,
}

impl CommandRegistry {
    pub fn load() -> Result<Self> {
        Self::from_builtins_and_overrides(None::<&Path>)
    }

    pub fn from_builtins_and_overrides(path: Option<impl AsRef<Path>>) -> Result<Self> {
        let mut registry = Self::from_spec_file(parse_builtins()?);

        if let Some(path) = path {
            registry.merge_override_file(path.as_ref())?;
        }

        Ok(registry)
    }

    pub fn from_spec_file(mut spec_file: SpecFile) -> Self {
        normalize_spec_file(&mut spec_file);
        Self {
            commands: spec_file.commands,
            fallback: CommandSpec::Single(CommandForm::default()),
        }
    }

    pub fn merge_override_file(&mut self, path: &Path) -> Result<()> {
        let source = fs::read_to_string(path)?;
        self.merge_override_str(&source, path)
    }

    pub fn merge_override_str(&mut self, source: &str, path: impl Into<PathBuf>) -> Result<()> {
        let path = path.into();
        let mut overrides: SpecOverrideFile =
            toml::from_str(source).map_err(|source| Error::Spec {
                path: path.clone(),
                source,
            })?;
        normalize_override_file(&mut overrides);

        for (name, override_spec) in overrides.commands {
            match self.commands.get_mut(&name) {
                Some(existing) => merge_command_spec(existing, override_spec),
                None => {
                    self.commands.insert(name, override_spec.into_full_spec());
                }
            }
        }

        Ok(())
    }

    pub fn get(&self, command_name: &str) -> &CommandSpec {
        self.commands
            .get(&command_name.to_ascii_lowercase())
            .unwrap_or(&self.fallback)
    }

    pub fn contains_builtin(&self, command_name: &str) -> bool {
        self.commands
            .contains_key(&command_name.to_ascii_lowercase())
    }
}

fn parse_builtins() -> Result<SpecFile> {
    let mut spec: SpecFile = toml::from_str(BUILTINS_TOML).map_err(|source| Error::Spec {
        path: PathBuf::from(BUILTINS_PATH),
        source,
    })?;
    normalize_spec_file(&mut spec);
    Ok(spec)
}

fn normalize_spec_file(spec: &mut SpecFile) {
    spec.commands = std::mem::take(&mut spec.commands)
        .into_iter()
        .map(|(name, mut command)| {
            normalize_command_spec(&mut command);
            (name.to_ascii_lowercase(), command)
        })
        .collect();
}

fn normalize_override_file(spec: &mut SpecOverrideFile) {
    spec.commands = std::mem::take(&mut spec.commands)
        .into_iter()
        .map(|(name, mut command)| {
            normalize_command_override(&mut command);
            (name.to_ascii_lowercase(), command)
        })
        .collect();
}

fn normalize_command_spec(spec: &mut CommandSpec) {
    match spec {
        CommandSpec::Single(form) => normalize_form(form),
        CommandSpec::Discriminated { forms, fallback } => {
            *forms = std::mem::take(forms)
                .into_iter()
                .map(|(name, mut form)| {
                    normalize_form(&mut form);
                    (name.to_ascii_uppercase(), form)
                })
                .collect();

            if let Some(fallback) = fallback {
                normalize_form(fallback);
            }
        }
    }
}

fn normalize_command_override(spec: &mut CommandSpecOverride) {
    match spec {
        CommandSpecOverride::Single(form) => normalize_form_override(form),
        CommandSpecOverride::Discriminated { forms, fallback } => {
            *forms = std::mem::take(forms)
                .into_iter()
                .map(|(name, mut form)| {
                    normalize_form_override(&mut form);
                    (name.to_ascii_uppercase(), form)
                })
                .collect();

            if let Some(fallback) = fallback {
                normalize_form_override(fallback);
            }
        }
    }
}

fn normalize_form(form: &mut CommandForm) {
    form.kwargs = std::mem::take(&mut form.kwargs)
        .into_iter()
        .map(|(name, mut kwarg)| {
            normalize_kwarg(&mut kwarg);
            (name.to_ascii_uppercase(), kwarg)
        })
        .collect();

    form.flags = std::mem::take(&mut form.flags)
        .into_iter()
        .map(|flag| flag.to_ascii_uppercase())
        .collect();
}

fn normalize_form_override(form: &mut CommandFormOverride) {
    form.kwargs = std::mem::take(&mut form.kwargs)
        .into_iter()
        .map(|(name, mut kwarg)| {
            normalize_kwarg_override(&mut kwarg);
            (name.to_ascii_uppercase(), kwarg)
        })
        .collect();

    form.flags = std::mem::take(&mut form.flags)
        .into_iter()
        .map(|flag| flag.to_ascii_uppercase())
        .collect();
}

fn normalize_kwarg(spec: &mut KwargSpec) {
    spec.kwargs = std::mem::take(&mut spec.kwargs)
        .into_iter()
        .map(|(name, mut kwarg)| {
            normalize_kwarg(&mut kwarg);
            (name.to_ascii_uppercase(), kwarg)
        })
        .collect();

    spec.flags = std::mem::take(&mut spec.flags)
        .into_iter()
        .map(|flag| flag.to_ascii_uppercase())
        .collect();
}

fn normalize_kwarg_override(spec: &mut KwargSpecOverride) {
    spec.kwargs = std::mem::take(&mut spec.kwargs)
        .into_iter()
        .map(|(name, mut kwarg)| {
            normalize_kwarg_override(&mut kwarg);
            (name.to_ascii_uppercase(), kwarg)
        })
        .collect();

    spec.flags = std::mem::take(&mut spec.flags)
        .into_iter()
        .map(|flag| flag.to_ascii_uppercase())
        .collect();
}

fn merge_command_spec(base: &mut CommandSpec, override_spec: CommandSpecOverride) {
    match (base, override_spec) {
        (CommandSpec::Single(base_form), CommandSpecOverride::Single(override_form)) => {
            merge_form(base_form, override_form);
        }
        (
            CommandSpec::Discriminated {
                forms: base_forms,
                fallback: base_fallback,
            },
            CommandSpecOverride::Discriminated {
                forms: override_forms,
                fallback: override_fallback,
            },
        ) => {
            for (name, override_form) in override_forms {
                match base_forms.get_mut(&name) {
                    Some(base_form) => merge_form(base_form, override_form),
                    None => {
                        base_forms.insert(name, override_form.into_full_form());
                    }
                }
            }

            if let Some(override_fallback) = override_fallback {
                match base_fallback {
                    Some(base_fallback) => merge_form(base_fallback, override_fallback),
                    None => {
                        *base_fallback = Some(override_fallback.into_full_form());
                    }
                }
            }
        }
        (base_spec, override_spec) => {
            *base_spec = override_spec.into_full_spec();
        }
    }
}

fn merge_form(base: &mut CommandForm, override_form: CommandFormOverride) {
    if let Some(pargs) = override_form.pargs {
        base.pargs = pargs;
    }

    merge_flags(&mut base.flags, override_form.flags);

    for (name, override_kwarg) in override_form.kwargs {
        match base.kwargs.get_mut(&name) {
            Some(base_kwarg) => merge_kwarg(base_kwarg, override_kwarg),
            None => {
                base.kwargs.insert(name, override_kwarg.into_full_spec());
            }
        }
    }

    if let Some(layout) = override_form.layout {
        merge_layout(
            base.layout.get_or_insert_with(LayoutOverrides::default),
            layout,
        );
    }
}

fn merge_kwarg(base: &mut KwargSpec, override_kwarg: KwargSpecOverride) {
    if let Some(nargs) = override_kwarg.nargs {
        base.nargs = nargs;
    }

    merge_flags(&mut base.flags, override_kwarg.flags);

    for (name, nested_override) in override_kwarg.kwargs {
        match base.kwargs.get_mut(&name) {
            Some(base_nested) => merge_kwarg(base_nested, nested_override),
            None => {
                base.kwargs.insert(name, nested_override.into_full_spec());
            }
        }
    }
}

fn merge_layout(base: &mut LayoutOverrides, override_layout: LayoutOverridesOverride) {
    if let Some(value) = override_layout.line_width {
        base.line_width = Some(value);
    }
    if let Some(value) = override_layout.tab_size {
        base.tab_size = Some(value);
    }
    if let Some(value) = override_layout.dangle_parens {
        base.dangle_parens = Some(value);
    }
    if let Some(value) = override_layout.always_wrap {
        base.always_wrap = Some(value);
    }
    if let Some(value) = override_layout.max_pargs_hwrap {
        base.max_pargs_hwrap = Some(value);
    }
}

fn merge_flags(base: &mut IndexSet<String>, override_flags: IndexSet<String>) {
    for flag in override_flags {
        base.insert(flag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::NArgs;

    #[test]
    fn registry_has_target_link_libraries_keywords() {
        let registry = CommandRegistry::load().unwrap();
        let CommandSpec::Single(form) = registry.get("target_link_libraries") else {
            panic!()
        };
        assert!(form.kwargs.contains_key("PUBLIC"));
        assert!(form.kwargs.contains_key("PRIVATE"));
        assert!(form.kwargs.contains_key("INTERFACE"));
    }

    #[test]
    fn registry_has_install_forms() {
        let registry = CommandRegistry::load().unwrap();
        assert!(matches!(
            registry.get("install"),
            CommandSpec::Discriminated { .. }
        ));
    }

    #[test]
    fn registry_unknown_command_uses_fallback() {
        let registry = CommandRegistry::load().unwrap();
        let spec = registry.get("my_unknown_command");
        let CommandSpec::Single(form) = spec else {
            panic!()
        };
        assert_eq!(form.pargs, NArgs::ZeroOrMore);
        assert!(form.kwargs.is_empty());
        assert!(form.flags.is_empty());
    }

    #[test]
    fn registry_knows_builtin_surface() {
        let registry = CommandRegistry::load().unwrap();
        assert!(registry.contains_builtin("cmake_minimum_required"));
        assert!(registry.contains_builtin("target_sources"));
        assert!(registry.contains_builtin("while"));
        assert!(registry.contains_builtin("external_project_add"));
    }

    #[test]
    fn user_override_entries_merge_with_builtins() {
        let mut registry = CommandRegistry::load().unwrap();
        let overrides = r#"
[commands.target_link_libraries.layout]
always_wrap = true

[commands.target_link_libraries.kwargs.LINKER_LANGUAGE]
nargs = 1
"#;

        registry
            .merge_override_str(overrides, PathBuf::from("test-overrides.toml"))
            .unwrap();

        let CommandSpec::Single(form) = registry.get("target_link_libraries") else {
            panic!()
        };
        assert_eq!(
            form.layout.as_ref().and_then(|layout| layout.always_wrap),
            Some(true)
        );
        assert!(form.kwargs.contains_key("PUBLIC"));
        assert_eq!(form.kwargs["LINKER_LANGUAGE"].nargs, NArgs::Fixed(1));
    }
}
