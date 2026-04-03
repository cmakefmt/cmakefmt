use std::collections::BTreeSet;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use cmakefmt::spec::registry::CommandRegistry;
use cmakefmt::{
    convert_legacy_config_files, default_config_template, files::discover_cmake_files,
    format_source_with_registry, format_source_with_registry_debug, CaseStyle, Config,
};
use rayon::prelude::*;
use regex::Regex;

const LONG_ABOUT: &str = "

Parse CMake listfiles and format them nicely.

Formatting is configurable with one or more TOML configuration files. If no
config file is specified on the command line, cmakefmt will try to find the
nearest .cmakefmt.toml for each input by walking up through parent directories
to the repository root or filesystem root. If no project-local config exists,
cmakefmt falls back to ~/.cmakefmt.toml when present.

cmakefmt can print a commented starter configuration for you as a customization
starting point with --print-default-config.

Legacy cmake-format JSON, YAML, and Python config files can be converted to
.cmakefmt.toml with --convert-legacy-config.";

/// A fast, correct CMake formatter.
#[derive(Parser, Debug)]
#[command(
    name = "cmakefmt",
    version,
    about = "Parse CMake listfiles and format them nicely.",
    long_about = LONG_ABOUT
)]
struct Cli {
    /// Files or directories to format. Use `-` for stdin.
    ///
    /// If omitted, `cmakefmt` recursively finds CMake files under the current
    /// working directory.
    files: Vec<String>,

    /// Format files in-place (modifies the files on disk).
    #[arg(
        short = 'i',
        long = "in-place",
        conflicts_with = "list_files",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    in_place: bool,

    /// Check if files are already formatted (exit 1 if not).
    #[arg(
        long,
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    check: bool,

    /// List the files that would be reformatted without changing them.
    #[arg(
        long = "list-files",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    list_files: bool,

    /// Regex filter applied to discovered CMake file paths.
    #[arg(
        long = "path-regex",
        value_name = "REGEX",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    file_regex: Option<String>,

    /// Print the default config template and exit.
    #[arg(long = "print-default-config")]
    dump_config: bool,

    /// Convert legacy cmake-format config files to `.cmakefmt.toml` and print
    /// the result to stdout.
    #[arg(
        long = "convert-legacy-config",
        value_name = "PATH",
        conflicts_with = "dump_config",
        conflicts_with = "check",
        conflicts_with = "list_files",
        conflicts_with = "in_place",
        conflicts_with = "debug",
        conflicts_with = "parallel",
        conflicts_with = "config_paths",
        conflicts_with = "line_width",
        conflicts_with = "tab_size",
        conflicts_with = "command_case",
        conflicts_with = "keyword_case",
        conflicts_with = "dangle_parens"
    )]
    convert_config_paths: Vec<PathBuf>,

    /// Print debug diagnostics about discovery, config resolution, barriers,
    /// and formatter decisions.
    #[arg(
        long,
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    debug: bool,

    /// Control ANSI colour for highlighted changed output lines.
    #[arg(long = "colour", alias = "color", value_enum, default_value_t = ColorChoice::Auto)]
    colour: ColorChoice,

    /// Format files in parallel when explicitly requested.
    ///
    /// If omitted entirely, formatting stays single-threaded. If provided
    /// without a value, cmakefmt uses the available CPU count.
    #[arg(
        short = 'j',
        long,
        value_name = "JOBS",
        num_args = 0..=1,
        default_missing_value = "0",
        conflicts_with = "dump_config",
        conflicts_with = "convert_config_paths"
    )]
    parallel: Option<usize>,

    /// One or more config files to merge in order. Later files override earlier ones.
    #[arg(long = "config-file", visible_alias = "config", value_name = "PATH")]
    config_paths: Vec<PathBuf>,

    /// Override the maximum line width.
    #[arg(long)]
    line_width: Option<usize>,

    /// Override the number of spaces per indent level.
    #[arg(long)]
    tab_size: Option<usize>,

    /// Normalise command name case (lower, upper, unchanged).
    #[arg(long)]
    command_case: Option<CaseStyle>,

    /// Normalise keyword case (lower, upper, unchanged).
    #[arg(long)]
    keyword_case: Option<CaseStyle>,

    /// Place closing paren on its own line when wrapping.
    #[arg(long)]
    dangle_parens: Option<bool>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum ColorChoice {
    /// Use colour only when stdout is a terminal that looks colour-capable.
    Auto,
    /// Always emit ANSI colour codes.
    Always,
    /// Never emit ANSI colour codes.
    Never,
}

