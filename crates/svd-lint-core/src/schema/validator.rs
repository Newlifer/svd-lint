use super::SchemaVersion;
use crate::{AnalysisResult, Diagnostic, NormalizeConfig, SourceFile, analyze_svd_with_config};
pub trait SchemaValidator {
    fn validate(&self, source: &SourceFile, schema_version: SchemaVersion) -> Vec<Diagnostic>;
}
/// Schema errors stop before typed parsing to avoid cascaded parser diagnostics.
pub fn analyze_with_schema(
    source: &SourceFile,
    config: &NormalizeConfig,
    validator: &dyn SchemaValidator,
    version: SchemaVersion,
) -> AnalysisResult {
    let diagnostics = validator.validate(source, version);
    if diagnostics.iter().any(|d| d.severity().is_error()) {
        return AnalysisResult {
            device: None,
            diagnostics,
            source_map: None,
        };
    }
    let mut result = analyze_svd_with_config(source, config);
    result.diagnostics.extend(diagnostics);
    result.diagnostics.sort_by_key(|d| {
        (
            d.primary_span.map(|s| s.start),
            d.code().as_str(),
            d.message.clone(),
        )
    });
    result
}
