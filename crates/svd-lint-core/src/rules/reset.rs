use super::{CheckContext, Rule, ranges::fits};
use crate::{Diagnostic, DiagnosticCode as Code, Severity};

pub(super) struct Reset;
impl Rule for Reset {
    fn id(&self) -> &'static str {
        "reset"
    }
    fn check(&self, context: &CheckContext<'_>, diagnostics: &mut Vec<Diagnostic>) {
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
                for (value, tag, code) in [
                    (
                        &register.properties.reset_value,
                        "resetValue",
                        Code::SemResetValueOutOfBounds,
                    ),
                    (
                        &register.properties.reset_mask,
                        "resetMask",
                        Code::SemResetMaskOutOfBounds,
                    ),
                ] {
                    if let Some(value) = value {
                        if !fits(value.value, width) {
                            let mut d = context.diagnostic(
                                code,
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
                if let (Some(value), Some(mask)) = (
                    &register.properties.reset_value,
                    &register.properties.reset_mask,
                ) {
                    if fits(value.value, width)
                        && fits(mask.value, width)
                        && value.value & !mask.value != 0
                    {
                        let mut d = context.diagnostic(Code::SemResetOutsideMask, Severity::Warning,
                            format!("resetValue has bits {:#x} outside resetMask; those reset bits are unspecified", value.value & !mask.value), &value.origin, "resetValue");
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
    }
}
