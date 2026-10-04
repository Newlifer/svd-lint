//! Lightweight structural checks performed before the typed `svd-parser`
//! pass.
//!
//! This is intentionally **not** a full XSD validation: only the handful of
//! requirements that the CMSIS-SVD specification states explicitly are
//! checked. When the XML is well-formed, all detected problems are
//! collected and returned together.

use roxmltree::Document;

use crate::diagnostics::{Diagnostic, DiagnosticCode, Span};

/// Required direct children of `<device>`, in specification order.
///
/// All of them are singletons: each must appear exactly once.
const REQUIRED_SINGLETONS: [&str; 6] = [
    "name",
    "version",
    "description",
    "addressUnitBits",
    "width",
    "peripherals",
];

/// Runs all preflight checks against a well-formed XML document.
///
/// The returned diagnostics are ordered by their primary span, so the
/// output is deterministic for a given input.
pub fn check_document(document: &Document) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let root = document.root_element();
    if root.tag_name().name() != "device" {
        diagnostics.push(
            Diagnostic::error(
                DiagnosticCode::InvalidRootElement,
                format!(
                    "root element must be <device>, found <{}>",
                    root.tag_name().name()
                ),
            )
            .with_span(span_of(root.range())),
        );
        return sorted(diagnostics);
    }

    check_required_singletons(root, &mut diagnostics);
    check_peripherals(root, &mut diagnostics);

    sorted(diagnostics)
}

fn check_required_singletons(root: roxmltree::Node<'_, '_>, diagnostics: &mut Vec<Diagnostic>) {
    for name in REQUIRED_SINGLETONS {
        let elements: Vec<_> = root
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == name)
            .collect();

        match elements.as_slice() {
            [] => diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::MissingRequiredElement,
                    format!("missing required element <{name}>"),
                )
                .with_span(span_of(root.range())),
            ),
            [first, duplicates @ ..] => {
                // An empty `<peripherals>` is reported by the peripheral
                // check below, not as SVD004.
                if name != "peripherals" && element_text_is_empty(*first) {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::EmptyRequiredElement,
                            format!("required element <{name}> must not be empty"),
                        )
                        .with_span(span_of(first.range())),
                    );
                }
                for duplicate in duplicates {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::DuplicateSingletonElement,
                            format!("element <{name}> must appear only once"),
                        )
                        .with_span(span_of(duplicate.range()))
                        .with_related(span_of(first.range()), "first occurrence is here"),
                    );
                }
            }
        }
    }
}

fn check_peripherals(root: roxmltree::Node<'_, '_>, diagnostics: &mut Vec<Diagnostic>) {
    let Some(peripherals) = root
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "peripherals")
    else {
        // Already reported as SVD002 by the singleton check.
        return;
    };

    let peripheral_nodes: Vec<_> = peripherals
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "peripheral")
        .collect();

    if peripheral_nodes.is_empty() {
        diagnostics.push(
            Diagnostic::error(
                DiagnosticCode::MissingRequiredElement,
                "element <peripherals> must contain at least one <peripheral>",
            )
            .with_span(span_of(peripherals.range())),
        );
        return;
    }

    for peripheral in peripheral_nodes {
        // A `derivedFrom` peripheral inherits its description from the
        // base peripheral, so a missing description is not a problem.
        if peripheral.attribute("derivedFrom").is_some() {
            continue;
        }
        let description = peripheral
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "description");
        let has_description = description.is_some_and(|d| !element_text_is_empty(d));
        if !has_description {
            let name = peripheral
                .children()
                .find(|n| n.is_element() && n.tag_name().name() == "name")
                .and_then(|n| n.text())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("<unnamed>");
            diagnostics.push(
                Diagnostic::warning(
                    DiagnosticCode::MissingPeripheralDescription,
                    format!("peripheral <{name}> has no description"),
                )
                .with_span(span_of(peripheral.range())),
            );
        }
    }
}

fn element_text_is_empty(node: roxmltree::Node<'_, '_>) -> bool {
    node.text().is_none_or(|text| text.trim().is_empty())
}

fn span_of(range: std::ops::Range<usize>) -> Span {
    // roxmltree guarantees start <= end; SourceMap re-validates every range
    // against the source text.
    Span::new_unchecked(range.start, range.end)
}

