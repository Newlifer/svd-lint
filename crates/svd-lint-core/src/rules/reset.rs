use super::{CheckContext, Rule, ranges::fits};
use crate::registry::{DiagnosticDefinition, RuleCategory, RuleSource, SpecReference};
use crate::{Diagnostic, Severity};

pub(crate) static RESET_VALUE_OUT_OF_BOUNDS: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM005"),
    name: "reset-value-out-of-bounds",
    description: "The effective reset value exceeds register width.",
    category: RuleCategory::Reset,
    default_severity: Severity::Error,
    source: RuleSource::CmsisSvd,
    configurable: true,
    svdconv_codes: &[],
    spec_refs: &[SpecReference {
        title: "Reset / reset-value-out-of-bounds",
        url: "https://arm-software.github.io/CMSIS_5/SVD/html/elem_registers.html",
    }],
    tags: &["check"],
};

pub(crate) static RESET_VALUE_OUT_OF_BOUNDS_CHECK: Rule = Rule {
    name: RESET_VALUE_OUT_OF_BOUNDS.name,
    description: RESET_VALUE_OUT_OF_BOUNDS.description,
    diagnostics: &[&RESET_VALUE_OUT_OF_BOUNDS],
    check: |context, output| check_reset_code(&RESET_VALUE_OUT_OF_BOUNDS, context, output),
};

pub(crate) static RESET_MASK_OUT_OF_BOUNDS: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM006"),
    name: "reset-mask-out-of-bounds",
    description: "The effective reset mask exceeds register width.",
    category: RuleCategory::Reset,
    default_severity: Severity::Error,
    source: RuleSource::CmsisSvd,
    configurable: true,
    svdconv_codes: &[],
    spec_refs: &[SpecReference {
        title: "Reset / reset-mask-out-of-bounds",
        url: "https://arm-software.github.io/CMSIS_5/SVD/html/elem_registers.html",
    }],
    tags: &["check"],
};

pub(crate) static RESET_MASK_OUT_OF_BOUNDS_CHECK: Rule = Rule {
    name: RESET_MASK_OUT_OF_BOUNDS.name,
    description: RESET_MASK_OUT_OF_BOUNDS.description,
    diagnostics: &[&RESET_MASK_OUT_OF_BOUNDS],
    check: |context, output| check_reset_code(&RESET_MASK_OUT_OF_BOUNDS, context, output),
};

pub(crate) static RESET_OUTSIDE_MASK: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM007"),
    name: "reset-outside-mask",
    description: "Reset bits outside the declared mask may be suspicious.",
    category: RuleCategory::Reset,
    default_severity: Severity::Warning,
    source: RuleSource::SvdLint,
    configurable: true,
    svdconv_codes: &[],
    spec_refs: &[],
    tags: &["heuristic"],
};

pub(crate) static RESET_OUTSIDE_MASK_CHECK: Rule = Rule {
    name: RESET_OUTSIDE_MASK.name,
    description: RESET_OUTSIDE_MASK.description,
    diagnostics: &[&RESET_OUTSIDE_MASK],
    check: |context, output| check_reset_code(&RESET_OUTSIDE_MASK, context, output),
};

fn check_reset_code(
    definition: &DiagnosticDefinition,
    context: &CheckContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let code = definition.code();
    for peripheral in &context.device.peripherals {
        for register in &peripheral.registers {
            let Some(width) = register
                .properties
                .size
                .as_ref()
                .filter(|s| s.value > 0)
                .map(|s| s.value)
            else {
                continue;
            };
            if code == RESET_OUTSIDE_MASK.code() {
                if let (Some(value), Some(mask)) = (
                    &register.properties.reset_value,
                    &register.properties.reset_mask,
                ) {
                    if fits(value.value, width)
                        && fits(mask.value, width)
                        && value.value & !mask.value != 0
                    {
                        let mut d = context.diagnostic(definition, Severity::Warning,
                            format!("resetValue has bits {:#x} outside resetMask; those reset bits are unspecified", value.value & !mask.value),
                            &value.origin, "resetValue");
                        context.related(
                            &mut d,
                            &mask.origin,
                            "resetMask",
                            "reset mask declared here",
                        );
                        diagnostics.push(d);
                    }
                }
            } else {
                let (value, tag) = if code == RESET_VALUE_OUT_OF_BOUNDS.code() {
                    (&register.properties.reset_value, "resetValue")
                } else {
                    (&register.properties.reset_mask, "resetMask")
                };
                if let Some(value) = value {
                    if !fits(value.value, width) {
                        let mut d = context.diagnostic(
                            definition,
                            Severity::Error,
                            format!("{tag} {:#x} does not fit in {width} bits", value.value),
                            &value.origin,
                            tag,
                        );
                        context.related(
                            &mut d,
                            &register.properties.size.as_ref().unwrap().origin,
                            "size",
                            "register size declared here",
                        );
                        diagnostics.push(d);
                    }
                }
            }
        }
    }
}
