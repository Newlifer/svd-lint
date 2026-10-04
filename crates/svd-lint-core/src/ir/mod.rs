//! Canonical physical register model, with source provenance.
mod origin;
pub use origin::{ArrayInstance, Origin, Resolved};

use crate::svd;
use serde::Serialize;

/// Properties after derivedFrom and container inheritance, in that order.
#[derive(Clone, Debug, Default, Serialize)]
pub struct RegisterProperties {
    pub size: Option<Resolved<u32>>,
    pub access: Option<Resolved<svd::Access>>,
    pub protection: Option<Resolved<svd::Protection>>,
    pub reset_value: Option<Resolved<u64>>,
    pub reset_mask: Option<Resolved<u64>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CanonicalDevice {
    pub name: String,
    pub address_unit_bits: u32,
    pub width: u32,
    pub peripherals: Vec<CanonicalPeripheral>,
    /// Typed device metadata (children are represented by canonical structures).
    pub metadata: svd::Device,
    /// XML nodes not represented by the typed model; recover their contents
    /// through the analysis SourceMap and the original SourceFile.
    pub unmodeled_nodes: Vec<crate::NodeId>,
    pub origin: Origin,
}

#[derive(Clone, Debug, Serialize)]
pub struct CanonicalPeripheral {
    pub name: String,
    pub base_address: u64,
    pub properties: RegisterProperties,
    pub registers: Vec<CanonicalRegister>,
    pub clusters: Vec<CanonicalCluster>,
    pub address_blocks: Vec<CanonicalAddressBlock>,
    pub interrupts: Vec<svd::Interrupt>,
    pub alternate_peripheral: Option<String>,
    pub metadata: svd::PeripheralInfo,
    pub origin: Origin,
}

#[derive(Clone, Debug, Serialize)]
pub struct CanonicalCluster {
    pub name: String,
    pub address: u64,
    pub properties: RegisterProperties,
    pub alternate_cluster: Option<String>,
    pub metadata: svd::ClusterInfo,
    pub origin: Origin,
}

#[derive(Clone, Debug, Serialize)]
pub struct CanonicalAddressBlock {
    pub address: u64,
    pub offset: u32,
    pub size: u32,
    pub usage: svd::AddressBlockUsage,
    pub protection: Option<svd::Protection>,
    pub origin: Origin,
}

#[derive(Clone, Debug, Serialize)]
pub struct CanonicalRegister {
    pub name: String,
    pub address: u64,
    #[serde(flatten)]
    pub properties: RegisterProperties,
    pub fields: Vec<CanonicalField>,
    pub alternate_register: Option<String>,
    pub alternate_group: Option<String>,
    pub metadata: svd::RegisterInfo,
    pub origin: Origin,
}

#[derive(Clone, Debug, Serialize)]
pub struct CanonicalField {
    pub name: String,
    pub qualified_name: String,
    pub bit_offset: u32,
    pub bit_width: u32,
    pub access: Option<Resolved<svd::Access>>,
    pub enumerated_values: Vec<CanonicalEnumeratedValues>,
    pub modified_write_values: Option<Resolved<svd::ModifiedWriteValues>>,
    pub read_action: Option<Resolved<svd::ReadAction>>,
    pub write_constraint: Option<Resolved<svd::WriteConstraint>>,
    pub metadata: svd::FieldInfo,
    pub origin: Origin,
}

#[derive(Clone, Debug, Serialize)]
pub struct CanonicalEnumeratedValues {
    pub name: Option<String>,
    pub usage: Option<Resolved<svd::Usage>>,
    pub values: Resolved<Vec<svd::EnumeratedValue>>,
    pub origin: Origin,
}
