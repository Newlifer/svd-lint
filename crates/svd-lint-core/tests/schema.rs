use svd_lint_core::{schema::*, *};
struct Rejected;
impl SchemaValidator for Rejected {
    fn validate(&self, _: &SourceFile, _: SchemaVersion) -> Vec<Diagnostic> {
        vec![Diagnostic::error(
            DiagnosticCode::XsdNonconformance,
            "invalid schema content",
        )]
    }
}
struct Accepted;
impl SchemaValidator for Accepted {
    fn validate(&self, _: &SourceFile, _: SchemaVersion) -> Vec<Diagnostic> {
        Vec::new()
    }
}
#[test]
fn schema_failure_stops_typed_parse_cascades() {
    let source = SourceFile::new("bad.svd", "<device schemaVersion=\"1.3\"><bad/></device>");
    let result = analyze_with_schema(
        &source,
        &NormalizeConfig::default(),
        &Rejected,
        SchemaVersion("1.3.10".into()),
    );
    assert!(result.device.is_none());
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        result.diagnostics[0].code,
        DiagnosticCode::XsdNonconformance
    );
}
#[test]
fn accepted_schema_runs_existing_pipeline() {
    let source = SourceFile::new(
        "good.svd",
        include_str!("../../../tests/fixtures/valid/minimal.svd"),
    );
    assert!(
        analyze_with_schema(
            &source,
            &NormalizeConfig::default(),
            &Accepted,
            SchemaVersion("1.1".into())
        )
        .device
        .is_some()
    );
    assert_eq!(
        SchemaVersion::detect(&source),
        Some(SchemaVersion("1.1".into()))
    );
    assert!(!SchemaVersion("../../bad".into()).supported());
}
