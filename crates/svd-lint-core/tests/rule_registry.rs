use std::collections::BTreeSet;
use svd_lint_core::{registry, *};

#[test]
fn registered_code_format_matches_severity() {
    for definition in registry::rule_registry() {
        let code = definition.code.as_str();
        let prefix = match definition.severity {
            Severity::Error => 'E',
            Severity::Warning => 'W',
            Severity::Note => 'N',
        };
        assert_eq!(code.len(), 4, "{code}");
        assert!(code.starts_with(prefix), "{code}");
        assert!(
            code.as_bytes()[1..].iter().all(u8::is_ascii_digit),
            "{code}"
        );
        assert!(
            (1..=999).contains(&code[1..].parse::<u16>().unwrap()),
            "{code}"
        );
    }
}

#[test]
fn codes_are_string_values_with_stable_registered_serialization() {
    const CUSTOM: DiagnosticCode = DiagnosticCode::new("CUSTOM001");
    assert_eq!(CUSTOM.as_str(), "CUSTOM001");
    assert_eq!(
        DiagnosticCode::from_code("W003"),
        Some(DiagnosticCode::new("W003"))
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
fn checks_emit_all_configurable_diagnostics() {
    let mut emitted = BTreeSet::new();
    for input in [
        include_str!("../../../tests/fixtures/semantics/invalid.svd"),
        include_str!("../../../tests/corpus/invalid/W001/W001-negative.svd"),
        include_str!("../../../tests/corpus/invalid/W006/negative.svd"),
        include_str!("../../../tests/corpus/invalid/W007/negative.svd"),
        include_str!("../../../tests/fixtures/valid/warning_no_description.svd"),
    ] {
        let source = SourceFile::new("registry.svd", input);
        let mut result = analyze_svd(&source);
        let device = result.device.as_ref().expect("fixture must normalize");
        let map = result.source_map.as_ref().unwrap();
        result.diagnostics.extend(rules::check_quality(device, map));
        for diagnostic in result.diagnostics {
            assert!(
                registry::find_rule(diagnostic.code().as_str()).is_some(),
                "unregistered diagnostic: {}",
                diagnostic.code(),
            );
            let definition = registry::find_rule(diagnostic.code().as_str()).unwrap();
            assert_eq!(diagnostic.severity(), definition.severity);
            emitted.insert(diagnostic.code());
        }
    }
    let expected = registry::rule_registry()
        .iter()
        .filter(|definition| definition.configurable)
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
        assert_eq!(diagnostic.code(), definition.code);
        assert_eq!(diagnostic.severity(), definition.severity);
    }
}

#[test]
fn deserialization_rejects_a_severity_that_does_not_match_the_code() {
    for definition in registry::rule_registry() {
        let diagnostic = definition.diagnostic("finding");
        let mut json = serde_json::to_value(&diagnostic).unwrap();
        let roundtrip: Diagnostic = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(roundtrip, diagnostic);
        json["severity"] = serde_json::json!(match definition.severity {
            Severity::Error => "warning",
            _ => "error",
        });
        assert!(serde_json::from_value::<Diagnostic>(json).is_err());
    }
}
