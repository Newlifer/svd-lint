use std::collections::BTreeSet;
use svd_lint_core::{registry, *};

#[test]
fn codes_are_string_values_with_stable_registered_serialization() {
    const CUSTOM: DiagnosticCode = DiagnosticCode::new("CUSTOM001");
    assert_eq!(CUSTOM.as_str(), "CUSTOM001");
    assert_eq!(
        DiagnosticCode::from_code("SEM004"),
        Some(DiagnosticCode::new("SEM004"))
    );
    assert_eq!(serde_json::to_string(&CUSTOM).unwrap(), "\"CUSTOM001\"");
    // Deserialization continues to reject unknown registry identifiers rather
    // than leaking arbitrary owned strings into a static identifier.
    assert!(serde_json::from_str::<DiagnosticCode>("\"UNKNOWN999\"").is_err());
    for code in DiagnosticCode::all() {
        let json = serde_json::to_string(code).unwrap();
        assert_eq!(
            serde_json::from_str::<DiagnosticCode>(&json).unwrap(),
            *code
        );
    }
}

#[test]
fn checks_emit_registered_diagnostics_and_keep_all_semantic_checks() {
    let mut emitted = BTreeSet::new();
    for input in [
        include_str!("../../../tests/fixtures/semantics/invalid.svd"),
        include_str!("../../../tests/corpus/invalid/SVD007/SVD007-negative.svd"),
    ] {
        let source = SourceFile::new("registry.svd", input);
        let mut result = analyze_svd(&source);
        let device = result.device.as_ref().expect("fixture must normalize");
        let map = result.source_map.as_ref().unwrap();
        result.diagnostics.extend(rules::check_quality(device, map));
        for diagnostic in result.diagnostics {
            assert!(
                registry::find_rule(diagnostic.code.as_str()).is_some(),
                "unregistered diagnostic: {}",
                diagnostic.code,
            );
            emitted.insert(diagnostic.code);
        }
    }
    let expected = registry::rule_registry()
        .iter()
        .filter(|definition| {
            definition.code.as_str().starts_with("SEM") || definition.code.as_str() == "SVD007"
        })
        .map(|definition| definition.code)
        .collect::<BTreeSet<_>>();
    assert_eq!(emitted, expected);
}

#[test]
fn catalogue_definitions_construct_their_own_diagnostics() {
    let mut codes = BTreeSet::new();
    for definition in registry::rule_registry() {
        assert!(
            codes.insert(definition.code),
            "duplicate code: {}",
            definition.code
        );
        let diagnostic = definition.diagnostic("A finding");
        assert_eq!(diagnostic.code, definition.code);
        assert_eq!(diagnostic.severity, definition.default_severity);
    }
}
