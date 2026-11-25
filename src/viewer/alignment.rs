use super::JsonViewer;
use crate::flatjson::Index;
use crate::toon_display::alignment::TableMetrics;
use crate::toon_display::normalize_node;

impl JsonViewer {
    /// Resolve only tables which still belong to the active projection.
    pub fn focused_table(&self) -> Option<Index> {
        if self.is_document_header(self.focused_line_index()) {
            return None;
        }
        let node = normalize_node(&self.flatjson, self.focused_node);
        self.layout
            .analysis
            .table_owner(&self.flatjson, node)
            .filter(|&table| {
                self.active_root_for(node) == self.active_root_for(table)
                    && self.active_root_for(table).is_some()
            })
    }

    pub fn table_alignment_enabled(&self, table: Index) -> bool {
        self.aligned_tables.contains_key(&table)
    }

    pub fn toggle_table_alignment(&mut self) -> Option<Index> {
        let table = self.focused_table()?;
        if self.aligned_tables.remove(&table).is_none() {
            self.aligned_tables.insert(
                table,
                TableMetrics::measure(&self.flatjson, &self.layout.analysis, table),
            );
        }
        self.reset_frame_geometry();
        self.ensure_visible();
        Some(table)
    }

    /// A collapsed preview or generated sequence header never owns a grid.
    pub fn aligned_table_for_line(&self, logical: usize) -> Option<Index> {
        let line = self.visible_line(logical)?;
        if line.separator || line.collapsed {
            return None;
        }
        // Nested groups and leaves share their direct row's physical line.
        let table = self
            .layout
            .analysis
            .table_owner(&self.flatjson, line.descriptor.node)?;
        self.table_alignment_enabled(table).then_some(table)
    }

    /// The expanded header must actually be in the active projection before
    /// its geometry can be used to clamp a saved horizontal offset.
    pub fn aligned_table_header_visible(&self, table: Index) -> bool {
        if self.active_root_for(table).is_none() {
            return false;
        }
        let absolute = self.layout.node(&self.flatjson, table).body_line;
        self.visible
            .visible_index(absolute)
            .is_some_and(|logical| self.aligned_table_for_line(logical) == Some(table))
    }

    /// Unreduced terminal-cell extent, including the longest actual annotation.
    pub fn aligned_table_width(&self, table: Index) -> usize {
        self.aligned_tables.get(&table).map_or(0, |metrics| {
            metrics.extent(self.layout.geometry.table_field_start(
                &self.flatjson,
                &self.layout.analysis,
                table,
            ))
        })
    }

    pub fn table_indentation(&self, table: Index) -> usize {
        self.layout
            .geometry
            .table_indentation(&self.flatjson, &self.layout.analysis, table)
    }
}
