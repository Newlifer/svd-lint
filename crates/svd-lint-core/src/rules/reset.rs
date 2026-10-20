use super::{CheckContext, ranges::fits};
use crate::diagnostic_definition::DiagnosticDefinition;
use crate::{CanonicalRegister, Diagnostic, Resolved, Severity};

pub(crate) static RESET_VALUE_OUT_OF_BOUNDS: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("E021"),
    name: "reset-value-out-of-bounds",
    description: "The effective reset value exceeds register width.",
    severity: Severity::Error,
    configurable: true,
    tags: &["check"],
};

pub(crate) static RESET_MASK_OUT_OF_BOUNDS: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("E022"),
    name: "reset-mask-out-of-bounds",
    description: "The effective reset mask exceeds register width.",
    severity: Severity::Error,
    configurable: true,
    tags: &["check"],
};

pub(crate) static RESET_OUTSIDE_MASK: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("W004"),
    name: "reset-outside-mask",
    description: "Reset bits outside the declared mask may be suspicious.",
    severity: Severity::Warning,
    configurable: true,
    tags: &["heuristic"],
};

/// Both properties fit the register width; mask analysis cannot create width cascades.
struct ResetPair<'a> {
    value: &'a Resolved<u64>,
    mask: &'a Resolved<u64>,
}

pub(super) fn check_reset(context: &CheckContext<'_>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for peripheral in &context.device.peripherals {
        for register in &peripheral.registers {
            let (pair, bounds_diagnostics) = check_bounds(context, register);
            diagnostics.extend(bounds_diagnostics);
            if let Some(pair) = pair {
                diagnostics.extend(check_mask(context, pair));
            }
        }
    }
    diagnostics
}

fn check_bounds<'a>(
    context: &CheckContext<'_>,
    register: &'a CanonicalRegister,
) -> (Option<ResetPair<'a>>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let Some(size) = register.properties.size.as_ref().filter(|s| s.value > 0) else {
        return (None, diagnostics);
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
                    format!("{tag} {:#x} does not fit in {width} bits", value.value),
                    &value.origin,
                    tag,
                );
                context.related(&mut d, &size.origin, "size", "register size declared here");
                diagnostics.push(d);
            }
        }
    }
    let pair = match (
        &register.properties.reset_value,
        &register.properties.reset_mask,
    ) {
        (Some(value), Some(mask)) if fits(value.value, width) && fits(mask.value, width) => {
            Some(ResetPair { value, mask })
        }
        _ => None,
    };
    (pair, diagnostics)
}

fn check_mask(context: &CheckContext<'_>, pair: ResetPair<'_>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let outside = pair.value.value & !pair.mask.value;
    if outside != 0 {
        let mut d = context.diagnostic(
            &RESET_OUTSIDE_MASK,
            format!("resetValue has bits {outside:#x} outside resetMask; those reset bits are unspecified"),
            &pair.value.origin,
            "resetValue",
        );
        context.related(
            &mut d,
            &pair.mask.origin,
            "resetMask",
            "reset mask declared here",
        );
        diagnostics.push(d);
    }
    diagnostics
}
