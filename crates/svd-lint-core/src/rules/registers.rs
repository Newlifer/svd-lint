use super::{CheckContext, Rule};
use crate::{Diagnostic, DiagnosticCode as Code, Severity};

pub(super) struct Registers;
impl Rule for Registers {
    fn id(&self) -> &'static str {
        "registers"
    }
    fn check(&self, context: &CheckContext<'_>, diagnostics: &mut Vec<Diagnostic>) {
        for peripheral in &context.device.peripherals {
            for register in &peripheral.registers {
                if let Some(size) = &register.properties.size {
                    if size.value == 0 {
                        diagnostics.push(context.diagnostic(
                            Code::SemZeroRegisterSize,
                            Severity::Error,
                            "register size must be greater than zero",
                            &size.origin,
                            "size",
                        ));
                    }
                }
            }
        }
    }
}
