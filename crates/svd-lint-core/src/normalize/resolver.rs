use std::collections::BTreeMap;

use super::derived::Data;
use crate::{Diagnostic, DiagnosticCode as Code, NodeId, SourceMap, ir::Origin, svd};

#[derive(Clone, Debug)]
pub(super) struct Definition {
    pub data: Data,
    pub path: String,
    pub node: Option<NodeId>,
    pub children: Vec<usize>,
    pub origins: BTreeMap<String, Origin>,
    pub origin: Origin,
    inherits_children: bool,
}

#[derive(Clone)]
enum State {
    Unvisited,
    Visiting,
    Resolved(Box<Definition>),
    Failed,
}

pub(super) struct Resolver<'a> {
    pub map: &'a SourceMap,
    pub definitions: Vec<Definition>,
    states: Vec<State>,
    index: BTreeMap<(String, &'static str), Vec<usize>>,
    stack: Vec<usize>,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'a> Resolver<'a> {
    pub fn new(device: &svd::Device, map: &'a SourceMap) -> (Self, Vec<usize>) {
        let mut resolver = Self {
            map,
            definitions: Vec::new(),
            states: Vec::new(),
            index: BTreeMap::new(),
            stack: Vec::new(),
            diagnostics: Vec::new(),
        };
        let container = map
            .root()
            .and_then(|n| map.children_named(n, "peripherals").next());
        let nodes = resolver.nodes(container, "peripheral");
        let roots = device
            .peripherals
            .iter()
            .enumerate()
            .map(|(i, p)| resolver.insert(Data::Peripheral(p.clone()), "", nodes.get(i).copied()))
            .collect();
        (resolver, roots)
    }

    fn nodes(&self, parent: Option<NodeId>, tag: &str) -> Vec<NodeId> {
        parent
            .map(|n| self.map.children_named(n, tag).collect())
            .unwrap_or_default()
    }

    fn insert(&mut self, data: Data, parent: &str, node: Option<NodeId>) -> usize {
        let path = if parent.is_empty() {
            data.name().to_owned()
        } else {
            format!("{parent}.{}", data.name())
        };
        let origin = Origin {
            declaration: node,
            usage: node,
            ..Origin::default()
        };
        let mut origins = BTreeMap::new();
        if let Some(n) = node.and_then(|n| self.map.node(n)) {
            for &child in &n.children {
                if let Some(info) = self.map.node(child) {
                    origins
                        .entry(info.name.to_string())
                        .or_insert_with(|| Origin {
                            declaration: Some(child),
                            usage: node,
                            ..Origin::default()
                        });
                }
            }
        }
        let id = self.definitions.len();
        self.index
            .entry((path.clone(), data.kind()))
            .or_default()
            .push(id);
        self.definitions.push(Definition {
            inherits_children: data.inherits_children(),
            data: data.clone(),
            path: path.clone(),
            node,
            children: Vec::new(),
            origins,
            origin,
        });
        self.states.push(State::Unvisited);
        let children = match data {
            Data::Peripheral(v) => self.insert_registers(
                v.registers.as_deref().unwrap_or_default(),
                &path,
                node.and_then(|n| self.map.children_named(n, "registers").next()),
            ),
            Data::Cluster(v) => self.insert_registers(&v.children, &path, node),
            Data::Register(v) => {
                let container = node.and_then(|n| self.map.children_named(n, "fields").next());
                let nodes = self.nodes(container, "field");
                v.fields
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .enumerate()
                    .map(|(i, f)| self.insert(Data::Field(f.clone()), &path, nodes.get(i).copied()))
                    .collect()
            }
            Data::Field(v) => {
                let nodes = self.nodes(node, "enumeratedValues");
                v.enumerated_values
                    .iter()
                    .enumerate()
                    .map(|(i, e)| {
                        self.insert(Data::Enumerated(e.clone()), &path, nodes.get(i).copied())
                    })
                    .collect()
            }
            Data::Enumerated(_) => Vec::new(),
        };
        self.definitions[id].children = children;
        self.definitions[id].data.clear_children();
        id
    }

    fn insert_registers(
        &mut self,
        children: &[svd::RegisterCluster],
        path: &str,
        container: Option<NodeId>,
    ) -> Vec<usize> {
        // Follow document order and tag kind, never a global name search.
        let nodes: Vec<_> = container
            .and_then(|n| self.map.node(n))
            .map(|n| {
                n.children
                    .iter()
                    .copied()
                    .filter(|n| {
                        self.map
                            .node(*n)
                            .is_some_and(|n| matches!(&*n.name, "register" | "cluster"))
                    })
                    .collect()
            })
            .unwrap_or_default();
        children
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let data = match c {
                    svd::RegisterCluster::Register(v) => Data::Register(v.clone()),
                    svd::RegisterCluster::Cluster(v) => Data::Cluster(v.clone()),
                };
                self.insert(data, path, nodes.get(i).copied())
            })
            .collect()
    }

    pub fn diagnostic(
        &mut self,
        code: Code,
        node: Option<NodeId>,
        property: &str,
        message: impl Into<String>,
    ) {
        let mut diagnostic = Diagnostic::error(code, message);
        diagnostic.primary_span = node.and_then(|id| {
            let info = self.map.node(id)?;
            if property == "derivedFrom" {
                info.attributes
                    .iter()
                    .find(|a| &*a.name == property)
                    .map(|a| a.value_range)
                    .or(Some(info.range))
            } else {
                self.map
                    .children_named(id, property)
                    .next()
                    .and_then(|n| self.map.node(n))
                    .map(|n| n.range)
                    .or(Some(info.range))
            }
        });
        self.diagnostics.push(diagnostic);
    }

    fn target(&mut self, definition: &Definition, reference: &str) -> Option<usize> {
        let scope = definition
            .path
            .rsplit_once('.')
            .map(|(scope, _)| scope)
            .unwrap_or("");
        let path = if reference.contains('.') || scope.is_empty() {
            reference.to_owned()
        } else {
            format!("{scope}.{reference}")
        };
        let candidates = if definition.data.kind() == "enumeratedValues" {
            let register = scope.rsplit_once('.').map(|(p, _)| p).unwrap_or("");
            match reference.split('.').count() {
                1 => self
                    .definitions
                    .iter()
                    .enumerate()
                    .filter(|(_, d)| {
                        d.data.kind() == "enumeratedValues"
                            && d.data.name() == reference
                            && d.path
                                .rsplit_once('.')
                                .and_then(|(p, _)| p.rsplit_once('.'))
                                .is_some_and(|(p, _)| p == register)
                    })
                    .map(|(id, _)| id)
                    .collect(),
                2 => self.lookup(&format!("{register}.{reference}"), definition.data.kind()),
                3 => {
                    let block = register.rsplit_once('.').map(|(p, _)| p).unwrap_or("");
                    self.lookup(&format!("{block}.{reference}"), definition.data.kind())
                }
                _ => self.lookup(&path, definition.data.kind()),
            }
        } else {
            self.lookup(&path, definition.data.kind())
        };
        match candidates.as_slice() {
            [id] => Some(*id),
            [] => {
                self.diagnostic(
                    Code::NormMissingReference,
                    definition.node,
                    "derivedFrom",
                    format!(
                        "derivedFrom target `{reference}` not found for `{}`",
                        definition.path
                    ),
                );
                None
            }
            _ => {
                self.diagnostic(
                    Code::NormAmbiguousReference,
                    definition.node,
                    "derivedFrom",
                    format!("ambiguous derivedFrom target `{reference}`"),
                );
                for id in candidates {
                    if let Some(span) = self.definitions[id]
                        .node
                        .and_then(|n| self.map.node(n))
                        .map(|n| n.range)
                    {
                        self.diagnostics
                            .last_mut()
                            .unwrap()
                            .related
                            .push(crate::RelatedSpan::new(span, "candidate definition"));
                    }
                }
                None
            }
        }
    }

    fn lookup(&mut self, path: &str, kind: &'static str) -> Vec<usize> {
        if let Some(ids) = self.index.get(&(path.to_owned(), kind)) {
            return ids.clone();
        }
        // Qualified references may name children inherited by a derived scope.
        let parts: Vec<_> = path.split('.').collect();
        let roots: Vec<_> = self
            .definitions
            .iter()
            .enumerate()
            .filter(|(_, d)| d.data.kind() == "peripheral")
            .map(|(id, _)| id)
            .collect();
        let mut current: Vec<_> = roots
            .into_iter()
            .filter(|id| self.matches(*id, parts[0]))
            .collect();
        for part in &parts[1..] {
            let mut next = Vec::new();
            for id in current {
                if let Some(parent) = self.resolve(id) {
                    for child in parent.children {
                        if self.matches(child, part) {
                            next.push(child);
                        }
                    }
                }
            }
            current = next;
        }
        current
            .into_iter()
            .filter(|id| self.definitions[*id].data.kind() == kind)
            .collect()
    }

    fn matches(&mut self, id: usize, name: &str) -> bool {
        let data = &self.definitions[id].data;
        if data.matches_name(name) {
            return true;
        }
        if data.dimension().is_some() || data.reference().is_none() {
            return false;
        }
        let possible = [data.name().to_owned(), data.name().replace("[%s]", "%s")]
            .iter()
            .any(|template| {
                template.split_once("%s").is_some_and(|(prefix, suffix)| {
                    name.strip_prefix(prefix)
                        .and_then(|s| s.strip_suffix(suffix))
                        .is_some_and(|s| !s.is_empty())
                })
            });
        possible && self.resolve(id).is_some_and(|d| d.data.matches_name(name))
    }

    pub fn resolve(&mut self, id: usize) -> Option<Definition> {
        match &self.states[id] {
            State::Resolved(v) => return Some((**v).clone()),
            State::Failed => return None,
            State::Visiting => {
                let node = self.definitions[*self.stack.last().unwrap_or(&id)].node;
                self.diagnostic(
                    Code::NormCycle,
                    node,
                    "derivedFrom",
                    "cyclic derivedFrom dependency",
                );
                let start = self.stack.iter().position(|v| *v == id).unwrap_or(0);
                for &member in &self.stack[start..] {
                    if let Some(info) = self.definitions[member].node.and_then(|n| self.map.node(n))
                    {
                        self.diagnostics
                            .last_mut()
                            .unwrap()
                            .related
                            .push(crate::RelatedSpan::new(
                                info.range,
                                self.definitions[member].path.clone(),
                            ));
                    }
                }
                return None;
            }
            State::Unvisited => {}
        }
        self.states[id] = State::Visiting;
        self.stack.push(id);
        let mut definition = self.definitions[id].clone();
        let mut success = true;
        if let Some(reference) = definition.data.reference().map(str::to_owned) {
            if let Some(base) = self
                .target(&definition, &reference)
                .and_then(|id| self.resolve(id))
            {
                if definition.inherits_children {
                    definition.children = base.children.clone();
                }
                let derived = definition.data.derive(&base.data);
                // Only properties actually inherited by svd-rs get base provenance.
                for (key, mut origin) in base.origins {
                    if !definition.origins.contains_key(&key) {
                        origin.usage = definition.node;
                        if let Some(n) = definition.node {
                            origin.derived_from.push(n);
                        }
                        definition.origins.insert(key, origin);
                    }
                }
                definition.origin.derived_from = base.origin.derived_from;
                if let Some(n) = base.node {
                    definition.origin.derived_from.push(n);
                }
                definition.data = derived;
            } else {
                success = false;
            }
        }
        self.stack.pop();
        self.states[id] = if success {
            State::Resolved(Box::new(definition.clone()))
        } else {
            State::Failed
        };
        success.then_some(definition)
    }
}
