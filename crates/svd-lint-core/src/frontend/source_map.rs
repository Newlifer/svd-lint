//! Map between XML nodes and their source positions.
//!
//! [`SourceMap`] owns everything it stores: no `roxmltree` nodes (which
//! borrow the source text) are kept in long-lived structures. Node ids are
//! local to a single map; ids from two independent parses must never be
//! compared.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::diagnostics::Span;

use super::source::{SourceFile, SourceId};

/// Identifier of a node inside one [`SourceMap`].
///
/// Only meaningful together with the map that produced it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u32);

/// Source ranges of one XML attribute.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttributeSpan {
    /// Attribute name (without namespace prefix resolution).
    pub name: Box<str>,
    /// Range of the whole attribute (`name="value"`).
    pub range: Span,
    /// Range of the attribute name only.
    pub name_range: Span,
    /// Range of the attribute value, excluding the surrounding quotes.
    pub value_range: Span,
}

/// Source information about one XML element.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeInfo {
    /// Local tag name of the element.
    pub name: Box<str>,
    /// Range of the whole element, from `<` of the start tag to `>` of the
    /// end tag (or of the self-closing tag).
    pub range: Span,
    /// Parent element, if any.
    pub parent: Option<NodeId>,
    /// Direct child elements, in document order.
    pub children: Vec<NodeId>,
    /// Attributes, in document order.
    pub attributes: Vec<AttributeSpan>,
}

/// Errors produced while building a [`SourceMap`].
#[derive(Debug, Error)]
pub enum SourceMapError {
    /// A range reported by the XML parser did not fit the source text.
    #[error("invalid span {start}..{end} for element `{element}`: {reason}")]
    InvalidSpan {
        /// Element the span belongs to.
        element: String,
        /// Span start.
        start: usize,
        /// Span end.
        end: usize,
        /// Why the span was rejected.
        reason: String,
    },
}

/// Owned index of all XML elements of one source file and their positions.
#[derive(Clone, Debug)]
pub struct SourceMap {
    source_id: SourceId,
    nodes: Vec<NodeInfo>,
    root: Option<NodeId>,
}

impl SourceMap {
    /// Builds a map from an already parsed `roxmltree` document.
    ///
    /// Every span reported by `roxmltree` is validated against `source`;
    /// a malformed span aborts the build with [`SourceMapError`].
    pub fn from_document(
        source: &SourceFile,
        document: &roxmltree::Document,
    ) -> Result<Self, SourceMapError> {
        let mut map = SourceMap {
            source_id: source.id(),
            nodes: Vec::new(),
            root: None,
        };
        let root = document.root_element();
        map.root = Some(map.insert_node(source, root, None)?);
        Ok(map)
    }

    fn insert_node(
        &mut self,
        source: &SourceFile,
        node: roxmltree::Node<'_, '_>,
        parent: Option<NodeId>,
    ) -> Result<NodeId, SourceMapError> {
        let id = NodeId(u32::try_from(self.nodes.len()).unwrap_or(u32::MAX));
        let range = Self::convert_span(source, node.tag_name().name(), node.range())?;
        let mut attributes = Vec::new();
        for attr in node.attributes() {
            attributes.push(AttributeSpan {
                name: attr.name().into(),
                range: Self::convert_span(source, node.tag_name().name(), attr.range())?,
                name_range: Self::convert_span(source, node.tag_name().name(), attr.range_qname())?,
                value_range: Self::convert_span(
                    source,
                    node.tag_name().name(),
                    attr.range_value(),
                )?,
            });
        }
        // Reserve the slot before recursing so children can reference it.
        self.nodes.push(NodeInfo {
            name: node.tag_name().name().into(),
            range,
            parent,
            children: Vec::new(),
            attributes,
        });
        let mut children = Vec::new();
        for child in node.children().filter(|c| c.is_element()) {
            children.push(self.insert_node(source, child, Some(id))?);
        }
        self.nodes[id.0 as usize].children = children;
        Ok(id)
    }

    fn convert_span(
        source: &SourceFile,
        element: &str,
        range: std::ops::Range<usize>,
    ) -> Result<Span, SourceMapError> {
        let span = Span::new_unchecked(range.start, range.end);
        source
            .validate_span(span)
            .map_err(|err| SourceMapError::InvalidSpan {
                element: element.to_owned(),
                start: range.start,
                end: range.end,
                reason: err.to_string(),
            })?;
        Ok(span)
    }

