use svd_lint_core::*;

const INPUT: &str =
    include_str!("../../../tests/corpus/regression/duplicate-fields-semantic-checks/input.svd");

fn analyze(xml: &str) -> AnalysisResult {
    analyze_svd(&SourceFile::new("duplicates.svd", xml))
}

#[test]
fn duplicate_fields_preserve_semantic_checks_but_do_not_export_ir() {
    let source = SourceFile::new("duplicates.svd", INPUT);
    let result = analyze_svd(&source);
    assert!(result.device.is_none());
    for code in [
        "NORM003", "SEM003", "SEM004", "SEM005", "SEM008", "SEM010", "SEM012",
    ] {
        assert!(
            result.diagnostics.iter().any(|d| d.code.as_str() == code),
            "missing {code}: {:?}",
            result.diagnostics
        );
    }
    let overlap = result
        .diagnostics
        .iter()
        .find(|d| d.code == DiagnosticCode::new("SEM004"))
        .unwrap();
    let primary = overlap.primary_span.unwrap();
    let related = overlap.related.first().unwrap().span;
    assert_eq!(source.snippet(primary), Some("<name>Reserved</name>"));
    assert_eq!(source.snippet(related), Some("<name>Reserved</name>"));
    assert_ne!(primary, related);
    let parsed = parse_svd(&source);
    let normalized = normalize(
        parsed.device.as_ref().unwrap(),
        parsed.source_map.as_ref().unwrap(),
        &NormalizeConfig::default(),
    );
    assert!(normalized.device.is_none());
    assert!(
        normalized
            .diagnostics
            .iter()
            .all(|d| d.code == DiagnosticCode::new("NORM003"))
    );
    assert_eq!(
        serde_json::to_value(result.diagnostics).unwrap(),
        serde_json::to_value(analyze(INPUT).diagnostics).unwrap()
    );
}

#[test]
fn ambiguous_derived_from_stays_blocking_without_picking_a_duplicate() {
    let input = INPUT.replacen(
        "</fields>",
        "<field derivedFrom=\"Reserved\"><name>DERIVED</name><bitRange>[2:2]</bitRange></field></fields>",
        1,
    );
    let result = analyze(&input);
    assert!(result.device.is_none());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::new("NORM003")
                && d.message.contains("ambiguous derivedFrom")
                && d.related.len() == 2)
    );
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.code.as_str().starts_with("SEM"))
    );
}

#[test]
fn missing_effective_properties_are_not_recovered_as_a_complete_model() {
    let result = analyze(&INPUT.replace("<size>32</size>", ""));
    assert!(result.device.is_none());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::new("NORM008"))
    );
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.code.as_str().starts_with("SEM"))
    );
}

#[test]
fn unique_names_restore_public_ir_without_hiding_semantic_errors() {
    let input = INPUT.replacen("<name>Reserved</name>", "<name>Distinct</name>", 1);
    let result = analyze(&input);
    assert!(result.device.is_some());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::new("SEM004"))
    );
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.code.as_str().starts_with("NORM"))
    );
}
