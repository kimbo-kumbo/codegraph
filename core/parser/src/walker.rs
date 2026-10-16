//! Общий обходчик tree-sitter AST.

use tree_sitter::Node as TsNode;

pub struct Walker<'a> {
    pub source: &'a str,
    pub rel_path: &'a str,
}

impl<'a> Walker<'a> {
    pub fn new(source: &'a str, rel_path: &'a str) -> Self {
        Self { source, rel_path }
    }

    pub fn walk<F>(&self, node: TsNode, on_node: &mut F)
    where
        F: FnMut(TsNode, &Self),
    {
        on_node(node, self);
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.walk(child, on_node);
        }
    }

    pub fn text(&self, node: TsNode) -> &str {
        &self.source[node.byte_range()]
    }

    pub fn position(&self, node: TsNode) -> (u32, u32) {
        let start = node.start_position();
        ((start.row + 1) as u32, (start.column + 1) as u32)
    }

    pub fn has_child_of_kind(&self, node: TsNode, kind: &str) -> bool {
        let mut cursor = node.walk();
        let found = node.children(&mut cursor).any(|c| c.kind() == kind);
        found
    }

    pub fn child_of_kind<'t>(&self, node: TsNode<'t>, kind: &str) -> Option<TsNode<'t>> {
        let mut cursor = node.walk();
        let found = node.children(&mut cursor).find(|c| c.kind() == kind);
        found
    }
}