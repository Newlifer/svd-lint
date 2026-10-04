//! Source-independent diagnostic model.
//!
//! This module deliberately does not depend on `miette` or any other
//! rendering crate; conversion to human-readable output happens in the CLI.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Severity of a [`Diagnostic`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Note,
}

impl Severity {
    /// Returns `true` if this severity must produce a non-zero exit code.
    pub fn is_error(self) -> bool {
        matches!(self, Severity::Error)
    }
}

/// Half-open byte range `[start, end)` into the source text.
///
/// All offsets are byte offsets into the UTF-8 source; start and end must
/// lie on UTF-8 character boundaries (validated by
/// [`crate::frontend::SourceFile::validate_span`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// Error returned when a [`Span`] is not well-formed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SpanError {
    /// `start` is greater than `end`.
    #[error("span start {start} is after span end {end}")]
    StartAfterEnd {
        /// Span start offset.
        start: usize,
        /// Span end offset.
        end: usize,
    },
    /// `end` is past the end of the source text.
    #[error("span end {end} exceeds source length {len}")]
    OutOfBounds {
        /// Span end offset.
        end: usize,
        /// Length of the source text in bytes.
        len: usize,
    },
    /// An offset does not lie on a UTF-8 character boundary.
    #[error("offset {offset} is not a UTF-8 character boundary")]
    NotCharBoundary {
        /// The offending offset.
        offset: usize,
    },
}

impl Span {
    /// Creates a span, validating that `start <= end`.
    pub fn new(start: usize, end: usize) -> Result<Self, SpanError> {
        if start > end {
            return Err(SpanError::StartAfterEnd { start, end });
        }
        Ok(Self { start, end })
    }

    /// Creates a span without validation. Callers must uphold `start <= end`
    /// and validate against the source text separately.
    pub(crate) fn new_unchecked(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Length of the span in bytes.
    pub fn len(self) -> usize {
        self.end - self.start
    }

    /// Returns `true` if the span covers no bytes.
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Byte range view of this span.
    pub fn to_range(self) -> std::ops::Range<usize> {
        self.start..self.end
    }
}

/// A secondary span related to a [`Diagnostic`], e.g. the first occurrence
/// of a duplicated element.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelatedSpan {
    /// Byte range in the source text.
    pub span: Span,
    /// Optional note attached to this range.
    pub message: Option<String>,
}

impl RelatedSpan {
    /// Creates a related span with a message.
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: Some(message.into()),
        }
    }
}

/// Stable machine-readable diagnostic code.
///
/// The string representation (e.g. `SVD002`) is part of the public API and
/// must never change, regardless of message wording.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    /// IO001: the file could not be read.
    IoReadFailed,
    /// IO002: the file is not valid UTF-8.
    IoInvalidUtf8,
    /// XML001: XML syntax error.
    XmlSyntax,
    /// SVD001: the root element is not `device`.
    InvalidRootElement,
    /// SVD002: a required element is missing.
    MissingRequiredElement,
    /// SVD003: a singleton element occurs more than once.
    DuplicateSingletonElement,
    /// SVD004: a required element is empty.
    EmptyRequiredElement,
    /// SVD005: a peripheral has no description.
    MissingPeripheralDescription,
    /// SVD006: the typed `svd-parser` pass failed.
    SvdParseError,
}

impl DiagnosticCode {
    /// All known codes, in declaration order.
    pub const ALL: &'static [DiagnosticCode] = &[
        DiagnosticCode::IoReadFailed,
        DiagnosticCode::IoInvalidUtf8,
        DiagnosticCode::XmlSyntax,
        DiagnosticCode::InvalidRootElement,
        DiagnosticCode::MissingRequiredElement,
        DiagnosticCode::DuplicateSingletonElement,
        DiagnosticCode::EmptyRequiredElement,
        DiagnosticCode::MissingPeripheralDescription,
        DiagnosticCode::SvdParseError,
    ];

    /// Stable string representation, e.g. `IO001` or `SVD002`.
    pub fn as_str(self) -> &'static str {
        match self {
            DiagnosticCode::IoReadFailed => "IO001",
            DiagnosticCode::IoInvalidUtf8 => "IO002",
            DiagnosticCode::XmlSyntax => "XML001",
            DiagnosticCode::InvalidRootElement => "SVD001",
            DiagnosticCode::MissingRequiredElement => "SVD002",
            DiagnosticCode::DuplicateSingletonElement => "SVD003",
            DiagnosticCode::EmptyRequiredElement => "SVD004",
            DiagnosticCode::MissingPeripheralDescription => "SVD005",
            DiagnosticCode::SvdParseError => "SVD006",
        }
    }

    /// Parses a code from its string representation.
    pub fn from_code(code: &str) -> Option<Self> {
        Some(match code {
            "IO001" => DiagnosticCode::IoReadFailed,
            "IO002" => DiagnosticCode::IoInvalidUtf8,
            "XML001" => DiagnosticCode::XmlSyntax,
            "SVD001" => DiagnosticCode::InvalidRootElement,
            "SVD002" => DiagnosticCode::MissingRequiredElement,
            "SVD003" => DiagnosticCode::DuplicateSingletonElement,
            "SVD004" => DiagnosticCode::EmptyRequiredElement,
            "SVD005" => DiagnosticCode::MissingPeripheralDescription,
            "SVD006" => DiagnosticCode::SvdParseError,
            _ => return None,
        })
    }
}

