use super::CheckContext;
use crate::diagnostic_definition::{DiagnosticDefinition, RuleCategory};
use crate::{CanonicalRegister, Diagnostic, Severity};

pub(crate) static ZERO_REGISTER_SIZE: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM001"),
    name: "zero-register-size",
    description: "A register must occupy a positive number of bits.",
    category: RuleCategory::Registers,
    default_severity: Severity::Error,
    configurable: true,
    tags: &["check"],
};

pub(super) fn check_registers(context: &CheckContext<'_>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for peripheral in &context.device.peripherals {
        for register in &peripheral.registers {
            diagnostics.extend(check_size(context, register));
        }
    }
    diagnostics
}

fn check_size(context: &CheckContext<'_>, register: &CanonicalRegister) -> Option<Diagnostic> {
    let size = register.properties.size.as_ref()?;
    (size.value == 0).then(|| {
        context.diagnostic(
            &ZERO_REGISTER_SIZE,
            Severity::Error,
            "register size must be greater than zero",
            &size.origin,
            "size",
        )
    })
}
