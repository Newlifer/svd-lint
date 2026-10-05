use super::{CheckContext, Rule, ranges::fits};
use crate::{Diagnostic, DiagnosticCode as Code, Severity, ir::*};
use std::collections::BTreeMap;

pub(super) struct Enums;
impl Rule for Enums {
    fn id(&self) -> &'static str {
        "enums"
    }
    fn check(&self, context: &CheckContext<'_>, diagnostics: &mut Vec<Diagnostic>) {
        for peripheral in &context.device.peripherals {
            for register in &peripheral.registers {
                for field in &register.fields {
                    if field.bit_width == 0 {
                        continue;
                    }
                    for values in &field.enumerated_values {
                        check_set(context, field, values, diagnostics);
                    }
                }
            }
        }
    }
}

fn check_set(
    context: &CheckContext<'_>,
    field: &CanonicalField,
    values: &CanonicalEnumeratedValues,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut names: BTreeMap<&str, usize> = BTreeMap::new();
    // Wildcard masks distinguish #1x from the numeric value 2; do not infer
    // duplicate semantics merely from svd-parser's zero-filled numeric value.
    let mut numbers = BTreeMap::new();
    let mut default = None;
    for (i, value) in values.values.value.iter().enumerate() {
        let origin = context.enum_item_origin(values, i);
        let pattern = context
            .element_node(&origin, "value")
            .and_then(|n| context.source_map.binary_pattern(n));
        if value.is_default == Some(true) {
            if let Some(previous) = default {
                let mut d = context.diagnostic(
                    Code::SemMultipleEnumDefaults,
                    Severity::Error,
                    "enumeratedValues contains multiple isDefault=true entries",
                    &origin,
                    "isDefault",
                );
                context.related(
                    &mut d,
                    &context.enum_item_origin(values, previous),
                    "isDefault",
                    "previous default entry",
                );
                diagnostics.push(d);
            } else {
                default = Some(i);
            }
        } else if let Some(number) = value.value {
            if !fits(number, field.bit_width) {
                let mut d = context.diagnostic(
                    Code::SemEnumValueOutOfBounds,
                    Severity::Error,
                    format!(
                        "enumerated value {number:#x} does not fit in {} bits",
                        field.bit_width
                    ),
                    &origin,
                    "value",
                );
                context.related(
                    &mut d,
                    &field.origin,
                    "bitWidth",
                    "field width declared here",
                );
                diagnostics.push(d);
            }
            let key = (number, pattern);
            if let Some(previous) = numbers.insert(key, i) {
                let earlier = &values.values.value[previous];
                if earlier.name != value.name || earlier.description != value.description {
                    let mut d = context.diagnostic(Code::SemDuplicateEnumEntry, Severity::Warning,
                        format!("enumerated value {number:#x} has multiple semantic descriptions in one usage set"), &origin, "value");
                    context.related(
                        &mut d,
                        &context.enum_item_origin(values, previous),
                        "value",
                        "previous value description",
                    );
                    diagnostics.push(d);
                }
            }
        }
        if let Some(previous) = names.insert(&value.name, i) {
            let earlier = &values.values.value[previous];
            let previous_origin = context.enum_item_origin(values, previous);
            let previous_pattern = context
                .element_node(&previous_origin, "value")
                .and_then(|n| context.source_map.binary_pattern(n));
            let contradictory = earlier.value != value.value
                || earlier.is_default != value.is_default
                || pattern != previous_pattern;
            let mut d = context.diagnostic(
                Code::SemDuplicateEnumEntry,
                if contradictory {
                    Severity::Error
                } else {
                    Severity::Warning
                },
                format!(
                    "duplicate enumerated name `{}` in one usage set",
                    value.name
                ),
                &origin,
                "name",
            );
            context.related(
                &mut d,
                &previous_origin,
                "name",
                "previous entry with this name",
            );
            diagnostics.push(d);
        }
    }
}
