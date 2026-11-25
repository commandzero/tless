use super::JsonViewer;
use crate::flatjson::{Index, OptionIndex};
use crate::toon_display::layout::{Layout, VisibleLine};
use crate::toon_display::normalize_node;

impl JsonViewer {
    pub fn visible_line(&self, index: usize) -> Option<VisibleLine> {
        self.layout
            .visible_line(&self.flatjson, &self.visible, index)
    }

    pub fn set_roots(&mut self, roots: Vec<Index>) {
        for &root in &roots {
            self.flatjson.expand(root);
            self.document_collapsed.remove(&root);
        }
        self.active_roots = roots;
        self.layout = Layout::new(
            &self.flatjson,
            &self.active_roots,
            usize::from(self.dimensions.width),
            self.line_numbers,
            &self.expanded_arrays,
        );
        self.visible = self
            .layout
            .project_with_documents(&self.flatjson, &self.document_collapsed);
        self.refresh_document_metadata();
        self.focus_first_root();
        self.top_visible_line = 0;
        self.top_continuation = 0;
        self.jump_distance = None;
        self.focus_byte_range = None;
        self.reset_frame_geometry();
        self.layout_generation = self.layout_generation.wrapping_add(1);
    }

    pub(super) fn focus_first_root(&mut self) {
        let first = self.active_roots.first().copied();
        self.focused_node = first.unwrap_or(0);
        self.desired_depth = first.map_or(0, |root| self.flatjson[root].depth);
        self.absolute_anchor_line = if self.is_sequence() {
            self.document_header_absolute(self.focused_node)
        } else {
            self.layout.node(&self.flatjson, self.focused_node).line
        };
    }

    /// Return the selected values in encounter order, retaining original node ids.
    pub fn active_roots(&self) -> &[Index] {
        &self.active_roots
    }

    pub fn document_roots(&self) -> &[Index] {
        &self.document_roots
    }

    /// Whether a parsed node belongs to one of the selected subtrees.
    pub fn contains_node(&self, node: Index) -> bool {
        self.active_root_for(node).is_some()
    }

    pub(super) fn active_root_for(&self, node: Index) -> Option<Index> {
        self.layout.active_root_for(&self.flatjson, node)
    }

    /// Selected roots are parentless only for interaction; source ancestry is
    /// left untouched for paths and copy operations.
    pub(super) fn effective_parent(&self, node: Index) -> OptionIndex {
        let node = normalize_node(&self.flatjson, node);
        if self.active_root_for(node) == Some(node) {
            OptionIndex::NIL
        } else if self.contains_node(node) {
            self.flatjson[node].parent
        } else {
            OptionIndex::NIL
        }
    }

    pub fn is_sequence(&self) -> bool {
        self.document_roots.len() > 1
    }

    pub(super) fn document_root(&self, mut node: Index) -> Index {
        node = normalize_node(&self.flatjson, node);
        if let Some(root) = self.active_root_for(node) {
            return root;
        }
        while let Some(parent) = self.flatjson[node].parent.as_option() {
            node = parent;
        }
        node
    }

    pub(super) fn is_document_root(&self, node: Index) -> bool {
        self.is_sequence()
            && self
                .active_roots
                .contains(&normalize_node(&self.flatjson, node))
    }

    pub fn is_document_collapsed(&self, root: Index) -> bool {
        self.document_collapsed.contains(&self.document_root(root))
    }

    /// Resolve a root's structural header.
    pub fn document_header_absolute(&self, root: Index) -> usize {
        let root = self.document_root(root);
        self.layout.node(&self.flatjson, root).line
    }

    pub fn is_document_header(&self, logical_line: usize) -> bool {
        self.visible_line(logical_line)
            .is_some_and(|line| line.separator)
    }

    /// Whether a visible line is the source-less body row paired with a
    /// generated sequence document header.
    pub fn is_document_body(&self, logical_line: usize) -> bool {
        let Some(visible) = self.visible_line(logical_line) else {
            return false;
        };
        self.is_sequence()
            && !self.is_document_header(logical_line)
            && self.is_document_root(visible.owner)
    }

    pub(super) fn is_navigable_line(&self, logical_line: usize) -> bool {
        self.visible_line(logical_line)
            .is_some_and(|line| !line.separator || self.is_document_header(logical_line))
    }

    pub fn document_root_for_line(&self, logical_line: usize) -> Option<Index> {
        self.visible_line(logical_line)
            .filter(|line| line.separator)
            .map(|line| line.owner)
    }

