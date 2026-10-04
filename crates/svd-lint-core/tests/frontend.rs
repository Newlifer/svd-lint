//! Integration tests for the frontend against the shared fixtures.

use std::path::{Path, PathBuf};

use svd_lint_core::{Diagnostic, DiagnosticCode, Severity, SourceFile, parse_svd};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
}

fn load(rel: &str) -> SourceFile {
    let path = fixtures().join(rel);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()));
    // Fixed logical name: tests must not depend on the checkout location.
    SourceFile::new(rel, text)
}

fn codes(diagnostics: &[Diagnostic]) -> Vec<DiagnosticCode> {
    diagnostics.iter().map(|d| d.code).collect()
}

#[test]
fn valid_minimal_svd_parses() {
    let result = parse_svd(&load("valid/minimal.svd"));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let device = result.device.expect("device must parse");
    assert_eq!(device.name, "Minimal");
    assert_eq!(device.peripherals.len(), 1);
    // Enumerated values are parsed, not ignored.
    let registers = device.peripherals[0].registers.as_ref().unwrap();
    let svd_lint_core::svd::RegisterCluster::Register(reg) = &registers[0] else {
        panic!("expected a register");
    };
    let fields: Vec<_> = reg.fields().collect();
    assert_eq!(fields[0].name, "EN");
    assert_eq!(fields[1].name, "MODE");
    assert!(!fields[1].enumerated_values.is_empty());
}

#[test]
fn derived_dim_cluster_parses_without_expansion() {
    use svd_lint_core::svd::{MaybeArray, RegisterCluster};

    let result = parse_svd(&load("valid/derived_dim_cluster.svd"));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let device = result.device.expect("device must parse");

    // derivedFrom is preserved, not resolved.
    assert_eq!(device.peripherals.len(), 2);
    assert_eq!(device.peripherals[1].derived_from.as_deref(), Some("GPIOA"));

    let children = device.peripherals[0].registers.as_ref().unwrap();
    assert_eq!(children.len(), 2);

    // dim arrays are preserved, not expanded.
    let RegisterCluster::Register(MaybeArray::Array(data, dim)) = &children[0] else {
        panic!("expected DATA register array");
    };
    assert_eq!(data.name, "DATA%s");
    assert_eq!(dim.dim, 4);
    assert_eq!(dim.dim_increment, 4);

    let RegisterCluster::Cluster(cluster) = &children[1] else {
        panic!("expected CFG cluster");
    };
    assert_eq!(cluster.name, "CFG");
    assert_eq!(cluster.children.len(), 2);
    assert!(matches!(
        &cluster.children[1],
        RegisterCluster::Register(MaybeArray::Array(_, _))
    ));
}

#[test]
fn bad_xml_yields_xml001() {
    let result = parse_svd(&load("invalid/bad_xml.svd"));
    assert!(result.device.is_none());
    assert_eq!(codes(&result.diagnostics), [DiagnosticCode::XmlSyntax]);
}

#[test]
fn wrong_root_yields_svd001() {
    let result = parse_svd(&load("invalid/wrong_root.svd"));
    assert_eq!(
        codes(&result.diagnostics),
        [DiagnosticCode::InvalidRootElement]
    );
}

#[test]
fn missing_version_yields_svd002() {
    let result = parse_svd(&load("invalid/missing_version.svd"));
    assert_eq!(
        codes(&result.diagnostics),
        [DiagnosticCode::MissingRequiredElement]
    );
    assert!(result.diagnostics[0].message.contains("<version>"));
}

#[test]
fn multiple_errors_are_reported_together() {
    let result = parse_svd(&load("invalid/multiple_errors.svd"));
    let codes = codes(&result.diagnostics);
    assert!(codes.contains(&DiagnosticCode::MissingRequiredElement)); // width
    assert!(codes.contains(&DiagnosticCode::DuplicateSingletonElement)); // version
    assert!(codes.contains(&DiagnosticCode::EmptyRequiredElement)); // description
    assert!(codes.contains(&DiagnosticCode::MissingPeripheralDescription)); // P0
    assert_eq!(result.diagnostics.len(), 4);
}

#[test]
fn empty_peripherals_yields_svd002() {
    let result = parse_svd(&load("invalid/empty_peripherals.svd"));
    assert_eq!(
        codes(&result.diagnostics),
        [DiagnosticCode::MissingRequiredElement]
    );
}

#[test]
fn bad_number_yields_svd006_without_panic() {
    let result = parse_svd(&load("invalid/bad_number.svd"));
    assert!(result.device.is_none());
    assert_eq!(codes(&result.diagnostics), [DiagnosticCode::SvdParseError]);
    assert!(result.diagnostics[0].primary_span.is_none());
}

#[test]
fn warning_only_file_has_no_errors() {
    let result = parse_svd(&load("valid/warning_no_description.svd"));
    assert!(!result.has_errors());
    assert_eq!(
        codes(&result.diagnostics),
        [DiagnosticCode::MissingPeripheralDescription]
    );
    assert_eq!(result.diagnostics[0].severity, Severity::Warning);
    // Warnings must not prevent the typed parse.
    assert!(result.device.is_some());
}

#[test]
fn source_map_links_peripheral_to_source() {
    let source = load("valid/minimal.svd");
    let result = parse_svd(&source);
    let map = result.source_map.expect("source map must exist");
    let peripheral = map
        .find_by_path(&["device", "peripherals", "peripheral"])
        .expect("peripheral node");
    let range = map.node(peripheral).unwrap().range;
    let snippet = source.snippet(range).unwrap();
    assert!(snippet.starts_with("<peripheral>"));
    assert!(snippet.contains("<name>TIMER0</name>"));
    let pos = source.line_col(range.start).unwrap();
    assert!(pos.line > 1);
}

/// Renders diagnostics in a stable, snapshot-friendly text form.
fn debug_dump(source: &SourceFile, diagnostics: &[Diagnostic]) -> String {
    let mut out = String::new();
    for d in diagnostics {
        let pos = d
            .primary_span
            .map(|s| {
                let start = source.line_col(s.start).unwrap();
                let end = source.line_col(s.end).unwrap();
                format!(
                    "[{}..{} @ {}:{}-{}:{}]",
                    s.start, s.end, start.line, start.column, end.line, end.column
                )
            })
            .unwrap_or_else(|| "[no span]".to_owned());
        out.push_str(&format!(
            "{:?} {} {} {}\n",
            d.severity, d.code, pos, d.message
        ));
        for r in &d.related {
            let start = source.line_col(r.span.start).unwrap();
            out.push_str(&format!(
                "  related [{}..{} @ {}:{}] {}\n",
                r.span.start,
                r.span.end,
                start.line,
                start.column,
                r.message.as_deref().unwrap_or("")
            ));
        }
    }
    out
}

#[test]
fn snapshot_multiple_errors() {
    let source = load("invalid/multiple_errors.svd");
    let result = parse_svd(&source);
    insta::assert_snapshot!(debug_dump(&source, &result.diagnostics));
}

#[test]
fn snapshot_bad_xml() {
    let source = load("invalid/bad_xml.svd");
    let result = parse_svd(&source);
    insta::assert_snapshot!(debug_dump(&source, &result.diagnostics));
}

#[test]
fn snapshot_bad_number() {
    let source = load("invalid/bad_number.svd");
    let result = parse_svd(&source);
    insta::assert_snapshot!(debug_dump(&source, &result.diagnostics));
}
