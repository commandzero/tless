use super::JsonViewer;
use crate::flatjson::Index;
use crate::toon_display::normalize_node;

impl JsonViewer {
    fn set_document_collapsed(&mut self, root: Index, collapsed: bool) {
        if collapsed {
            self.document_collapsed.insert(root);
        } else {
            self.document_collapsed.remove(&root);
        }
    }

    fn set_node_collapsed(&mut self, node: Index, collapsed: bool) {
        if collapsed {
            self.flatjson.collapse(node);
        } else {
            self.flatjson.expand(node);
        }
    }

    pub(super) fn collapse(&mut self, node: Index, collapsed: bool) {
        if self.is_document_root(node) && self.focused_document_header() {
            self.set_document_collapsed(node, collapsed);
        } else if self.layout.node(&self.flatjson, node).collapsible {
            self.set_node_collapsed(node, collapsed);
        }
    }

    pub(super) fn toggle_collapsed(&mut self, node: Index) {
        if self.is_document_root(node) && self.focused_document_header() {
            self.collapse(node, !self.is_document_collapsed(node));
            self.refresh_projection();
        } else if self.layout.node(&self.flatjson, node).inline_array {
            self.flatjson.expand(node);
            self.expanded_arrays.insert(node);
            self.rebuild_layout();
        } else if self.is_document_root(node)
            && !self.line_is_collapsible(self.focused_line_index())
        {
            // Scalar and empty sequence roots have a selectable body line but
            // no parsed collapse state; only their generated header collapses.
        } else {
            self.collapse(node, !self.flatjson[node].is_collapsed());
            self.refresh_projection();
        }
    }

    pub(super) fn collapse_siblings(&mut self, collapsed: bool, deep: bool) {
        let bulk_document_rows = self
            .active_root_for(self.focused_node)
            .is_some_and(|root| root == normalize_node(&self.flatjson, self.focused_node))
            && self.is_sequence();
        for node in self.siblings_for_collapse(bulk_document_rows) {
            if bulk_document_rows && self.is_document_root(node) {
                self.set_document_collapsed(node, collapsed);
            } else {
                self.collapse(node, collapsed);
            }
            if deep {
                self.collapse_document_body_and_descendants(node, collapsed);
            }
        }
        self.refresh_projection();
    }

    fn siblings_for_collapse(&self, document_rows: bool) -> Vec<Index> {
        if document_rows {
            return self.active_roots.clone();
        }
        let Some(parent) = self.effective_parent(self.focused_node).as_option() else {
            return vec![self.focused_node];
        };
        let first = self.flatjson[parent].first_child().as_option();
        std::iter::successors(first, |&node| self.flatjson[node].next_sibling.as_option())
            .filter(|&node| self.contains_node(node))
            .collect()
    }

    fn collapse_document_body_and_descendants(&mut self, node: Index, collapsed: bool) {
        if self.has_collapsible_document_body(node) {
            self.set_node_collapsed(node, collapsed);
        }
        let Some(end) = self.flatjson[node].pair_index().as_option() else {
            return;
        };
        for descendant in node + 1..end {
            if self.contains_node(descendant) {
                self.collapse(descendant, collapsed);
            }
        }
    }

    fn has_collapsible_document_body(&self, node: Index) -> bool {
        if !self.is_document_root(node) {
            return false;
        }
        let collection =
            self.flatjson[node].is_array() || self.layout.analysis.node(&self.flatjson, node).table;
        collection && self.layout.node(&self.flatjson, node).entry_count > 0
    }
}
