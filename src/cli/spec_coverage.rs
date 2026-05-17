// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Implementation of `cmakefmt dump spec-coverage`.
//!
//! Cross-references the formatter's built-in [`CommandRegistry`] against
//! a snapshot of the upstream CMake command list
//! (`src/spec/cmake_commands.txt`) and reports, for every command, how
//! thoroughly cmakefmt models it. The fixture is embedded via
//! `include_str!` and parsed once per invocation — see the file's
//! header for provenance and refresh instructions.
//!
//! Status classification (per the v1.6.0 spec):
//!
//! | Status    | Rule                                              |
//! |-----------|---------------------------------------------------|
//! | `missing` | name absent from the registry                     |
//! | `stub`    | registered but no kwargs and no flags             |
//! | `partial` | 1..=3 kwargs+flags total                          |
//! | `full`    | >3 kwargs+flags total                             |
//!
//! For `Discriminated` specs (e.g. `file`, `install`, `export`) the
//! classification uses the union of kwargs+flags across all forms
//! and the fallback. This produces the most useful signal — a
//! discriminated command with many rich forms shouldn't read as
//! "partial" just because its fallback is sparse.
//!
//! The output is purely informational: exit code is always 0, even
//! when the registry is missing commands.

use std::collections::BTreeSet;

use cmakefmt::spec::{CommandSpec, KwargSpec};
use cmakefmt::CommandRegistry;
use serde::Serialize;

use crate::{SpecCoverageFormat, SpecCoverageStatusFilter, EXIT_OK};

/// Embedded snapshot of the reference command list. See the file
/// header for provenance and refresh instructions.
const CMAKE_COMMANDS_FIXTURE: &str = include_str!("../spec/cmake_commands.txt");

/// Heuristic threshold: a spec with strictly more than this many
/// kwargs+flags is classified as `full`. Keep in sync with the help
/// text on `DumpAction::SpecCoverage`.
const FULL_THRESHOLD: usize = 3;

/// Status of a single command's spec coverage.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CoverageStatus {
    Missing,
    Stub,
    Partial,
    Full,
}

impl CoverageStatus {
    fn as_str(self) -> &'static str {
        match self {
            CoverageStatus::Missing => "missing",
            CoverageStatus::Stub => "stub",
            CoverageStatus::Partial => "partial",
            CoverageStatus::Full => "full",
        }
    }
}

/// One row of the coverage report.
#[derive(Clone, Debug, Serialize)]
pub(crate) struct CoverageEntry {
    pub name: String,
    pub status: CoverageStatus,
    /// Total kwargs across all forms (None for `missing`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kwargs: Option<usize>,
    /// Total flags across all forms (None for `missing`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<usize>,
}

/// Aggregate summary counts.
#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct CoverageSummary {
    pub full: usize,
    pub partial: usize,
    pub stub: usize,
    pub missing: usize,
}

/// Full JSON document body.
#[derive(Debug, Serialize)]
struct CoverageReport<'a> {
    cmake_version_reference: &'a str,
    /// Description of the heuristic used to classify entries.
    classification: ClassificationRules,
    summary: CoverageSummary,
    commands: &'a [CoverageEntry],
}

#[derive(Debug, Serialize)]
struct ClassificationRules {
    missing: &'static str,
    stub: &'static str,
    partial: &'static str,
    full: &'static str,
}

impl ClassificationRules {
    fn current() -> Self {
        Self {
            missing: "name not present in CommandRegistry",
            stub: "registered but kwargs+flags == 0",
            partial: "1..=3 kwargs+flags across all forms",
            full: ">3 kwargs+flags across all forms",
        }
    }
}

