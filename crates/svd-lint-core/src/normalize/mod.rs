//! Stage 2: resolve definitions before applying container defaults and expanding arrays.
mod arrays;
mod derived;
mod properties;
mod resolver;
mod unmodeled;

use crate::{Diagnostic, DiagnosticCode as Code, SourceMap, ir::*, svd};
use derived::Data;
use properties::{contextualize, contextualize_properties, from_raw, properties, resolved};
use resolver::{Definition, Resolver};
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub struct NormalizeConfig {
    /// Global budget for materialized peripherals, clusters, registers and fields.
    pub max_instances: usize,
}

impl Default for NormalizeConfig {
    fn default() -> Self {
        Self {
            max_instances: 100_000,
        }
    }
}

#[derive(Debug)]
pub struct NormalizeResult {
    /// Never contains an incomplete model when normalization reports errors.
    pub device: Option<CanonicalDevice>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn normalize(
    device: &svd::Device,
    source_map: &SourceMap,
    config: &NormalizeConfig,
) -> NormalizeResult {
    let (resolver, roots) = Resolver::new(device, source_map);
    let mut builder = Builder {
        resolver,
        config,
        created: 0,
        names: BTreeSet::new(),
        active: Vec::new(),
    };
    // Resolve even unused definitions so independent bad references are reported.
    for id in 0..builder.resolver.definitions.len() {
        builder.resolver.resolve(id);
    }
    let origin = Origin {
        declaration: source_map.root(),
        usage: source_map.root(),
        ..Origin::default()
    };
    let defaults = from_raw(
        device.default_register_properties,
        &RegisterProperties::default(),
        |tag| Origin {
            declaration: source_map
                .root()
                .and_then(|n| source_map.children_named(n, tag).next()),
            usage: source_map.root(),
            ..Origin::default()
        },
    );
    let mut peripherals = Vec::new();
    for id in roots {
        let Some(definition) = builder.resolver.resolve(id) else {
            continue;
        };
        let Data::Peripheral(raw) = &definition.data else {
            continue;
        };
        for instance in arrays::instances(
            &definition,
            &origin,
            &mut builder.resolver,
            config,
            &mut builder.created,
        ) {
            let Some(address) =
                builder.address(raw.base_address, 0, instance.increment, &definition)
            else {
                continue;
            };
            if !builder.unique(&instance.name, &definition) {
                continue;
            }
            let mut props = properties(&definition, &defaults);
            contextualize_properties(&mut props, &instance.origin);
            let mut peripheral = CanonicalPeripheral {
                name: instance.name,
                base_address: address,
                properties: props,
                registers: Vec::new(),
                clusters: Vec::new(),
                address_blocks: Vec::new(),
                interrupts: raw.interrupt.clone(),
                alternate_peripheral: raw.alternate_peripheral.clone(),
                metadata: (**raw).clone(),
                origin: instance.origin,
            };
            let block_nodes = definition
                .origins
                .get("addressBlock")
                .and_then(|o| o.declaration)
                .and_then(|n| source_map.node(n))
                .and_then(|n| n.parent)
                .map(|n| {
                    source_map
                        .children_named(n, "addressBlock")
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            for (i, block) in raw
                .address_block
                .as_deref()
                .unwrap_or_default()
                .iter()
                .enumerate()
            {
                if let Some(block_address) =
                    builder.address(address, u64::from(block.offset), 0, &definition)
                {
                    let mut block_origin = peripheral.origin.clone();
                    block_origin.declaration = block_nodes.get(i).copied();
                    peripheral.address_blocks.push(CanonicalAddressBlock {
                        address: block_address,
                        offset: block.offset,
                        size: block.size,
                        usage: block.usage,
                        protection: block.protection,
                        origin: block_origin,
                    });
                }
            }
            for &child in &definition.children {
                let parent_props = peripheral.properties.clone();
                let parent_origin = peripheral.origin.clone();
                let parent_name = peripheral.name.clone();
                builder.registers(
                    child,
                    &parent_name,
                    address,
                    &parent_props,
                    &parent_origin,
                    &mut peripheral,
                );
            }
            peripherals.push(peripheral);
        }
    }
    let mut diagnostics = builder.resolver.diagnostics;
    diagnostics.sort_by_key(|d| (d.primary_span.map(|s| (s.start, s.end)), d.code.as_str()));
    let mut metadata = device.clone();
    metadata.peripherals.clear();
    let canonical = CanonicalDevice {
        name: device.name.clone(),
        address_unit_bits: device.address_unit_bits,
        width: device.width,
        peripherals,
        metadata,
        unmodeled_nodes: unmodeled::nodes(source_map),
        origin,
    };
    NormalizeResult {
        device: (!diagnostics.iter().any(|d| d.severity.is_error())).then_some(canonical),
        diagnostics,
    }
}

struct Builder<'a> {
    resolver: Resolver<'a>,
    config: &'a NormalizeConfig,
    created: usize,
    names: BTreeSet<String>,
    active: Vec<usize>,
}

impl Builder<'_> {
    fn address(
        &mut self,
        base: u64,
        offset: u64,
        increment: u64,
        definition: &Definition,
    ) -> Option<u64> {
        let address = base
            .checked_add(offset)
            .and_then(|v| v.checked_add(increment));
        if address.is_none() {
            self.resolver.diagnostic(
                Code::NormAddressOverflow,
                definition.node,
                "addressOffset",
                format!("address overflow for `{}`", definition.path),
            );
        }
        address
    }

    fn unique(&mut self, name: &str, definition: &Definition) -> bool {
        if self.names.insert(name.to_owned()) {
            return true;
        }
        self.resolver.diagnostic(
            Code::NormAmbiguousReference,
            definition.node,
            "name",
            format!("duplicate canonical path `{name}`"),
        );
        false
    }

    #[allow(clippy::too_many_arguments)]
    fn registers(
        &mut self,
        id: usize,
        parent: &str,
        base: u64,
        defaults: &RegisterProperties,
        parent_origin: &Origin,
        peripheral: &mut CanonicalPeripheral,
    ) {
        let Some(definition) = self.resolver.resolve(id) else {
            return;
        };
        // A cluster can derive children from a containing cluster. Guard this
        // structural recursion separately from reference-graph cycles.
        if self.active.contains(&id) {
            self.resolver.diagnostic(
                Code::NormCycle,
                definition.node,
                "derivedFrom",
                "recursive inherited cluster containment",
            );
            for &active in &self.active {
                if let Some(info) = self.resolver.definitions[active]
                    .node
                    .and_then(|node| self.resolver.map.node(node))
                {
                    self.resolver.diagnostics.last_mut().unwrap().related.push(
                        crate::RelatedSpan::new(
                            info.range,
                            self.resolver.definitions[active].path.clone(),
                        ),
                    );
                }
            }
            return;
        }
        self.active.push(id);
        for instance in arrays::instances(
            &definition,
            parent_origin,
            &mut self.resolver,
            self.config,
            &mut self.created,
        ) {
            let name = format!("{parent}.{}", instance.name);
            if !self.unique(&name, &definition) {
                continue;
            }
            let mut props = properties(&definition, defaults);
            contextualize_properties(&mut props, &instance.origin);
            match &definition.data {
                Data::Cluster(raw) => {
                    let Some(address) = self.address(
                        base,
                        u64::from(raw.address_offset),
                        instance.increment,
                        &definition,
                    ) else {
                        continue;
                    };
                    peripheral.clusters.push(CanonicalCluster {
                        name: name.clone(),
                        address,
                        properties: props.clone(),
                        alternate_cluster: raw.alternate_cluster.clone(),
                        metadata: (**raw).clone(),
                        origin: instance.origin.clone(),
                    });
                    for &child in &definition.children {
                        self.registers(child, &name, address, &props, &instance.origin, peripheral);
                    }
                }
                Data::Register(raw) => {
                    let Some(address) = self.address(
                        base,
                        u64::from(raw.address_offset),
                        instance.increment,
                        &definition,
                    ) else {
                        continue;
                    };
                    if props.size.is_none() {
                        self.resolver.diagnostic(
                            Code::NormMissingProperty,
                            definition.node,
                            "size",
                            format!("cannot resolve register size for `{name}`"),
                        );
                        continue;
                    }
                    let mut fields = Vec::new();
                    for &child in &definition.children {
                        fields.extend(self.fields(child, &name, &props, &instance.origin));
                    }
                    peripheral.registers.push(CanonicalRegister {
                        name,
                        address,
                        properties: props,
                        fields,
                        alternate_register: raw.alternate_register.clone(),
                        alternate_group: raw.alternate_group.clone(),
                        metadata: (**raw).clone(),
                        origin: instance.origin,
                    });
                }
                _ => {}
            }
        }
        self.active.pop();
    }

    fn fields(
        &mut self,
        id: usize,
        parent: &str,
        defaults: &RegisterProperties,
        parent_origin: &Origin,
    ) -> Vec<CanonicalField> {
        let Some(definition) = self.resolver.resolve(id) else {
            return Vec::new();
        };
        let Data::Field(raw) = &definition.data else {
            return Vec::new();
        };
        let mut fields = Vec::new();
        for instance in arrays::instances(
            &definition,
            parent_origin,
            &mut self.resolver,
            self.config,
            &mut self.created,
        ) {
            let qualified_name = format!("{parent}.{}", instance.name);
            if !self.unique(&qualified_name, &definition) {
                continue;
            }
            let offset = u64::from(raw.bit_range.offset)
                .checked_add(instance.increment)
                .and_then(|v| u32::try_from(v).ok());
            let Some(bit_offset) = offset else {
                self.resolver.diagnostic(
                    Code::NormAddressOverflow,
                    definition.node,
                    "bitOffset",
                    "field bit offset overflow",
                );
                continue;
            };
            let mut access = raw
                .access
                .map(|v| resolved(v, &definition, "access"))
                .or_else(|| defaults.access.clone());
            if let Some(v) = &mut access {
                contextualize(&mut v.origin, &instance.origin);
            }
            let mut enumerated_values = Vec::new();
            for &child in &definition.children {
                if let Some(enumeration) = self.resolver.resolve(child) {
                    if let Data::Enumerated(mut value) = enumeration.data.clone() {
                        value.derived_from = None;
                        let mut enum_origin = enumeration.origin.clone();
                        contextualize(&mut enum_origin, &instance.origin);
                        let mut values = resolved(value.values, &enumeration, "enumeratedValue");
                        // The value list belongs to its XML grouping element.
                        if enumeration.origins.contains_key("enumeratedValue") {
                            values.origin.declaration = values
                                .origin
                                .declaration
                                .and_then(|n| self.resolver.map.node(n))
                                .and_then(|n| n.parent);
                        }
                        contextualize(&mut values.origin, &instance.origin);
                        let mut usage = value.usage.map(|v| resolved(v, &enumeration, "usage"));
                        if let Some(v) = &mut usage {
                            contextualize(&mut v.origin, &instance.origin);
                        }
                        enumerated_values.push(CanonicalEnumeratedValues {
                            name: value.name,
                            usage,
                            values,
                            origin: enum_origin,
                        });
                    }
                }
            }
            let mut modified_write_values = raw
                .modified_write_values
                .map(|v| resolved(v, &definition, "modifiedWriteValues"));
            let mut read_action = raw
                .read_action
                .map(|v| resolved(v, &definition, "readAction"));
            let mut write_constraint = raw
                .write_constraint
                .map(|v| resolved(v, &definition, "writeConstraint"));
            if let Some(v) = &mut modified_write_values {
                contextualize(&mut v.origin, &instance.origin);
            }
            if let Some(v) = &mut read_action {
                contextualize(&mut v.origin, &instance.origin);
            }
            if let Some(v) = &mut write_constraint {
                contextualize(&mut v.origin, &instance.origin);
            }
            fields.push(CanonicalField {
                name: instance.name,
                qualified_name,
                bit_offset,
                bit_width: raw.bit_range.width,
                access,
                enumerated_values,
                modified_write_values,
                read_action,
                write_constraint,
                metadata: (**raw).clone(),
                origin: instance.origin,
            });
        }
        fields
    }
}
