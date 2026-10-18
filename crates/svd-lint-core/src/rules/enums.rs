use super::{CheckContext, Rule, ranges::fits};
use crate::registry::{
    Compatibility, DiagnosticDefinition, RuleCategory, RuleSource, SpecReference, SvdConvReference,
};
use crate::{Diagnostic, Severity, ir::*};
use std::collections::BTreeMap;

pub(crate) static ENUM_VALUE_OUT_OF_BOUNDS: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM012"),
    name: "enum-out-of-bounds",
    description: "A numeric enum value exceeds the field width.",
    category: RuleCategory::Enumerations,
    default_severity: Severity::Error,
    source: RuleSource::CmsisSvd,
    configurable: true,
    svdconv_codes: &[SvdConvReference {
        code: "M335",
        compatibility: Compatibility::Equivalent,
    }],
    spec_refs: &[SpecReference {
        title: "Enumerations / enum-out-of-bounds",
        url: "https://arm-software.github.io/CMSIS_5/SVD/html/elem_registers.html",
    }],
    tags: &["check"],
};

pub(crate) static ENUM_VALUE_OUT_OF_BOUNDS_CHECK: Rule = Rule {
    name: ENUM_VALUE_OUT_OF_BOUNDS.name,
    description: ENUM_VALUE_OUT_OF_BOUNDS.description,
    diagnostics: &[&ENUM_VALUE_OUT_OF_BOUNDS],
    check: |context, output| check_enums_code(&ENUM_VALUE_OUT_OF_BOUNDS, context, output),
};

pub(crate) static DUPLICATE_ENUM_ENTRY: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM013"),
    name: "duplicate-enum-entry",
    description: "Names or semantic descriptions conflict within one enum usage set.",
    category: RuleCategory::Enumerations,
    default_severity: Severity::Error,
    source: RuleSource::SvdLint,
    configurable: true,
    svdconv_codes: &[SvdConvReference {
        code: "M333",
        compatibility: Compatibility::Partial,
    }],
    spec_refs: &[],
    tags: &["heuristic"],
};

pub(crate) static DUPLICATE_ENUM_ENTRY_CHECK: Rule = Rule {
    name: DUPLICATE_ENUM_ENTRY.name,
    description: DUPLICATE_ENUM_ENTRY.description,
    diagnostics: &[&DUPLICATE_ENUM_ENTRY],
    check: |context, output| check_enums_code(&DUPLICATE_ENUM_ENTRY, context, output),
};

pub(crate) static MULTIPLE_ENUM_DEFAULTS: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM014"),
    name: "multiple-enum-defaults",
    description: "Only one entry may be default in an enum set.",
    category: RuleCategory::Enumerations,
    default_severity: Severity::Error,
    source: RuleSource::CmsisSvd,
    configurable: true,
    svdconv_codes: &[],
    spec_refs: &[SpecReference {
        title: "Enumerations / multiple-enum-defaults",
        url: "https://arm-software.github.io/CMSIS_5/SVD/html/elem_registers.html",
    }],
    tags: &["check"],
};

pub(crate) static MULTIPLE_ENUM_DEFAULTS_CHECK: Rule = Rule {
    name: MULTIPLE_ENUM_DEFAULTS.name,
    description: MULTIPLE_ENUM_DEFAULTS.description,
    diagnostics: &[&MULTIPLE_ENUM_DEFAULTS],
    check: |context, output| check_enums_code(&MULTIPLE_ENUM_DEFAULTS, context, output),
};

fn check_enums_code(
    definition: &DiagnosticDefinition,
    context: &CheckContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for peripheral in &context.device.peripherals {
        for register in &peripheral.registers {
            for field in &register.fields {
                if field.bit_width == 0 {
                    continue;
                }
                for values in &field.enumerated_values {
                    check_set(definition, context, field, values, diagnostics);
                }
            }
        }
    }
}

fn check_set(
    definition: &DiagnosticDefinition,
    context: &CheckContext<'_>,
    field: &CanonicalField,
    values: &CanonicalEnumeratedValues,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let code = definition.code();
    let duplicates = code == DUPLICATE_ENUM_ENTRY.code();
    let bounds = code == ENUM_VALUE_OUT_OF_BOUNDS.code();
    let defaults = code == MULTIPLE_ENUM_DEFAULTS.code();
    let mut names: BTreeMap<&str, usize> = BTreeMap::new();
    let mut numbers = BTreeMap::new();
    let mut default = None;
    for (i, value) in values.values.value.iter().enumerate() {
        let origin = context.enum_item_origin(values, i);
        let pattern = if duplicates {
            context
                .element_node(&origin, "value")
                .and_then(|n| context.source_map.binary_pattern(n))
        } else {
            None
        };
        if value.is_default == Some(true) {
            if defaults {
                if let Some(previous) = default {
                    let mut d = context.diagnostic(
                        definition,
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
            }
        } else if let Some(number) = value.value {
            if bounds && !fits(number, field.bit_width) {
                let mut d = context.diagnostic(
                    definition,
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
            if duplicates {
                // A wildcard and its zero-filled numeric value are distinct.
                let key = (number, pattern);
                if let Some(previous) = numbers.insert(key, i) {
                    let earlier: &crate::svd::EnumeratedValue = &values.values.value[previous];
                    if earlier.name != value.name || earlier.description != value.description {
                        let mut d = context.diagnostic(definition, Severity::Warning,
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
        }
        if duplicates {
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
                    definition,
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
}
