use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use cmakefmt::{default_config_template, format_source, CaseStyle, Config};
use regex::Regex;
use walkdir::WalkDir;

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
    #[arg(long = "list-files", alias = "dry-run", conflicts_with = "dump_config")]
    list_files: bool,

    /// Regex filter applied to discovered CMake file paths.
    #[arg(short = 'f', long = "file-regex", conflicts_with = "dump_config")]
    file_regex: Option<String>,

    /// Print the default config template and exit.
    #[arg(long = "dump-config")]
    dump_config: bool,

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

    let file_filter = compile_file_filter(cli.file_regex.as_deref())?;
    let targets = collect_targets(cli, file_filter.as_ref())?;
    let mut any_would_change = false;

    for target in targets {
        match target {
            InputTarget::Stdin => {
                let mut source = String::new();
                io::stdin()
                    .read_to_string(&mut source)
                    .map_err(cmakefmt::Error::Io)?;

                let config = build_config(cli, None)?;
                let formatted = format_source(&source, &config)?;
                let would_change = formatted != source;
                any_would_change |= would_change;

                if cli.list_files {
                    if would_change {
                        println!("<stdin>");
                    }
                } else if cli.check {
                    if would_change {
                        eprintln!("<stdin> would be reformatted");
                    }
                } else {
                    io::stdout()
                        .write_all(formatted.as_bytes())
                        .map_err(cmakefmt::Error::Io)?;
                }
            }
            InputTarget::Path(path) => {
                let source = std::fs::read_to_string(&path)
                    .map_err(|e| cmakefmt::Error::Formatter(format!("{}: {e}", path.display())))?;
                let config = build_config(cli, Some(&path))?;

                let formatted = match format_source(&source, &config) {
                    Ok(f) => f,
                    Err(cmakefmt::Error::Parse(parse_err)) => {
                        return Err(cmakefmt::Error::Formatter(format!(
                            "{}: {parse_err}",
                            path.display()
                        )));
                    }
                    Err(e) => return Err(e),
                };

                let would_change = formatted != source;
                any_would_change |= would_change;

                if cli.list_files {
                    if would_change {
                        println!("{}", path.display());
                    }
                } else if cli.check {
                    if would_change {
                        eprintln!("{} would be reformatted", path.display());
                    }
                } else if cli.in_place {
                    if would_change {
                        std::fs::write(&path, &formatted).map_err(cmakefmt::Error::Io)?;
                    }
                } else {
                    io::stdout()
                        .write_all(formatted.as_bytes())
                        .map_err(cmakefmt::Error::Io)?;
                }
            }
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

fn discover_cmake_files(root: &Path, file_filter: Option<&Regex>) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| is_cmake_file(path))
        .filter(|path| matches_filter(path, file_filter))
        .collect()
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

fn is_cmake_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(OsStr::to_str) else {
        return false;
    };

    if file_name == "CMakeLists.txt" {
        return true;
    }

    file_name.ends_with(".cmake") || file_name.ends_with(".cmake.in")
}

fn matches_filter(path: &Path, file_filter: Option<&Regex>) -> bool {
    let Some(file_filter) = file_filter else {
        return true;
    };

    file_filter.is_match(&path.to_string_lossy())
}

/// Build a Config by layering: defaults → config file → CLI overrides.
fn build_config(cli: &Cli, file_path: Option<&Path>) -> Result<Config, cmakefmt::Error> {
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

    Ok(config)
}

enum InputTarget {
    Stdin,
    Path(PathBuf),
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
            "dump-config",
            "dry-run",
            "file-regex",
            "help",
            "in-place",
            "list-files",
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
