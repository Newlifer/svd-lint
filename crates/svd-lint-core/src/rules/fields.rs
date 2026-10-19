use super::{CheckContext, ranges};
use crate::diagnostic_definition::{DiagnosticDefinition, RuleCategory};
use crate::{Diagnostic, Severity};

pub(crate) static ZERO_FIELD_WIDTH: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM002"),
    name: "zero-field-width",
    description: "A field must occupy a positive number of bits.",
    category: RuleCategory::Fields,
    default_severity: Severity::Error,
    configurable: true,
    tags: &["check"],
};

pub(crate) static FIELD_OUT_OF_BOUNDS: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM003"),
    name: "field-out-of-bounds",
    description: "A field extends past its register size.",
    category: RuleCategory::Fields,
    default_severity: Severity::Error,
    configurable: true,
    tags: &["check"],
};

pub(crate) static FIELD_OVERLAP: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM004"),
    name: "overlapping-fields",
    description: "Two fields occupy overlapping bit ranges.",
    category: RuleCategory::Fields,
    default_severity: Severity::Warning,
    configurable: true,
    tags: &["heuristic"],
};

pub(super) fn check_fields(context: &CheckContext<'_>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for peripheral in &context.device.peripherals {
        for register in &peripheral.registers {
            let size = register.properties.size.as_ref();
            let mut intervals = Vec::new();
            for field in &register.fields {
                if field.bit_width == 0 {
                    diagnostics.push(context.diagnostic(
                        &ZERO_FIELD_WIDTH,
                        Severity::Error,
                        "field width must be greater than zero",
                        &field.origin,
                        "bitWidth",
                    ));

                    continue;
                }
                let Some(size) = size.filter(|s| s.value > 0) else {
                    continue;
                };
                if let Some(interval) = ranges::field(field.bit_offset, field.bit_width, size.value)
                {
                    intervals.push((interval, field));
                } else {
                    let mut d = context.diagnostic(
                        &FIELD_OUT_OF_BOUNDS,
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
                            &FIELD_OVERLAP,
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

    diagnostics
}
