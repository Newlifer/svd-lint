//! Integration tests for the `svd-lint` binary.

use assert_cmd::Command;
use predicates::str::contains;

/// Path to a fixture, relative to this crate's directory (the working
/// directory cargo uses for tests). Kept relative so tests do not depend on
/// the absolute checkout location.
fn fixture(rel: &str) -> String {
    format!("../../tests/fixtures/{rel}")
}

fn svd_lint() -> Command {
    Command::cargo_bin("svd-lint").expect("binary must build")
}

#[test]
fn valid_file_exits_zero_with_quiet_stderr() {
    svd_lint()
        .args(["check", &fixture("valid/minimal.svd")])
        .assert()
        .code(0)
        .stderr(predicates::str::is_empty());
}

#[test]
fn check_reports_normalization_errors_in_stable_json_shape() {
    let assert = svd_lint()
        .args([
            "check",
            &fixture("normalization/errors.svd"),
            "--format",
            "json",
        ])
        .assert()
        .code(1);
    let report: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "NORM001")
    );
    assert!(report["diagnostics"][0]["span"]["start_line"].is_number());
    assert_eq!(report.as_object().unwrap().len(), 2);
}

#[test]
fn dump_ir_is_deterministic_json_with_physical_registers() {
    let run = || {
        svd_lint()
            .args([
                "dump-ir",
                &fixture("normalization/complex.svd"),
                "--format",
                "json",
            ])
            .assert()
            .code(0)
            .get_output()
            .stdout
            .clone()
    };
    let first = run();
    assert_eq!(first, run());
    let ir: serde_json::Value = serde_json::from_slice(&first).unwrap();
    assert_eq!(ir["peripherals"][0]["name"], "TIMER2");
    assert_eq!(ir["peripherals"][0]["registers"][0]["address"], 0x5004);
    assert_eq!(ir["peripherals"][0]["registers"][0]["size"]["value"], 16);
}

#[test]
fn dump_ir_failure_outputs_diagnostics_without_partial_ir() {
    let assert = svd_lint()
        .args([
            "dump-ir",
            &fixture("normalization/errors.svd"),
            "--format",
            "json",
        ])
        .assert()
        .code(1);
    let report: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert!(report.get("peripherals").is_none());
    assert!(!report["diagnostics"].as_array().unwrap().is_empty());
}

#[test]
fn warning_only_file_exits_zero() {
    svd_lint()
        .args(["check", &fixture("valid/warning_no_description.svd")])
        .assert()
        .code(0)
        .stderr(contains("SVD005"));
}

#[test]
fn bad_xml_exits_one_with_xml001() {
    svd_lint()
        .args(["check", &fixture("invalid/bad_xml.svd")])
        .assert()
        .code(1)
        .stderr(contains("XML001"));
}

#[test]
fn missing_version_exits_one_with_svd002() {
    svd_lint()
        .args(["check", &fixture("invalid/missing_version.svd")])
        .assert()
        .code(1)
        .stderr(contains("SVD002"));
}

#[test]
fn multiple_errors_are_all_reported() {
    svd_lint()
        .args(["check", &fixture("invalid/multiple_errors.svd")])
        .assert()
        .code(1)
        .stderr(contains("SVD002"))
        .stderr(contains("SVD003"))
        .stderr(contains("SVD004"))
        .stderr(contains("SVD005"));
}

#[test]
fn bad_number_exits_one_with_svd006_without_panic() {
    let assert = svd_lint()
        .args(["check", &fixture("invalid/bad_number.svd")])
        .assert()
        .code(1)
        .stderr(contains("SVD006"));
    // A panic would produce exit code 101 and "panicked" on stderr.
    let output = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();
    assert!(!output.contains("panicked"), "{output}");
}

#[test]
fn nonexistent_file_exits_two_with_io001() {
    svd_lint()
        .args(["check", "../../tests/fixtures/does_not_exist.svd"])
        .assert()
        .code(2)
        .stderr(contains("IO001"));
}

#[test]
fn invalid_utf8_exits_two_with_io002() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("broken.svd");
    std::fs::write(&path, [0xFF, 0xFE, 0x41, 0x42, 0x80]).unwrap();
    svd_lint()
        .args(["check".as_ref(), path.as_os_str()])
        .assert()
        .code(2)
        .stderr(contains("IO002"));
}

#[test]
fn json_output_is_valid_and_structured() {
    let file = fixture("invalid/multiple_errors.svd");
    let assert = svd_lint()
        .args(["check", &file, "--format", "json"])
        .assert()
        .code(1)
        .stderr(predicates::str::is_empty());
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("stdout must be JSON");

    assert_eq!(json["file"], serde_json::Value::String(file));
    let diagnostics = json["diagnostics"].as_array().expect("diagnostics array");
    assert_eq!(diagnostics.len(), 4);

    let codes: Vec<&str> = diagnostics
        .iter()
        .map(|d| d["code"].as_str().unwrap())
        .collect();
    for expected in ["SVD002", "SVD003", "SVD004", "SVD005"] {
        assert!(codes.contains(&expected), "missing {expected} in {codes:?}");
    }

    // Byte ranges and computed line/column positions are present.
    let dup = diagnostics
        .iter()
        .find(|d| d["code"] == "SVD003")
        .expect("SVD003 diagnostic");
    let span = &dup["span"];
    assert!(span["start"].as_u64().unwrap() < span["end"].as_u64().unwrap());
    assert_eq!(span["start_line"], 5);
    assert_eq!(span["start_column"], 3);
    assert_eq!(dup["related"][0]["span"]["start_line"], 4);
    assert!(dup["related"][0]["message"].is_string());
}

#[test]
fn json_output_for_io_error() {
    let assert = svd_lint()
        .args([
            "check",
            "../../tests/fixtures/does_not_exist.svd",
            "--format",
            "json",
        ])
        .assert()
        .code(2);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["diagnostics"][0]["code"], "IO001");
}

#[test]
fn snapshot_json_multiple_errors() {
    let assert = svd_lint()
        .args([
            "check",
            &fixture("invalid/multiple_errors.svd"),
            "--format",
            "json",
        ])
        .assert()
        .code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    insta::assert_snapshot!(stdout);
}

#[test]
fn snapshot_text_multiple_errors() {
    let assert = svd_lint()
        .args(["check", &fixture("invalid/multiple_errors.svd")])
        .assert()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    insta::assert_snapshot!(stderr);
}
