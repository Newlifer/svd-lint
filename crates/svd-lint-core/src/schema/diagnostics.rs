//! Diagnostic metadata for this pipeline component.
use crate::diagnostic_definition::{DiagnosticDefinition, RuleCategory};
pub static UNKNOWN_VERSION: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("XSD001"),
    name: "unknown-schema-version",
    description: "schemaVersion is missing or unsupported.",
    category: RuleCategory::Structure,
    default_severity: crate::Severity::Error,
    configurable: false,
    tags: &["check"],
};

pub static NONCONFORMANCE: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("XSD002"),
    name: "schema-nonconformance",
    description: "The document fails XSD validation.",
    category: RuleCategory::Structure,
    default_severity: crate::Severity::Error,
    configurable: false,
    tags: &["check"],
};

pub static MISSING_SCHEMA: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("XSD003"),
    name: "schema-not-found",
    description: "The requested schema is unavailable.",
    category: RuleCategory::Structure,
    default_severity: crate::Severity::Error,
    configurable: false,
    tags: &["check"],
};

pub static BACKEND_ERROR: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("XSD004"),
    name: "schema-backend-failed",
    description: "The XSD backend could not load or compile a schema.",
    category: RuleCategory::Structure,
    default_severity: crate::Severity::Error,
    configurable: false,
    tags: &["check"],
};
