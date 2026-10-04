//! `svd-lint` command line interface.

mod output;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use svd_lint_core::{Diagnostic, DiagnosticCode, SourceFile, parse_svd};

/// Static analyzer for CMSIS-SVD files.
#[derive(Parser)]
#[command(name = "svd-lint", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Checks an SVD file and reports diagnostics.
    Check(CheckArgs),
}

#[derive(Parser)]
struct CheckArgs {
    /// Path to the CMSIS-SVD file to check.
    file: PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum Format {
    /// Human-readable diagnostics on stderr.
    Text,
    /// Machine-readable JSON report on stdout.
    Json,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Commands::Check(args) => run_check(&args),
    }
}

fn run_check(args: &CheckArgs) -> ExitCode {
    let file_name = args.file.to_string_lossy().into_owned();

    let bytes = match std::fs::read(&args.file) {
        Ok(bytes) => bytes,
        Err(err) => {
            let diagnostic = Diagnostic::error(
                DiagnosticCode::IoReadFailed,
                format!("failed to read `{file_name}`: {err}"),
            );
            return finish(
                args.format,
                &file_name,
                std::slice::from_ref(&diagnostic),
                None,
                true,
            );
        }
    };

    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => {
            let diagnostic = Diagnostic::error(
                DiagnosticCode::IoInvalidUtf8,
                format!("`{file_name}` is not valid UTF-8"),
            );
            return finish(
                args.format,
                &file_name,
                std::slice::from_ref(&diagnostic),
                None,
                true,
            );
        }
    };

    let source = SourceFile::new(file_name, text);
    let result = parse_svd(&source);
    finish(
        args.format,
        source.name(),
        &result.diagnostics,
        Some(&source),
        false,
    )
}

fn finish(
    format: Format,
    file_name: &str,
    diagnostics: &[Diagnostic],
    source: Option<&SourceFile>,
    io_failure: bool,
) -> ExitCode {
    match format {
        Format::Text => {
            if !diagnostics.is_empty() {
                let text = output::render_text(diagnostics, source, output::colors_enabled());
                eprint!("{text}");
            }
        }
        Format::Json => {
            println!("{}", output::render_json(file_name, diagnostics, source));
        }
    }
    ExitCode::from(output::exit_code(diagnostics, io_failure) as u8)
}
