//! The interactive layout owns structure; rendered rows belong to its caller.
use super::geometry::{Geometry, Kind, Row};
use super::index::Analysis;
use super::{DisplayLine, FlatJson, OptionIndex, normalize_node};
use crate::chunked_vec::ChunkedVec;
use std::collections::HashSet;
use std::ops::Range;

pub struct Layout {
    pub analysis: Analysis,
    pub geometry: Geometry,
}

#[derive(Clone, Debug)]
pub struct NodeLayout {
    pub line: usize,
    pub body_line: usize,
    #[cfg(test)]
    pub body_extent: Range<usize>,
    pub collapsible: bool,
    pub entry_count: usize,
    pub inline_array: bool,
    pub table_row: bool,
    pub table_cell: bool,
    #[cfg(test)]
    pub descendant_warnings: usize,
    pub occurrence: Option<usize>,
    pub occurrence_total: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
pub struct VisibleLine {
    pub absolute: usize,
    pub owner: usize,
    pub separator: bool,
    pub collapsed: bool,
}

struct Collapsed {
    absolute: usize,
    visible: usize,
    end: usize,
    owner: usize,
}

/// Identity mapping when expanded; only collapsed intervals need storage.
pub struct Projection {
    total: usize,
    len: usize,
    collapsed: Vec<Collapsed>,
}

impl Projection {
    fn new(total: usize) -> Self {
        Self {
            total,
            len: total,
            collapsed: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    fn collapse(&mut self, absolute: usize, end: usize, owner: usize) {
        self.collapsed.push(Collapsed {
            absolute,
            end,
            owner,
            visible: absolute - (self.total - self.len),
        });
        self.len -= end - absolute - 1;
    }

    fn absolute(&self, visible: usize) -> Option<(usize, Option<usize>)> {
        if visible >= self.len {
            return None;
        }
        let preceding = self
            .collapsed
            .partition_point(|range| range.visible <= visible);
        let Some(range) = preceding.checked_sub(1).map(|i| &self.collapsed[i]) else {
            return Some((visible, None));
        };
        if range.visible == visible {
            Some((range.absolute, Some(range.owner)))
        } else {
            Some((visible + range.end - range.visible - 1, None))
        }
    }

    pub fn visible_index(&self, absolute: usize) -> Option<usize> {
        if absolute >= self.total {
            return None;
        }
        let preceding = self
            .collapsed
            .partition_point(|range| range.absolute <= absolute);
        let Some(range) = preceding.checked_sub(1).map(|i| &self.collapsed[i]) else {
            return Some(absolute);
        };
        if absolute == range.absolute {
            Some(range.visible)
        } else if absolute < range.end {
            None
        } else {
            Some(absolute - (range.end - range.visible - 1))
        }
    }
}

impl Layout {
    #[cfg(test)]
    pub fn formatted_rows() -> usize {
        super::format::FORMATTED_ROWS.with(std::cell::Cell::get)
    }

    pub fn new(
        flat: &FlatJson,
        roots: &[usize],
        width: usize,
        numbers: bool,
        expanded: &HashSet<usize>,
    ) -> Self {
        let analysis = Analysis::new(flat, roots);
        let geometry = Geometry::new(flat, &analysis, width, numbers, expanded);
        Self { analysis, geometry }
    }

    pub fn reflow(
        &mut self,
        flat: &FlatJson,
        width: usize,
        numbers: bool,
        expanded: &HashSet<usize>,
    ) {
        self.geometry = Geometry::new(flat, &self.analysis, width, numbers, expanded);
    }

    pub fn active_root_for(&self, flat: &FlatJson, node: usize) -> Option<usize> {
        self.analysis.root_for(normalize_node(flat, node))
    }

    pub fn node(&self, flat: &FlatJson, node: usize) -> NodeLayout {
        let node = normalize_node(flat, node);
        let info = self.analysis.nodes[node];
        let pos = self.geometry.positions[node];
        let root = self.analysis.is_root(node);
        let sequence = self.analysis.roots.len() > 1;
        let occurrence = self.analysis.occurrences.get(&node).copied();
        NodeLayout {
            line: pos.line,
            body_line: pos.body_line,
            #[cfg(test)]
            body_extent: pos.body_line..pos.end,
            collapsible: if sequence && root {
                true
            } else {
                info.child_count != 0 && !info.table_row && !(root && !flat[node].is_array())
            },
            entry_count: info.child_count,
            inline_array: pos.inline,
            table_row: info.table_row,
            table_cell: info.table_cell,
            #[cfg(test)]
            descendant_warnings: info.hidden_warnings,
            occurrence: occurrence.map(|pair| pair.0),
            occurrence_total: occurrence.map(|pair| pair.1),
        }
    }

    pub fn source_line(&self, flat: &FlatJson, node: usize, source: Option<usize>) -> usize {
        self.geometry
            .source_line(flat, &self.analysis, normalize_node(flat, node), source)
    }

    pub fn rows(&self) -> &ChunkedVec<Row> {
        &self.geometry.rows
    }

    pub fn project_with_documents(
        &self,
        flat: &FlatJson,
        documents: &HashSet<usize>,
    ) -> Projection {
        let mut visible = Projection::new(self.geometry.rows.len());
        if !flat.has_collapsed() && documents.is_empty() {
            return visible;
        }
        let mut absolute = 0;
        while let Some(&row) = self.geometry.rows.get(absolute) {
            let mut collapsed = None;
            if row.kind == Kind::Document {
                if documents.contains(&row.node) {
                    collapsed = Some(row.node);
                }
            } else {
                let mut node = row.node;
                loop {
                    if self.geometry.positions[node].body_line != absolute {
                        break;
                    }
                    if flat[node].is_collapsed() && self.node(flat, node).collapsible {
                        collapsed = Some(node);
                    }
                    if flat[node].index_in_parent != 0 || self.analysis.is_root(node) {
                        break;
                    }
                    let OptionIndex::Index(parent) = flat[node].parent else {
                        break;
                    };
                    node = parent;
                }
            }
            if let Some(node) = collapsed {
                visible.collapse(
                    absolute,
                    self.geometry.positions[node].end.max(absolute + 1),
                    node,
                );
            }
            absolute = collapsed.map_or(absolute + 1, |node| {
                self.geometry.positions[node].end.max(absolute + 1)
            });
        }
        visible
    }

    pub fn visible_line(&self, projection: &Projection, index: usize) -> Option<VisibleLine> {
        let (absolute, collapsed) = projection.absolute(index)?;
        let row = self.geometry.rows[absolute];
        Some(VisibleLine {
            absolute,
            owner: collapsed.unwrap_or(row.owner),
            separator: row.separator(),
            collapsed: collapsed.is_some(),
        })
    }

    pub fn render(&self, flat: &FlatJson, visible: VisibleLine, focused: usize) -> DisplayLine {
        super::format::row(
            flat,
            &self.analysis,
            &self.geometry,
            self.geometry.rows[visible.absolute],
            visible.collapsed.then_some(visible.owner),
            focused,
        )
    }

    pub fn highlight_shared_fields(
        &self,
        flat: &FlatJson,
        line: &mut DisplayLine,
        matches: &[Range<usize>],
        current: &Range<usize>,
    ) {
        super::format::highlight_shared_fields(flat, &self.analysis, line, matches, current);
    }
}
