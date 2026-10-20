//! Diagnostic metadata for this pipeline component.
use crate::diagnostic_definition::DiagnosticDefinition;
pub static IO_READ_FAILED: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("E001"),
    name: "io-read-failed",
    description: "The input file could not be read.",
    severity: crate::Severity::Error,
    configurable: false,
    tags: &["check"],
};

pub static IO_INVALID_UTF8: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("E002"),
    name: "io-invalid-utf8",
    description: "The input is not valid UTF-8.",
    severity: crate::Severity::Error,
    configurable: false,
    tags: &["check"],
};

pub static XML_SYNTAX: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("E003"),
    name: "xml-syntax",
    description: "The input is not well-formed XML.",
    severity: crate::Severity::Error,
    configurable: false,
    tags: &["check"],
};

pub static INVALID_ROOT_ELEMENT: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("E004"),
    name: "invalid-root",
    description: "The root element must be device.",
    severity: crate::Severity::Error,
    configurable: false,
    tags: &["check"],
};

pub static MISSING_REQUIRED_ELEMENT: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("E005"),
    name: "missing-required-element",
    description: "A required element is absent.",
    severity: crate::Severity::Error,
    configurable: false,
    tags: &["check"],
};

pub static DUPLICATE_SINGLETON_ELEMENT: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("E006"),
    name: "duplicate-singleton",
    description: "A singleton element is repeated.",
    severity: crate::Severity::Error,
    configurable: false,
    tags: &["check"],
};

pub static EMPTY_REQUIRED_ELEMENT: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("E007"),
    name: "empty-required-element",
    description: "A required element is empty.",
    severity: crate::Severity::Error,
    configurable: false,
    tags: &["check"],
};

pub static MISSING_PERIPHERAL_DESCRIPTION: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("W002"),
    name: "missing-peripheral-description",
    description: "A peripheral description improves readability.",
    severity: crate::Severity::Warning,
    configurable: true,
    tags: &["check"],
};

pub static SVD_PARSE_ERROR: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("E008"),
    name: "typed-parse-failed",
    description: "The typed SVD model could not be built, or XML nesting exceeds the tool resource limit.",
    severity: crate::Severity::Error,
    configurable: false,
    tags: &["check"],
};
