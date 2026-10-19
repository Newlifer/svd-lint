use super::CheckContext;
use crate::{Diagnostic, SourceMap, ir::CanonicalDevice};
use std::collections::BTreeSet;

/// Runs built-in checks over immutable IR, then deduplicates and orders findings.
pub fn check_rules(device: &CanonicalDevice, source_map: &SourceMap) -> Vec<Diagnostic> {
    let context = CheckContext { device, source_map };
    let mut diagnostics = Vec::new();
    diagnostics.extend(super::registers::check_registers(&context));
    diagnostics.extend(super::fields::check_fields(&context));
    diagnostics.extend(super::reset::check_reset(&context));
    diagnostics.extend(super::memory::check_register_overlaps(&context));
    diagnostics.extend(super::alternatives::check_alternatives(&context));
    diagnostics.extend(super::memory::check_register_blocks(&context));
    diagnostics.extend(super::memory::check_address_blocks(&context));
    diagnostics.extend(super::enums::check_enums(&context));
    diagnostics.extend(super::access::check_access(&context));
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
