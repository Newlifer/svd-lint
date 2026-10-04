//! Rendering of core diagnostics into text (via `miette`) and JSON.

use std::fmt;
use std::io::IsTerminal;

use miette::{GraphicalReportHandler, GraphicalTheme, LabeledSpan, NamedSource};
use serde::Serialize;
use svd_lint_core::{Diagnostic, Severity, SourceFile};

/// Exit code: analysis ran, no error-severity diagnostics.
pub const EXIT_OK: i32 = 0;
/// Exit code: the SVD file contains errors.
pub const EXIT_ERRORS: i32 = 1;
/// Exit code: the file could not be read or analysis could not start.
pub const EXIT_IO: i32 = 2;

/// Maps diagnostics to the process exit code.
pub fn exit_code(diagnostics: &[Diagnostic], io_failure: bool) -> i32 {
    if io_failure {
        return EXIT_IO;
    }
    if diagnostics.iter().any(|d| d.severity.is_error()) {
        EXIT_ERRORS
    } else {
        EXIT_OK
    }
}

/// Whether terminal colors should be used for text output.
pub fn colors_enabled() -> bool {
    std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

/// Renders all diagnostics as human-readable text.
///
/// `source` is `None` for I/O level diagnostics (no file content exists).
pub fn render_text(diagnostics: &[Diagnostic], source: Option<&SourceFile>, color: bool) -> String {
    let theme = if color {
        GraphicalTheme::unicode()
    } else {
        GraphicalTheme::unicode_nocolor()
    };
    let handler = GraphicalReportHandler::new_themed(theme);
    let mut out = String::new();
    for diagnostic in diagnostics {
        // miette requires the named source to be `'static`, so the text is
        // cloned once per rendered diagnostic.
        let named = source.map(|s| NamedSource::new(s.name(), s.text().to_owned()));
        let rendered = CliDiagnostic {
            diagnostic,
            named_source: named,
        };
        // Rendering into a String cannot fail.
        let _ = handler.render_report(&mut out, &rendered);
        out.push('\n');
    }
    out
}

/// Wrapper adapting a core [`Diagnostic`] to `miette` for rendering.
///
/// `svd-lint-core` itself never depends on `miette`.
struct CliDiagnostic<'a> {
    diagnostic: &'a Diagnostic,
    named_source: Option<NamedSource<String>>,
}

impl fmt::Debug for CliDiagnostic<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.diagnostic.code, self.diagnostic.message)
    }
}

impl fmt::Display for CliDiagnostic<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.diagnostic.message)
    }
}

impl std::error::Error for CliDiagnostic<'_> {}

impl miette::Diagnostic for CliDiagnostic<'_> {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        Some(Box::new(self.diagnostic.code))
    }

    fn severity(&self) -> Option<miette::Severity> {
        Some(match self.diagnostic.severity {
            Severity::Error => miette::Severity::Error,
            Severity::Warning => miette::Severity::Warning,
            Severity::Note => miette::Severity::Advice,
        })
    }

    fn labels<'a>(&'a self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + 'a>> {
        self.named_source.as_ref()?;
        let mut labels = Vec::new();
        if let Some(span) = self.diagnostic.primary_span {
            labels.push(LabeledSpan::new_with_span(None, (span.start, span.len())));
        }
        for related in &self.diagnostic.related {
            labels.push(LabeledSpan::new_with_span(
                related.message.clone(),
                (related.span.start, related.span.len()),
            ));
        }
        Some(Box::new(labels.into_iter()))
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        self.named_source
            .as_ref()
            .map(|named| named as &dyn miette::SourceCode)
    }
}

/// Location-augmented span for JSON output.
#[derive(Serialize)]
struct JsonSpan {
    start: usize,
    end: usize,
    start_line: Option<u32>,
    start_column: Option<u32>,
    end_line: Option<u32>,
    end_column: Option<u32>,
}

impl JsonSpan {
    fn new(span: svd_lint_core::Span, source: Option<&SourceFile>) -> Self {
        let start = source.and_then(|s| s.line_col(span.start));
        let end = source.and_then(|s| s.line_col(span.end));
        Self {
            start: span.start,
            end: span.end,
            start_line: start.map(|p| p.line),
            start_column: start.map(|p| p.column),
            end_line: end.map(|p| p.line),
            end_column: end.map(|p| p.column),
        }
    }
}

#[derive(Serialize)]
struct JsonRelated {
    span: JsonSpan,
    message: Option<String>,
}

#[derive(Serialize)]
struct JsonDiagnostic {
    code: String,
    severity: String,
    message: String,
    span: Option<JsonSpan>,
    related: Vec<JsonRelated>,
}

/// Stable JSON report structure written to stdout.
#[derive(Serialize)]
struct JsonReport {
    file: String,
    diagnostics: Vec<JsonDiagnostic>,
}

/// Builds the JSON report as a string (pretty-printed).
pub fn render_json(
    file_name: &str,
    diagnostics: &[Diagnostic],
    source: Option<&SourceFile>,
) -> String {
    let report = JsonReport {
        file: file_name.to_owned(),
        diagnostics: diagnostics
            .iter()
            .map(|d| JsonDiagnostic {
                code: d.code.as_str().to_owned(),
                severity: match d.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Note => "note",
                }
                .to_owned(),
                message: d.message.clone(),
                span: d.primary_span.map(|s| JsonSpan::new(s, source)),
                related: d
                    .related
                    .iter()
                    .map(|r| JsonRelated {
                        span: JsonSpan::new(r.span, source),
                        message: r.message.clone(),
                    })
                    .collect(),
            })
            .collect(),
    };
    serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_owned())
}
