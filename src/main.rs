use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use cmfmt::{format_source, CaseStyle, Config};

/// A fast, correct CMake formatter.
#[derive(Parser, Debug)]
#[command(name = "cmfmt", version, about)]
struct Cli {
    /// Files to format. Use `-` for stdin.
    #[arg(required = true)]
    files: Vec<String>,

    /// Format files in-place (modifies the files on disk).
    #[arg(short = 'i', long = "in-place")]
    in_place: bool,

    /// Check if files are already formatted (exit 1 if not).
    #[arg(long)]
    check: bool,

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

fn run(cli: &Cli) -> Result<u8, cmfmt::Error> {
    let mut any_would_change = false;

    for file_arg in &cli.files {
        if file_arg == "-" {
            // Read from stdin, write to stdout.
            let mut source = String::new();
            io::stdin()
                .read_to_string(&mut source)
                .map_err(cmfmt::Error::Io)?;

            let config = build_config(cli, None)?;
            let formatted = format_source(&source, &config)?;

            if cli.check {
                if formatted != source {
                    any_would_change = true;
                    eprintln!("<stdin> would be reformatted");
                }
            } else {
                io::stdout()
                    .write_all(formatted.as_bytes())
                    .map_err(cmfmt::Error::Io)?;
            }
        } else {
            let path = PathBuf::from(file_arg);
            let source = std::fs::read_to_string(&path)
                .map_err(|e| cmfmt::Error::Formatter(format!("{}: {e}", path.display())))?;

            let config = build_config(cli, Some(&path))?;

            let formatted = match format_source(&source, &config) {
                Ok(f) => f,
                Err(cmfmt::Error::Parse(parse_err)) => {
                    return Err(cmfmt::Error::Formatter(format!(
                        "{}: {parse_err}",
                        path.display()
                    )));
                }
                Err(e) => return Err(e),
            };

            if cli.check {
                if formatted != source {
                    any_would_change = true;
                    eprintln!("{} would be reformatted", path.display());
                }
            } else if cli.in_place {
                if formatted != source {
                    std::fs::write(&path, &formatted).map_err(cmfmt::Error::Io)?;
                }
            } else {
                io::stdout()
                    .write_all(formatted.as_bytes())
                    .map_err(cmfmt::Error::Io)?;
            }
        }
    }

    if cli.check && any_would_change {
        Ok(EXIT_CHECK_FAILED)
    } else {
        Ok(EXIT_OK)
    }
}

/// Build a Config by layering: defaults → config file → CLI overrides.
fn build_config(cli: &Cli, file_path: Option<&std::path::Path>) -> Result<Config, cmfmt::Error> {
    let mut config = if let Some(config_path) = &cli.config_path {
        Config::from_file(config_path)?
    } else if let Some(path) = file_path {
        Config::for_file(path)?
    } else {
        Config::default()
    };

    // CLI flag overrides (highest precedence).
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
