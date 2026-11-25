use super::{Cursor, JsonViewer};
use crate::flatjson::Index;
use crate::toon_display::normalize_node;

impl JsonViewer {
    pub fn focused_line_index(&self) -> usize {
        self.visible
            .visible_index(self.absolute_anchor_line)
            .unwrap_or(0)
    }

    pub(super) fn focused_cursor(&self) -> Cursor {
        let logical = self.focused_line_index();
        let line = self.row_text(logical);
        let byte = self
            .focus_byte_range
            .as_ref()
            .map(|range| range.start)
            .or_else(|| {
                line.spans
                    .iter()
                    .find(|span| span.node == self.focused_node && span.source.is_some())
                    .map(|span| span.range.start)
            });
        let continuation = byte
            .and_then(|byte| {
                self.row_iter(logical, &line)
                    .position(|row| row.bytes.contains(&byte))
            })
            .unwrap_or(0);
        Cursor {
            logical,
            continuation,
        }
    }

    pub fn focused_physical_row(&self) -> usize {
        let cursor = self.focused_cursor();
        if cursor.logical == self.top_visible_line {
            return cursor.continuation.saturating_sub(self.top_continuation);
        }
        self.physical_rows
            .iter()
            .position(|row| row.logical_line == cursor.logical)
            .map_or(0, |first| first + cursor.continuation)
    }

    #[cfg(test)]
    pub fn index_of_focused_node_on_screen(&self) -> u16 {
        self.focused_line_index()
            .saturating_sub(self.top_visible_line) as u16
    }

    pub(super) fn focus(&mut self, node: usize) {
        self.focus_byte_range = None;
        let requested = normalize_node(&self.flatjson, node);
        if !self.contains_node(requested) {
            return;
        }
        self.focused_node = self.visible_ancestor(requested);
        self.frame.clear();
        if !self.is_document_root(self.focused_node) {
            self.absolute_anchor_line = self.layout.node(&self.flatjson, self.focused_node).line;
            return;
        }
        let header = self.document_header_absolute(self.focused_node);
        self.absolute_anchor_line = self
            .collapsed_body_anchor(requested, header)
            .unwrap_or(header);
        if self.absolute_anchor_line == header {
            self.desired_depth = self.flatjson[self.focused_node].depth;
        }
    }

    fn collapsed_body_anchor(&self, requested: Index, header: usize) -> Option<usize> {
        if self.is_document_root(requested) {
            return None;
        }
        let node = self.focused_node;
        if !self.flatjson[node].is_collapsed() || self.is_document_collapsed(node) {
            return None;
        }
        let body = self.layout.node(&self.flatjson, node).body_line;
        (body != header).then_some(body)
    }

    pub(super) fn reveal(&mut self, node: usize, source: Option<usize>) {
        let node = normalize_node(&self.flatjson, node);
        if !self.contains_node(node) {
            return;
        }
        let mut current = node;
        let mut changed = false;
        let root = self.document_root(node);
        if self.is_document_collapsed(root) {
            self.document_collapsed.remove(&root);
            changed = true;
        }
        while let Some(parent) = self.effective_parent(current).as_option() {
            changed |= self.flatjson[parent].is_collapsed();
            self.flatjson.expand(parent);
            current = parent;
        }
        if changed {
            self.refresh_projection();
        }
        self.focus(node);
        if source.is_some() {
            self.absolute_anchor_line = self.layout.source_line(&self.flatjson, node, source);
        }
    }

    pub(super) fn vertical(&mut self, count: usize, down: bool) {
        let mut index = self.focused_line_index();
        for _ in 0..count {
            let next = if down {
                ((index + 1)..self.visible.len()).find(|&i| self.is_navigable_line(i))
            } else {
                (0..index).rev().find(|&i| self.is_navigable_line(i))
            };
            let Some(next) = next else { break };
            index = next;
        }
        self.focus_line(index, true);
    }

    pub(super) fn focus_line(&mut self, index: usize, retain_field: bool) {
        let line = self.visible_line(index).unwrap();
        let node = if retain_field {
            self.corresponding_table_cell(line.owner)
                .unwrap_or(line.owner)
        } else {
            line.owner
        };
        self.focus(node);
        // A sequence root owns both its structural header and its body lines.
        // Keep vertical/body selection on the body line while structural
        // focus continues to resolve to the document header.
        if self.is_document_root(line.owner) && !self.is_document_header(index) {
            self.absolute_anchor_line = self.visible_line(index).unwrap().absolute;
        }
    }

    fn corresponding_table_cell(&self, owner: Index) -> Option<Index> {
        let analysis = &self.layout.analysis;
        let flat = &self.flatjson;
        if !analysis.table_cell(flat, self.focused_node) {
            return None;
        }
        let source_row = analysis.table_row_owner(flat, self.focused_node)?;
        let target_row = analysis.table_row_owner(flat, owner)?;
        if analysis.table_owner(flat, source_row) != analysis.table_owner(flat, target_row)
            || !flat[target_row].is_expanded()
        {
            return None;
        }
        let ordinal = analysis
            .table_cells(flat, source_row)
            .position(|cell| cell == self.focused_node)?;
        analysis.table_cells(flat, target_row).nth(ordinal)
    }
}
