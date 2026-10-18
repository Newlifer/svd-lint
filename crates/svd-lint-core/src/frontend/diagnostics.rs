//! Diagnostic metadata for this pipeline component.
use crate::registry::{
    Compatibility, DiagnosticDefinition, RuleCategory, RuleSource, SpecReference, SvdConvReference,
};
pub static IO_READ_FAILED: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("IO001"),
    name: "io-read-failed",
    description: "The input file could not be read.",
    category: RuleCategory::Structure,
    default_severity: crate::Severity::Error,
    source: RuleSource::SvdLint,
    configurable: false,
    svdconv_codes: &[],
    spec_refs: &[],
    tags: &["check"],
};

pub static IO_INVALID_UTF8: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("IO002"),
    name: "io-invalid-utf8",
    description: "The input is not valid UTF-8.",
    category: RuleCategory::Structure,
    default_severity: crate::Severity::Error,
    source: RuleSource::SvdLint,
    configurable: false,
    svdconv_codes: &[],
    spec_refs: &[],
    tags: &["check"],
};

pub static XML_SYNTAX: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("XML001"),
    name: "xml-syntax",
    description: "The input is not well-formed XML.",
    category: RuleCategory::Structure,
    default_severity: crate::Severity::Error,
    source: RuleSource::CmsisSvd,
    configurable: false,
    svdconv_codes: &[],
    spec_refs: &[SpecReference {
        title: "Structure / xml-syntax",
        url: "https://arm-software.github.io/CMSIS_5/SVD/html/elem_device.html",
    }],
    tags: &["check"],
};

pub static INVALID_ROOT_ELEMENT: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SVD001"),
    name: "invalid-root",
    description: "The root element must be device.",
    category: RuleCategory::Structure,
    default_severity: crate::Severity::Error,
    source: RuleSource::CmsisSvd,
    configurable: false,
    svdconv_codes: &[],
    spec_refs: &[SpecReference {
        title: "Structure / invalid-root",
        url: "https://arm-software.github.io/CMSIS_5/SVD/html/elem_device.html",
    }],
    tags: &["check"],
};

pub static MISSING_REQUIRED_ELEMENT: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SVD002"),
    name: "missing-required-element",
    description: "A required element is absent.",
    category: RuleCategory::Structure,
    default_severity: crate::Severity::Error,
    source: RuleSource::CmsisSvd,
    configurable: false,
    svdconv_codes: &[SvdConvReference {
        code: "M316",
        compatibility: Compatibility::Partial,
    }],
    spec_refs: &[SpecReference {
        title: "Structure / missing-required-element",
        url: "https://arm-software.github.io/CMSIS_5/SVD/html/elem_device.html",
    }],
    tags: &["check"],
};

pub static DUPLICATE_SINGLETON_ELEMENT: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SVD003"),
    name: "duplicate-singleton",
    description: "A singleton element is repeated.",
    category: RuleCategory::Structure,
    default_severity: crate::Severity::Error,
    source: RuleSource::CmsisSvd,
    configurable: false,
    svdconv_codes: &[SvdConvReference {
        code: "M203",
        compatibility: Compatibility::Partial,
    }],
    spec_refs: &[SpecReference {
        title: "Structure / duplicate-singleton",
        url: "https://arm-software.github.io/CMSIS_5/SVD/html/elem_device.html",
    }],
    tags: &["check"],
};

pub static EMPTY_REQUIRED_ELEMENT: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SVD004"),
    name: "empty-required-element",
    description: "A required element is empty.",
    category: RuleCategory::Structure,
    default_severity: crate::Severity::Error,
    source: RuleSource::CmsisSvd,
    configurable: false,
    svdconv_codes: &[SvdConvReference {
        code: "M205",
        compatibility: Compatibility::Partial,
    }],
    spec_refs: &[SpecReference {
        title: "Structure / empty-required-element",
        url: "https://arm-software.github.io/CMSIS_5/SVD/html/elem_device.html",
    }],
    tags: &["check"],
};

pub static MISSING_PERIPHERAL_DESCRIPTION: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SVD005"),
    name: "missing-peripheral-description",
    description: "A peripheral description improves readability.",
    category: RuleCategory::Quality,
    default_severity: crate::Severity::Warning,
    source: RuleSource::SvdLint,
    configurable: true,
    svdconv_codes: &[SvdConvReference {
        code: "M317",
        compatibility: Compatibility::Partial,
    }],
    spec_refs: &[],
    tags: &["check"],
};

pub static SVD_PARSE_ERROR: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SVD006"),
    name: "typed-parse-failed",
    description: "The typed SVD model could not be built, or XML nesting exceeds the tool resource limit.",
    category: RuleCategory::Structure,
    default_severity: crate::Severity::Error,
    source: RuleSource::SvdLint,
    configurable: false,
    svdconv_codes: &[
        SvdConvReference {
            code: "M202",
            compatibility: Compatibility::Partial,
        },
        SvdConvReference {
            code: "M204",
            compatibility: Compatibility::Partial,
        },
    ],
    spec_refs: &[],
    tags: &["check"],
};
