use super::CheckContext;
use crate::{Diagnostic, SourceMap, ir::CanonicalDevice};
use std::collections::BTreeSet;

/// Runs built-in checks over immutable IR, then deduplicates and orders findings.
pub fn check_rules(device: &CanonicalDevice, source_map: &SourceMap) -> Vec<Diagnostic> {
    let context = CheckContext { device, source_map };
    let mut diagnostics = Vec::new();
    super::registers::check_registers(&context, &mut diagnostics);
    super::fields::check_fields(&context, &mut diagnostics);
    super::reset::check_reset(&context, &mut diagnostics);
    super::memory::check_register_overlaps(&context, &mut diagnostics);
    super::alternatives::check_alternatives(&context, &mut diagnostics);
    super::memory::check_register_blocks(&context, &mut diagnostics);
    super::memory::check_address_blocks(&context, &mut diagnostics);
    super::enums::check_enums(&context, &mut diagnostics);
    super::access::check_access(&context, &mut diagnostics);
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
