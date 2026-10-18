use super::{CheckContext, Rule, ranges};
use crate::registry::{
    Compatibility, DiagnosticDefinition, RuleCategory, RuleSource, SpecReference, SvdConvReference,
};
use crate::{Diagnostic, Severity};

pub(crate) static ZERO_FIELD_WIDTH: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM002"),
    name: "zero-field-width",
    description: "A field must occupy a positive number of bits.",
    category: RuleCategory::Fields,
    default_severity: Severity::Error,
    source: RuleSource::CmsisSvd,
    configurable: true,
    svdconv_codes: &[SvdConvReference {
        code: "M310",
        compatibility: Compatibility::Partial,
    }],
    spec_refs: &[SpecReference {
        title: "Fields / zero-field-width",
        url: "https://arm-software.github.io/CMSIS_5/SVD/html/elem_registers.html",
    }],
    tags: &["check"],
};

pub(crate) static ZERO_FIELD_WIDTH_CHECK: Rule = Rule {
    name: ZERO_FIELD_WIDTH.name,
    description: ZERO_FIELD_WIDTH.description,
    diagnostics: &[&ZERO_FIELD_WIDTH],
    check: |context, output| check_fields_code(&ZERO_FIELD_WIDTH, context, output),
};

pub(crate) static FIELD_OUT_OF_BOUNDS: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM003"),
    name: "field-out-of-bounds",
    description: "A field extends past its register size.",
    category: RuleCategory::Fields,
    default_severity: Severity::Error,
    source: RuleSource::CmsisSvd,
    configurable: true,
    svdconv_codes: &[
        SvdConvReference {
            code: "M324",
            compatibility: Compatibility::Partial,
        },
        SvdConvReference {
            code: "M345",
            compatibility: Compatibility::Partial,
        },
    ],
    spec_refs: &[SpecReference {
        title: "Fields / field-out-of-bounds",
        url: "https://arm-software.github.io/CMSIS_5/SVD/html/elem_registers.html",
    }],
    tags: &["check"],
};

pub(crate) static FIELD_OUT_OF_BOUNDS_CHECK: Rule = Rule {
    name: FIELD_OUT_OF_BOUNDS.name,
    description: FIELD_OUT_OF_BOUNDS.description,
    diagnostics: &[&FIELD_OUT_OF_BOUNDS],
    check: |context, output| check_fields_code(&FIELD_OUT_OF_BOUNDS, context, output),
};

pub(crate) static FIELD_OVERLAP: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM004"),
    name: "overlapping-fields",
    description: "Two fields occupy overlapping bit ranges.",
    category: RuleCategory::Fields,
    default_severity: Severity::Warning,
    source: RuleSource::SvdLint,
    configurable: true,
    svdconv_codes: &[SvdConvReference {
        code: "M338",
        compatibility: Compatibility::Equivalent,
    }],
    spec_refs: &[],
    tags: &["heuristic"],
};

pub(crate) static FIELD_OVERLAP_CHECK: Rule = Rule {
    name: FIELD_OVERLAP.name,
    description: FIELD_OVERLAP.description,
    diagnostics: &[&FIELD_OVERLAP],
    check: |context, output| check_fields_code(&FIELD_OVERLAP, context, output),
};

fn check_fields_code(
    definition: &DiagnosticDefinition,
    context: &CheckContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let code = definition.code();
    for peripheral in &context.device.peripherals {
        for register in &peripheral.registers {
            let size = register.properties.size.as_ref();
            let mut intervals = Vec::new();
            for field in &register.fields {
                if field.bit_width == 0 {
                    if code == ZERO_FIELD_WIDTH.code() {
                        diagnostics.push(context.diagnostic(
                            definition,
                            Severity::Error,
                            "field width must be greater than zero",
                            &field.origin,
                            "bitWidth",
                        ));
                    }
                    continue;
                }
                if code == ZERO_FIELD_WIDTH.code() {
                    continue;
                }
                let Some(size) = size.filter(|s| s.value > 0) else {
                    continue;
                };
                if let Some(interval) = ranges::field(field.bit_offset, field.bit_width, size.value)
                {
                    if code == FIELD_OVERLAP.code() {
                        intervals.push((interval, field));
                    }
                } else if code == FIELD_OUT_OF_BOUNDS.code() {
                    let mut d = context.diagnostic(
                        definition,
                        Severity::Error,
                        format!(
                            "field interval [{}, {}) exceeds register size {}",
                            field.bit_offset,
                            u64::from(field.bit_offset) + u64::from(field.bit_width),
                            size.value
                        ),
                        &field.origin,
                        "bitOffset",
                    );
                    context.related(&mut d, &size.origin, "size", "register size declared here");
                    diagnostics.push(d);
                }
            }
            intervals.sort_by_key(|(range, field)| (range.start, range.end, &field.name));
            for (i, &(range, field)) in intervals.iter().enumerate() {
                for &(other, following) in &intervals[i + 1..] {
                    if other.start >= range.end {
                        break;
                    }
                    if range.overlaps(other) {
                        let mut d = context.diagnostic(
                            definition,
                            Severity::Warning,
                            format!(
                                "field intervals [{}, {}) and [{}, {}) overlap",
                                range.start, range.end, other.start, other.end
                            ),
                            &following.origin,
                            "name",
                        );
                        context.related(&mut d, &field.origin, "name", "overlapping field");
                        diagnostics.push(d);
                    }
                }
            }
        }
    }
}
