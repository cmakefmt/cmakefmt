//! Built-in and override-backed command registry.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use indexmap::{IndexMap, IndexSet};

use crate::config::file::{detect_config_format, ConfigFileFormat};
use crate::error::{Error, Result};

use super::{
    CommandForm, CommandFormOverride, CommandSpec, CommandSpecOverride, KwargSpec,
    KwargSpecOverride, LayoutOverrides, LayoutOverridesOverride, SpecFile, SpecMetadata,
    SpecOverrideFile,
};

const BUILTINS_PATH: &str = "src/spec/builtins.toml";
const BUILTINS_TOML: &str = include_str!("builtins.toml");

#[derive(Debug, Clone)]
pub struct CommandRegistry {
    metadata: SpecMetadata,
    commands: IndexMap<String, CommandSpec>,
    fallback: CommandSpec,
}

impl CommandRegistry {
    /// Load the embedded built-in registry from `builtins.toml`.
    pub fn load() -> Result<Self> {
        Self::from_builtins_and_overrides(None::<&Path>)
    }

    /// Return the lazily initialized singleton built-in registry.
    pub fn builtins() -> &'static Self {
        static BUILTINS: OnceLock<CommandRegistry> = OnceLock::new();
        BUILTINS.get_or_init(|| {
            CommandRegistry::from_builtins_and_overrides(None::<&Path>)
                .expect("embedded built-in command registry should deserialize")
        })
    }

    /// Load the embedded built-ins and optionally merge a user override file.
    pub fn from_builtins_and_overrides(path: Option<impl AsRef<Path>>) -> Result<Self> {
        let mut registry = Self::from_spec_file(parse_builtins()?);

        if let Some(path) = path {
            registry.merge_override_file(path.as_ref())?;
        }

        Ok(registry)
    }

    /// Build a registry directly from a deserialized [`SpecFile`].
    pub fn from_spec_file(mut spec_file: SpecFile) -> Self {
        normalize_spec_file(&mut spec_file);
        Self {
            metadata: spec_file.metadata,
            commands: spec_file.commands,
            fallback: CommandSpec::Single(CommandForm::default()),
        }
    }

    /// Merge a supported user override file from disk into the registry.
    pub fn merge_override_file(&mut self, path: &Path) -> Result<()> {
        let source = fs::read_to_string(path)?;
        self.merge_override_source(&source, path.to_path_buf(), detect_config_format(path)?)
    }

    /// Merge TOML override contents into the registry.
    pub fn merge_override_str(&mut self, source: &str, path: impl Into<PathBuf>) -> Result<()> {
        self.merge_override_source(source, path.into(), ConfigFileFormat::Toml)
    }

    fn merge_override_source(
        &mut self,
        source: &str,
        path: PathBuf,
        format: ConfigFileFormat,
    ) -> Result<()> {
        let mut overrides: SpecOverrideFile = match format {
            ConfigFileFormat::Toml => toml::from_str(source).map_err(|toml_err| {
                let (line, column) = crate::config::file::toml_line_col(
                    source,
                    toml_err.span().map(|span| span.start),
                );
                Error::Spec {
                    path: path.clone(),
                    details: crate::error::FileParseError {
                        format: format.as_str(),
                        message: toml_err.to_string().into_boxed_str(),
                        line,
                        column,
                    },
                    source_message: toml_err.to_string().into_boxed_str(),
                }
            })?,
            ConfigFileFormat::Yaml => serde_yaml::from_str(source).map_err(|yaml_err| {
                let location = yaml_err.location();
                Error::Spec {
                    path: path.clone(),
                    details: crate::error::FileParseError {
                        format: format.as_str(),
                        message: yaml_err.to_string().into_boxed_str(),
                        line: location.as_ref().map(|loc| loc.line()),
                        column: location.as_ref().map(|loc| loc.column()),
                    },
                    source_message: yaml_err.to_string().into_boxed_str(),
                }
            })?,
        };
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

    /// Get the command spec for `command_name`, falling back to a permissive
    /// default when the command is unknown.
    pub fn get(&self, command_name: &str) -> &CommandSpec {
        if let Some(spec) = self.commands.get(command_name) {
            return spec;
        }

        if !has_ascii_uppercase(command_name) {
            return &self.fallback;
        }

        self.commands
            .get(&command_name.to_ascii_lowercase())
            .unwrap_or(&self.fallback)
    }

    /// Return `true` when the command is present in the built-in registry.
    pub fn contains_builtin(&self, command_name: &str) -> bool {
        self.commands.contains_key(command_name)
            || (has_ascii_uppercase(command_name)
                && self
                    .commands
                    .contains_key(&command_name.to_ascii_lowercase()))
    }

    /// Report the audited upstream CMake version for the built-in spec.
    pub fn audited_cmake_version(&self) -> &str {
        &self.metadata.cmake_version
    }
}

