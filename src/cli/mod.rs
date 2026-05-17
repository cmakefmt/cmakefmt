// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Implementation modules for the `cmakefmt` binary.
//!
//! The `Cli` definition and top-level dispatch live in `main.rs`; the helpers
//! that drive discovery, formatting, reporting, and diff rendering live in the
//! submodules here. Nothing in this module is meant to be reachable from the
//! library crate — keep visibility limited to `pub(crate)` so the split stays
//! an implementation detail of the binary.

pub(crate) mod commands;
pub(crate) mod diff;
pub(crate) mod errors;
pub(crate) mod process;
pub(crate) mod report;
pub(crate) mod runtime;
pub(crate) mod spec_coverage;
pub(crate) mod summary;
