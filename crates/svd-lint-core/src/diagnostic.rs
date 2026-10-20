//! Source-independent diagnostic model.
//!
//! This module deliberately does not depend on `miette` or any other
//! rendering crate; conversion to human-readable output happens in the CLI.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
/// A stable identifier: E001..E999, W001..W999 or N001..N999.
/// Each registered code has one fixed severity, matching its prefix.
/// Registered codes live next to their checker or pipeline component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiagnosticCode(&'static str);

impl DiagnosticCode {
    /// Registered identifiers, in deterministic catalogue order.
    pub fn all() -> &'static [Self] {
        static CODES: std::sync::OnceLock<Vec<DiagnosticCode>> = std::sync::OnceLock::new();
        CODES.get_or_init(|| {
            crate::registry::rule_registry()
                .iter()
                .map(|m| m.code())
                .collect()
        })
    }

    /// Creates an identifier from a static code; registration is checked separately.
    pub const fn new(code: &'static str) -> Self {
        Self(code)
    }
    pub const fn as_str(self) -> &'static str {
        self.0
    }
    /// Looks up a known code without allocating or leaking the input string.
    pub fn from_code(code: &str) -> Option<Self> {
        crate::registry::find_rule(code).map(|definition| definition.code)
    }
}

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

/// Результат проверки
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "DiagnosticData")]
pub struct Diagnostic {
    /// Код проверки (E001, E002 и т.д.)
    code: DiagnosticCode,
    /// Тяжесть греха: ошибка, предупреждение
    severity: Severity,
    /// Человекочитаемое сообщение греха
    pub message: String,
    /// Основной диапазон байтов в сырках
    pub primary_span: Option<Span>,
    /// Связанные диапазоны в сырках
    pub related: Vec<RelatedSpan>,
}

impl Diagnostic {
    /// Creates a diagnostic without spans.
    pub fn new(code: DiagnosticCode, message: impl Into<String>) -> Self {
        let severity = crate::registry::find_rule(code.as_str())
            .expect("diagnostic code must be registered")
            .severity;
        Self {
            code,
            severity,
            message: message.into(),
            primary_span: None,
            related: Vec::new(),
        }
    }

    /// The stable identifier of this finding.
    pub fn code(&self) -> DiagnosticCode {
        self.code
    }

    /// The fixed severity declared for this diagnostic code.
    pub fn severity(&self) -> Severity {
        self.severity
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

// Validate the same invariant when accepting serialized diagnostics.
#[derive(Deserialize)]
struct DiagnosticData {
    code: DiagnosticCode,
    severity: Severity,
    message: String,
    primary_span: Option<Span>,
    related: Vec<RelatedSpan>,
}

impl TryFrom<DiagnosticData> for Diagnostic {
    type Error = String;

    fn try_from(data: DiagnosticData) -> Result<Self, Self::Error> {
        let mut diagnostic = Self::new(data.code, data.message);
        if data.severity != diagnostic.severity() {
            return Err(format!(
                "severity does not match diagnostic code {}",
                data.code
            ));
        }
        diagnostic.primary_span = data.primary_span;
        diagnostic.related = data.related;
        Ok(diagnostic)
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
        // Representative public identifiers stay stable; new catalogue entries
        // do not require maintaining a second exhaustive list in this test.
        assert_eq!(
            crate::frontend::diagnostics::IO_READ_FAILED.code().as_str(),
            "E001"
        );
        assert_eq!(
            crate::normalize::diagnostics::AMBIGUOUS_REFERENCE
                .code()
                .as_str(),
            "E011"
        );
        assert_eq!(crate::rules::fields::FIELD_OVERLAP.code().as_str(), "W003");
        let mut seen = std::collections::BTreeSet::new();
        for code in DiagnosticCode::all() {
            assert!(seen.insert(code.as_str()), "duplicate code {code}");
            assert_eq!(DiagnosticCode::from_code(code.as_str()), Some(*code));
        }
        assert_eq!(DiagnosticCode::from_code("E999"), None);
        assert_eq!(DiagnosticCode::from_code("SEM004"), None);
    }

    #[test]
    fn diagnostic_serializes_with_stable_shape() {
        let diag = Diagnostic::new(
            crate::frontend::diagnostics::MISSING_REQUIRED_ELEMENT.code(),
            "missing <name>",
        )
        .with_span(Span::new(10, 20).unwrap())
        .with_related(Span::new(0, 5).unwrap(), "required by CMSIS-SVD");
        let json = serde_json::to_string(&diag).unwrap();
        let expected = serde_json::json!({
            "code": "E005",
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
        let diag = Diagnostic::new(
            crate::frontend::diagnostics::MISSING_PERIPHERAL_DESCRIPTION.code(),
            "no desc",
        );
        let json = serde_json::to_string(&diag).unwrap();
        assert!(json.contains("\"primary_span\":null"));
    }
}