fn has_ascii_uppercase(s: &str) -> bool {
    s.bytes().any(|byte| byte.is_ascii_uppercase())
}

fn parse_builtins() -> Result<SpecFile> {
    let mut spec: SpecFile = toml::from_str(BUILTINS_TOML).map_err(|source| Error::Spec {
        path: PathBuf::from(BUILTINS_PATH),
        details: crate::error::FileParseError {
            format: "TOML",
            message: source.to_string().into_boxed_str(),
            line: None,
            column: None,
        },
        source_message: source.to_string().into_boxed_str(),
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
    fn registry_reports_audited_cmake_version() {
        let registry = CommandRegistry::load().unwrap();
        assert_eq!(registry.audited_cmake_version(), "4.3.1");
    }

    #[test]
    fn registry_knows_project_43_keywords() {
        let registry = CommandRegistry::load().unwrap();
        let CommandSpec::Single(form) = registry.get("project") else {
            panic!()
        };
        assert!(form.flags.contains("COMPAT_VERSION"));
        assert!(form.flags.contains("SPDX_LICENSE"));
    }

    #[test]
    fn registry_knows_export_package_info_form() {
        let registry = CommandRegistry::load().unwrap();
        let CommandSpec::Discriminated { .. } = registry.get("export") else {
            panic!()
        };
        let form = registry.get("export").form_for(Some("PACKAGE_INFO"));
        assert_eq!(form.pargs, NArgs::Fixed(1));
        assert!(form.kwargs.contains_key("EXPORT"));
        assert!(form.kwargs.contains_key("CXX_MODULES_DIRECTORY"));
    }

    #[test]
    fn registry_knows_install_package_info_form() {
        let registry = CommandRegistry::load().unwrap();
        let form = registry.get("install").form_for(Some("PACKAGE_INFO"));
        assert_eq!(form.pargs, NArgs::Fixed(1));
        assert!(form.kwargs.contains_key("DESTINATION"));
        assert!(form.kwargs.contains_key("COMPAT_VERSION"));
    }

    #[test]
    fn registry_knows_install_export_namespace_keyword() {
        let registry = CommandRegistry::load().unwrap();
        let form = registry.get("install").form_for(Some("EXPORT"));
        assert!(form.kwargs.contains_key("DESTINATION"));
        assert!(form.kwargs.contains_key("NAMESPACE"));
        assert!(form.kwargs.contains_key("FILE"));
        assert!(form.flags.contains("EXCLUDE_FROM_ALL"));
    }

    #[test]
    fn registry_knows_install_targets_export_and_includes_sections() {
        let registry = CommandRegistry::load().unwrap();
        let form = registry.get("install").form_for(Some("TARGETS"));
        assert!(form.kwargs.contains_key("EXPORT"));
        assert!(form.kwargs.contains_key("INCLUDES"));
        assert!(form
            .kwargs
            .get("INCLUDES")
            .is_some_and(|spec| spec.kwargs.contains_key("DESTINATION")));
        assert!(form.kwargs.contains_key("RUNTIME_DEPENDENCY_SET"));
        assert!(form.flags.contains("RUNTIME"));
        assert!(form.flags.contains("LIBRARY"));
        assert!(form.flags.contains("ARCHIVE"));
    }

    #[test]
    fn registry_knows_cmake_language_trace_form() {
        let registry = CommandRegistry::load().unwrap();
        let form = registry.get("cmake_language").form_for(Some("TRACE"));
        assert!(form.flags.contains("ON"));
        assert!(form.flags.contains("OFF"));
        assert!(form.flags.contains("EXPAND"));
    }

    #[test]
    fn registry_knows_cmake_pkg_config_import_keywords() {
        let registry = CommandRegistry::load().unwrap();
        let form = registry.get("cmake_pkg_config").form_for(Some("IMPORT"));
        assert!(form.kwargs.contains_key("NAME"));
        assert!(form.kwargs.contains_key("BIND_PC_REQUIRES"));
    }

    #[test]
    fn registry_knows_file_archive_create_threads() {
        let registry = CommandRegistry::load().unwrap();
        let form = registry.get("file").form_for(Some("ARCHIVE_CREATE"));
        assert!(form.kwargs.contains_key("THREADS"));
        assert!(form.kwargs.contains_key("COMPRESSION_LEVEL"));
    }

    #[test]
    fn registry_knows_file_strings_keywords() {
        let registry = CommandRegistry::load().unwrap();
        let form = registry.get("file").form_for(Some("STRINGS"));
        assert_eq!(form.pargs, NArgs::Fixed(2));
        assert!(form.kwargs.contains_key("REGEX"));
        assert!(form.kwargs.contains_key("LIMIT_COUNT"));
    }

    #[test]
    fn registry_knows_cmake_package_config_helpers_commands() {
        let registry = CommandRegistry::load().unwrap();
        let configure = registry.get("configure_package_config_file").form_for(None);
        assert!(configure.kwargs.contains_key("INSTALL_DESTINATION"));
        assert!(configure.kwargs.contains_key("PATH_VARS"));

        let version = registry
            .get("write_basic_package_version_file")
            .form_for(None);
        assert!(version.kwargs.contains_key("COMPATIBILITY"));
        assert!(version.kwargs.contains_key("VERSION"));
    }

    #[test]
    fn registry_knows_utility_module_commands() {
        let registry = CommandRegistry::load().unwrap();
        assert_eq!(
            registry.get("cmake_dependent_option").form_for(None).pargs,
            NArgs::Fixed(5)
        );
        assert_eq!(
            registry.get("check_language").form_for(None).pargs,
            NArgs::Fixed(1)
        );
        assert_eq!(
            registry.get("check_include_file").form_for(None).pargs,
            NArgs::AtLeast(2)
        );
        assert_eq!(
            registry.get("check_compiler_flag").form_for(None).pargs,
            NArgs::Fixed(3)
        );
        assert_eq!(
            registry
                .get("check_objc_compiler_flag")
                .form_for(None)
                .pargs,
            NArgs::Fixed(2)
        );
        assert_eq!(
            registry.get("check_cxx_symbol_exists").form_for(None).pargs,
            NArgs::Fixed(3)
        );
        assert!(registry
            .get("cmake_push_check_state")
            .form_for(None)
            .flags
            .contains("RESET"));
        let print_props = registry.get("cmake_print_properties").form_for(None);
        assert!(print_props.kwargs.contains_key("TARGETS"));
        assert!(print_props.kwargs.contains_key("PROPERTIES"));
        let pie = registry.get("check_pie_supported").form_for(None);
        assert!(pie.kwargs.contains_key("OUTPUT_VARIABLE"));
        assert!(pie.kwargs.contains_key("LANGUAGES"));
        let source_compiles = registry.get("check_source_compiles").form_for(None);
        assert!(source_compiles.kwargs.contains_key("SRC_EXT"));
        assert!(source_compiles.kwargs.contains_key("FAIL_REGEX"));
        let find_dependency = registry.get("find_dependency").form_for(None);
        assert!(find_dependency.flags.contains("REQUIRED"));
        assert!(find_dependency.kwargs.contains_key("COMPONENTS"));
    }

    #[test]
    fn registry_knows_supported_deprecated_module_commands() {
        let registry = CommandRegistry::load().unwrap();
        let version = registry
            .get("write_basic_config_version_file")
            .form_for(None);
        assert_eq!(version.pargs, NArgs::Fixed(1));
        assert!(version.kwargs.contains_key("COMPATIBILITY"));
        assert!(version.flags.contains("ARCH_INDEPENDENT"));
        assert_eq!(
            registry.get("check_cxx_accepts_flag").form_for(None).pargs,
            NArgs::Fixed(2)
        );
    }

    #[test]
    fn registry_knows_fetchcontent_commands() {
        let registry = CommandRegistry::load().unwrap();
        let declare = registry.get("fetchcontent_declare").form_for(None);
        assert_eq!(declare.pargs, NArgs::Fixed(1));
        assert!(declare.flags.contains("EXCLUDE_FROM_ALL"));
        assert!(declare.kwargs.contains_key("FIND_PACKAGE_ARGS"));

        let get_properties = registry.get("fetchcontent_getproperties").form_for(None);
        assert!(get_properties.kwargs.contains_key("SOURCE_DIR"));
        assert!(get_properties.kwargs.contains_key("BINARY_DIR"));
        assert!(get_properties.kwargs.contains_key("POPULATED"));

        let populate = registry.get("fetchcontent_populate").form_for(None);
        assert!(populate.flags.contains("QUIET"));
        assert!(populate.kwargs.contains_key("SUBBUILD_DIR"));
    }

    #[test]
    fn registry_knows_common_test_and_package_helper_modules() {
        let registry = CommandRegistry::load().unwrap();

        let google_add = registry.get("gtest_add_tests").form_for(None);
        assert!(google_add.kwargs.contains_key("TARGET"));
        assert!(google_add.kwargs.contains_key("SOURCES"));
        assert!(google_add.flags.contains("SKIP_DEPENDENCY"));

        let google_discover = registry.get("gtest_discover_tests").form_for(None);
        assert!(google_discover.kwargs.contains_key("DISCOVERY_MODE"));
        assert!(google_discover.kwargs.contains_key("XML_OUTPUT_DIR"));
        assert!(google_discover.flags.contains("NO_PRETTY_TYPES"));

        assert_eq!(
            registry.get("processorcount").form_for(None).pargs,
            NArgs::Fixed(1)
        );

        let fp_hsa = registry
            .get("find_package_handle_standard_args")
            .form_for(None);
        assert!(fp_hsa.flags.contains("DEFAULT_MSG"));
        assert!(fp_hsa.kwargs.contains_key("REQUIRED_VARS"));
        assert!(fp_hsa.kwargs.contains_key("VERSION_VAR"));

        let fp_check = registry.get("find_package_check_version").form_for(None);
        assert_eq!(fp_check.pargs, NArgs::Fixed(2));
        assert!(fp_check.flags.contains("HANDLE_VERSION_RANGE"));
    }

    #[test]
    fn registry_knows_externalproject_helper_commands() {
        let registry = CommandRegistry::load().unwrap();
        let step = registry.get("externalproject_add_step").form_for(None);
        assert_eq!(step.pargs, NArgs::Fixed(2));
        assert!(step.kwargs.contains_key("COMMAND"));
        assert!(step.kwargs.contains_key("DEPENDEES"));
        assert!(step.kwargs.contains_key("ENVIRONMENT_MODIFICATION"));

        let targets = registry
            .get("externalproject_add_steptargets")
            .form_for(None);
        assert_eq!(targets.pargs, NArgs::AtLeast(2));
        assert!(targets.flags.contains("NO_DEPENDS"));

        let deps = registry
            .get("externalproject_add_stepdependencies")
            .form_for(None);
        assert_eq!(deps.pargs, NArgs::AtLeast(3));

        let props = registry.get("externalproject_get_property").form_for(None);
        assert_eq!(props.pargs, NArgs::AtLeast(2));
    }

    #[test]
    fn registry_knows_packaging_and_find_helper_module_commands() {
        let registry = CommandRegistry::load().unwrap();

        assert_eq!(
            registry.get("find_package_message").form_for(None).pargs,
            NArgs::Fixed(3)
        );
        assert_eq!(
            registry
                .get("select_library_configurations")
                .form_for(None)
                .pargs,
            NArgs::Fixed(1)
        );

        let component = registry.get("cpack_add_component").form_for(None);
        assert!(component.flags.contains("HIDDEN"));
        assert!(component.kwargs.contains_key("DISPLAY_NAME"));
        assert!(component.kwargs.contains_key("DEPENDS"));

        let group = registry.get("cpack_add_component_group").form_for(None);
        assert!(group.flags.contains("EXPANDED"));
        assert!(group.kwargs.contains_key("PARENT_GROUP"));

        let downloads = registry.get("cpack_configure_downloads").form_for(None);
        assert_eq!(downloads.pargs, NArgs::Fixed(1));
        assert!(downloads.kwargs.contains_key("UPLOAD_DIRECTORY"));
    }

    #[test]
    fn registry_knows_export_header_module_commands() {
        let registry = CommandRegistry::load().unwrap();
        let export_header = registry.get("generate_export_header").form_for(None);
        assert_eq!(export_header.pargs, NArgs::Fixed(1));
        assert!(export_header.flags.contains("DEFINE_NO_DEPRECATED"));
        assert!(export_header.kwargs.contains_key("EXPORT_FILE_NAME"));
        assert!(export_header.kwargs.contains_key("PREFIX_NAME"));

        assert_eq!(
            registry
                .get("add_compiler_export_flags")
                .form_for(None)
                .pargs,
            NArgs::Optional
        );
    }

    #[test]
    fn registry_knows_remaining_utility_module_commands() {
        let registry = CommandRegistry::load().unwrap();

        for command in [
            "android_add_test_data",
            "add_file_dependencies",
            "cmake_add_fortran_subdirectory",
            "cmake_expand_imported_targets",
            "cmake_force_c_compiler",
            "cmake_force_cxx_compiler",
            "cmake_force_fortran_compiler",
            "ctest_coverage_collect_gcov",
            "copy_and_fixup_bundle",
            "fixup_bundle",
            "fixup_bundle_item",
            "verify_app",
            "verify_bundle_prerequisites",
            "verify_bundle_symlinks",
            "get_bundle_main_executable",
            "get_dotapp_dir",
            "get_bundle_and_executable",
            "get_bundle_all_executables",
            "get_bundle_keys",
            "get_item_key",
            "get_item_rpaths",
            "clear_bundle_keys",
            "set_bundle_key_values",
            "copy_resolved_framework_into_bundle",
            "copy_resolved_item_into_bundle",
            "cpack_ifw_add_package_resources",
            "cpack_ifw_add_repository",
            "cpack_ifw_configure_component",
            "cpack_ifw_configure_component_group",
            "cpack_ifw_update_repository",
            "cpack_ifw_configure_file",
            "csharp_set_windows_forms_properties",
            "csharp_set_designer_cs_properties",
            "csharp_set_xaml_cs_properties",
            "csharp_get_filename_keys",
            "csharp_get_filename_key_base",
            "csharp_get_dependentupon_name",
            "externaldata_expand_arguments",
            "externaldata_add_test",
            "externaldata_add_target",
            "fortrancinterface_header",
            "fortrancinterface_verify",
            "fetchcontent_setpopulated",
            "gnuinstalldirs_get_absolute_install_dir",
            "find_jar",
            "add_jar",
            "install_jar",
            "install_jar_exports",
            "export_jars",
            "create_javadoc",
            "create_javah",
            "install_jni_symlink",
            "swig_add_library",
            "swig_link_libraries",
            "print_enabled_features",
            "print_disabled_features",
            "set_feature_info",
            "set_package_info",
        ] {
            assert!(
                registry.contains_builtin(command),
                "missing built-in {command}"
            );
        }

        assert_eq!(
            registry
                .get("ctest_coverage_collect_gcov")
                .form_for(None)
                .pargs,
            NArgs::ZeroOrMore
        );
        assert_eq!(
            registry
                .get("fortrancinterface_verify")
                .form_for(None)
                .pargs,
            NArgs::ZeroOrMore
        );
        assert_eq!(
            registry.get("add_jar").form_for(None).pargs,
            NArgs::AtLeast(2)
        );
        assert_eq!(
            registry
                .get("cpack_ifw_configure_file")
                .form_for(None)
                .pargs,
            NArgs::Fixed(2)
        );
        assert_eq!(
            registry
                .get("gnuinstalldirs_get_absolute_install_dir")
                .form_for(None)
                .pargs,
            NArgs::AtLeast(3)
        );
    }

    #[test]
    fn registry_knows_string_json_43_modes() {
        let registry = CommandRegistry::load().unwrap();
        let form = registry.get("string").form_for(Some("JSON"));
        assert!(form.flags.contains("GET_RAW"));
        assert!(form.flags.contains("STRING_ENCODE"));
        assert!(form.kwargs.contains_key("ERROR_VARIABLE"));
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
