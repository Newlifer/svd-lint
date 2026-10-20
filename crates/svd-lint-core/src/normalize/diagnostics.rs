//! Diagnostic metadata for this pipeline component.
use crate::diagnostic_definition::DiagnosticDefinition;
pub static MISSING_REFERENCE: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("E009"),
    name: "missing-reference",
    description: "A derivedFrom target cannot be resolved.",
    severity: crate::Severity::Error,
    configurable: false,
    tags: &["check"],
};

pub static CYCLE: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("E010"),
    name: "reference-cycle",
    description: "A derivedFrom cycle prevents normalization.",
    severity: crate::Severity::Error,
    configurable: false,
    tags: &["check"],
};

pub static AMBIGUOUS_REFERENCE: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("E011"),
    name: "ambiguous-reference",
    description: "A reference or canonical path is ambiguous.",
    severity: crate::Severity::Error,
    configurable: false,
    tags: &["check"],
};

pub static INVALID_DIMENSION: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("E012"),
    name: "invalid-dimension",
    description: "An array dimension is invalid.",
    severity: crate::Severity::Error,
    configurable: false,
    tags: &["check"],
};

pub static INVALID_INDEX: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("E013"),
    name: "invalid-array-index",
    description: "Array indices do not match the dimension.",
    severity: crate::Severity::Error,
    configurable: false,
    tags: &["check"],
};

pub static INVALID_TEMPLATE: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("E014"),
    name: "invalid-array-template",
    description: "The array name template cannot be expanded.",
    severity: crate::Severity::Error,
    configurable: false,
    tags: &["check"],
};

pub static ADDRESS_OVERFLOW: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("E015"),
    name: "address-overflow",
    description: "An address or field offset cannot be represented.",
    severity: crate::Severity::Error,
    configurable: false,
    tags: &["check"],
};

pub static MISSING_PROPERTY: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("E016"),
    name: "missing-register-size",
    description: "The effective register size cannot be determined.",
    severity: crate::Severity::Error,
    configurable: false,
    tags: &["check"],
};

pub static EXPANSION_LIMIT: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("E017"),
    name: "expansion-limit",
    description: "The materialization budget or dependency depth limit was exceeded.",
    severity: crate::Severity::Error,
    configurable: false,
    tags: &["check"],
};