/// Exit codes matching the spec in ARCHITECTURE.md.
const EXIT_OK: u8 = 0;
const EXIT_CHECK_FAILED: u8 = 1;
const EXIT_ERROR: u8 = 2;

fn main() -> ExitCode {
    let cli = Cli::parse();

    match run(&cli) {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(EXIT_ERROR)
        }
    }
}

fn run(cli: &Cli) -> Result<u8, cmakefmt::Error> {
    if cli.dump_config {
        print!("{}", default_config_template());
        return Ok(EXIT_OK);
    }

    if !cli.convert_config_paths.is_empty() {
        if !cli.files.is_empty() {
            return Err(cmakefmt::Error::Formatter(
                "--convert-legacy-config does not accept formatting input paths".to_owned(),
            ));
        }
        print!(
            "{}",
            convert_legacy_config_files(&cli.convert_config_paths)?
        );
        return Ok(EXIT_OK);
    }

    let stdout_mode = !cli.list_files && !cli.check && !cli.in_place;
    let colorize_stdout = stdout_mode && should_colorize_stdout(cli.colour);
    let file_filter = compile_file_filter(cli.file_regex.as_deref())?;
    let targets = collect_targets(cli, file_filter.as_ref())?;
    let parallel_jobs = resolve_parallel_jobs(cli.parallel)?;
    let mut any_would_change = false;

    if cli.debug {
        log_debug(format!(
            "discovered {} target(s){}",
            targets.len(),
            debug_parallel_suffix(parallel_jobs)
        ));
    }

    let results = if parallel_jobs > 1 && targets.iter().all(InputTarget::is_path) {
        process_targets_parallel(&targets, cli, parallel_jobs, colorize_stdout)?
    } else {
        if cli.debug && parallel_jobs > 1 && targets.iter().any(|target| !target.is_path()) {
            log_debug("parallel mode ignored because stdin input must run serially");
        }
        process_targets_serial(&targets, cli, colorize_stdout)?
    };
    let multi_target_stdout = stdout_mode && results.len() > 1;

    for (index, result) in results.iter().enumerate() {
        if cli.debug {
            for line in &result.debug_lines {
                log_debug(line);
            }
        }

        any_would_change |= result.would_change;

        if cli.list_files {
            if result.would_change {
                println!("{}", result.display_name);
            }
        } else if cli.check {
            if result.would_change {
                eprintln!("{} would be reformatted", result.display_name);
            }
        } else if cli.in_place {
            if let Some(path) = &result.path {
                if result.would_change {
                    std::fs::write(path, &result.formatted).map_err(cmakefmt::Error::Io)?;
                }
            }
        } else {
            if multi_target_stdout {
                if index > 0 {
                    io::stdout().write_all(b"\n").map_err(cmakefmt::Error::Io)?;
                }
                write_stdout_header(&result.display_name)?;
            }
            let display_output = result
                .highlighted_output
                .as_deref()
                .unwrap_or(&result.formatted);
            io::stdout()
                .write_all(display_output.as_bytes())
                .map_err(cmakefmt::Error::Io)?;
        }
    }

    if (cli.check || cli.list_files) && any_would_change {
        Ok(EXIT_CHECK_FAILED)
    } else {
        Ok(EXIT_OK)
    }
}

fn write_stdout_header(display_name: &str) -> Result<(), cmakefmt::Error> {
    writeln!(io::stdout(), "### {display_name}").map_err(cmakefmt::Error::Io)?;
    Ok(())
}

fn compile_file_filter(pattern: Option<&str>) -> Result<Option<Regex>, cmakefmt::Error> {
    pattern
        .map(|pattern| {
            Regex::new(pattern).map_err(|err| {
                cmakefmt::Error::Formatter(format!("invalid file regex {pattern:?}: {err}"))
            })
        })
        .transpose()
}

