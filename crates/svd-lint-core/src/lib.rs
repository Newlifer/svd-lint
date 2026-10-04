//! svd-lint-core: Stage 1 of the CMSIS-SVD static analyzer.
//!
//! Provides the source handling ([`frontend::SourceFile`]), XML position
//! index ([`frontend::SourceMap`]), preflight checks, the typed
//! `svd-parser` frontend ([`frontend::parse_svd`]) and the renderer-agnostic
//! [`diagnostics`] model. Rendering (text/JSON) lives in `svd-lint-cli`;
//! this crate never depends on it.

pub mod diagnostics;
pub mod frontend;

pub use diagnostics::{Diagnostic, DiagnosticCode, RelatedSpan, Severity, Span, SpanError};
pub use frontend::{
    AttributeSpan, FrontendResult, LineCol, NodeId, NodeInfo, SourceFile, SourceId, SourceMap,
    SourceMapError, parse_svd,
};
// Re-export of the typed CMSIS-SVD model used by `FrontendResult::device`.
pub use svd_parser::svd;
