//! Independent semantic checks over immutable Canonical IR.
pub(crate) mod access;
pub(crate) mod alternatives;
mod checks;
pub mod context;
pub mod engine;
pub(crate) mod enums;
pub(crate) mod fields;
pub(crate) mod memory;
pub(crate) mod quality;
mod ranges;
pub(crate) mod registers;
pub(crate) mod reset;
pub use quality::check_quality;

pub use context::CheckContext;
pub use engine::check_rules;
