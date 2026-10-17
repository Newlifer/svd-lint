//! Opt-in quality checks; never part of the default semantic pipeline.
use super::CheckContext;
use crate::{CanonicalDevice, Diagnostic, DiagnosticCode, Severity, SourceMap};
pub fn check_quality(device: &CanonicalDevice, map: &SourceMap) -> Vec<Diagnostic> {
    let context = CheckContext {
        device,
        source_map: map,
    };
    let mut diagnostics = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for p in &device.peripherals {
        for r in &p.registers {
            if r.metadata.description.as_deref().is_some_and(|d| {
                let d = d.trim();
                d.chars().count() < 4
                    || d.eq_ignore_ascii_case(r.name.rsplit('.').next().unwrap_or(&r.name))
            }) {
                let d = context.diagnostic(
                    DiagnosticCode::LowQualityDescription,
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
