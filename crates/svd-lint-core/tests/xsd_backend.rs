#![cfg(feature = "xsd")]
use svd_lint_core::{schema::*, *};
#[test]
fn real_backend_validates_and_positions_schema_errors_before_typed_parse() {
    let dir = tempfile::tempdir().unwrap();
    let schema = dir.path().join("CMSIS-SVD_1_3_10.xsd");
    std::fs::write(&schema,r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:element name="device"><xs:complexType><xs:sequence><xs:element name="name" type="xs:string"/></xs:sequence></xs:complexType></xs:element></xs:schema>"#).unwrap();
    let validator = UppsalaValidator {
        provider: DirectoryProvider::new(dir.path()),
    };
    let version = SchemaVersion("1.3.10".into());
    assert!(
        validator
            .validate(
                &SourceFile::new("valid.xml", "<device><name>D</name></device>"),
                version.clone()
            )
            .is_empty()
    );
    let input = SourceFile::new("invalid.xml", "<device><bad/></device>");
    let result = analyze_with_schema(
        &input,
        &NormalizeConfig::default(),
        &validator,
        version.clone(),
    );
    assert!(result.device.is_none());
    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| d.code == DiagnosticCode::XsdNonconformance)
    );
    assert!(result.diagnostics.iter().any(|d| d.primary_span.is_some()));
    std::fs::write(&schema, "<not-a-schema/>").unwrap();
    assert_eq!(
        validator.validate(&input, version.clone())[0].code,
        DiagnosticCode::XsdBackendError
    );
    std::fs::remove_file(&schema).unwrap();
    assert_eq!(
        validator.validate(&input, version)[0].code,
        DiagnosticCode::XsdMissingSchema
    );
}