    pub fn line_is_collapsible(&self, logical_line: usize) -> bool {
        let Some(visible) = self.visible_line(logical_line) else {
            return false;
        };
        if self.is_document_header(logical_line) {
            return true;
        }
        // Sequence roots have a separate document-header collapse control.
        // Their body has its own control only for a nonempty array or an
        // explicitly rendered keyed table.
        if self.is_document_root(visible.owner) {
            let node = self.layout.node(&self.flatjson, visible.owner);
            return (self.flatjson[visible.owner].is_array() && node.entry_count > 0)
                || (self
                    .layout
                    .analysis
                    .node(&self.flatjson, visible.owner)
                    .table
                    && node.collapsible);
        }
        self.layout.node(&self.flatjson, visible.owner).collapsible
    }

    pub(super) fn focused_document_header(&self) -> bool {
        self.is_document_root(self.focused_node)
            && self.is_document_header(self.focused_line_index())
    }

    /// Collapse state used by app-level search tracking and rendering.
    pub fn effective_collapsed(&self, node: Index) -> bool {
        self.flatjson[node].is_collapsed()
            || (self.is_sequence() && self.is_document_collapsed(node))
    }

    pub(super) fn visible_ancestor(&self, node: Index) -> Index {
        let mut current = normalize_node(&self.flatjson, node);
        if !self.contains_node(current) {
            return self.active_roots.first().copied().unwrap_or(current);
        }
        let mut visible = current;
        while let Some(parent) = self.effective_parent(current).as_option() {
            if self.flatjson[parent].is_collapsed() {
                visible = parent;
            }
            current = parent;
        }
        let root = self.active_root_for(current).unwrap_or(current);
        if self.is_document_collapsed(root) {
            root
        } else {
            visible
        }
    }

    pub(super) fn refresh_document_metadata(&mut self) {
        self.document_roots = self.active_roots.clone();
    }

    pub(super) fn rebuild_layout(&mut self) {
        let top_line = self.top_logical_line();
        let top_anchor = self.visible_line(top_line).map(|line| {
            let document_body =
                self.is_document_root(line.owner) && !self.is_document_header(top_line);
            (line.owner, document_body)
        });
        let focused_node = self.focused_node;
        let focused_document_body =
            self.is_document_root(focused_node) && !self.focused_document_header();
        self.layout.reflow(
            &self.flatjson,
            usize::from(self.dimensions.width),
            self.line_numbers,
            &self.expanded_arrays,
        );
        self.refresh_document_metadata();
        self.refresh_projection();
        self.focus(focused_node);
        if focused_document_body {
            self.restore_body_focus(focused_node);
        }
        if let Some(index) = top_anchor.and_then(|(node, body)| self.reflowed_top_index(node, body))
        {
            self.top_visible_line = index;
        }
        self.reset_frame_geometry();
        self.layout_generation = self.layout_generation.wrapping_add(1);
    }

    fn restore_body_focus(&mut self, node: Index) {
        if !self.is_document_root(node) {
            return;
        }
        let body = self.layout.node(&self.flatjson, node).body_line;
        if self.visible.visible_index(body).is_some() {
            self.absolute_anchor_line = body;
        }
    }

    fn reflowed_top_index(&self, node: Index, document_body: bool) -> Option<usize> {
        let top = self.visible_ancestor(node);
        let anchor = match (self.is_document_root(top), document_body) {
            (true, true) => self.layout.node(&self.flatjson, top).body_line,
            (true, false) => self.document_header_absolute(top),
            (false, _) => self.layout.node(&self.flatjson, top).line,
        };
        self.visible.visible_index(anchor)
    }

    pub(super) fn refresh_projection(&mut self) {
        let previous_top_absolute = self
            .visible_line(self.top_logical_line())
            .map(|line| line.absolute);
        self.focus_byte_range = None;
        self.visible = self
            .layout
            .project_with_documents(&self.flatjson, &self.document_collapsed);
        let recovered = self.visible_ancestor(self.focused_node);
        let anchor_visible = self
            .visible
            .visible_index(self.absolute_anchor_line)
            .is_some();
        if recovered != self.focused_node || !anchor_visible {
            self.focus(recovered);
        }
        self.top_visible_line = previous_top_absolute
            .and_then(|absolute| self.visible.visible_index(absolute))
            .unwrap_or_else(|| {
                self.top_visible_line
                    .min(self.visible.len().saturating_sub(1))
            });
        self.reset_frame_geometry();
        self.layout_generation = self.layout_generation.wrapping_add(1);
    }
}
