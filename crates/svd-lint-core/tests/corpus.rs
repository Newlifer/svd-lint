use serde_json::Value;
use std::{collections::BTreeSet, fs, path::Path};
use svd_lint_core::{registry::*, *};
fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn xml_depth_guard_has_a_boundary_and_skips_xml_literals() {
    let literals = format!(
        "<!--{}--><![CDATA[{}]]><?ignored {}?>",
        "<node>".repeat(100),
        "<node>".repeat(100),
        "<node>".repeat(100)
    );
    let input = format!(
        "{}{}{}",
        "<node a=\"&gt;\">".repeat(64),
        literals,
        "</node>".repeat(64)
    );
    let result = parse_svd(&SourceFile::new("boundary.svd", input));
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code() == DiagnosticCode::new("E004"))
    );
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.code() == DiagnosticCode::new("E008"))
    );
    let input = format!("{}{}", "<node>".repeat(65), "</node>".repeat(65));
    let result = parse_svd(&SourceFile::new("deep.svd", input));
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code(), DiagnosticCode::new("E008"));
    assert!(result.source_map.is_none());
}
#[test]
fn corpus_codes_and_sidecars_match_actual_analysis() {
    let root = root().join("tests/corpus");
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(root.join("manifests/cases.json")).unwrap())
            .unwrap();
    for case in manifest["cases"].as_array().unwrap() {
        let path = root.join(case["file"].as_str().unwrap());
        let source = SourceFile::new(
            case["file"].as_str().unwrap(),
            fs::read_to_string(&path).unwrap(),
        );
        let mut result = analyze_svd(&source);
        if case["profile"] == "strict" {
            if let (Some(device), Some(map)) = (&result.device, &result.source_map) {
                result.diagnostics.extend(rules::check_quality(device, map));
            }
        }
        let actual = result
            .diagnostics
            .into_iter()
            .map(|d| {
                assert!(
                    find_rule(d.code().as_str()).is_some(),
                    "unregistered diagnostic: {}",
                    d.code()
                );
                assert_eq!(d.severity(), find_rule(d.code().as_str()).unwrap().severity);
                d.code().as_str()
            })
            .collect::<BTreeSet<_>>();
        let expected = case["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected, "{}", path.display());
        let sidecar: Value =
            serde_json::from_str(&fs::read_to_string(path.with_extension("json")).unwrap())
                .unwrap();
        assert_eq!(sidecar["expected"], case["diagnostics"]);
        for c in sidecar["unexpected"].as_array().unwrap() {
            assert!(!actual.contains(c.as_str().unwrap()));
        }
    }
}
#[test]
fn registry_documents_every_code_and_enforces_configurable_case_coverage() {
    let root = root();
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(root.join("tests/corpus/manifests/cases.json")).unwrap(),
    )
    .unwrap();
    let docs = fs::read_to_string(root.join("docs/rules.md")).unwrap();
    let mut seen = BTreeSet::new();
    for rule in rule_registry() {
        assert!(
            seen.insert(rule.code.as_str()),
            "duplicate {}",
            rule.code.as_str()
        );
        assert!(DiagnosticCode::from_code(rule.code.as_str()).is_some());
        assert!(docs.contains(&format!("## {} —", rule.code.as_str())));
        assert!(!rule.description.is_empty());
        assert!(
            manifest["cases"]
                .as_array()
                .unwrap()
                .iter()
                .any(|case| case["rule"] == rule.code.as_str())
                || matches!(
                    rule.code.as_str(),
                    "E001" | "E002" | "E031" | "E032" | "E033" | "E034"
                ),
            "{} has no corpus or dedicated I/O/schema integration test",
            rule.code.as_str()
        );
        if rule.configurable {
            for kind in ["negative", "positive", "boundary"] {
                assert!(
                    manifest["cases"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|c| c["rule"] == rule.code.as_str() && c["kind"] == kind),
                    "{} missing {kind}",
                    rule.code.as_str()
                );
            }
        }
    }
    assert_eq!(seen.len(), DiagnosticCode::all().len());
}

#[test]
fn regression_array_indices_trim_delimiter_whitespace_and_resolve_references() {
    let source = SourceFile::new(
        "regression.svd",
        std::fs::read_to_string(
            root().join("tests/corpus/regression/dim-index-whitespace/input.svd"),
        )
        .unwrap(),
    );
    let result = analyze_svd(&source);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let registers = &result.device.unwrap().peripherals[0].registers;
    assert_eq!(
        registers
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>(),
        ["P.RF", "P.RG", "P.RH", "P.COPY"]
    );
    assert_eq!(registers[2].origin.arrays[0].index, "H");
}
