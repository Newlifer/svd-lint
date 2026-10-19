use super::{CheckContext, ranges::fits};
use crate::diagnostic_definition::{DiagnosticDefinition, RuleCategory};
use crate::{Diagnostic, Severity};

pub(crate) static RESET_VALUE_OUT_OF_BOUNDS: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM005"),
    name: "reset-value-out-of-bounds",
    description: "The effective reset value exceeds register width.",
    category: RuleCategory::Reset,
    default_severity: Severity::Error,
    configurable: true,
    tags: &["check"],
};

pub(crate) static RESET_MASK_OUT_OF_BOUNDS: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM006"),
    name: "reset-mask-out-of-bounds",
    description: "The effective reset mask exceeds register width.",
    category: RuleCategory::Reset,
    default_severity: Severity::Error,
    configurable: true,
    tags: &["check"],
};

pub(crate) static RESET_OUTSIDE_MASK: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM007"),
    name: "reset-outside-mask",
    description: "Reset bits outside the declared mask may be suspicious.",
    category: RuleCategory::Reset,
    default_severity: Severity::Warning,
    configurable: true,
    tags: &["heuristic"],
};

pub(super) fn check_reset(context: &CheckContext<'_>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for peripheral in &context.device.peripherals {
        for register in &peripheral.registers {
            let Some(size) = register.properties.size.as_ref().filter(|s| s.value > 0) else {
                continue;
            };
            let width = size.value;
            for (definition, value, tag) in [
                (
                    &RESET_VALUE_OUT_OF_BOUNDS,
                    &register.properties.reset_value,
                    "resetValue",
                ),
                (
                    &RESET_MASK_OUT_OF_BOUNDS,
                    &register.properties.reset_mask,
                    "resetMask",
                ),
            ] {
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
                            &size.origin,
                            "size",
                            "register size declared here",
                        );
                        diagnostics.push(d);
                    }
                }
            }
            if let (Some(value), Some(mask)) = (
                &register.properties.reset_value,
                &register.properties.reset_mask,
            ) {
                // Invalid widths are diagnosed above; do not cascade into a mask warning.
                if fits(value.value, width)
                    && fits(mask.value, width)
                    && value.value & !mask.value != 0
                {
                    let mut d = context.diagnostic(
                        &RESET_OUTSIDE_MASK,
                        Severity::Warning,
                        format!("resetValue has bits {:#x} outside resetMask; those reset bits are unspecified", value.value & !mask.value),
                        &value.origin,
                        "resetValue",
                    );
                    context.related(
                        &mut d,
                        &mask.origin,
                        "resetMask",
                        "reset mask declared here",
                    );
                    diagnostics.push(d);
                }
            }
        }
    }

    diagnostics
}