fn sorted(mut diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    diagnostics.sort_by_key(|d| d.primary_span.map(|s| (s.start, s.end)));
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Severity;

    fn check(xml: &str) -> Vec<Diagnostic> {
        let doc = Document::parse(xml).unwrap();
        check_document(&doc)
    }

    fn codes(diagnostics: &[Diagnostic]) -> Vec<DiagnosticCode> {
        diagnostics.iter().map(|d| d.code).collect()
    }

    const VALID: &str = "\
<device>
  <name>D</name>
  <version>1.0</version>
  <description>desc</description>
  <addressUnitBits>8</addressUnitBits>
  <width>32</width>
  <peripherals>
    <peripheral>
      <name>P0</name>
      <description>p</description>
      <baseAddress>0x40000000</baseAddress>
    </peripheral>
  </peripherals>
</device>";

    #[test]
    fn valid_document_produces_no_diagnostics() {
        assert!(check(VALID).is_empty());
    }

    #[test]
    fn wrong_root_is_svd001() {
        let diags = check("<chip><name>D</name></chip>");
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidRootElement]);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn missing_required_element_is_svd002() {
        let xml = VALID.replace("  <version>1.0</version>\n", "");
        let diags = check(&xml);
        assert_eq!(codes(&diags), [DiagnosticCode::MissingRequiredElement]);
        assert!(diags[0].message.contains("<version>"));
    }

    #[test]
    fn duplicate_singleton_is_svd003_with_related_span() {
        let xml = VALID.replace(
            "  <version>1.0</version>",
            "  <version>1.0</version>\n  <version>2.0</version>",
        );
        let diags = check(&xml);
        assert_eq!(codes(&diags), [DiagnosticCode::DuplicateSingletonElement]);
        assert_eq!(diags[0].related.len(), 1);
        // The related span points at the first occurrence.
        let first_start = xml.find("<version>").unwrap();
        assert_eq!(diags[0].related[0].span.start, first_start);
    }

    #[test]
    fn empty_required_element_is_svd004() {
        let xml = VALID.replace(
            "<description>desc</description>",
            "<description>  </description>",
        );
        let diags = check(&xml);
        assert_eq!(codes(&diags), [DiagnosticCode::EmptyRequiredElement]);
    }

    #[test]
    fn self_closing_required_element_is_svd004() {
        let xml = VALID.replace("<width>32</width>", "<width/>");
        let diags = check(&xml);
        assert_eq!(codes(&diags), [DiagnosticCode::EmptyRequiredElement]);
    }

    #[test]
    fn peripherals_without_peripheral_is_svd002() {
        let xml = "\
<device>
  <name>D</name>
  <version>1.0</version>
  <description>desc</description>
  <addressUnitBits>8</addressUnitBits>
  <width>32</width>
  <peripherals>
  </peripherals>
</device>";
        let diags = check(xml);
        assert_eq!(codes(&diags), [DiagnosticCode::MissingRequiredElement]);
        assert!(diags[0].message.contains("peripheral"));
    }

    #[test]
    fn peripheral_without_description_is_svd005_warning() {
        let xml = VALID.replace("      <description>p</description>\n", "");
        let diags = check(&xml);
        assert_eq!(
            codes(&diags),
            [DiagnosticCode::MissingPeripheralDescription]
        );
        assert_eq!(diags[0].severity, Severity::Warning);
        assert!(diags[0].message.contains("P0"));
    }

    #[test]
    fn derived_peripheral_without_description_is_allowed() {
        let xml = VALID.replace(
            "<name>P0</name>\n      <description>p</description>",
            "<name>P0</name>",
        );
        let xml = xml.replace("<peripheral>", "<peripheral derivedFrom=\"BASE\">");
        assert!(check(&xml).is_empty());
    }

    #[test]
    fn multiple_independent_errors_are_collected() {
        let xml = "\
<device>
  <name>D</name>
  <version>1.0</version>
  <version>2.0</version>
  <description></description>
  <addressUnitBits>8</addressUnitBits>
  <peripherals>
    <peripheral>
      <name>P0</name>
      <baseAddress>0x40000000</baseAddress>
    </peripheral>
  </peripherals>
</device>";
        let diags = check(xml);
        let codes = codes(&diags);
        // missing <width>, duplicate <version>, empty <description>,
        // missing peripheral description.
        assert!(codes.contains(&DiagnosticCode::MissingRequiredElement));
        assert!(codes.contains(&DiagnosticCode::DuplicateSingletonElement));
        assert!(codes.contains(&DiagnosticCode::EmptyRequiredElement));
        assert!(codes.contains(&DiagnosticCode::MissingPeripheralDescription));
        assert_eq!(diags.len(), 4);
        // Diagnostics are ordered by position in the document.
        let starts: Vec<_> = diags
            .iter()
            .filter_map(|d| d.primary_span)
            .map(|s| s.start)
            .collect();
        assert!(starts.windows(2).all(|w| w[0] <= w[1]));
    }
}
