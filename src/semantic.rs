// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Semantic-level normalisation for parsed CMake.
//!
//! Helpers that strip away parts of the AST that don't affect CMake
//! semantics — comments, line endings, keyword casing — so two files
//! can be compared "would they behave identically?" without worrying
//! about cosmetic-only differences.
//!
//! Used by:
//!
//! - `cmakefmt --verify` and `cmakefmt --in-place` (in `main.rs`)
//!   to confirm formatted output preserves CMake semantics.
//! - `tests/idempotency.rs` to assert formatter idempotency on the
//!   real-world corpus.
//!
//! Previously these helpers were duplicated between `main.rs` and the
//! integration test, with a hand-coded "keep in sync" comment that
//! Phase 47g's deduplication audit flagged. They now live here as the
//! single source of truth.
//!
//! All helpers walk a fully-parsed `CommandInvocation` in place. The
//! public surface is `normalize_command_literals` (strip cosmetic
//! differences from a single command) and `normalize_keyword_args`
//! (uppercase known keyword tokens for case-insensitive comparison).
//! Internal helpers stay private to the module.

use std::collections::BTreeSet;

use crate::parser::ast::{Argument, CommandInvocation};
use crate::spec::registry::CommandRegistry;
use crate::spec::{CommandForm, KwargSpec};

/// Strip comment and line-ending differences from a parsed
/// `CommandInvocation` so two semantically-equivalent commands
/// compare equal regardless of cosmetic formatting.
pub fn normalize_command_literals(command: &mut CommandInvocation) {
    // Strip trailing and inline comments — they have no CMake semantic
    // meaning.
    command.trailing_comment = None;
    command
        .arguments
        .retain(|a| !matches!(a, Argument::InlineComment(_)));

    for argument in &mut command.arguments {
        match argument {
            Argument::Bracket(bracket) => normalize_line_endings(&mut bracket.raw),
            Argument::Quoted(value) | Argument::Unquoted(value) => normalize_line_endings(value),
            Argument::InlineComment(_) => unreachable!(),
        }
    }
}

/// Uppercase any unquoted argument that matches a known keyword for
/// the command's spec. CMake keywords are case-insensitive at the
/// language level, so two files that differ only in the casing of
/// `PUBLIC` vs `public` are semantically equivalent.
pub fn normalize_keyword_args(command: &mut CommandInvocation, registry: &CommandRegistry) {
    let spec = registry.get(&command.name);
    let first_arg = command.arguments.iter().find_map(first_arg_text);
    let form = spec.form_for(first_arg);
    let keyword_set = collect_keywords(form);

    for arg in &mut command.arguments {
        if let Argument::Unquoted(value) = arg {
            let upper = value.to_ascii_uppercase();
            if keyword_set.contains(upper.as_str()) {
                *value = upper;
            }
        }
    }
}

/// Strip Windows-style `\r\n` line endings to plain `\n` in place.
///
/// Public because callers normalise `TemplatePlaceholder` statements
/// directly (these aren't `CommandInvocation`s and so don't go through
/// [`normalize_command_literals`]).
pub fn normalize_line_endings(value: &mut String) {
    if value.contains('\r') {
        *value = value.replace("\r\n", "\n");
    }
}

fn first_arg_text(argument: &Argument) -> Option<&str> {
    match argument {
        Argument::Quoted(_) | Argument::Bracket(_) | Argument::InlineComment(_) => None,
        Argument::Unquoted(value) => Some(value.as_str()),
    }
}

fn collect_keywords(form: &CommandForm) -> BTreeSet<String> {
    let mut keywords = BTreeSet::new();
    collect_form_keywords(form, &mut keywords);
    keywords
}

fn collect_form_keywords(form: &CommandForm, keywords: &mut BTreeSet<String>) {
    keywords.extend(form.flags.iter().cloned());

    for (name, spec) in &form.kwargs {
        keywords.insert(name.clone());
        collect_kwarg_keywords(spec, keywords);
    }
}

fn collect_kwarg_keywords(spec: &KwargSpec, keywords: &mut BTreeSet<String>) {
    keywords.extend(spec.flags.iter().cloned());

    for (name, child) in &spec.kwargs {
        keywords.insert(name.clone());
        collect_kwarg_keywords(child, keywords);
    }
}