fn collect_targets(
    cli: &Cli,
    file_filter: Option<&Regex>,
) -> Result<Vec<InputTarget>, cmakefmt::Error> {
    let inputs = if cli.files.is_empty() {
        vec![".".to_owned()]
    } else {
        cli.files.clone()
    };

    let mut targets = Vec::new();
    let mut seen_paths = BTreeSet::new();

    for input in inputs {
        if input == "-" {
            targets.push(InputTarget::Stdin);
            continue;
        }

        let path = PathBuf::from(&input);
        if path.is_file() {
            push_unique_path(&mut targets, &mut seen_paths, path);
            continue;
        }

        if path.is_dir() {
            for discovered in discover_cmake_files(&path, file_filter) {
                push_unique_path(&mut targets, &mut seen_paths, discovered);
            }
            continue;
        }

        return Err(cmakefmt::Error::Formatter(format!(
            "{}: no such file or directory",
            path.display()
        )));
    }

    Ok(targets)
}

fn push_unique_path(
    targets: &mut Vec<InputTarget>,
    seen_paths: &mut BTreeSet<PathBuf>,
    path: PathBuf,
) {
    if seen_paths.insert(path.clone()) {
        targets.push(InputTarget::Path(path));
    }
}

/// Build a formatting context by layering: defaults → config files → CLI
/// overrides, and by merging any `[commands]` spec overrides from the same
/// config files into the command registry.
fn build_context(
    cli: &Cli,
    file_path: Option<&Path>,
) -> Result<(Config, CommandRegistry, Vec<PathBuf>), cmakefmt::Error> {
    let config_sources = if !cli.config_paths.is_empty() {
        cli.config_paths.clone()
    } else if let Some(path) = file_path {
        Config::config_sources_for(path)
    } else {
        Vec::new()
    };

    let mut config = Config::from_files(&config_sources)?;
    let mut registry = CommandRegistry::builtins().clone();
    for path in &config_sources {
        registry.merge_override_file(path)?;
    }

    if let Some(v) = cli.line_width {
        config.line_width = v;
    }
    if let Some(v) = cli.tab_size {
        config.tab_size = v;
    }
    if let Some(v) = cli.command_case {
        config.command_case = v;
    }
    if let Some(v) = cli.keyword_case {
        config.keyword_case = v;
    }
    if let Some(v) = cli.dangle_parens {
        config.dangle_parens = v;
    }

    Ok((config, registry, config_sources))
}

#[derive(Clone)]
enum InputTarget {
    Stdin,
    Path(PathBuf),
}

impl InputTarget {
    fn is_path(&self) -> bool {
        matches!(self, InputTarget::Path(_))
    }
}

struct ProcessedTarget {
    path: Option<PathBuf>,
    display_name: String,
    formatted: String,
    highlighted_output: Option<String>,
    would_change: bool,
    debug_lines: Vec<String>,
}

fn process_targets_serial(
    targets: &[InputTarget],
    cli: &Cli,
    colorize_stdout: bool,
) -> Result<Vec<ProcessedTarget>, cmakefmt::Error> {
    targets
        .iter()
        .map(|target| process_target(target, cli, colorize_stdout))
        .collect()
}

fn process_targets_parallel(
    targets: &[InputTarget],
    cli: &Cli,
    parallel_jobs: usize,
    colorize_stdout: bool,
) -> Result<Vec<ProcessedTarget>, cmakefmt::Error> {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(parallel_jobs)
        .build()
        .map_err(|err| cmakefmt::Error::Formatter(format!("failed to build thread pool: {err}")))?;

    pool.install(|| {
        targets
            .par_iter()
            .map(|target| process_target(target, cli, colorize_stdout))
            .collect()
    })
}

fn process_target(
    target: &InputTarget,
    cli: &Cli,
    colorize_stdout: bool,
) -> Result<ProcessedTarget, cmakefmt::Error> {
    match target {
        InputTarget::Stdin => process_stdin(cli, colorize_stdout),
        InputTarget::Path(path) => process_path(path, cli, colorize_stdout),
    }
}

