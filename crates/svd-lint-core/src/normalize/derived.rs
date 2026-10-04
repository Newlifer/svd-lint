//! Typed inheritance uses svd-rs rather than merging child lists by name.
use crate::svd::{self, DeriveFrom};

#[derive(Clone, Debug)]
pub(super) enum Data {
    Peripheral(svd::Peripheral),
    Cluster(svd::Cluster),
    Register(svd::Register),
    Field(svd::Field),
    Enumerated(svd::EnumeratedValues),
}

impl Data {
    /// Children are indexed independently; keep only scalar metadata in caches.
    pub fn clear_children(&mut self) {
        match self {
            Self::Peripheral(v) => v.registers = None,
            Self::Cluster(v) => v.children.clear(),
            Self::Register(v) => v.fields = None,
            Self::Field(v) => v.enumerated_values.clear(),
            Self::Enumerated(_) => {}
        }
    }
    pub fn name(&self) -> &str {
        match self {
            Self::Peripheral(v) => &v.name,
            Self::Cluster(v) => &v.name,
            Self::Register(v) => &v.name,
            Self::Field(v) => &v.name,
            Self::Enumerated(v) => v.name.as_deref().unwrap_or(""),
        }
    }
    pub fn matches_name(&self, name: &str) -> bool {
        if self.name() == name {
            return true;
        }
        let Some(dim) = self.dimension() else {
            return false;
        };
        if dim.dim == 0 {
            return false;
        }
        // Resolve physical names lazily, without indexing every array instance.
        for template in [self.name().to_owned(), self.name().replace("[%s]", "%s")] {
            let Some((prefix, suffix)) = template.split_once("%s") else {
                continue;
            };
            let Some(index) = name
                .strip_prefix(prefix)
                .and_then(|s| s.strip_suffix(suffix))
            else {
                continue;
            };
            if dim
                .dim_index
                .as_ref()
                .map(|v| v.iter().any(|s| s == index))
                .unwrap_or_else(|| {
                    index
                        .parse::<u32>()
                        .is_ok_and(|n| n < dim.dim && n.to_string() == index)
                })
            {
                return true;
            }
        }
        false
    }
    pub fn reference(&self) -> Option<&str> {
        match self {
            Self::Peripheral(v) => v.derived_from.as_deref(),
            Self::Cluster(v) => v.derived_from.as_deref(),
            Self::Register(v) => v.derived_from.as_deref(),
            Self::Field(v) => v.derived_from.as_deref(),
            Self::Enumerated(v) => v.derived_from.as_deref(),
        }
    }
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Peripheral(_) => "peripheral",
            Self::Cluster(_) => "cluster",
            Self::Register(_) => "register",
            Self::Field(_) => "field",
            Self::Enumerated(_) => "enumeratedValues",
        }
    }
    pub fn dimension(&self) -> Option<&svd::DimElement> {
        match self {
            Self::Peripheral(svd::MaybeArray::Array(_, d))
            | Self::Cluster(svd::MaybeArray::Array(_, d))
            | Self::Register(svd::MaybeArray::Array(_, d))
            | Self::Field(svd::MaybeArray::Array(_, d)) => Some(d),
            _ => None,
        }
    }
    pub fn inherits_children(&self) -> bool {
        match self {
            Self::Peripheral(v) => v.registers.is_none(),
            Self::Cluster(v) => v.children.is_empty(),
            Self::Register(v) => v.fields.is_none(),
            Self::Field(v) => v.enumerated_values.is_empty(),
            Self::Enumerated(_) => false,
        }
    }
    pub fn derive(&self, base: &Self) -> Self {
        match (self, base) {
            (Self::Peripheral(v), Self::Peripheral(b)) => Self::Peripheral(v.derive_from(b)),
            (Self::Cluster(v), Self::Cluster(b)) => Self::Cluster(v.derive_from(b)),
            (Self::Register(v), Self::Register(b)) => Self::Register(v.derive_from(b)),
            (Self::Field(v), Self::Field(b)) => Self::Field(v.derive_from(b)),
            (Self::Enumerated(v), Self::Enumerated(b)) => Self::Enumerated(v.derive_from(b)),
            _ => unreachable!("resolver filters targets by kind"),
        }
    }
    pub fn properties(&self) -> svd::RegisterProperties {
        match self {
            Self::Peripheral(v) => v.default_register_properties,
            Self::Cluster(v) => v.default_register_properties,
            Self::Register(v) => v.properties,
            _ => svd::RegisterProperties::default(),
        }
    }
}
