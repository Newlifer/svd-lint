//! Opt-in description quality check.
use super::CheckContext;
use crate::diagnostic_definition::{DiagnosticDefinition, RuleCategory};
use crate::{CanonicalDevice, Diagnostic, Severity, SourceMap};

pub(crate) static LOW_QUALITY_DESCRIPTION: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SVD007"),
    name: "low-quality-description",
    description: "A register description repeats its name or is too short to explain its purpose.",
    category: RuleCategory::Quality,
    default_severity: Severity::Warning,
    configurable: true,
    tags: &["quality", "strict"],
};

pub(super) fn check_descriptions(context: &CheckContext<'_>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let definition = &LOW_QUALITY_DESCRIPTION;
    let mut seen = std::collections::BTreeSet::new();
    for p in &context.device.peripherals {
        for r in &p.registers {
            if r.metadata.description.as_deref().is_some_and(|d| {
                let d = d.trim();
                d.chars().count() < 4
                    || d.eq_ignore_ascii_case(r.name.rsplit('.').next().unwrap_or(&r.name))
            }) {
                let d = context.diagnostic(
                    definition,
                    Severity::Warning,
                    "register description is too short or repeats its name",
                    &r.origin,
                    "description",
                );
                if seen.insert(d.primary_span.map(|s| (s.start, s.end))) {
                    diagnostics.push(d);
                }
            }
        }
    }
    diagnostics.sort_by_key(|d| d.primary_span.map(|s| s.start));

    diagnostics
}

pub fn check_quality(device: &CanonicalDevice, map: &SourceMap) -> Vec<Diagnostic> {
    let context = CheckContext {
        device,
        source_map: map,
    };
    check_descriptions(&context)
}
