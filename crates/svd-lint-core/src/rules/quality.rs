//! Opt-in description quality check.
use super::{CheckContext, Rule};
use crate::registry::{
    Compatibility, DiagnosticDefinition, RuleCategory, RuleSource, SvdConvReference,
};
use crate::{CanonicalDevice, Diagnostic, Severity, SourceMap};

pub(crate) static LOW_QUALITY_DESCRIPTION: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SVD007"),
    name: "low-quality-description",
    description: "A register description repeats its name or is too short to explain its purpose.",
    category: RuleCategory::Quality,
    default_severity: Severity::Warning,
    source: RuleSource::SvdLint,
    configurable: true,
    svdconv_codes: &[SvdConvReference {
        code: "M320",
        compatibility: Compatibility::Related,
    }],
    spec_refs: &[],
    tags: &["quality", "strict"],
};

pub(crate) static LOW_QUALITY_DESCRIPTION_CHECK: Rule = Rule {
    name: LOW_QUALITY_DESCRIPTION.name,
    description: LOW_QUALITY_DESCRIPTION.description,
    diagnostics: &[&LOW_QUALITY_DESCRIPTION],
    check: |context, diagnostics| {
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
    },
};

pub fn check_quality(device: &CanonicalDevice, map: &SourceMap) -> Vec<Diagnostic> {
    let context = CheckContext {
        device,
        source_map: map,
    };
    let mut diagnostics = Vec::new();
    (LOW_QUALITY_DESCRIPTION_CHECK.check)(&context, &mut diagnostics);
    diagnostics
}
