//! Diagnostic metadata for this pipeline component.
use crate::registry::{
    Compatibility, DiagnosticDefinition, RuleCategory, RuleSource, SpecReference, SvdConvReference,
};
pub static MISSING_REFERENCE: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("NORM001"),
    name: "missing-reference",
    description: "A derivedFrom target cannot be resolved.",
    category: RuleCategory::Normalization,
    default_severity: crate::Severity::Error,
    source: RuleSource::CmsisSvd,
    configurable: false,
    svdconv_codes: &[SvdConvReference {
        code: "M206",
        compatibility: Compatibility::Equivalent,
    }],
    spec_refs: &[SpecReference {
        title: "Normalization / missing-reference",
        url: "https://arm-software.github.io/CMSIS_5/SVD/html/svd_description.html",
    }],
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
    svdconv_codes: &[],
    spec_refs: &[SpecReference {
        title: "Normalization / reference-cycle",
        url: "https://arm-software.github.io/CMSIS_5/SVD/html/svd_description.html",
    }],
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
    svdconv_codes: &[SvdConvReference {
        code: "M336",
        compatibility: Compatibility::Partial,
    }],
    spec_refs: &[],
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
    svdconv_codes: &[],
    spec_refs: &[SpecReference {
        title: "Normalization / invalid-dimension",
        url: "https://arm-software.github.io/CMSIS_5/SVD/html/svd_description.html",
    }],
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
    svdconv_codes: &[SvdConvReference {
        code: "M308",
        compatibility: Compatibility::Partial,
    }],
    spec_refs: &[SpecReference {
        title: "Normalization / invalid-array-index",
        url: "https://arm-software.github.io/CMSIS_5/SVD/html/svd_description.html",
    }],
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
    svdconv_codes: &[
        SvdConvReference {
            code: "M207",
            compatibility: Compatibility::Partial,
        },
        SvdConvReference {
            code: "M239",
            compatibility: Compatibility::Partial,
        },
    ],
    spec_refs: &[SpecReference {
        title: "Normalization / invalid-array-template",
        url: "https://arm-software.github.io/CMSIS_5/SVD/html/svd_description.html",
    }],
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
    svdconv_codes: &[],
    spec_refs: &[],
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
    svdconv_codes: &[],
    spec_refs: &[],
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
    svdconv_codes: &[],
    spec_refs: &[],
    tags: &["check"],
};