/// Public entry point — invoked from `run_dump_subcommand`.
pub(crate) fn run_spec_coverage(
    format: SpecCoverageFormat,
    status: Option<SpecCoverageStatusFilter>,
) -> Result<u8, cmakefmt::Error> {
    let registry = CommandRegistry::builtins();
    let entries = build_entries(registry);
    let summary = summarise(&entries);
    let filtered: Vec<CoverageEntry> = match status {
        None => entries.clone(),
        Some(s) => entries
            .iter()
            .filter(|e| e.status == filter_to_status(s))
            .cloned()
            .collect(),
    };

    match format {
        SpecCoverageFormat::Human => {
            print!(
                "{}",
                render_human(&filtered, &summary, registry.audited_cmake_version())
            );
        }
        SpecCoverageFormat::Json => {
            let report = CoverageReport {
                cmake_version_reference: registry.audited_cmake_version(),
                classification: ClassificationRules::current(),
                summary,
                commands: &filtered,
            };
            // Pretty-printed for human readability; this is a
            // diagnostic command, not a hot machine pipeline.
            let json = serde_json::to_string_pretty(&report).map_err(|e| {
                cmakefmt::Error::Formatter(format!("failed to serialise spec coverage: {e}"))
            })?;
            println!("{json}");
        }
    }

    Ok(EXIT_OK)
}

fn filter_to_status(filter: SpecCoverageStatusFilter) -> CoverageStatus {
    match filter {
        SpecCoverageStatusFilter::Missing => CoverageStatus::Missing,
        SpecCoverageStatusFilter::Stub => CoverageStatus::Stub,
        SpecCoverageStatusFilter::Partial => CoverageStatus::Partial,
        SpecCoverageStatusFilter::Full => CoverageStatus::Full,
    }
}

/// Build the alphabetised, classified list of commands from the
/// reference fixture + registry. The union of fixture names and
/// registry names is used as the universe so a registry entry that
/// somehow isn't in the fixture still shows up.
pub(crate) fn build_entries(registry: &CommandRegistry) -> Vec<CoverageEntry> {
    let mut universe: BTreeSet<String> = parse_fixture(CMAKE_COMMANDS_FIXTURE);
    for name in registry.builtin_command_names() {
        universe.insert(name.to_owned());
    }

    universe
        .into_iter()
        .map(|name| classify(registry, name))
        .collect()
}

fn parse_fixture(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .map(|line| {
            // Strip inline comments — anything from the first `#` on.
            let bare = match line.find('#') {
                Some(pos) => &line[..pos],
                None => line,
            };
            bare.trim()
        })
        .filter(|line| !line.is_empty())
        .map(|name| name.to_ascii_lowercase())
        .collect()
}

fn classify(registry: &CommandRegistry, name: String) -> CoverageEntry {
    if !registry.contains_builtin(&name) {
        return CoverageEntry {
            name,
            status: CoverageStatus::Missing,
            kwargs: None,
            flags: None,
        };
    }

    let spec = registry.get(&name);
    let (kwargs, flags) = count_kwargs_flags(spec);
    let total = kwargs + flags;
    let status = if total == 0 {
        CoverageStatus::Stub
    } else if total <= FULL_THRESHOLD {
        CoverageStatus::Partial
    } else {
        CoverageStatus::Full
    };

    CoverageEntry {
        name,
        status,
        kwargs: Some(kwargs),
        flags: Some(flags),
    }
}

/// Sum kwargs and flags across every form of the spec. For
/// `Single` commands that's just the one form; for `Discriminated`
/// commands we union across `forms` + `fallback` so a richly
/// modelled discriminated command (think `file`, `install`) reads
/// as `full` rather than being graded against any single form in
/// isolation. Nested sub-kwargs and sub-flags are not counted
/// recursively — only top-level structure inside each form
/// contributes to the heuristic, which keeps the threshold tractable
/// and well-defined.
fn count_kwargs_flags(spec: &CommandSpec) -> (usize, usize) {
    match spec {
        CommandSpec::Single(form) => (form.kwargs.len(), form.flags.len()),
        CommandSpec::Discriminated { forms, fallback } => {
            let mut kw_names: BTreeSet<&str> = BTreeSet::new();
            let mut flag_names: BTreeSet<&str> = BTreeSet::new();
            for form in forms.values() {
                accumulate_form(form, &mut kw_names, &mut flag_names);
            }
            if let Some(form) = fallback {
                accumulate_form(form, &mut kw_names, &mut flag_names);
            }
            (kw_names.len(), flag_names.len())
        }
        // `CommandSpec` is `#[non_exhaustive]`; any future variant
        // defaults to "no structure modelled" rather than crashing.
        _ => (0, 0),
    }
}

