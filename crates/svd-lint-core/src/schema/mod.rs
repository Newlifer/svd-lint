//! Optional schema boundary. No experimental validator is part of the default pipeline.
pub mod diagnostics;
mod provider;
mod validator;
pub use provider::{DirectoryProvider, SchemaVersion};
pub use validator::{SchemaValidator, analyze_with_schema};
#[cfg(feature = "xsd")]
mod backend;
#[cfg(feature = "xsd")]
pub use backend::UppsalaValidator;
