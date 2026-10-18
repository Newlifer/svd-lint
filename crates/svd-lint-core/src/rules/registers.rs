use super::Rule;
use crate::Severity;
use crate::registry::{
    Compatibility, DiagnosticDefinition, RuleCategory, RuleSource, SpecReference, SvdConvReference,
};

pub(crate) static ZERO_REGISTER_SIZE: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM001"),
    name: "zero-register-size",
    description: "A register must occupy a positive number of bits.",
    category: RuleCategory::Registers,
    default_severity: Severity::Error,
    source: RuleSource::CmsisSvd,
    configurable: true,
    svdconv_codes: &[SvdConvReference {
        code: "M302",
        compatibility: Compatibility::Partial,
    }],
    spec_refs: &[SpecReference {
        title: "Registers / zero-register-size",
        url: "https://arm-software.github.io/CMSIS_5/SVD/html/elem_registers.html",
    }],
    tags: &["check"],
};

pub(crate) static ZERO_REGISTER_SIZE_CHECK: Rule = Rule {
    name: ZERO_REGISTER_SIZE.name,
    description: ZERO_REGISTER_SIZE.description,
    diagnostics: &[&ZERO_REGISTER_SIZE],
    check: |context, diagnostics| {
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
    },
};
