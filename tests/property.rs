// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Property-based tests for the formatter.
//!
//! These tests use `proptest` to generate a wide variety of CMake-like inputs
//! and assert invariants that must hold for *any* input:
//!
//! 1. **Idempotency**: `format(format(x)) == format(x)`
//! 2. **Determinism**: formatting the same input twice yields the same result
//! 3. **Semantic preservation**: the re-parsed token stream of the formatted
//!    output matches the original (modulo comments/whitespace)
//! 4. **No panics**: the formatter never crashes on valid CMake
//!
//! The generators below cover the constructs most likely to stress the
//! formatter's structural decisions — control flow, quoted/bracket/generator
//! arguments, multiline literals, and comments — across a matrix of
//! configurations (line width, indentation, sorting, dangling parens, and case
//! styles).

use cmakefmt::semantic::semantic_equivalent;
use cmakefmt::{format_source, CaseStyle, Config};
use proptest::prelude::*;

/// Strategy: a simple unquoted argument / identifier.
fn simple_arg() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z_][a-zA-Z0-9_]{0,8}").unwrap()
}

/// Strategy: a single argument, drawn from the forms a real CMake call uses.
///
/// All forms are constructed to be valid, balanced CMake so the parser accepts
/// them: quoted strings use a safe character class (no escapes/newlines),
/// bracket arguments are balanced `[[ ... ]]`, and generator expressions are
/// chosen from a fixed set of well-formed examples.
fn argument() -> impl Strategy<Value = String> {
    prop_oneof![
        5 => simple_arg(),
        2 => prop::string::string_regex(r#""[a-zA-Z0-9 _]{0,10}""#).unwrap(),
        1 => prop::string::string_regex(r"\[\[[a-zA-Z0-9 _]{0,10}\]\]").unwrap(),
        1 => prop::sample::select(vec![
            "\"line one\n    line two\"".to_string(),
            "[[\n    literal line\n]]".to_string(),
        ]),
        1 => prop::sample::select(vec![
            "$<CONFIG:Debug>".to_string(),
            "$<BOOL:1>".to_string(),
            "$<TARGET_FILE:foo>".to_string(),
            "${VAR}".to_string(),
            "$ENV{HOME}".to_string(),
        ]),
    ]
}

/// Strategy: a plain command invocation with 0-5 mixed arguments.
fn command() -> impl Strategy<Value = String> {
    (
        prop::string::string_regex("[a-z_][a-z0-9_]{0,12}").unwrap(),
        prop::collection::vec(argument(), 0..5),
    )
        .prop_map(|(name, args)| format!("{}({})", name, args.join(" ")))
}

/// Strategy: a standalone line comment.
fn comment() -> impl Strategy<Value = String> {
    prop::string::string_regex("# [a-zA-Z0-9 ]{0,15}").unwrap()
}

/// Strategy: the inner body of a control-flow block — 1-3 commands, one per
/// line. Defined as its own function (rather than a local binding) so each
/// `statement` arm can build a fresh, independent body strategy; the
/// `prop_map` closure makes the strategy non-`Clone`.
fn block_body() -> impl Strategy<Value = String> {
    prop::collection::vec(command(), 1..4).prop_map(|cmds| cmds.join("\n"))
}

/// Strategy: a single statement — a command, a comment, or a balanced
/// control-flow block (`if`/`endif`, `foreach`/`endforeach`) wrapping a few
/// inner commands. Blocks are always balanced so the result parses.
fn statement() -> impl Strategy<Value = String> {
    prop_oneof![
        6 => command(),
        2 => comment(),
        1 => block_body().prop_map(|b| format!("if(SOMETHING)\n{b}\nendif()")),
        1 => block_body().prop_map(|b| format!("foreach(x IN ITEMS a b c)\n{b}\nendforeach()")),
    ]
}

/// Strategy: a whole program — 1-8 statements, each on its own line.
fn program() -> impl Strategy<Value = String> {
    prop::collection::vec(statement(), 1..8).prop_map(|stmts| stmts.join("\n"))
}

/// Strategy: a configuration drawn from a matrix of the options most likely to
/// interact with the generated constructs.
fn config() -> impl Strategy<Value = Config> {
    (
        prop::sample::select(vec![40usize, 80, 120]),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        prop::sample::select(vec![2usize, 4]),
        prop::sample::select(vec![
            CaseStyle::Lower,
            CaseStyle::Upper,
            CaseStyle::Unchanged,
        ]),
        prop::sample::select(vec![
            CaseStyle::Lower,
            CaseStyle::Upper,
            CaseStyle::Unchanged,
        ]),
    )
        .prop_map(
            |(
                line_width,
                enable_sort,
                autosort,
                dangle_parens,
                use_tabchars,
                tab_size,
                command_case,
                keyword_case,
            )| {
                Config {
                    line_width,
                    enable_sort,
                    autosort,
                    dangle_parens,
                    use_tabchars,
                    tab_size,
                    command_case,
                    keyword_case,
                    ..Config::default()
                }
            },
        )
}

proptest! {
    // Generated inputs are small and structured; 256 cases keeps the suite fast
    // while still exploring a wide variety of argument shapes and configs.
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Idempotency: formatting an already-formatted document is a no-op, across
    /// the config matrix.
    #[test]
    fn format_is_idempotent(source in program(), config in config()) {
        let Ok(formatted) = format_source(&source, &config) else {
            return Ok(());
        };
        let twice = format_source(&formatted, &config)
            .expect("formatting already-formatted output must succeed");
        prop_assert_eq!(formatted, twice);
    }

    /// Determinism: same input, same config, same output.
    #[test]
    fn format_is_deterministic(source in program(), config in config()) {
        let first = format_source(&source, &config);
        let second = format_source(&source, &config);
        prop_assert_eq!(first.ok(), second.ok());
    }

    /// Semantic preservation: formatting never changes the parsed CMake
    /// program (comments, whitespace, and cosmetic casing aside).
    #[test]
    fn format_preserves_semantics(source in program(), config in config()) {
        let Ok(formatted) = format_source(&source, &config) else {
            return Ok(());
        };
        prop_assert!(semantic_equivalent(&source, &formatted));
    }

    /// No panics: the formatter handles any generated input without crashing.
    #[test]
    fn format_never_panics(source in program(), config in config()) {
        let _ = format_source(&source, &config);
    }
}