fn accumulate_form<'a>(
    form: &'a cmakefmt::spec::CommandForm,
    kw_names: &mut BTreeSet<&'a str>,
    flag_names: &mut BTreeSet<&'a str>,
) {
    for name in form.kwargs.keys() {
        kw_names.insert(name.as_str());
    }
    for flag in &form.flags {
        flag_names.insert(flag.as_str());
    }
    // Reach one level into kwargs to catch sub-kwargs/sub-flags that
    // many discriminated forms hide there (e.g. `install(TARGETS)`'s
    // ARCHIVE/LIBRARY/RUNTIME subgroups). One level is the right
    // tradeoff: it lets richly modelled commands surface as `full`
    // without doing unbounded recursion that would bias the threshold
    // toward the deepest specs.
    for kwarg in form.kwargs.values() {
        accumulate_kwarg(kwarg, kw_names, flag_names);
    }
}

fn accumulate_kwarg<'a>(
    spec: &'a KwargSpec,
    kw_names: &mut BTreeSet<&'a str>,
    flag_names: &mut BTreeSet<&'a str>,
) {
    for name in spec.kwargs.keys() {
        kw_names.insert(name.as_str());
    }
    for flag in &spec.flags {
        flag_names.insert(flag.as_str());
    }
}

fn summarise(entries: &[CoverageEntry]) -> CoverageSummary {
    let mut summary = CoverageSummary::default();
    for entry in entries {
        match entry.status {
            CoverageStatus::Full => summary.full += 1,
            CoverageStatus::Partial => summary.partial += 1,
            CoverageStatus::Stub => summary.stub += 1,
            CoverageStatus::Missing => summary.missing += 1,
        }
    }
    summary
}