fn process_stdin(cli: &Cli, colorize_stdout: bool) -> Result<ProcessedTarget, cmakefmt::Error> {
    let mut source = String::new();
    io::stdin()
        .read_to_string(&mut source)
        .map_err(cmakefmt::Error::Io)?;

    let (config, registry, config_sources) = build_context(cli, None)?;
    let mut debug_lines = vec![
        "processing <stdin>".to_owned(),
        describe_config_sources(&config_sources),
    ];
    let (formatted, mut formatter_debug) = if cli.debug {
        format_source_with_registry_debug(&source, &config, &registry)?
    } else {
        (
            format_source_with_registry(&source, &config, &registry)?,
            Vec::new(),
        )
    };
    debug_lines.append(&mut formatter_debug);

    let would_change = formatted != source;
    debug_lines.push(format!("result <stdin>: would_change={would_change}"));
    let highlighted_output = colorize_stdout
        .then(|| highlight_changed_lines(&source, &formatted))
        .filter(|_| would_change);

    Ok(ProcessedTarget {
        path: None,
        display_name: "<stdin>".to_owned(),
        formatted,
        highlighted_output,
        would_change,
        debug_lines,
    })
}

fn process_path(
    path: &Path,
    cli: &Cli,
    colorize_stdout: bool,
) -> Result<ProcessedTarget, cmakefmt::Error> {
    let source = std::fs::read_to_string(path)
        .map_err(|err| cmakefmt::Error::Formatter(format!("{}: {err}", path.display())))?;
    let (config, registry, config_sources) = build_context(cli, Some(path))?;
    let mut debug_lines = vec![
        format!("processing {}", path.display()),
        describe_config_sources(&config_sources),
    ];

    let (formatted, mut formatter_debug) = if cli.debug {
        match format_source_with_registry_debug(&source, &config, &registry) {
            Ok(result) => result,
            Err(cmakefmt::Error::Parse(parse_err)) => {
                return Err(cmakefmt::Error::Formatter(format!(
                    "{}: {parse_err}",
                    path.display()
                )));
            }
            Err(err) => return Err(err),
        }
    } else {
        match format_source_with_registry(&source, &config, &registry) {
            Ok(formatted) => (formatted, Vec::new()),
            Err(cmakefmt::Error::Parse(parse_err)) => {
                return Err(cmakefmt::Error::Formatter(format!(
                    "{}: {parse_err}",
                    path.display()
                )));
            }
            Err(err) => return Err(err),
        }
    };
    debug_lines.append(&mut formatter_debug);

    let would_change = formatted != source;
    debug_lines.push(format!(
        "result {}: would_change={would_change}",
        path.display()
    ));
    let highlighted_output = colorize_stdout
        .then(|| highlight_changed_lines(&source, &formatted))
        .filter(|_| would_change);

    Ok(ProcessedTarget {
        path: Some(path.to_path_buf()),
        display_name: path.display().to_string(),
        formatted,
        highlighted_output,
        would_change,
        debug_lines,
    })
}

fn resolve_parallel_jobs(requested: Option<usize>) -> Result<usize, cmakefmt::Error> {
    match requested {
        None => Ok(1),
        Some(0) => std::thread::available_parallelism()
            .map(|parallelism| parallelism.get())
            .map_err(cmakefmt::Error::Io),
        Some(jobs) => Ok(jobs.max(1)),
    }
}

fn should_colorize_stdout(choice: ColorChoice) -> bool {
    match choice {
        ColorChoice::Auto => {
            io::stdout().is_terminal()
                && std::env::var_os("NO_COLOR").is_none()
                && std::env::var("TERM").map_or(true, |term| term != "dumb")
        }
        ColorChoice::Always => true,
        ColorChoice::Never => false,
    }
}

fn highlight_changed_lines(source: &str, formatted: &str) -> String {
    let original_lines = split_lines_with_endings(source);
    let formatted_lines = split_lines_with_endings(formatted);
    let changed_mask = changed_formatted_line_mask(&original_lines, &formatted_lines);
    let mut output = String::with_capacity(formatted.len() + changed_mask.len() * 10);

    for (line, changed) in formatted_lines.iter().zip(changed_mask) {
        if changed {
            push_cyan_line(&mut output, line);
        } else {
            output.push_str(line);
        }
    }

    output
}

