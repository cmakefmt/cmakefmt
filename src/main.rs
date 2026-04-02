use std::collections::BTreeSet;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use cmakefmt::spec::registry::CommandRegistry;
use cmakefmt::{
    default_config_template, files::discover_cmake_files, format_source_with_registry,
    format_source_with_registry_debug, CaseStyle, Config,
};
use rayon::prelude::*;
use regex::Regex;

/// A fast, correct CMake formatter.
#[derive(Parser, Debug)]
#[command(name = "cmakefmt", version, about)]
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
        conflicts_with = "dump_config"
    )]
    in_place: bool,

    /// Check if files are already formatted (exit 1 if not).
    #[arg(long, conflicts_with = "dump_config")]
    check: bool,

    /// List the files that would be reformatted without changing them.
    #[arg(long = "list-files", conflicts_with = "dump_config")]
    list_files: bool,

    /// Regex filter applied to discovered CMake file paths.
    #[arg(short = 'f', long = "file-regex", conflicts_with = "dump_config")]
    file_regex: Option<String>,

    /// Print the default config template and exit.
    #[arg(long = "dump-config")]
    dump_config: bool,

    /// Print debug diagnostics about discovery, config resolution, barriers,
    /// and formatter decisions.
    #[arg(long, conflicts_with = "dump_config")]
    debug: bool,

    /// Format files in parallel. Pass no value to use the available CPU count.
    #[arg(
        short = 'j',
        long,
        value_name = "JOBS",
        num_args = 0..=1,
        default_missing_value = "0",
        conflicts_with = "dump_config"
    )]
    parallel: Option<usize>,

    /// Path to a specific config file to use.
    #[arg(long = "config")]
    config_path: Option<PathBuf>,

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

    let registry = CommandRegistry::load()?;
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
        process_targets_parallel(&targets, cli, &registry, parallel_jobs)?
    } else {
        if cli.debug && parallel_jobs > 1 && targets.iter().any(|target| !target.is_path()) {
            log_debug("parallel mode ignored because stdin input must run serially");
        }
        process_targets_serial(&targets, cli, &registry)?
    };

    for result in results {
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
            io::stdout()
                .write_all(result.formatted.as_bytes())
                .map_err(cmakefmt::Error::Io)?;
        }
    }

    if (cli.check || cli.list_files) && any_would_change {
        Ok(EXIT_CHECK_FAILED)
    } else {
        Ok(EXIT_OK)
    }
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

/// Build a Config by layering: defaults → config file → CLI overrides.
fn build_config(
    cli: &Cli,
    file_path: Option<&Path>,
) -> Result<(Config, Vec<PathBuf>), cmakefmt::Error> {
    let config_sources = if let Some(config_path) = &cli.config_path {
        vec![config_path.clone()]
    } else if let Some(path) = file_path {
        Config::config_sources_for(path)
    } else {
        Vec::new()
    };

    let mut config = if let Some(config_path) = &cli.config_path {
        Config::from_file(config_path)?
    } else if let Some(path) = file_path {
        Config::for_file(path)?
    } else {
        Config::default()
    };

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

    Ok((config, config_sources))
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
    would_change: bool,
    debug_lines: Vec<String>,
}

fn process_targets_serial(
    targets: &[InputTarget],
    cli: &Cli,
    registry: &CommandRegistry,
) -> Result<Vec<ProcessedTarget>, cmakefmt::Error> {
    targets
        .iter()
        .map(|target| process_target(target, cli, registry))
        .collect()
}

fn process_targets_parallel(
    targets: &[InputTarget],
    cli: &Cli,
    registry: &CommandRegistry,
    parallel_jobs: usize,
) -> Result<Vec<ProcessedTarget>, cmakefmt::Error> {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(parallel_jobs)
        .build()
        .map_err(|err| cmakefmt::Error::Formatter(format!("failed to build thread pool: {err}")))?;

    pool.install(|| {
        targets
            .par_iter()
            .map(|target| process_target(target, cli, registry))
            .collect()
    })
}

fn process_target(
    target: &InputTarget,
    cli: &Cli,
    registry: &CommandRegistry,
) -> Result<ProcessedTarget, cmakefmt::Error> {
    match target {
        InputTarget::Stdin => process_stdin(cli, registry),
        InputTarget::Path(path) => process_path(path, cli, registry),
    }
}

fn process_stdin(
    cli: &Cli,
    registry: &CommandRegistry,
) -> Result<ProcessedTarget, cmakefmt::Error> {
    let mut source = String::new();
    io::stdin()
        .read_to_string(&mut source)
        .map_err(cmakefmt::Error::Io)?;

    let (config, config_sources) = build_config(cli, None)?;
    let mut debug_lines = vec![
        "processing <stdin>".to_owned(),
        describe_config_sources(&config_sources),
    ];
    let (formatted, mut formatter_debug) = if cli.debug {
        format_source_with_registry_debug(&source, &config, registry)?
    } else {
        (
            format_source_with_registry(&source, &config, registry)?,
            Vec::new(),
        )
    };
    debug_lines.append(&mut formatter_debug);

    let would_change = formatted != source;
    debug_lines.push(format!("result <stdin>: would_change={would_change}"));

    Ok(ProcessedTarget {
        path: None,
        display_name: "<stdin>".to_owned(),
        formatted,
        would_change,
        debug_lines,
    })
}

fn process_path(
    path: &Path,
    cli: &Cli,
    registry: &CommandRegistry,
) -> Result<ProcessedTarget, cmakefmt::Error> {
    let source = std::fs::read_to_string(path)
        .map_err(|err| cmakefmt::Error::Formatter(format!("{}: {err}", path.display())))?;
    let (config, config_sources) = build_config(cli, Some(path))?;
    let mut debug_lines = vec![
        format!("processing {}", path.display()),
        describe_config_sources(&config_sources),
    ];

    let (formatted, mut formatter_debug) = if cli.debug {
        match format_source_with_registry_debug(&source, &config, registry) {
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
        match format_source_with_registry(&source, &config, registry) {
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

    Ok(ProcessedTarget {
        path: Some(path.to_path_buf()),
        display_name: path.display().to_string(),
        formatted,
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
            "config",
            "debug",
            "dump-config",
            "file-regex",
            "help",
            "in-place",
            "list-files",
            "parallel",
            "version",
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
