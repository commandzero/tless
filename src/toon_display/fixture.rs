//! Test-only collectors exercise the same row formatter as the viewport.
use super::geometry::{Geometry, GeometryOptions};
use super::index::Analysis;
use super::layout::{Layout, NodeLayout};
use super::{DisplayLine, FlatJson, WarningKind};
use std::collections::HashSet;

pub struct Warning {
    pub kind: WarningKind,
}
pub struct FixtureRow {
    pub absolute: usize,
    pub line: DisplayLine,
}
pub struct Fixture {
    pub lines: Vec<DisplayLine>,
    pub nodes: Vec<NodeLayout>,
    pub warnings: Vec<Warning>,
    pub layout: Layout,
}
impl Fixture {
    pub fn canonical(flat: &FlatJson) -> Self {
        Self::build(
            flat,
            GeometryOptions {
                terminal_width: usize::MAX,
                numbers: false,
                inline_limit: usize::MAX,
            },
            &HashSet::new(),
            &crate::path_filter::document_roots(flat),
        )
    }
    pub fn for_view(flat: &FlatJson, width: usize, expanded: &HashSet<usize>) -> Self {
        Self::for_view_with_roots(
            flat,
            width,
            expanded,
            &crate::path_filter::document_roots(flat),
        )
    }
    pub fn for_view_with_roots(
        flat: &FlatJson,
        width: usize,
        expanded: &HashSet<usize>,
        roots: &[usize],
    ) -> Self {
        let options = GeometryOptions::new(width.saturating_add(2), false);
        Self::build(flat, options, expanded, roots)
    }
    fn build(
        flat: &FlatJson,
        options: GeometryOptions,
        expanded: &HashSet<usize>,
        roots: &[usize],
    ) -> Self {
        let analysis = Analysis::new(flat, roots);
        let geometry = Geometry::new(flat, &analysis, options, expanded);
        let layout = Layout { analysis, geometry };
        let lines = (0..layout.line_count())
            .map(|line| {
                let row = layout.row(flat, line).unwrap();
                super::format::Formatter::new(flat, &layout, row.owner, None).row(row, None)
            })
            .collect();
        let nodes = (0..flat.0.len())
            .map(|node| layout.node(flat, node))
            .collect();
        let warnings = (0..flat.0.len())
            .flat_map(|node| {
                layout
                    .analysis
                    .warnings(node)
                    .map(move |kind| Warning { kind })
            })
            .collect();
        Self {
            lines,
            nodes,
            warnings,
            layout,
        }
    }
    pub fn active_root_for(&self, node: usize) -> Option<usize> {
        self.layout.analysis.root_for(node)
    }
    pub fn project(&self, flat: &FlatJson) -> Vec<FixtureRow> {
        self.project_with_documents(flat, &HashSet::new())
    }
    pub fn project_with_documents(
        &self,
        flat: &FlatJson,
        documents: &HashSet<usize>,
    ) -> Vec<FixtureRow> {
        let projection = self.layout.project_with_documents(flat, documents);
        (0..projection.len())
            .map(|index| {
                let row = self.layout.visible_line(flat, &projection, index).unwrap();
                FixtureRow {
                    absolute: row.absolute,
                    line: self.layout.render(flat, row, row.owner, None),
                }
            })
            .collect()
    }
}
