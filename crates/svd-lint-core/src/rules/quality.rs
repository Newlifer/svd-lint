//! Opt-in description quality check.
use super::CheckContext;
use crate::diagnostic_definition::DiagnosticDefinition;
use crate::{CanonicalDevice, CanonicalRegister, Diagnostic, Severity, SourceMap};

pub(crate) static LOW_QUALITY_DESCRIPTION: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("W001"),
    name: "low-quality-description",
    description: "A register description repeats its name or is too short to explain its purpose.",
    severity: Severity::Warning,
    configurable: true,
    tags: &["quality", "strict"],
};

pub(super) fn check_descriptions(context: &CheckContext<'_>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for peripheral in &context.device.peripherals {
        for register in &peripheral.registers {
            if let Some(diagnostic) = check_description(context, register) {
                if seen.insert(diagnostic.primary_span.map(|span| (span.start, span.end))) {
                    diagnostics.push(diagnostic);
                }
            }
        }
    }
    diagnostics.sort_by_key(|diagnostic| diagnostic.primary_span.map(|span| span.start));
    diagnostics
}

fn check_description(
    context: &CheckContext<'_>,
    register: &CanonicalRegister,
) -> Option<Diagnostic> {
    let description = register.metadata.description.as_deref()?.trim();
    let name = register.name.rsplit('.').next().unwrap_or(&register.name);
    (description.chars().count() < 4 || description.eq_ignore_ascii_case(name)).then(|| {
        context.diagnostic(
            &LOW_QUALITY_DESCRIPTION,
            "register description is too short or repeats its name",
            &register.origin,
            "description",
        )
    })
}

pub fn check_quality(device: &CanonicalDevice, map: &SourceMap) -> Vec<Diagnostic> {
    let context = CheckContext {
        device,
        source_map: map,
    };
    check_descriptions(&context)
}
