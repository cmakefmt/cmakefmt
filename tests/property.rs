// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Property-based tests for the formatter.
//!
//! These complement the snapshot suite by exercising the formatter against
//! generated CMake inputs and asserting invariants that should hold for
//! *every* input, not just the ones a human happened to write a fixture for.
//!
//! Properties currently enforced (with the default [`Config`]):
//!
//! 1. **Token sequence preservation** — the ordered sequence of non-comment
//!    tokens (command names + their unquoted/quoted/bracket arguments) in
//!    the formatted output must match the sequence in the input. Catches
//!    any formatter pass that drops, duplicates, reorders, or rewrites a
//!    semantic token. (Autosort-class bugs that reorder positional tokens
//!    would surface here under a non-default config; with `enable_sort:
//!    false` and `autosort: false` — the defaults — no reordering is ever
//!    legal.)
//!
//! 2. **Idempotency** — `format(format(x)) == format(x)`. The formatter
//!    must reach a fixed point in one pass. Catches oscillating layouts
//!    and "almost-stable" formatting decisions that flip on re-application.
//!
//! Property tests are run with a modest default case count
//! (`PROPTEST_CASES=64`) so they fit comfortably in the standard `cargo
//! test` time budget. Set `PROPTEST_CASES` higher locally for deeper
//! exploration, e.g. `PROPTEST_CASES=2048 cargo test --test property`.

use cmakefmt::parser::ast::{Argument, Statement};
use cmakefmt::{format_source, parser, Config};
use proptest::prelude::*;

/// Collect the ordered sequence of *semantic* tokens (command names and
/// their non-comment arguments) from a CMake source string. Used as the
/// equivalence baseline for token-preservation properties.
///
/// Inline and standalone comments are deliberately excluded — comment
/// reflow legitimately changes byte content (wrapping long lines, etc.)
/// even when nothing semantic moves.
///
/// Command names and unquoted argument text are folded to ASCII lowercase
/// because the formatter is permitted to normalise casing under
/// `command_case` / `keyword_case`; with the default config (`lower` /
/// `upper`) the case may legitimately differ between input and output.
fn collect_token_seq(src: &str) -> Vec<String> {
    let file = parser::parse(src).expect("generator should produce parseable CMake");
    let mut tokens = Vec::new();
    for stmt in &file.statements {
        if let Statement::Command(cmd) = stmt {
            tokens.push(cmd.name.to_ascii_lowercase());
            for arg in &cmd.arguments {
                match arg {
                    Argument::Bracket(b) => tokens.push(b.raw.clone()),
                    Argument::Quoted(s) => tokens.push(s.clone()),
                    Argument::Unquoted(s) => tokens.push(s.to_ascii_lowercase()),
                    Argument::InlineComment(_) => {}
                }
            }
        }
    }
    tokens
}

// ── Generators ───────────────────────────────────────────────────────────

/// Simple unquoted identifier. Constrained to lowercase ASCII so the
/// case-fold in `collect_token_seq` is the only normalisation needed for
/// equivalence.
fn arb_token() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,12}"
}

/// Sequence of 0–5 argument tokens.
fn arb_args() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(arb_token(), 0..6)
}

/// One of a handful of commonly-used CMake commands. Chosen to exercise
/// the most-traffic specs in `builtins.yaml` (target_*, set, message,
/// include, etc.). Control-flow constructs (`if`/`endif`, `foreach`/…)
/// are deliberately excluded because they require structural balance
/// that a flat command-list generator can't produce.
fn arb_command_name() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("set"),
        Just("message"),
        Just("target_link_libraries"),
        Just("target_compile_options"),
        Just("target_include_directories"),
        Just("add_executable"),
        Just("add_library"),
        Just("add_subdirectory"),
        Just("include"),
        Just("option"),
    ]
}

/// `command_name(arg1 arg2 …)`. Always emits at least one argument so the
/// resulting source parses cleanly; commands like `target_link_libraries()`
/// with zero arguments are syntactically valid but semantically suspicious
/// and would distract from the formatter properties under test.
fn arb_command() -> impl Strategy<Value = String> {
    (arb_command_name(), arb_args()).prop_map(|(name, args)| {
        let body = if args.is_empty() {
            "x".to_owned()
        } else {
            args.join(" ")
        };
        format!("{name}({body})")
    })
}

/// 0–10 commands joined by newlines, with a trailing newline so the input
/// is well-formed text. Empty inputs (zero commands) are explicitly
/// allowed — the formatter must be a fixed point on `""` too.
fn arb_cmake() -> impl Strategy<Value = String> {
    prop::collection::vec(arb_command(), 0..10).prop_map(|cmds| {
        let mut s = cmds.join("\n");
        s.push('\n');
        s
    })
}

// ── Properties ────────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig {
        // Default to 64 cases per property so the suite runs in well under
        // a second on typical hardware. Override with PROPTEST_CASES=…
        // for deeper exploration locally.
        cases: 64,
        ..ProptestConfig::default()
    })]

    /// The formatter must preserve the ordered sequence of semantic
    /// tokens. With the default config (`enable_sort: false`,
    /// `autosort: false`) no reordering is legal — any divergence here
    /// indicates the formatter has dropped, duplicated, rewritten, or
    /// shuffled a token between input and output.
    #[test]
    fn formatter_preserves_token_sequence(src in arb_cmake()) {
        let config = Config::default();
        let formatted = format_source(&src, &config).expect("format should succeed on generated CMake");
        prop_assert_eq!(
            collect_token_seq(&src),
            collect_token_seq(&formatted),
            "token sequence changed:\ninput:\n{}\nformatted:\n{}",
            src,
            formatted
        );
    }

    /// `format(format(x)) == format(x)` for every generated input.
    /// Tests the formatter's idempotency contract against a far wider
    /// surface than the four-file fixture corpus.
    #[test]
    fn formatter_is_idempotent(src in arb_cmake()) {
        let config = Config::default();
        let once = format_source(&src, &config).expect("first format should succeed");
        let twice = format_source(&once, &config).expect("second format should succeed");
        prop_assert_eq!(once, twice);
    }
}
