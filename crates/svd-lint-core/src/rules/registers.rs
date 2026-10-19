use super::CheckContext;
use crate::diagnostic_definition::{DiagnosticDefinition, RuleCategory};
use crate::{Diagnostic, Severity};

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
    let definition = &ZERO_REGISTER_SIZE;
    for peripheral in &context.device.peripherals {
        for register in &peripheral.registers {
            if let Some(size) = &register.properties.size {
                if size.value == 0 {
                    diagnostics.push(context.diagnostic(
                        definition,
                        Severity::Error,
                        "register size must be greater than zero",
                        &size.origin,
                        "size",
                    ));
                }
            }
        }
    }

    diagnostics
}
