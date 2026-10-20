use super::{CheckContext, ranges};
use crate::diagnostic_definition::DiagnosticDefinition;
use crate::{CanonicalField, CanonicalRegister, Diagnostic, Severity};

pub(crate) static ZERO_FIELD_WIDTH: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("E019"),
    name: "zero-field-width",
    description: "A field must occupy a positive number of bits.",
    severity: Severity::Error,
    configurable: true,
    tags: &["check"],
};

pub(crate) static FIELD_OUT_OF_BOUNDS: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("E020"),
    name: "field-out-of-bounds",
    description: "A field extends past its register size.",
    severity: Severity::Error,
    configurable: true,
    tags: &["check"],
};

pub(crate) static FIELD_OVERLAP: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("W003"),
    name: "overlapping-fields",
    description: "Two fields occupy overlapping bit ranges.",
    severity: Severity::Warning,
    configurable: true,
    tags: &["heuristic"],
};

/// Only fields with a positive width and valid register bounds enter overlap analysis.
struct FieldRange<'a> {
    field: &'a CanonicalField,
    interval: ranges::Interval,
}

pub(super) fn check_fields(context: &CheckContext<'_>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for peripheral in &context.device.peripherals {
        for register in &peripheral.registers {
            let (ranges, bounds_diagnostics) = check_bounds(context, register);
            diagnostics.extend(bounds_diagnostics);
            diagnostics.extend(check_overlaps(context, ranges));
        }
    }
    diagnostics
}

fn check_bounds<'a>(
    context: &CheckContext<'_>,
    register: &'a CanonicalRegister,
) -> (Vec<FieldRange<'a>>, Vec<Diagnostic>) {
    let mut ranges = Vec::new();
    let mut diagnostics = Vec::new();
    let size = register.properties.size.as_ref();
    for field in &register.fields {
        if field.bit_width == 0 {
            diagnostics.push(context.diagnostic(
                &ZERO_FIELD_WIDTH,
                "field width must be greater than zero",
                &field.origin,
                "bitWidth",
            ));
            continue;
        }
        let Some(size) = size.filter(|s| s.value > 0) else {
            continue;
        };
        if let Some(interval) = ranges::field(field.bit_offset, field.bit_width, size.value) {
            ranges.push(FieldRange { field, interval });
        } else {
            let mut d = context.diagnostic(
                &FIELD_OUT_OF_BOUNDS,
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
    (ranges, diagnostics)
}

fn check_overlaps(context: &CheckContext<'_>, mut ranges: Vec<FieldRange<'_>>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    ranges.sort_by_key(|range| (range.interval.start, range.interval.end, &range.field.name));
    for (i, left) in ranges.iter().enumerate() {
        for right in &ranges[i + 1..] {
            if right.interval.start >= left.interval.end {
                break;
            }
            if left.interval.overlaps(right.interval) {
                let mut d = context.diagnostic(
                    &FIELD_OVERLAP,
                    format!(
                        "field intervals [{}, {}) and [{}, {}) overlap",
                        left.interval.start,
                        left.interval.end,
                        right.interval.start,
                        right.interval.end
                    ),
                    &right.field.origin,
                    "name",
                );
                context.related(&mut d, &left.field.origin, "name", "overlapping field");
                diagnostics.push(d);
            }
        }
    }
    diagnostics
}
