use super::{
    CheckContext, Rule,
    ranges::{self, Interval},
};
use crate::{Diagnostic, DiagnosticCode as Code, Severity, ir::*};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug)]
pub(super) enum Alternate {
    Absent,
    Valid(usize),
    Invalid(&'static str),
    Unknown,
}

/// Read-only indexes of the existing IR, not a second register model.
pub(super) struct Layout<'a> {
    pub peripheral: &'a CanonicalPeripheral,
    pub intervals: Vec<Option<Interval>>,
    pub alternatives: Vec<Alternate>,
    targets: Vec<Option<usize>>,
    roots: Vec<usize>,
    cluster_groups: Vec<BTreeSet<(usize, usize)>>,
    uncertain_clusters: Vec<bool>,
}

fn substitute(reference: &str, origin: &crate::Origin) -> String {
    origin
        .arrays
        .last()
        .map(|d| reference.replace("%s", &d.index))
        .unwrap_or_else(|| reference.to_owned())
}

impl<'a> Layout<'a> {
    pub fn new(peripheral: &'a CanonicalPeripheral, unit: u32) -> Self {
        let intervals: Vec<_> = peripheral
            .registers
            .iter()
            .map(|r| {
                r.properties
                    .size
                    .as_ref()
                    .and_then(|s| ranges::memory(r.address, s.value, unit))
            })
            .collect();
        let paths: BTreeMap<_, _> = peripheral
            .registers
            .iter()
            .enumerate()
            .map(|(i, r)| (r.name.as_str(), i))
            .collect();
        let mut names: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for (i, r) in peripheral.registers.iter().enumerate() {
            names
                .entry(r.name.rsplit('.').next().unwrap_or(&r.name))
                .or_default()
                .push(i);
        }
        let mut targets = vec![None; peripheral.registers.len()];
        let alternatives: Vec<_> = peripheral
            .registers
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let Some(reference) = &r.alternate_register else {
                    return Alternate::Absent;
                };
                // alternateRegister is an identifier in this peripheral, not derivedFrom.
                let reference = substitute(reference, &r.origin);
                if reference.is_empty() || reference.contains('.') {
                    return Alternate::Invalid(
                        "alternateRegister must be a peripheral-local register identifier",
                    );
                }
                // In an array of clusters, resolve the register in the same physical
                // cluster first, rather than treating all repeated members as ambiguous.
                let local = r
                    .name
                    .rsplit_once('.')
                    .map(|(scope, _)| format!("{scope}.{reference}"));
                let candidates = local
                    .as_deref()
                    .and_then(|n| paths.get(n))
                    .map(|i| vec![*i])
                    .unwrap_or_else(|| names.get(reference.as_str()).cloned().unwrap_or_default());
                let target = match candidates.as_slice() {
                    [target] => *target,
                    [] => {
                        return Alternate::Invalid(
                            "alternateRegister target does not exist in this peripheral",
                        );
                    }
                    _ => {
                        return Alternate::Invalid(
                            "alternateRegister target is ambiguous in this peripheral",
                        );
                    }
                };
                targets[i] = Some(target);
                if target >= i {
                    return Alternate::Invalid(
                        "alternateRegister must reference a previously declared register",
                    );
                }
                match (intervals[i], intervals[target]) {
                    (Some(a), Some(b)) if a == b => Alternate::Valid(target),
                    (Some(_), Some(_)) => {
                        Alternate::Invalid("alternateRegister describes a different memory region")
                    }
                    _ => Alternate::Unknown,
                }
            })
            .collect();
        let mut roots = Vec::with_capacity(alternatives.len());
        for (i, status) in alternatives.iter().enumerate() {
            roots.push(if let Alternate::Valid(target) = status {
                roots[*target]
            } else {
                i
            });
        }
        let (cluster_groups, uncertain_clusters) = cluster_views(peripheral);
        Self {
            peripheral,
            intervals,
            alternatives,
            targets,
            roots,
            cluster_groups,
            uncertain_clusters,
        }
    }

    pub fn permitted(&self, a: usize, b: usize) -> bool {
        let left = &self.peripheral.registers[a];
        let right = &self.peripheral.registers[b];
        if self.intervals[a] == self.intervals[b] {
            if self.roots[a] == self.roots[b] {
                return true;
            }
            if left
                .alternate_group
                .as_ref()
                .is_some_and(|g| !g.is_empty() && right.alternate_group.as_ref() == Some(g))
            {
                return true;
            }
        }
        self.cluster_groups[a].iter().any(|&(group, member)| {
            self.cluster_groups[b]
                .range((group, 0)..=(group, usize::MAX))
                .any(|&(_, other)| member != other)
        })
    }

    pub fn uncertain(&self, a: usize, b: usize) -> bool {
        let left = &self.peripheral.registers[a];
        let right = &self.peripheral.registers[b];
        left.address == right.address
            || left.alternate_group.is_some()
            || right.alternate_group.is_some()
            || self.uncertain_clusters[a]
            || self.uncertain_clusters[b]
    }
}

