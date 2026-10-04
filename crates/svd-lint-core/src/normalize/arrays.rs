use super::{
    NormalizeConfig,
    resolver::{Definition, Resolver},
};
use crate::{
    DiagnosticCode as Code,
    ir::{ArrayInstance, Origin},
};
use std::collections::BTreeSet;

pub(super) struct Instance {
    pub name: String,
    pub increment: u64,
    pub origin: Origin,
}

pub(super) fn instances(
    definition: &Definition,
    parent: &Origin,
    resolver: &mut Resolver<'_>,
    config: &NormalizeConfig,
    created: &mut usize,
) -> Vec<Instance> {
    let dim = definition.data.dimension();
    let explicit_dim = definition
        .origins
        .get("dim")
        .and_then(|o| o.declaration)
        .and_then(|n| resolver.map.node(n))
        .is_some_and(|n| n.parent == definition.node);
    if dim.is_none() && explicit_dim {
        resolver.diagnostic(
            Code::NormInvalidDimension,
            definition.node,
            "dim",
            "dim requires dimIncrement",
        );
        return Vec::new();
    }
    let count = dim.map(|d| d.dim as usize).unwrap_or(1);
    if count == 0 {
        resolver.diagnostic(
            Code::NormInvalidDimension,
            definition.node,
            "dim",
            "array dimension must be positive",
        );
        return Vec::new();
    }
    if count > config.max_instances.saturating_sub(*created) {
        resolver.diagnostic(
            Code::NormExpansionLimit,
            definition.node,
            "dim",
            format!(
                "normalization instance limit ({}) exceeded",
                config.max_instances
            ),
        );
        return Vec::new();
    }
    let template = definition.data.name();
    if dim.is_none() && template.contains("%s") {
        resolver.diagnostic(
            Code::NormInvalidTemplate,
            definition.node,
            "name",
            "array placeholder requires dim",
        );
        return Vec::new();
    }
    if let Some(dim) = dim {
        if template.matches("%s").count() != 1
            || (template.contains("[%s]") && !template.ends_with("[%s]"))
        {
            resolver.diagnostic(
                Code::NormInvalidTemplate,
                definition.node,
                "name",
                format!("invalid array name template `{template}`"),
            );
            return Vec::new();
        }
        if let Some(indexes) = &dim.dim_index {
            let unique: BTreeSet<_> = indexes.iter().collect();
            if indexes.len() != count
                || unique.len() != count
                || indexes.iter().any(|s| s.is_empty())
            {
                resolver.diagnostic(
                    Code::NormInvalidIndex,
                    definition.node,
                    "dimIndex",
                    "dimIndex must contain one distinct nonempty index per instance",
                );
                return Vec::new();
            }
        }
    }
    *created += count;
    let dimension = dim.map(|d| std::sync::Arc::new(d.clone()));
    (0..count)
        .map(|ordinal| {
            let mut origin = definition.origin.clone();
            origin.arrays = parent.arrays.clone();
            let mut container = definition
                .node
                .and_then(|n| resolver.map.node(n))
                .and_then(|n| n.parent);
            while let Some(node) = container {
                let Some(info) = resolver.map.node(node) else {
                    break;
                };
                if matches!(
                    info.name.as_ref(),
                    "device" | "peripheral" | "cluster" | "register" | "field"
                ) {
                    break;
                }
                container = info.parent;
            }
            if parent.usage != parent.declaration || container != parent.declaration {
                origin.usage = parent.usage;
                origin
                    .derived_from
                    .extend(parent.derived_from.iter().copied());
            }
            let (name, increment) = if let Some(dim) = dim {
                let index = dim
                    .dim_index
                    .as_ref()
                    .map(|v| v[ordinal].clone())
                    .unwrap_or_else(|| ordinal.to_string());
                origin.arrays.push(ArrayInstance {
                    declaration: definition.node,
                    template: template.to_owned(),
                    index: index.clone(),
                    ordinal: ordinal as u32,
                    dimension: dimension.as_ref().unwrap().clone(),
                });
                // Both operands are u32; multiplication is performed in u64.
                (
                    template.replace("%s", &index),
                    (ordinal as u64) * u64::from(dim.dim_increment),
                )
            } else {
                (template.to_owned(), 0)
            };
            Instance {
                name,
                increment,
                origin,
            }
        })
        .collect()
}
