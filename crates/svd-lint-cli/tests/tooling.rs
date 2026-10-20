use assert_cmd::Command;
use serde_json::Value;
fn cli() -> Command {
    Command::cargo_bin("svd-lint").unwrap()
}
fn fixture(name: &str) -> String {
    format!("../../tests/fixtures/{name}")
}

#[test]
fn duplicate_field_names_do_not_hide_semantic_errors_in_check() {
    let input = "../../tests/corpus/regression/duplicate-fields-semantic-checks/input.svd";
    for format in ["text", "json", "sarif"] {
        let result = cli()
            .args(["check", input, "--format", format])
            .assert()
            .code(1);
        let output = if format == "text" {
            &result.get_output().stderr
        } else {
            &result.get_output().stdout
        };
        let output = String::from_utf8_lossy(output);
        for code in ["E011", "E020", "W003", "E021", "W006", "E025", "E027"] {
            assert!(output.contains(code), "check/{format} is missing {code}");
        }
    }
    let result = cli()
        .args(["dump-ir", input, "--format", "json"])
        .assert()
        .code(1);
    let report: Value = serde_json::from_slice(&result.get_output().stdout).unwrap();
    assert!(report.get("peripherals").is_none());
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "E011")
    );
}
#[test]
fn registry_and_explain_are_machine_readable() {
    let result = cli()
        .args(["list-rules", "--format", "json"])
        .assert()
        .success();
    let rules: Value = serde_json::from_slice(&result.get_output().stdout).unwrap();
    assert_eq!(
        rules.as_array().unwrap().len(),
        svd_lint_core::DiagnosticCode::all().len()
    );
    let result = cli()
        .args(["explain", "W003", "--format", "json"])
        .assert()
        .success();
    let rule: Value = serde_json::from_slice(&result.get_output().stdout).unwrap();
    assert_eq!(rule["code"], "W003");
    assert_eq!(rule["severity"], "warning");
    assert!(rule.get("category").is_none());
    assert!(rule.get("default_severity").is_none());
    assert!(rule.get("svdconv_codes").is_none());
    assert!(rule.get("spec_refs").is_none());
    cli().args(["explain", "M338"]).assert().code(2);
}
#[test]
fn profiles_and_output_formats_preserve_fixed_severity() {
    let input = "../../tests/corpus/invalid/W003/negative.svd";
    for profile in ["default", "strict", "svdconv"] {
        for format in ["json", "sarif"] {
            let result = cli()
                .args(["check", input, "--profile", profile, "--format", format])
                .assert()
                .success();
            let report: Value = serde_json::from_slice(&result.get_output().stdout).unwrap();
            let (diagnostics, code, severity) = if format == "json" {
                (&report["diagnostics"], "code", "severity")
            } else {
                (&report["runs"][0]["results"], "ruleId", "level")
            };
            let finding = diagnostics
                .as_array()
                .unwrap()
                .iter()
                .find(|finding| finding[code] == "W003")
                .unwrap();
            assert_eq!(finding[severity], "warning", "{profile}/{format}");
        }
    }
}
#[test]
fn fail_on_is_consistent_for_all_check_formats_and_profiles() {
    for format in ["text", "json", "sarif"] {
        cli()
            .args([
                "check",
                &fixture("valid/warning_no_description.svd"),
                "--format",
                format,
                "--fail-on",
                "warning",
            ])
            .assert()
            .code(1);
        cli()
            .args([
                "check",
                &fixture("valid/warning_no_description.svd"),
                "--format",
                format,
                "--fail-on",
                "error",
            ])
            .assert()
            .success();
    }
}
#[test]
fn config_suppresses_and_rejects_severity_changes_or_disabling_fatal_analysis() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("svd-lint.toml");
    std::fs::write(&config, "[rules.W002]\nseverity = \"allow\"\n").unwrap();
    cli()
        .args([
            "check",
            &fixture("valid/warning_no_description.svd"),
            "--fail-on",
            "warning",
            "--config",
        ])
        .arg(&config)
        .assert()
        .success();
    std::fs::write(&config, "[rules.W002]\nseverity = \"error\"\n").unwrap();
    cli()
        .args([
            "check",
            &fixture("valid/warning_no_description.svd"),
            "--format",
            "json",
            "--config",
        ])
        .arg(&config)
        .assert()
        .code(2);
    for text in [
        "[rules.E018]\nseverity = \"warning\"\n",
        "[[overrides]]\nfiles=[\"*.svd\"]\nrules=[\"W002\"]\nseverity=\"error\"\n",
    ] {
        std::fs::write(&config, text).unwrap();
        cli()
            .args(["check", &fixture("valid/minimal.svd"), "--config"])
            .arg(&config)
            .assert()
            .code(2);
    }
    for code in ["E001", "E003", "E015", "E005", "E032"] {
        std::fs::write(&config, format!("[rules.{code}]\nenabled = false\n")).unwrap();
        cli()
            .args(["check", &fixture("valid/minimal.svd"), "--config"])
            .arg(&config)
            .assert()
            .code(2);
    }
    std::fs::write(&config, "[rules.UNKNOWN]\nseverity = \"allow\"\n").unwrap();
    cli()
        .args(["check", &fixture("valid/minimal.svd"), "--config"])
        .arg(&config)
        .assert()
        .code(2);
}
#[test]
fn strict_quality_and_cli_profile_precedence_work() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.svd");
    let content = std::fs::read_to_string(fixture("valid/minimal.svd"))
        .unwrap()
        .replace(
            "<description>Control register</description>",
            "<description>CTRL</description>",
        );
    std::fs::write(&input, content).unwrap();
    let config = dir.path().join("svd-lint.toml");
    std::fs::write(&config, "profile = \"strict\"").unwrap();
    cli()
        .arg("check")
        .arg(&input)
        .args(["--config"])
        .arg(&config)
        .args(["--fail-on", "warning"])
        .assert()
        .code(1);
    cli()
        .arg("check")
        .arg(&input)
        .args(["--config"])
        .arg(&config)
        .args(["--profile", "default", "--fail-on", "warning"])
        .assert()
        .success();
}
#[test]
fn sarif_contains_metadata_primary_related_and_omits_unknown_positions() {
    let result = cli()
        .args([
            "check",
            &fixture("semantics/invalid.svd"),
            "--format",
            "sarif",
        ])
        .assert()
        .code(1);
    let data: Value = serde_json::from_slice(&result.get_output().stdout).unwrap();
    assert_eq!(data["version"], "2.1.0");
    assert_eq!(
        data["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .unwrap()
            .len(),
        svd_lint_core::DiagnosticCode::all().len()
    );
    let results = data["runs"][0]["results"].as_array().unwrap();
    assert!(results.iter().any(|r| r.get("relatedLocations").is_some()));
    assert!(results.iter().any(|r| r["ruleId"] == "W003"
        && r["locations"][0]["physicalLocation"]["region"]["startLine"].is_number()));
    let result = cli()
        .args(["check", "absent.svd", "--format", "sarif"])
        .assert()
        .code(2);
    let data: Value = serde_json::from_slice(&result.get_output().stdout).unwrap();
    assert!(data["runs"][0]["results"][0].get("locations").is_none());
}
#[test]
fn xsd_is_explicit_and_failure_does_not_cascade() {
    let result = cli()
        .args([
            "check",
            &fixture("valid/minimal.svd"),
            "--xsd",
            "--schema-version",
            "9",
            "--format",
            "json",
        ])
        .assert()
        .code(1);
    let data: Value = serde_json::from_slice(&result.get_output().stdout).unwrap();
    assert_eq!(data["diagnostics"].as_array().unwrap().len(), 1);
    assert_eq!(data["diagnostics"][0]["code"], "E031");
    let result = cli()
        .args([
            "check",
            &fixture("valid/minimal.svd"),
            "--xsd",
            "--format",
            "json",
        ])
        .assert()
        .code(1);
    let data: Value = serde_json::from_slice(&result.get_output().stdout).unwrap();
    assert_eq!(data["diagnostics"][0]["code"], "E033");
}
#[test]
fn file_overrides_are_config_only_and_unknown_toml_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");
    std::fs::write(&config,"[[overrides]]\nfiles=[\"**/warning_no_description.svd\"]\nrules=[\"W002\"]\nseverity=\"allow\"").unwrap();
    cli()
        .args([
            "check",
            &fixture("valid/warning_no_description.svd"),
            "--fail-on",
            "warning",
            "--config",
        ])
        .arg(&config)
        .assert()
        .success();
    std::fs::write(&config, "proflie=\"strict\"").unwrap();
    cli()
        .args(["check", &fixture("valid/minimal.svd"), "--config"])
        .arg(&config)
        .assert()
        .code(2);
}