    /// Id of the source file this map was built from.
    pub fn source_id(&self) -> SourceId {
        self.source_id
    }

    /// Root element of the document, if the map is not empty.
    pub fn root(&self) -> Option<NodeId> {
        self.root
    }

    /// Number of indexed elements.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Looks up a node by id.
    pub fn node(&self, id: NodeId) -> Option<&NodeInfo> {
        self.nodes.get(id.0 as usize)
    }

    /// Direct child elements of `id` with the given tag name.
    pub fn children_named<'a>(
        &'a self,
        id: NodeId,
        name: &'a str,
    ) -> impl Iterator<Item = NodeId> + 'a {
        self.node(id)
            .into_iter()
            .flat_map(move |node| node.children.iter().copied())
            .filter(move |child| self.node(*child).is_some_and(|n| &*n.name == name))
    }

    /// Finds the first element reachable by a path of tag names from the
    /// root, e.g. `&["peripherals", "peripheral"]`.
    pub fn find_by_path(&self, path: &[&str]) -> Option<NodeId> {
        let mut current = self.root?;
        for (depth, name) in path.iter().enumerate() {
            if depth == 0 {
                if &*self.node(current)?.name != *name {
                    return None;
                }
            } else {
                current = self.children_named(current, name).next()?;
            }
        }
        Some(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = "<device schemaVersion=\"1.1\">\n  <name>Тест</name>\n  <peripherals>\n    <peripheral derivedFrom=\"A\">\n      <name>B</name>\n    </peripheral>\n  </peripherals>\n</device>\n";

    fn map() -> (SourceFile, SourceMap) {
        let source = SourceFile::new("test.svd", XML);
        let doc = roxmltree::Document::parse(XML).unwrap();
        let map = SourceMap::from_document(&source, &doc).unwrap();
        (source, map)
    }

    #[test]
    fn indexes_all_elements() {
        let (_, map) = map();
        assert_eq!(map.node_count(), 5); // device, name, peripherals, peripheral, name
        let root = map.root().unwrap();
        assert_eq!(&*map.node(root).unwrap().name, "device");
    }

    #[test]
    fn element_ranges_match_source() {
        let (source, map) = map();
        let name = map.find_by_path(&["device", "name"]).unwrap();
        let range = map.node(name).unwrap().range;
        assert_eq!(source.snippet(range), Some("<name>Тест</name>"));
    }

    #[test]
    fn multiline_positions_via_map() {
        let (source, map) = map();
        let periph = map
            .find_by_path(&["device", "peripherals", "peripheral"])
            .unwrap();
        let pos = source
            .line_col(map.node(periph).unwrap().range.start)
            .unwrap();
        assert_eq!((pos.line, pos.column), (4, 5));
    }

    #[test]
    fn attribute_name_and_value_ranges() {
        let (source, map) = map();
        let root = map.root().unwrap();
        let attrs = &map.node(root).unwrap().attributes;
        assert_eq!(attrs.len(), 1);
        let attr = &attrs[0];
        assert_eq!(&*attr.name, "schemaVersion");
        assert_eq!(source.snippet(attr.name_range), Some("schemaVersion"));
        assert_eq!(source.snippet(attr.value_range), Some("1.1"));
        assert_eq!(source.snippet(attr.range), Some("schemaVersion=\"1.1\""));
    }

    #[test]
    fn parents_and_children_link_both_ways() {
        let (_, map) = map();
        let peripherals = map.find_by_path(&["device", "peripherals"]).unwrap();
        let children: Vec<_> = map.children_named(peripherals, "peripheral").collect();
        assert_eq!(children.len(), 1);
        assert_eq!(map.node(children[0]).unwrap().parent, Some(peripherals));
    }

    #[test]
    fn wrong_path_returns_none() {
        let (_, map) = map();
        assert!(map.find_by_path(&["device", "registers"]).is_none());
        assert!(map.find_by_path(&["peripheral"]).is_none());
    }

    #[test]
    fn span_validation_is_enforced() {
        // A synthetic document whose spans cannot fit the (shorter) source
        // must be rejected rather than stored.
        let xml = "<device><name>x</name></device>";
        let doc = roxmltree::Document::parse(xml).unwrap();
        let short = SourceFile::new("short.svd", "<dev");
        assert!(SourceMap::from_document(&short, &doc).is_err());
    }
}
