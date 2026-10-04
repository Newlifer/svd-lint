//! SVD frontend: source handling, XML structure and typed parsing.

mod parser;
mod preflight;
mod source;
mod source_map;

pub use parser::{FrontendResult, parse_svd};
pub use source::{LineCol, SourceFile, SourceId};
pub use source_map::{AttributeSpan, NodeId, NodeInfo, SourceMap, SourceMapError};
