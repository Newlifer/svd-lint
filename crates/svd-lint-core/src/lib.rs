//! svd-lint-core: frontend and canonical normalization of CMSIS-SVD.
//!
//! Provides the source handling ([`frontend::SourceFile`]), XML position
//! index ([`frontend::SourceMap`]), preflight checks, the typed
//! `svd-parser` frontend ([`frontend::parse_svd`]) and the renderer-agnostic
//! [`diagnostics`] model, plus [`ir`] and [`normalize`]. Rendering lives in `svd-lint-cli`;
//! this crate never depends on it.

pub mod diagnostics;
pub mod frontend;
pub mod ir;
pub mod normalize;

pub use ir::{
    CanonicalAddressBlock, CanonicalDevice, CanonicalField, CanonicalPeripheral, CanonicalRegister,
    Origin, Resolved,
};
pub use normalize::{NormalizeConfig, NormalizeResult, normalize};

/// Combined Stage 1 and Stage 2 result. SourceMap owns the meaning of IR node ids.
#[derive(Debug)]
pub struct AnalysisResult {
    pub device: Option<CanonicalDevice>,
    pub diagnostics: Vec<Diagnostic>,
    pub source_map: Option<SourceMap>,
}

impl AnalysisResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity.is_error())
    }
}

pub fn analyze_svd(source: &SourceFile) -> AnalysisResult {
    analyze_svd_with_config(source, &NormalizeConfig::default())
}

pub fn analyze_svd_with_config(source: &SourceFile, config: &NormalizeConfig) -> AnalysisResult {
    let parsed = parse_svd(source);
    let mut diagnostics = parsed.diagnostics;
    let device = if let (Some(device), Some(map)) = (&parsed.device, &parsed.source_map) {
        let normalized = normalize(device, map, config);
        diagnostics.extend(normalized.diagnostics);
        normalized.device
    } else {
        None
    };
    diagnostics.sort_by_key(|d| (d.primary_span.map(|s| (s.start, s.end)), d.code.as_str()));
    AnalysisResult {
        device,
        diagnostics,
        source_map: parsed.source_map,
    }
}

pub use diagnostics::{Diagnostic, DiagnosticCode, RelatedSpan, Severity, Span, SpanError};
pub use frontend::{
    AttributeSpan, FrontendResult, LineCol, NodeId, NodeInfo, SourceFile, SourceId, SourceMap,
    SourceMapError, parse_svd,
};
// Re-export of the typed CMSIS-SVD model used by `FrontendResult::device`.
pub use svd_parser::svd;
