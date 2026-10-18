//! Backend admitted after the isolated CMSIS-SVD compatibility gate.
use super::diagnostics;
use super::{DirectoryProvider, SchemaValidator, SchemaVersion};
use crate::{Diagnostic, SourceFile, Span};
use std::collections::BTreeSet;
pub struct UppsalaValidator {
    pub provider: DirectoryProvider,
}
impl SchemaValidator for UppsalaValidator {
    fn validate(&self, source: &SourceFile, version: SchemaVersion) -> Vec<Diagnostic> {
        if !version.supported() {
            return vec![Diagnostic::error(
                diagnostics::UNKNOWN_VERSION.code(),
                "unknown schemaVersion",
            )];
        }
        let Some(path) = self.provider.schema(&version) else {
            return vec![Diagnostic::error(
                diagnostics::MISSING_SCHEMA.code(),
                format!("schema {} not found", version.0),
            )];
        };
        let run = std::panic::catch_unwind(|| -> Result<Vec<Diagnostic>, String> {
            let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let header = roxmltree::Document::parse(&text).map_err(|e| e.to_string())?;
            let element = header.root_element();
            if element.tag_name().name() != "schema"
                || element.tag_name().namespace() != Some("http://www.w3.org/2001/XMLSchema")
            {
                return Err("the schema file must have an XML Schema schema root".into());
            }
            let schema = uppsala::parse(&text).map_err(|e| e.to_string())?;
            let validator =
                uppsala::XsdValidator::from_schema(&schema).map_err(|e| e.to_string())?;
            let document = match uppsala::parse(source.text()) {
                Ok(doc) => doc,
                Err(e) => {
                    return Ok(vec![Diagnostic::error(
                        diagnostics::NONCONFORMANCE.code(),
                        e.to_string(),
                    )]);
                }
            };
            let mut seen = BTreeSet::new();
            let diagnostics = validator
                .validate(&document)
                .into_iter()
                .filter(|e| seen.insert((e.line, e.column, e.message.clone())))
                .map(|error| {
                    let mut d =
                        Diagnostic::error(diagnostics::NONCONFORMANCE.code(), error.message);
                    d.primary_span = error
                        .line
                        .zip(error.column)
                        .and_then(|(line, column)| {
                            source.offset_at(u32::try_from(line).ok()?, u32::try_from(column).ok()?)
                        })
                        .and_then(|offset| Span::new(offset, offset).ok());
                    d
                })
                .collect();
            Ok(diagnostics)
        });
        match run {
            Ok(Ok(d)) => d,
            Ok(Err(e)) => vec![Diagnostic::error(diagnostics::BACKEND_ERROR.code(), e)],
            Err(_) => vec![Diagnostic::error(
                diagnostics::BACKEND_ERROR.code(),
                "XSD backend panicked",
            )],
        }
    }
}
