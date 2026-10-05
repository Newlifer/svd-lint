use svd_lint_core::*;

const VALID: &str = include_str!("../../../tests/fixtures/semantics/valid.svd");
const INVALID: &str = include_str!("../../../tests/fixtures/semantics/invalid.svd");

#[test]
fn valid_fixture_passes_all_three_stages() {
    let result = analyze_svd(&SourceFile::new("valid.svd", VALID));
    assert!(result.device.is_some());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
}

#[test]
fn independent_semantic_failures_include_every_sem_code_and_preserve_ir() {
    let input = SourceFile::new("invalid.svd", INVALID);
    let parsed = parse_svd(&input);
    assert!(parsed.device.is_some());
    let normalized = normalize(
        parsed.device.as_ref().unwrap(),
        parsed.source_map.as_ref().unwrap(),
        &NormalizeConfig::default(),
    );
    assert!(normalized.device.is_some());
    assert!(normalized.diagnostics.is_empty());
    let result = analyze_svd(&input);
    assert!(result.has_errors());
    assert!(result.device.is_some());
    for i in 1..=16 {
        let code = format!("SEM{i:03}");
        assert!(
            result.diagnostics.iter().any(|d| d.code.as_str() == code),
            "missing {code}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn semantic_diagnostics_snapshot_and_determinism() {
    let input = SourceFile::new("invalid.svd", INVALID);
    let result = analyze_svd(&input);
    assert_eq!(result.diagnostics, analyze_svd(&input).diagnostics);
    insta::assert_snapshot!(serde_json::to_string_pretty(&result.diagnostics).unwrap());
}