impl std::fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for DiagnosticCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DiagnosticCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let code = String::deserialize(deserializer)?;
        DiagnosticCode::from_code(&code)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown diagnostic code: {code}")))
    }
}

/// A single diagnostic produced by the analyzer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Stable diagnostic code.
    pub code: DiagnosticCode,
    /// Severity level.
    pub severity: Severity,
    /// Human-readable message.
    pub message: String,
    /// Primary byte range in the source; `None` when no honest location is
    /// known. Coordinates are never invented.
    pub primary_span: Option<Span>,
    /// Additional related ranges.
    pub related: Vec<RelatedSpan>,
}

impl Diagnostic {
    /// Creates a diagnostic without spans.
    pub fn new(code: DiagnosticCode, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            code,
            severity,
            message: message.into(),
            primary_span: None,
            related: Vec::new(),
        }
    }

    /// Creates an error diagnostic without spans.
    pub fn error(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self::new(code, Severity::Error, message)
    }

    /// Creates a warning diagnostic without spans.
    pub fn warning(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self::new(code, Severity::Warning, message)
    }

    /// Sets the primary span.
    pub fn with_span(mut self, span: Span) -> Self {
        self.primary_span = Some(span);
        self
    }

    /// Adds a related span.
    pub fn with_related(mut self, span: Span, message: impl Into<String>) -> Self {
        self.related.push(RelatedSpan::new(span, message));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_new_validates_order() {
        assert!(Span::new(0, 10).is_ok());
        assert!(Span::new(10, 10).is_ok());
        assert_eq!(
            Span::new(11, 10),
            Err(SpanError::StartAfterEnd { start: 11, end: 10 })
        );
    }

    #[test]
    fn span_len_and_range() {
        let span = Span::new(3, 8).unwrap();
        assert_eq!(span.len(), 5);
        assert!(!span.is_empty());
        assert_eq!(span.to_range(), 3..8);
        assert!(Span::new(4, 4).unwrap().is_empty());
    }

    #[test]
    fn codes_are_stable_and_unique() {
        let expected = [
            "IO001", "IO002", "XML001", "SVD001", "SVD002", "SVD003", "SVD004", "SVD005", "SVD006",
        ];
        let actual: Vec<&str> = DiagnosticCode::ALL.iter().map(|c| c.as_str()).collect();
        assert_eq!(actual, expected);
        for code in DiagnosticCode::ALL {
            assert_eq!(DiagnosticCode::from_code(code.as_str()), Some(*code));
        }
        assert_eq!(DiagnosticCode::from_code("SVD999"), None);
    }

    #[test]
    fn diagnostic_serializes_with_stable_shape() {
        let diag = Diagnostic::error(DiagnosticCode::MissingRequiredElement, "missing <name>")
            .with_span(Span::new(10, 20).unwrap())
            .with_related(Span::new(0, 5).unwrap(), "required by CMSIS-SVD");
        let json = serde_json::to_string(&diag).unwrap();
        let expected = serde_json::json!({
            "code": "SVD002",
            "severity": "error",
            "message": "missing <name>",
            "primary_span": { "start": 10, "end": 20 },
            "related": [{ "span": { "start": 0, "end": 5 }, "message": "required by CMSIS-SVD" }],
        });
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&json).unwrap(),
            expected
        );
        let roundtrip: Diagnostic = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, diag);
    }

    #[test]
    fn diagnostic_without_span_serializes_null() {
        let diag = Diagnostic::warning(DiagnosticCode::MissingPeripheralDescription, "no desc");
        let json = serde_json::to_string(&diag).unwrap();
        assert!(json.contains("\"primary_span\":null"));
    }
}
