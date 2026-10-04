use super::resolver::Definition;
use crate::{
    ir::{Origin, RegisterProperties, Resolved},
    svd,
};

pub(super) fn resolved<T>(value: T, definition: &Definition, tag: &str) -> Resolved<T> {
    Resolved {
        value,
        origin: definition
            .origins
            .get(tag)
            .cloned()
            .unwrap_or_else(|| definition.origin.clone()),
    }
}

pub(super) fn properties(
    definition: &Definition,
    parent: &RegisterProperties,
) -> RegisterProperties {
    from_raw(definition.data.properties(), parent, |tag| {
        definition
            .origins
            .get(tag)
            .cloned()
            .unwrap_or_else(|| definition.origin.clone())
    })
}

pub(super) fn from_raw(
    raw: svd::RegisterProperties,
    parent: &RegisterProperties,
    origin: impl Fn(&str) -> Origin,
) -> RegisterProperties {
    RegisterProperties {
        size: raw
            .size
            .map(|value| Resolved {
                value,
                origin: origin("size"),
            })
            .or_else(|| parent.size.clone()),
        access: raw
            .access
            .map(|value| Resolved {
                value,
                origin: origin("access"),
            })
            .or_else(|| parent.access.clone()),
        protection: raw
            .protection
            .map(|value| Resolved {
                value,
                origin: origin("protection"),
            })
            .or_else(|| parent.protection.clone()),
        reset_value: raw
            .reset_value
            .map(|value| Resolved {
                value,
                origin: origin("resetValue"),
            })
            .or_else(|| parent.reset_value.clone()),
        reset_mask: raw
            .reset_mask
            .map(|value| Resolved {
                value,
                origin: origin("resetMask"),
            })
            .or_else(|| parent.reset_mask.clone()),
    }
}

pub(super) fn contextualize(origin: &mut Origin, usage: &Origin) {
    origin.usage = usage.usage;
    for &node in &usage.derived_from {
        if !origin.derived_from.contains(&node) {
            origin.derived_from.push(node);
        }
    }
    origin.arrays = usage.arrays.clone();
}

pub(super) fn contextualize_properties(properties: &mut RegisterProperties, usage: &Origin) {
    if let Some(v) = &mut properties.size {
        contextualize(&mut v.origin, usage);
    }
    if let Some(v) = &mut properties.access {
        contextualize(&mut v.origin, usage);
    }
    if let Some(v) = &mut properties.protection {
        contextualize(&mut v.origin, usage);
    }
    if let Some(v) = &mut properties.reset_value {
        contextualize(&mut v.origin, usage);
    }
    if let Some(v) = &mut properties.reset_mask {
        contextualize(&mut v.origin, usage);
    }
}
