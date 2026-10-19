//! Diagnostic metadata for this pipeline component.
use crate::diagnostic_definition::{DiagnosticDefinition, RuleCategory, RuleSource};
pub static MISSING_REFERENCE: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("NORM001"),
    name: "missing-reference",
    description: "A derivedFrom target cannot be resolved.",
    category: RuleCategory::Normalization,
    default_severity: crate::Severity::Error,
    source: RuleSource::CmsisSvd,
    configurable: false,
    tags: &["check"],
};

pub static CYCLE: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("NORM002"),
    name: "reference-cycle",
    description: "A derivedFrom cycle prevents normalization.",
    category: RuleCategory::Normalization,
    default_severity: crate::Severity::Error,
    source: RuleSource::CmsisSvd,
    configurable: false,
    tags: &["check"],
};

pub static AMBIGUOUS_REFERENCE: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("NORM003"),
    name: "ambiguous-reference",
    description: "A reference or canonical path is ambiguous.",
    category: RuleCategory::Normalization,
    default_severity: crate::Severity::Error,
    source: RuleSource::SvdLint,
    configurable: false,
    tags: &["check"],
};

pub static INVALID_DIMENSION: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("NORM004"),
    name: "invalid-dimension",
    description: "An array dimension is invalid.",
    category: RuleCategory::Normalization,
    default_severity: crate::Severity::Error,
    source: RuleSource::CmsisSvd,
    configurable: false,
    tags: &["check"],
};

pub static INVALID_INDEX: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("NORM005"),
    name: "invalid-array-index",
    description: "Array indices do not match the dimension.",
    category: RuleCategory::Normalization,
    default_severity: crate::Severity::Error,
    source: RuleSource::CmsisSvd,
    configurable: false,
    tags: &["check"],
};

pub static INVALID_TEMPLATE: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("NORM006"),
    name: "invalid-array-template",
    description: "The array name template cannot be expanded.",
    category: RuleCategory::Normalization,
    default_severity: crate::Severity::Error,
    source: RuleSource::CmsisSvd,
    configurable: false,
    tags: &["check"],
};

pub static ADDRESS_OVERFLOW: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("NORM007"),
    name: "address-overflow",
    description: "An address or field offset cannot be represented.",
    category: RuleCategory::Normalization,
    default_severity: crate::Severity::Error,
    source: RuleSource::SvdLint,
    configurable: false,
    tags: &["check"],
};

pub static MISSING_PROPERTY: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("NORM008"),
    name: "missing-register-size",
    description: "The effective register size cannot be determined.",
    category: RuleCategory::Normalization,
    default_severity: crate::Severity::Error,
    source: RuleSource::SvdLint,
    configurable: false,
    tags: &["check"],
};

pub static EXPANSION_LIMIT: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("NORM009"),
    name: "expansion-limit",
    description: "The materialization budget or dependency depth limit was exceeded.",
    category: RuleCategory::Normalization,
    default_severity: crate::Severity::Error,
    source: RuleSource::SvdLint,
    configurable: false,
    tags: &["check"],
};
