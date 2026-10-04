use crate::NodeId;
use serde::Serialize;

/// One dimension in a possibly nested array instance.
#[derive(Clone, Debug, Serialize)]
pub struct ArrayInstance {
    pub declaration: Option<NodeId>,
    pub template: String,
    pub index: String,
    pub ordinal: u32,
    /// Keeps dimName, dimArrayIndex and the original array semantics.
    pub dimension: std::sync::Arc<crate::svd::DimElement>,
}

/// Node ids are meaningful only with the SourceMap from the same analysis.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Origin {
    pub declaration: Option<NodeId>,
    pub usage: Option<NodeId>,
    /// From original definition to the most recent derived use.
    pub derived_from: Vec<NodeId>,
    pub arrays: Vec<ArrayInstance>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Resolved<T> {
    pub value: T,
    pub origin: Origin,
}
