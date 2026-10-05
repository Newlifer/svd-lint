use crate::{Diagnostic, DiagnosticCode, NodeId, Origin, Severity, SourceMap, Span, ir::*};

/// Source provenance helpers shared by every rule; checks never modify the IR.
pub struct CheckContext<'a> {
    pub device: &'a CanonicalDevice,
    pub source_map: &'a SourceMap,
}

impl CheckContext<'_> {
    pub fn node_span(&self, node: Option<NodeId>) -> Option<Span> {
        node.and_then(|n| self.source_map.node(n)).map(|n| n.range)
    }

    /// Resolves a property at its actual declaration, falling back to its owner.
    pub fn element_node(&self, origin: &Origin, tag: &str) -> Option<NodeId> {
        let declaration = origin.declaration.or(origin.usage)?;
        let node = self.source_map.node(declaration)?;
        if node.name.as_ref() == tag {
            return Some(declaration);
        }
        self.source_map
            .children_named(declaration, tag)
            .next()
            .or(Some(declaration))
    }

    pub fn element_span(&self, origin: &Origin, tag: &str) -> Option<Span> {
        self.node_span(self.element_node(origin, tag))
    }

    pub fn attribute_span(&self, origin: &Origin, name: &str) -> Option<Span> {
        origin
            .declaration
            .or(origin.usage)
            .and_then(|n| self.source_map.node(n))
            .and_then(|n| n.attributes.iter().find(|a| a.name.as_ref() == name))
            .map(|a| a.value_range)
            .or_else(|| self.node_span(origin.declaration.or(origin.usage)))
    }

    pub(super) fn diagnostic(
        &self,
        code: DiagnosticCode,
        severity: Severity,
        message: impl Into<String>,
        origin: &Origin,
        tag: &str,
    ) -> Diagnostic {
        let mut diagnostic = Diagnostic::new(code, severity, message);
        diagnostic.primary_span = self.element_span(origin, tag);
        let owner = origin
            .declaration
            .and_then(|n| self.source_map.node(n))
            .and_then(|n| n.parent);
        if origin.usage != origin.declaration && origin.usage != owner {
            if let Some(span) = self.node_span(origin.usage) {
                diagnostic
                    .related
                    .push(crate::RelatedSpan::new(span, "property is used here"));
            }
        }
        diagnostic
    }

    pub(super) fn related(
        &self,
        diagnostic: &mut Diagnostic,
        origin: &Origin,
        tag: &str,
        message: &str,
    ) {
        if let Some(span) = self.element_span(origin, tag) {
            diagnostic
                .related
                .push(crate::RelatedSpan::new(span, message));
        }
    }

    /// Item provenance follows the declaration of the effective value list,
    /// including lists inherited via derivedFrom.
    pub(super) fn enum_item_origin(
        &self,
        values: &CanonicalEnumeratedValues,
        index: usize,
    ) -> Origin {
        let mut origin = values.values.origin.clone();
        origin.declaration = origin.declaration.and_then(|n| {
            self.source_map
                .children_named(n, "enumeratedValue")
                .nth(index)
        });
        origin
    }
}