fn split_lines_with_endings(source: &str) -> Vec<&str> {
    if source.is_empty() {
        Vec::new()
    } else {
        source.split_inclusive('\n').collect()
    }
}

fn changed_formatted_line_mask<'a>(original: &[&'a str], formatted: &[&'a str]) -> Vec<bool> {
    let mut prefix_len = 0;
    while prefix_len < original.len()
        && prefix_len < formatted.len()
        && original[prefix_len] == formatted[prefix_len]
    {
        prefix_len += 1;
    }

    let mut suffix_len = 0;
    while suffix_len < original.len().saturating_sub(prefix_len)
        && suffix_len < formatted.len().saturating_sub(prefix_len)
        && original[original.len() - 1 - suffix_len] == formatted[formatted.len() - 1 - suffix_len]
    {
        suffix_len += 1;
    }

    let original_mid = &original[prefix_len..original.len() - suffix_len];
    let formatted_mid = &formatted[prefix_len..formatted.len() - suffix_len];

    let mut changed = vec![false; prefix_len];
    changed.extend(diff_middle_mask(original_mid, formatted_mid));
    changed.extend(std::iter::repeat_n(false, suffix_len));
    changed
}

fn diff_middle_mask<'a>(original: &[&'a str], formatted: &[&'a str]) -> Vec<bool> {
    const MAX_DP_CELLS: usize = 2_000_000;

    if formatted.is_empty() {
        return Vec::new();
    }

    if original.is_empty() {
        return vec![true; formatted.len()];
    }

    if original.len().saturating_mul(formatted.len()) > MAX_DP_CELLS {
        return vec![true; formatted.len()];
    }

    let mut dp = vec![vec![0u32; formatted.len() + 1]; original.len() + 1];

    for i in (0..original.len()).rev() {
        for j in (0..formatted.len()).rev() {
            dp[i][j] = if original[i] == formatted[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }

    let mut changed = vec![true; formatted.len()];
    let mut i = 0;
    let mut j = 0;

    while i < original.len() && j < formatted.len() {
        if original[i] == formatted[j] {
            changed[j] = false;
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }

    changed
}

fn push_cyan_line(output: &mut String, line: &str) {
    const CYAN: &str = "\u{1b}[36m";
    const RESET: &str = "\u{1b}[0m";

    if let Some(stripped) = line.strip_suffix('\n') {
        output.push_str(CYAN);
        output.push_str(stripped);
        output.push_str(RESET);
        output.push('\n');
    } else {
        output.push_str(CYAN);
        output.push_str(line);
        output.push_str(RESET);
    }
}

fn describe_config_sources(config_sources: &[PathBuf]) -> String {
    if config_sources.is_empty() {
        "config sources: defaults only".to_owned()
    } else {
        format!(
            "config sources: {}",
            config_sources
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn debug_parallel_suffix(parallel_jobs: usize) -> String {
    if parallel_jobs > 1 {
        format!(" (parallel jobs: {parallel_jobs})")
    } else {
        String::new()
    }
}

fn log_debug(message: impl AsRef<str>) {
    eprintln!("debug: {}", message.as_ref());
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;
    use cmakefmt::default_config_template;

    #[test]
    fn dump_config_covers_config_backed_long_flags() {
        let template = default_config_template();
        let non_config_flags = [
            "check",
            "config-file",
            "convert-legacy-config",
            "colour",
            "debug",
            "path-regex",
            "help",
            "print-default-config",
            "list-files",
            "parallel",
            "version",
            "in-place",
        ];

        for arg in Cli::command().get_arguments() {
            let Some(long) = arg.get_long() else {
                continue;
            };

            if non_config_flags.contains(&long) {
                continue;
            }

            let template_key = long.replace('-', "_");
            assert!(
                template.contains(&template_key),
                "CLI flag --{long} is not represented in default_config_template(); \
                 update src/config/file.rs or add --{long} to the non-config flag allowlist in src/main.rs tests"
            );
        }
    }
}
