//! `svd-lint` command line interface.

mod output;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use svd_lint_core::{Diagnostic, DiagnosticCode, SourceFile, analyze_svd};

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
    /// Dumps canonical IR as JSON after successful analysis.
    DumpIr(CheckArgs),
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
        Commands::Check(args) => run_check(&args, false),
        Commands::DumpIr(args) => run_check(&args, true),
    }
}

fn run_check(args: &CheckArgs, dump_ir: bool) -> ExitCode {
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
    let result = analyze_svd(&source);
    if let Some(device) = result.device.as_ref().filter(|_| dump_ir) {
        println!(
            "{}",
            serde_json::to_string_pretty(device).expect("IR is JSON serializable")
        );
        if !result.diagnostics.is_empty() {
            eprint!(
                "{}",
                output::render_text(&result.diagnostics, Some(&source), output::colors_enabled())
            );
        }
        return ExitCode::from(output::exit_code(&result.diagnostics, false) as u8);
    }
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
