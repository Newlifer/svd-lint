//! Command line frontend and presentation formats.
mod config;
mod output;
mod sarif;
use clap::{Args, Parser, Subcommand, ValueEnum};
use config::{Config, Profile};
use std::{path::PathBuf, process::ExitCode};
use svd_lint_core::{
    Diagnostic, DiagnosticCode, Severity, SourceFile, analyze_svd,
    registry::{find_rule, rule_registry},
};
#[derive(Parser)]
#[command(
    name = "svd-lint",
    version,
    about = "Static analyzer for CMSIS-SVD files"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    Check(CheckArgs),
    DumpIr(CheckArgs),
    ListRules {
        #[arg(long,value_enum,default_value_t=Format::Text)]
        format: Format,
    },
    Explain {
        code: String,
        #[arg(long,value_enum,default_value_t=Format::Text)]
        format: Format,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum Format {
    Text,
    Json,
    Sarif,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
enum FailOn {
    Warning,
    #[default]
    Error,
}
#[derive(Args)]
struct Options {
    #[arg(long,value_enum,default_value_t=Format::Text)]
    format: Format,
    #[arg(long, value_enum)]
    profile: Option<Profile>,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long,value_enum,default_value_t=FailOn::Error)]
    fail_on: FailOn,
    #[arg(long)]
    xsd: bool,
    #[arg(long)]
    schema_version: Option<String>,
    #[arg(long)]
    schema_dir: Option<PathBuf>,
}
#[derive(Args)]
struct CheckArgs {
    file: PathBuf,
    #[command(flatten)]
    options: Options,
}
fn fail(diagnostics: &[Diagnostic], policy: FailOn, io: bool) -> u8 {
    if policy == FailOn::Error {
        return output::exit_code(diagnostics, io) as u8;
    }
    if io {
        2
    } else if diagnostics.iter().any(|d| {
        d.severity == Severity::Error
            || (policy == FailOn::Warning && d.severity == Severity::Warning)
    }) {
        1
    } else {
        0
    }
}
fn schema_diagnostics(source: &SourceFile, options: &Options) -> Vec<Diagnostic> {
    use svd_lint_core::schema::{DirectoryProvider, SchemaVersion};
    if !options.xsd && options.schema_version.is_none() && options.schema_dir.is_none() {
        return Vec::new();
    }
    let Some(version) = options
        .schema_version
        .as_ref()
        .map(|v| SchemaVersion(v.clone()))
        .or_else(|| SchemaVersion::detect(source))
    else {
        return vec![Diagnostic::error(
            DiagnosticCode::XsdUnknownVersion,
            "schemaVersion is missing",
        )];
    };
    if !version.supported() {
        return vec![Diagnostic::error(
            DiagnosticCode::XsdUnknownVersion,
            format!("unsupported schemaVersion {}", version.0),
        )];
    }
    if let Some(dir) = &options.schema_dir {
        if DirectoryProvider::new(dir).schema(&version).is_none() {
            return vec![Diagnostic::error(
                DiagnosticCode::XsdMissingSchema,
                format!("schema {} not found in {}", version.0, dir.display()),
            )];
        }
    } else {
        return vec![Diagnostic::error(
            DiagnosticCode::XsdMissingSchema,
            "provide --schema-dir with versioned CMSIS-SVD schemas",
        )];
    }
    #[cfg(feature = "xsd")]
    {
        use svd_lint_core::schema::{SchemaValidator, UppsalaValidator};
        UppsalaValidator {
            provider: DirectoryProvider::new(options.schema_dir.as_ref().unwrap()),
        }
        .validate(source, version)
    }
    #[cfg(not(feature = "xsd"))]
    {
        vec![Diagnostic::error(
            DiagnosticCode::XsdBackendError,
            "XSD validation requires a build with --features xsd",
        )]
    }
}
fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::ListRules { format } => {
            if format == Format::Json {
                println!("{}", serde_json::to_string_pretty(rule_registry()).unwrap());
            } else {
                for r in rule_registry() {
                    println!(
                        "{} {:32} {:?} {:?}",
                        r.code, r.name, r.default_severity, r.category
                    );
                }
            }
            Ok(0)
        }
        Commands::Explain { code, format } => match find_rule(&code) {
            Some(rule) => {
                if format == Format::Json {
                    println!("{}", serde_json::to_string_pretty(rule).unwrap());
                } else {
                    println!(
                        "{} — {}\n\nSeverity: {:?}\nCategory: {:?}\nSource: {:?}\n\n{}",
                        rule.code,
                        rule.name,
                        rule.default_severity,
                        rule.category,
                        rule.source,
                        rule.description
                    );
                    for r in rule.svdconv_codes {
                        println!("SVDConv: {} ({:?})", r.code, r.compatibility);
                    }
                    for r in rule.spec_refs {
                        println!("CMSIS-SVD: {} — {}", r.title, r.url);
                    }
                }
                Ok(0)
            }
            None => Err(format!("unknown rule {code}")),
        },
        Commands::Check(args) => run_check(&args, false),
        Commands::DumpIr(args) => run_check(&args, true),
    };
    match result {
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            eprintln!("svd-lint: {message}");
            ExitCode::from(2)
        }
    }
}
fn read(path: &PathBuf) -> Result<SourceFile, Diagnostic> {
    let name = path.to_string_lossy().replace('\\', "/");
    let bytes = std::fs::read(path).map_err(|e| {
        Diagnostic::error(
            DiagnosticCode::IoReadFailed,
            format!("failed to read `{name}`: {e}"),
        )
    })?;
    let text = String::from_utf8(bytes).map_err(|_| {
        Diagnostic::error(
            DiagnosticCode::IoInvalidUtf8,
            format!("`{name}` is not valid UTF-8"),
        )
    })?;
    Ok(SourceFile::new(name, text))
}
fn analyze(
    source: &SourceFile,
    options: &Options,
    config: &Config,
) -> (Vec<Diagnostic>, Option<svd_lint_core::CanonicalDevice>) {
    let schema = schema_diagnostics(source, options);
    if !schema.is_empty() {
        return (schema, None);
    }
    let mut result = analyze_svd(source);
    let quality_requested = options.profile.unwrap_or(config.profile) != Profile::Default
        || config.rules.contains_key("SVD007")
        || config
            .overrides
            .iter()
            .any(|o| o.rules.iter().any(|c| c == "SVD007"));
    if quality_requested {
        if let (Some(device), Some(map)) = (&result.device, &result.source_map) {
            result
                .diagnostics
                .extend(svd_lint_core::rules::check_quality(device, map));
        }
    }
    config.apply(&mut result.diagnostics, source.name(), options.profile);
    result.diagnostics.sort_by_key(|d| {
        (
            d.primary_span.map(|s| s.start),
            d.code.as_str(),
            d.message.clone(),
        )
    });
    (result.diagnostics, result.device)
}
fn run_check(args: &CheckArgs, dump: bool) -> Result<u8, String> {
    let options = &args.options;
    let config = Config::load(options.config.as_deref())?;
    let (source, diagnostics, ir, io) = match read(&args.file) {
        Ok(source) => {
            let (d, ir) = analyze(&source, options, &config);
            (Some(source), d, ir, false)
        }
        Err(d) => (None, vec![d], None, true),
    };
    let path = args.file.to_string_lossy().replace('\\', "/");
    if let Some(ir) = ir.filter(|_| dump) {
        println!("{}", serde_json::to_string_pretty(&ir).unwrap());
        eprint!(
            "{}",
            output::render_text(&diagnostics, source.as_ref(), output::colors_enabled())
        );
    } else {
        emit(options.format, &path, &diagnostics, source.as_ref());
    }
    Ok(fail(&diagnostics, options.fail_on, io))
}
fn emit(format: Format, path: &str, diagnostics: &[Diagnostic], source: Option<&SourceFile>) {
    match format {
        Format::Text => eprint!(
            "{}",
            output::render_text(diagnostics, source, output::colors_enabled())
        ),
        Format::Json => println!("{}", output::render_json(path, diagnostics, source)),
        Format::Sarif => println!(
            "{}",
            serde_json::to_string_pretty(&sarif::document(sarif::results(
                path,
                diagnostics,
                source
            )))
            .unwrap()
        ),
    }
}
