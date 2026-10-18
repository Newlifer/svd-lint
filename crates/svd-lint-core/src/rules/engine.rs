use super::CheckContext;
use crate::registry::DiagnosticDefinition;
use crate::{Diagnostic, SourceMap, ir::CanonicalDevice};
use std::collections::BTreeSet;

/// An ordinary function or a closure without captures checks immutable IR.
pub struct Rule {
    pub name: &'static str,
    pub description: &'static str,
    pub diagnostics: &'static [&'static DiagnosticDefinition],
    pub check: fn(&CheckContext<'_>, &mut Vec<Diagnostic>),
}

/// Stateless built-in rules. Individual rules are independent of CLI and each other.
pub struct RuleEngine;

impl RuleEngine {
    pub fn check(&self, context: &CheckContext<'_>) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for rule in crate::registry::default_rules() {
            (rule.check)(context, &mut diagnostics);
        }
        // Messages encode semantic values, intervals and addresses, not array
        // names. Source-identical declaration errors collapse; distinct physical
        // conflicts remain distinguishable. Usage notes do not split a finding.
        let mut seen = BTreeSet::new();
        diagnostics.retain(|d| {
            seen.insert((
                d.code.as_str(),
                d.primary_span.map(|s| (s.start, s.end)),
                d.message.clone(),
                d.related
                    .iter()
                    .filter(|r| r.message.as_deref() != Some("property is used here"))
                    .map(|r| (r.span.start, r.span.end, r.message.clone()))
                    .collect::<Vec<_>>(),
            ))
        });
        diagnostics.sort_by_key(|d| {
            (
                d.primary_span.map(|s| (s.start, s.end)),
                d.code.as_str(),
                d.message.clone(),
                d.related
                    .iter()
                    .map(|r| (r.span.start, r.span.end))
                    .collect::<Vec<_>>(),
            )
        });
        diagnostics
    }
}

pub fn check_rules(device: &CanonicalDevice, source_map: &SourceMap) -> Vec<Diagnostic> {
    RuleEngine.check(&CheckContext { device, source_map })
}
