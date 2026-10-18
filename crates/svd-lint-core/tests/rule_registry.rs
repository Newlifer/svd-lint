use std::collections::BTreeSet;
use svd_lint_core::{
    registry,
    rules::{CheckContext, Rule},
    *,
};

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
fn every_rule_uses_its_declared_diagnostic_definitions() {
    let source = SourceFile::new(
        "invalid.svd",
        include_str!("../../../tests/fixtures/semantics/invalid.svd"),
    );
    let parsed = parse_svd(&source);
    let map = parsed.source_map.unwrap();
    let normalized = normalize(
        parsed.device.as_ref().unwrap(),
        &map,
        &NormalizeConfig::default(),
    );
    let device = normalized.device.unwrap();
    let context = CheckContext {
        device: &device,
        source_map: &map,
    };
    let mut executed = BTreeSet::new();
    for rule in registry::executable_rules() {
        assert!(!rule.diagnostics.is_empty());
        for definition in rule.diagnostics {
            assert!(executed.insert(definition.code));
            assert!(std::ptr::eq(
                registry::find_rule(definition.code.as_str()).unwrap(),
                *definition
            ));
            let generated = definition.diagnostic("Additional explanation");
            assert_eq!(generated.code, definition.code);
            assert_eq!(generated.severity, definition.default_severity);
        }
        let mut diagnostics = Vec::new();
        (rule.check)(&context, &mut diagnostics);
        assert!(
            diagnostics.iter().all(|d| rule
                .diagnostics
                .iter()
                .any(|definition| definition.code == d.code)),
            "{} emitted another rule's code",
            rule.name
        );
    }
    let expected = registry::rule_registry()
        .iter()
        .filter(|r| r.code.as_str().starts_with("SEM") || r.code.as_str() == "SVD007")
        .map(|r| r.code)
        .collect::<BTreeSet<_>>();
    assert_eq!(executed, expected);
}

#[test]
fn inline_check_reports_multiple_codes_with_independent_severity() {
    use registry::DiagnosticDefinition;
    static WARNING: DiagnosticDefinition = DiagnosticDefinition {
        code: DiagnosticCode::new("TEST001"),
        default_severity: Severity::Warning,
        ..frontend::diagnostics::IO_READ_FAILED
    };
    static ERROR: DiagnosticDefinition = DiagnosticDefinition {
        code: DiagnosticCode::new("TEST002"),
        default_severity: Severity::Error,
        ..WARNING
    };
    static RULE: Rule = Rule {
        name: "multiple-findings",
        description: "One check can emit findings with different severities.",
        diagnostics: &[&WARNING, &ERROR],
        check: |_, output| {
            output.push(WARNING.diagnostic("A warning"));
            output.push(ERROR.diagnostic("An error"));
        },
    };
    let source = SourceFile::new(
        "valid.svd",
        include_str!("../../../tests/fixtures/semantics/valid.svd"),
    );
    let parsed = parse_svd(&source);
    let map = parsed.source_map.unwrap();
    let device = normalize(
        parsed.device.as_ref().unwrap(),
        &map,
        &NormalizeConfig::default(),
    )
    .device
    .unwrap();
    let mut diagnostics = Vec::new();
    (RULE.check)(
        &CheckContext {
            device: &device,
            source_map: &map,
        },
        &mut diagnostics,
    );
    assert_eq!(
        diagnostics
            .iter()
            .map(|d| (d.code, d.severity))
            .collect::<Vec<_>>(),
        [
            (WARNING.code, Severity::Warning),
            (ERROR.code, Severity::Error)
        ]
    );
    assert_eq!(RULE.diagnostics.len(), 2);
}
