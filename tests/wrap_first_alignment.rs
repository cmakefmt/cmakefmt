// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use cmakefmt::{
    format_source, format_source_with_registry, semantic::semantic_equivalent,
    spec::registry::CommandRegistry, ArgumentCommentStyle, Config, DangleAlign,
    WrapAfterFirstArgAlign,
};

#[test]
fn wrap_first_alignment_preserves_comments_and_is_idempotent_across_layouts() {
    for source in [
        r"if(ON)
target_compile_definitions(mylib # target
 INTERFACE # visibility
 # definition
 USING_DLL COMMON_USING_DLL EXTRA_DEFINE)
endif()
",
        r"if(ON)
set(SOURCES # variable
 # source files
 first.cpp second.cpp third.cpp fourth.cpp)
endif()
",
        r"custom_command(
 # leading
 first
 second # inline
 # standalone
 third fourth fifth)
",
    ] {
        for align in [
            WrapAfterFirstArgAlign::UnderFirstArg,
            WrapAfterFirstArgAlign::DefaultIndent,
        ] {
            for use_tabchars in [false, true] {
                for dangle in [
                    None,
                    Some(DangleAlign::Prefix),
                    Some(DangleAlign::Open),
                    Some(DangleAlign::Close),
                ] {
                    for argument_comment_style in [
                        ArgumentCommentStyle::Preserve,
                        ArgumentCommentStyle::Standalone,
                    ] {
                        let config = Config {
                            line_width: 50,
                            wrap_after_first_arg: true,
                            wrap_after_first_arg_align: align,
                            use_tabchars,
                            tab_size: 4,
                            dangle_parens: dangle.is_some(),
                            dangle_align: dangle.unwrap_or_default(),
                            argument_comment_style,
                            ..Config::default()
                        };
                        let formatted = format_source(source, &config).unwrap();
                        assert_eq!(
                            formatted,
                            format_source(&formatted, &config).unwrap(),
                            "{config:?}"
                        );
                        assert!(
                            semantic_equivalent(source, &formatted),
                            "{config:?}\n{formatted}"
                        );
                        // Short comments must survive verbatim and in source order.
                        let comments = |text: &str| {
                            text.lines()
                                .filter_map(|line| {
                                    line.find('#').map(|i| line[i..].trim().to_owned())
                                })
                                .collect::<Vec<_>>()
                        };
                        assert_eq!(
                            comments(source),
                            comments(&formatted),
                            "{config:?}\n{formatted}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn wrap_first_alignment_tabs_comments_and_dangling_paren() {
    let source = "if(ON)\ntarget_compile_definitions(mylib # target\n INTERFACE\n # definition\n USING_DLL COMMON_USING_DLL EXTRA_DEFINE)\nendif()\n";
    let config = Config {
        line_width: 50,
        wrap_after_first_arg: true,
        wrap_after_first_arg_align: WrapAfterFirstArgAlign::DefaultIndent,
        use_tabchars: true,
        tab_size: 4,
        dangle_parens: true,
        ..Config::default()
    };
    let formatted = format_source(source, &config).unwrap();
    insta::assert_snapshot!(formatted.replace('\t', "\\t"), @r"
    if(ON)
    \ttarget_compile_definitions(mylib # target
    \t\tINTERFACE
    \t\t\t# definition
    \t\t\tUSING_DLL COMMON_USING_DLL EXTRA_DEFINE
    \t)
    endif()
    ");
    assert!(formatted.contains("\n\t\tINTERFACE\n"), "{formatted:?}");
    assert!(formatted.contains("\n\t)\n"), "{formatted:?}");
}

#[test]
fn wrap_first_alignment_loads_files_and_obeys_override_precedence() {
    for (extension, contents) in [
        ("yaml", "format:\n  line_width: 50\n  always_wrap: [set]\n  wrap_after_first_arg_align: default-indent\ncommands:\n  set:\n    layout:\n      wrap_after_first_arg_align: under-first-arg\nper_command_overrides:\n  set:\n    wrap_after_first_arg_align: default-indent\n"),
        ("toml", "[format]\nline_width = 50\nalways_wrap = ['set']\nwrap_after_first_arg_align = 'default-indent'\n[commands.set.layout]\nwrap_after_first_arg_align = 'under-first-arg'\n[per_command_overrides.set]\nwrap_after_first_arg_align = 'default-indent'\n"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(format!(".cmakefmt.{extension}"));
        std::fs::write(&path, contents).unwrap();
        let mut config = Config::from_file(&path).unwrap();
        let mut registry = CommandRegistry::load().unwrap();
        registry.merge_override_file(&path).unwrap();
        let source = "set(SOURCES first.cpp second.cpp third.cpp fourth.cpp fifth.cpp)\n";
        let default_indent = "set(SOURCES\n  first.cpp second.cpp third.cpp fourth.cpp\n  fifth.cpp)\n";
        let under_first_arg = "set(SOURCES\n    first.cpp second.cpp third.cpp fourth.cpp\n    fifth.cpp)\n";
        // Per-command config beats the spec, which beats the global setting.
        assert_eq!(config.wrap_after_first_arg_align, WrapAfterFirstArgAlign::DefaultIndent);
        assert_eq!(format_source_with_registry(source, &config, &registry).unwrap(), default_indent);
        config.per_command_overrides.clear();
        assert_eq!(format_source_with_registry(source, &config, &registry).unwrap(), under_first_arg);
        assert_eq!(format_source(source, &config).unwrap(), default_indent);

        let invalid = contents.replace("default-indent", "same-indent");
        std::fs::write(&path, invalid).unwrap();
        assert!(Config::from_file(&path).is_err());
    }
}