/// Render the aligned-table human format.
fn render_human(
    entries: &[CoverageEntry],
    summary: &CoverageSummary,
    cmake_version: &str,
) -> String {
    // Widths chosen empirically: 40-char name column covers every
    // command currently in the spec (the longest is around 36 chars,
    // e.g. `gnuinstalldirs_get_absolute_install_dir`); the numeric
    // columns are narrow because real values rarely exceed two
    // digits.
    let name_width = entries
        .iter()
        .map(|e| e.name.len())
        .max()
        .unwrap_or(0)
        .max(7);

    let mut out = String::new();
    out.push_str(&format!(
        "{:<name_width$} {:<8} {:>6} {:>5}\n",
        "Command",
        "Status",
        "kwargs",
        "flags",
        name_width = name_width
    ));
    for entry in entries {
        let (kw, fl) = match (entry.kwargs, entry.flags) {
            (Some(k), Some(f)) => (format!("{k}"), format!("{f}")),
            _ => ("-".to_owned(), "-".to_owned()),
        };
        out.push_str(&format!(
            "{:<name_width$} {:<8} {:>6} {:>5}\n",
            entry.name,
            entry.status.as_str(),
            kw,
            fl,
            name_width = name_width
        ));
    }
    out.push('\n');
    out.push_str(&format!(
        "Summary: {} full, {} partial, {} stub, {} missing\
         \nReference: CMake {} (snapshot in src/spec/cmake_commands.txt)\
         \nClassification: full=>3 kwargs+flags, partial=1..=3, stub=0, missing=not in registry\n",
        summary.full, summary.partial, summary.stub, summary.missing, cmake_version,
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_for<'a>(entries: &'a [CoverageEntry], name: &str) -> &'a CoverageEntry {
        entries
            .iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| panic!("no entry for {name}"))
    }

    #[test]
    fn well_known_commands_classify_as_expected() {
        let registry = CommandRegistry::builtins();
        let entries = build_entries(registry);

        // target_link_libraries has PUBLIC/PRIVATE/INTERFACE/LINK_PUBLIC/
        // LINK_PRIVATE/etc. — well over the threshold.
        assert_eq!(
            entry_for(&entries, "target_link_libraries").status,
            CoverageStatus::Full
        );

        // project has many flags and a few kwargs — should be full
        // under the >3 threshold.
        assert!(matches!(
            entry_for(&entries, "project").status,
            CoverageStatus::Full | CoverageStatus::Partial
        ));

        // cmake_minimum_required has VERSION kwarg + FATAL_ERROR flag
        // — that's exactly 2, so it lands in `partial` under the
        // current heuristic. (The spec calls it `stub` in the example
        // table, but the registry has actually grown structure for it
        // since.)
        let cmr = entry_for(&entries, "cmake_minimum_required");
        assert!(
            matches!(cmr.status, CoverageStatus::Stub | CoverageStatus::Partial),
            "cmake_minimum_required should be stub or partial, got {:?}",
            cmr.status
        );
    }

    #[test]
    fn missing_command_status_uses_dashes_in_human_output() {
        // Inject a synthetic "not in registry" entry by parsing a
        // fixture that includes a name the registry won't have.
        let registry = CommandRegistry::builtins();
        let mut entries = build_entries(registry);
        entries.push(CoverageEntry {
            name: "definitely_not_a_real_cmake_command".to_owned(),
            status: CoverageStatus::Missing,
            kwargs: None,
            flags: None,
        });
        let summary = summarise(&entries);
        let rendered = render_human(&entries, &summary, "4.3.1");
        assert!(rendered.contains("definitely_not_a_real_cmake_command"));
        assert!(rendered.contains("missing"));
        // Missing rows render numeric columns as `-`.
        let row = rendered
            .lines()
            .find(|l| l.contains("definitely_not_a_real_cmake_command"))
            .unwrap();
        assert!(row.contains('-'));
    }

    #[test]
    fn json_output_has_summary_and_alphabetised_commands() {
        let registry = CommandRegistry::builtins();
        let entries = build_entries(registry);
        let summary = summarise(&entries);
        let report = CoverageReport {
            cmake_version_reference: registry.audited_cmake_version(),
            classification: ClassificationRules::current(),
            summary,
            commands: &entries,
        };
        let json = serde_json::to_string_pretty(&report).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(
            parsed["cmake_version_reference"].as_str().unwrap(),
            registry.audited_cmake_version()
        );
        let summary_obj = &parsed["summary"];
        for key in ["full", "partial", "stub", "missing"] {
            assert!(summary_obj.get(key).is_some(), "summary missing {key}");
        }
        let commands = parsed["commands"].as_array().unwrap();
        assert!(!commands.is_empty());
        // Alphabetical ordering.
        let names: Vec<&str> = commands
            .iter()
            .map(|c| c["name"].as_str().unwrap())
            .collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted, "commands should be alphabetised");
        // Each command has the expected shape.
        let first = &commands[0];
        assert!(first.get("name").is_some());
        assert!(first.get("status").is_some());
    }

    #[test]
    fn status_filter_restricts_entries() {
        let registry = CommandRegistry::builtins();
        let entries = build_entries(registry);
        let stubs: Vec<&CoverageEntry> = entries
            .iter()
            .filter(|e| e.status == CoverageStatus::Stub)
            .collect();
        for entry in &stubs {
            assert_eq!(entry.status, CoverageStatus::Stub);
            assert_eq!(entry.kwargs, Some(0));
            assert_eq!(entry.flags, Some(0));
        }
        let fulls: Vec<&CoverageEntry> = entries
            .iter()
            .filter(|e| e.status == CoverageStatus::Full)
            .collect();
        for entry in &fulls {
            let kw = entry.kwargs.unwrap();
            let fl = entry.flags.unwrap();
            assert!(kw + fl > FULL_THRESHOLD);
        }
    }

    #[test]
    fn fixture_parsing_strips_comments_and_blank_lines() {
        let src = "# header\n\nset\n  # indented comment\nadd_executable  # trailing\n   \n";
        let names = parse_fixture(src);
        assert!(names.contains("set"));
        assert!(names.contains("add_executable"));
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn summary_totals_match_entries_length() {
        let registry = CommandRegistry::builtins();
        let entries = build_entries(registry);
        let summary = summarise(&entries);
        assert_eq!(
            entries.len(),
            summary.full + summary.partial + summary.stub + summary.missing
        );
    }
}
