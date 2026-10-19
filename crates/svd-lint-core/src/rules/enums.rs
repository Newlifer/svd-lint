use super::{CheckContext, ranges::fits};
use crate::diagnostic_definition::{DiagnosticDefinition, RuleCategory};
use crate::{Diagnostic, Severity, ir::*};
use std::collections::BTreeMap;

pub(crate) static ENUM_VALUE_OUT_OF_BOUNDS: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM012"),
    name: "enum-out-of-bounds",
    description: "A numeric enum value exceeds the field width.",
    category: RuleCategory::Enumerations,
    default_severity: Severity::Error,
    configurable: true,
    tags: &["check"],
};

pub(crate) static DUPLICATE_ENUM_ENTRY: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM013"),
    name: "duplicate-enum-entry",
    description: "Names or semantic descriptions conflict within one enum usage set.",
    category: RuleCategory::Enumerations,
    default_severity: Severity::Error,
    configurable: true,
    tags: &["heuristic"],
};

pub(crate) static MULTIPLE_ENUM_DEFAULTS: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM014"),
    name: "multiple-enum-defaults",
    description: "Only one entry may be default in an enum set.",
    category: RuleCategory::Enumerations,
    default_severity: Severity::Error,
    configurable: true,
    tags: &["check"],
};

pub(super) fn check_enums(context: &CheckContext<'_>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for peripheral in &context.device.peripherals {
        for register in &peripheral.registers {
            for field in &register.fields {
                if field.bit_width == 0 {
                    continue;
                }
                for values in &field.enumerated_values {
                    diagnostics.extend(check_set(context, field, values));
                }
            }
        }
    }

    diagnostics
}

struct EnumEntry<'a> {
    value: &'a crate::svd::EnumeratedValue,
    origin: crate::Origin,
    pattern: Option<&'a str>,
}

fn prepare_entries<'a>(
    context: &'a CheckContext<'_>,
    values: &'a CanonicalEnumeratedValues,
) -> Vec<EnumEntry<'a>> {
    values
        .values
        .value
        .iter()
        .enumerate()
        .map(|(i, value)| {
            let origin = context.enum_item_origin(values, i);
            let pattern = context
                .element_node(&origin, "value")
                .and_then(|node| context.source_map.binary_pattern(node));
            EnumEntry {
                value,
                origin,
                pattern,
            }
        })
        .collect()
}

fn check_set(
    context: &CheckContext<'_>,
    field: &CanonicalField,
    values: &CanonicalEnumeratedValues,
) -> Vec<Diagnostic> {
    let entries = prepare_entries(context, values);
    let mut diagnostics = check_bounds(context, field, &entries);
    diagnostics.extend(check_defaults(context, &entries));
    diagnostics.extend(check_duplicate_values(context, &entries));
    diagnostics.extend(check_duplicate_names(context, &entries));
    diagnostics
}

fn check_bounds(
    context: &CheckContext<'_>,
    field: &CanonicalField,
    entries: &[EnumEntry<'_>],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for entry in entries {
        // Default entries describe the remaining values, not a numeric enum value.
        if entry.value.is_default == Some(true) {
            continue;
        }
        if let Some(number) = entry.value.value {
            if !fits(number, field.bit_width) {
                let mut d = context.diagnostic(
                    &ENUM_VALUE_OUT_OF_BOUNDS,
                    Severity::Error,
                    format!(
                        "enumerated value {number:#x} does not fit in {} bits",
                        field.bit_width
                    ),
                    &entry.origin,
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
        }
    }
    diagnostics
}

fn check_defaults(context: &CheckContext<'_>, entries: &[EnumEntry<'_>]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut default: Option<&EnumEntry<'_>> = None;
    for entry in entries
        .iter()
        .filter(|entry| entry.value.is_default == Some(true))
    {
        if let Some(previous) = default {
            let mut d = context.diagnostic(
                &MULTIPLE_ENUM_DEFAULTS,
                Severity::Error,
                "enumeratedValues contains multiple isDefault=true entries",
                &entry.origin,
                "isDefault",
            );
            context.related(
                &mut d,
                &previous.origin,
                "isDefault",
                "previous default entry",
            );
            diagnostics.push(d);
        } else {
            default = Some(entry);
        }
    }
    diagnostics
}

fn check_duplicate_values(
    context: &CheckContext<'_>,
    entries: &[EnumEntry<'_>],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut numbers = BTreeMap::new();
    for (i, entry) in entries.iter().enumerate() {
        if entry.value.is_default == Some(true) {
            continue;
        }
        let Some(number) = entry.value.value else {
            continue;
        };
        // A wildcard and its zero-filled numeric value are distinct.
        if let Some(previous) = numbers.insert((number, entry.pattern), i) {
            let earlier: &EnumEntry<'_> = &entries[previous];
            if earlier.value.name != entry.value.name
                || earlier.value.description != entry.value.description
            {
                let mut d = context.diagnostic(
                    &DUPLICATE_ENUM_ENTRY,
                    Severity::Warning,
                    format!("enumerated value {number:#x} has multiple semantic descriptions in one usage set"),
                    &entry.origin,
                    "value",
                );
                context.related(
                    &mut d,
                    &earlier.origin,
                    "value",
                    "previous value description",
                );
                diagnostics.push(d);
            }
        }
    }
    diagnostics
}

fn check_duplicate_names(context: &CheckContext<'_>, entries: &[EnumEntry<'_>]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut names = BTreeMap::new();
    for (i, entry) in entries.iter().enumerate() {
        if let Some(previous) = names.insert(&entry.value.name, i) {
            let earlier: &EnumEntry<'_> = &entries[previous];
            let contradictory = earlier.value.value != entry.value.value
                || earlier.value.is_default != entry.value.is_default
                || earlier.pattern != entry.pattern;
            let mut d = context.diagnostic(
                &DUPLICATE_ENUM_ENTRY,
                if contradictory {
                    Severity::Error
                } else {
                    Severity::Warning
                },
                format!(
                    "duplicate enumerated name `{}` in one usage set",
                    entry.value.name
                ),
                &entry.origin,
                "name",
            );
            context.related(
                &mut d,
                &earlier.origin,
                "name",
                "previous entry with this name",
            );
            diagnostics.push(d);
        }
    }
    diagnostics
}