pub(super) struct Alternatives;
impl Rule for Alternatives {
    fn id(&self) -> &'static str {
        "alternatives"
    }
    fn check(&self, context: &CheckContext<'_>, diagnostics: &mut Vec<Diagnostic>) {
        for peripheral in &context.device.peripherals {
            let layout = Layout::new(peripheral, context.device.address_unit_bits);
            for (i, (r, status)) in peripheral
                .registers
                .iter()
                .zip(&layout.alternatives)
                .enumerate()
            {
                if let Alternate::Invalid(reason) = status {
                    let mut d = context.diagnostic(
                        Code::SemInvalidAlternateRegister,
                        Severity::Error,
                        *reason,
                        &r.origin,
                        "alternateRegister",
                    );
                    if let Some(target) =
                        layout.targets[i].map(|target| &peripheral.registers[target])
                    {
                        context.related(&mut d, &target.origin, "name", "referenced register");
                    }
                    diagnostics.push(d);
                }
            }
        }
    }
}

fn root(parents: &mut [usize], mut id: usize) -> usize {
    while parents[id] != id {
        parents[id] = parents[parents[id]];
        id = parents[id];
    }
    id
}

/// Index intentional cluster views once, instead of scanning all clusters for
/// every overlapping register pair. Nested and transitive views are retained.
type ClusterMemberships = Vec<BTreeSet<(usize, usize)>>;

fn cluster_views(peripheral: &CanonicalPeripheral) -> (ClusterMemberships, Vec<bool>) {
    let indexes: BTreeMap<_, _> = peripheral
        .clusters
        .iter()
        .enumerate()
        .map(|(i, c)| (c.name.as_str(), i))
        .collect();
    let mut parents: Vec<_> = (0..peripheral.clusters.len()).collect();
    let mut participating = vec![false; parents.len()];
    for (i, cluster) in peripheral.clusters.iter().enumerate() {
        let Some(reference) = &cluster.alternate_cluster else {
            continue;
        };
        let Some((scope, _)) = cluster.name.rsplit_once('.') else {
            continue;
        };
        let target_name = format!("{scope}.{}", substitute(reference, &cluster.origin));
        let Some(&target) = indexes.get(target_name.as_str()) else {
            continue;
        };
        if target == i || peripheral.clusters[target].address != cluster.address {
            continue;
        }
        participating[i] = true;
        participating[target] = true;
        let a = root(&mut parents, i);
        let b = root(&mut parents, target);
        parents[a.max(b)] = a.min(b);
    }
    let mut groups = Vec::with_capacity(peripheral.registers.len());
    let mut uncertain = Vec::with_capacity(peripheral.registers.len());
    for register in &peripheral.registers {
        let mut memberships = BTreeSet::new();
        let mut possible = false;
        let mut scope = register.name.rsplit_once('.').map(|(scope, _)| scope);
        while let Some(path) = scope {
            if let Some(&cluster) = indexes.get(path) {
                possible |= peripheral.clusters[cluster].alternate_cluster.is_some();
                if participating[cluster] {
                    memberships.insert((root(&mut parents, cluster), cluster));
                }
            }
            scope = path.rsplit_once('.').map(|(scope, _)| scope);
        }
        groups.push(memberships);
        uncertain.push(possible);
    }
    (groups, uncertain)
}
