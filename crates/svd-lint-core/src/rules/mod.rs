//! Independent semantic checks over immutable Canonical IR.
mod access;
mod alternatives;
pub mod context;
pub mod engine;
mod enums;
mod fields;
mod memory;
mod ranges;
mod registers;
mod reset;

pub use context::CheckContext;
pub use engine::{Rule, RuleEngine, check_semantics};
