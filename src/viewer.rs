use crate::flatjson::{FlatJson, Index, OptionIndex};
use crate::toon_display::layout::{Layout, Projection, VisibleLine};
use crate::toon_display::{DisplayLine, normalize_node};
use crate::types::TTYDimensions;
use crate::wrapped_view::{PhysicalRow, rows};
use std::collections::HashSet;
use std::ops::Range;
#[cfg(test)]
use unicode_width::UnicodeWidthStr;

/// Parsed node focus is independent of its painted line (inline values and table cells).
pub struct JsonViewer {
    pub flatjson: FlatJson,
    pub layout: Layout,
    pub visible: Projection,
    /// Index into the visibility projection, not the parsed node list.
    pub top_visible_line: Index,
    /// Continuation ordinal within the top logical line, never a document-wide row index.
    pub top_continuation: usize,
    /// Physical rows and text for the current frame only.
    pub physical_rows: Vec<PhysicalRow>,
    frame: Vec<(usize, DisplayLine)>,
    /// Session-only optional wrapping state. It starts disabled.
    pub wrapping_enabled: bool,
    pub focused_node: Index,
    pub absolute_anchor_line: usize,
    jump_distance: Option<usize>,
    desired_depth: usize,
    pub dimensions: TTYDimensions,
    pub scrolloff_setting: u16,
    expanded_arrays: HashSet<usize>,
    /// Collapse state for sequence document rows. This is deliberately kept
    /// separate from `FlatJson` so a root's document row can be collapsed
    /// independently from a root array's TOON body layout.
    document_collapsed: HashSet<Index>,
    active_roots: Vec<Index>,
    document_roots: Vec<Index>,
    line_numbers: bool,
    wrap_width: usize,
    wrap_indentation: usize,
    pub layout_generation: usize,
    pub physical_generation: usize,
    focus_byte_range: Option<Range<usize>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Cursor {
    logical: usize,
    continuation: usize,
}

impl JsonViewer {
    pub fn visible_line(&self, index: usize) -> Option<VisibleLine> {
        self.layout.visible_line(&self.visible, index)
    }
    #[cfg(test)]
    pub fn new(flatjson: FlatJson) -> Self {
        let roots = crate::path_filter::document_roots(&flatjson);
        Self::with_roots(flatjson, roots, TTYDimensions::default(), true, false)
    }

    /// Construct directly from already-resolved active roots. This avoids
    /// briefly constructing a full excluded layout during startup filtering.
    pub fn with_roots(
        flatjson: FlatJson,
        roots: Vec<Index>,
        dimensions: TTYDimensions,
        line_numbers: bool,
        expand_roots: bool,
    ) -> Self {
        let mut flatjson = flatjson;
        if expand_roots {
            for &root in &roots {
                flatjson.expand(root);
            }
        }
        let layout =
            Self::layout_for_view(&flatjson, dimensions, line_numbers, &HashSet::new(), &roots);
        let visible = layout.project_with_documents(&flatjson, &HashSet::new());
        let initial_root = roots.first().copied().unwrap_or(0);
        let absolute_anchor_line = layout.node(&flatjson, initial_root).line;
        let wrap_width = layout.geometry.width;
        let mut viewer = Self {
            flatjson,
            layout,
            visible,
            top_visible_line: 0,
            top_continuation: 0,
            physical_rows: Vec::new(),
            frame: Vec::new(),
            wrapping_enabled: false,
            focused_node: initial_root,
            absolute_anchor_line,
            jump_distance: None,
            desired_depth: 0,
            dimensions,
            scrolloff_setting: 3,
            expanded_arrays: HashSet::new(),
            document_collapsed: HashSet::new(),
            active_roots: roots,
            document_roots: vec![],
            line_numbers,
            wrap_width,
            wrap_indentation: 0,
            layout_generation: 0,
            physical_generation: 0,
            focus_byte_range: None,
        };
        viewer.refresh_document_metadata();
        viewer.desired_depth = viewer
            .active_roots
            .first()
            .map(|root| viewer.flatjson[*root].depth)
            .unwrap_or(0);
        if viewer.is_sequence() {
            if let Some(root) = viewer.document_roots().first().copied() {
                viewer.absolute_anchor_line = viewer.document_header_absolute(root);
            }
        }
        viewer.refresh_frame();
        viewer
    }
    pub fn set_roots(&mut self, roots: Vec<Index>) {
        for &root in &roots {
            self.flatjson.expand(root);
            self.document_collapsed.remove(&root);
        }
        self.active_roots = roots;
        self.layout = Self::layout_for_view(
            &self.flatjson,
            self.dimensions,
            self.line_numbers,
            &self.expanded_arrays,
            &self.active_roots,
        );
        self.visible = self
            .layout
            .project_with_documents(&self.flatjson, &self.document_collapsed);
        self.refresh_document_metadata();
        self.focused_node = self.active_roots.first().copied().unwrap_or(0);
        self.absolute_anchor_line = self.layout.node(&self.flatjson, self.focused_node).line;
        if self.is_sequence() {
            if let Some(root) = self.document_roots.first().copied() {
                self.absolute_anchor_line = self.document_header_absolute(root);
            }
        }
        self.top_visible_line = 0;
        self.top_continuation = 0;
        self.jump_distance = None;
        self.desired_depth = self
            .active_roots
            .first()
            .map(|root| self.flatjson[*root].depth)
            .unwrap_or(0);
        self.focus_byte_range = None;
        self.reset_frame_geometry();
        self.layout_generation = self.layout_generation.wrapping_add(1);
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

    fn active_root_for(&self, node: Index) -> Option<Index> {
        self.layout.active_root_for(&self.flatjson, node)
    }

    /// Selected roots are parentless only for interaction; source ancestry is
    /// left untouched for paths and copy operations.
    fn effective_parent(&self, node: Index) -> OptionIndex {
        let node = normalize_node(&self.flatjson, node);
        if self.active_root_for(node) == Some(node) {
            OptionIndex::Nil
        } else if self.contains_node(node) {
            self.flatjson[node].parent
        } else {
            OptionIndex::Nil
        }
    }

    pub fn is_sequence(&self) -> bool {
        self.document_roots.len() > 1
    }

    fn document_root(&self, mut node: Index) -> Index {
        node = normalize_node(&self.flatjson, node);
        if let Some(root) = self.active_root_for(node) {
            return root;
        }
        while let OptionIndex::Index(parent) = self.flatjson[node].parent {
            node = parent;
        }
        node
    }

    fn is_document_root(&self, node: Index) -> bool {
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

    fn is_navigable_line(&self, logical_line: usize) -> bool {
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
        // Sequence roots have one structural collapse control on their header;
        // the standalone body line must not inherit that control, including
        // scalar and empty-root bodies.
        if self.is_document_root(visible.owner) {
            return self.flatjson[visible.owner].is_array()
                && self.layout.node(&self.flatjson, visible.owner).entry_count > 0;
        }
        self.layout.node(&self.flatjson, visible.owner).collapsible
    }

    fn focused_document_header(&self) -> bool {
        self.is_document_root(self.focused_node)
            && self.is_document_header(self.focused_line_index())
    }

    /// Collapse state used by app-level search tracking and rendering.
    pub fn effective_collapsed(&self, node: Index) -> bool {
        self.flatjson[node].is_collapsed()
            || (self.is_sequence() && self.is_document_collapsed(node))
    }

    fn visible_ancestor(&self, node: Index) -> Index {
        let mut current = normalize_node(&self.flatjson, node);
        if !self.contains_node(current) {
            return self.active_roots.first().copied().unwrap_or(current);
        }
        let mut visible = current;
        while let OptionIndex::Index(parent) = self.effective_parent(current) {
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

    fn refresh_document_metadata(&mut self) {
        self.document_roots = self.active_roots.clone();
    }

    fn wrap_eligible(&self, logical_line: usize) -> bool {
        let Some(visible) = self.visible_line(logical_line) else {
            return false;
        };
        !self.is_document_header(logical_line) && !self.effective_collapsed(visible.owner)
    }

    fn top_logical_line(&self) -> usize {
        self.top_visible_line
    }

    fn top_cursor(&self) -> Cursor {
        Cursor {
            logical: self.top_visible_line,
            continuation: self.top_continuation,
        }
    }

    fn set_top(&mut self, cursor: Cursor) {
        self.top_visible_line = cursor.logical;
        self.top_continuation = cursor.continuation;
    }

    pub fn render_line(&self, logical: usize) -> DisplayLine {
        self.layout.render(
            &self.flatjson,
            self.visible_line(logical).unwrap(),
            self.focused_node,
        )
    }

    pub fn rendered_line(&self, logical: usize) -> &DisplayLine {
        &self
            .frame
            .iter()
            .find(|(index, _)| *index == logical)
            .expect("requested row belongs to the frame or explicit focus target")
            .1
    }

    pub fn highlight_shared_fields(&mut self, matches: &[Range<usize>], current: &Range<usize>) {
        for (_, line) in &mut self.frame {
            self.layout
                .highlight_shared_fields(&self.flatjson, line, matches, current);
        }
    }

    fn row_text(&self, logical: usize) -> std::borrow::Cow<'_, DisplayLine> {
        match self.frame.iter().find(|(index, _)| *index == logical) {
            Some((_, line)) => std::borrow::Cow::Borrowed(line),
            None => std::borrow::Cow::Owned(self.render_line(logical)),
        }
    }

    fn row_iter<'a>(&self, logical: usize, line: &'a DisplayLine) -> crate::wrapped_view::Rows<'a> {
        rows(
            logical,
            &line.text,
            self.wrap_width,
            self.wrap_indentation,
            self.is_wrapped_line(logical),
        )
    }

    fn walk(&self, mut cursor: Cursor, mut count: usize, down: bool) -> Cursor {
        if !self.wrapping_enabled {
            cursor.logical = if down {
                cursor
                    .logical
                    .saturating_add(count)
                    .min(self.visible.len().saturating_sub(1))
            } else {
                cursor.logical.saturating_sub(count)
            };
            cursor.continuation = 0;
            return cursor;
        }
        while count > 0 {
            if !down {
                if count <= cursor.continuation {
                    cursor.continuation -= count;
                    break;
                }
                if cursor.logical == 0 {
                    return Cursor {
                        logical: 0,
                        continuation: 0,
                    };
                }
                count -= cursor.continuation + 1;
                cursor.logical -= 1;
                let line = self.row_text(cursor.logical);
                cursor.continuation = self
                    .row_iter(cursor.logical, &line)
                    .count()
                    .saturating_sub(1);
            } else {
                let line = self.row_text(cursor.logical);
                let available = self
                    .row_iter(cursor.logical, &line)
                    .skip(cursor.continuation)
                    .take(count.saturating_add(1))
                    .count();
                if available > count {
                    cursor.continuation += count;
                    break;
                }
                if cursor.logical + 1 >= self.visible.len() {
                    cursor.continuation += available.saturating_sub(1);
                    break;
                }
                count -= available;
                cursor.logical += 1;
                cursor.continuation = 0;
            }
        }
        cursor
    }

    fn refresh_frame(&mut self) {
        self.frame.clear();
        self.physical_rows.clear();
        self.top_visible_line = self
            .top_visible_line
            .min(self.visible.len().saturating_sub(1));
        if !self.wrapping_enabled {
            self.top_continuation = 0;
        }
        let height = usize::from(self.dimensions.height).max(1);
        for logical in self.top_visible_line..self.visible.len() {
            let line = self.render_line(logical);
            let skip = if logical == self.top_visible_line {
                self.top_continuation
            } else {
                0
            };
            let remaining = height - self.physical_rows.len();
            let physical = rows(
                logical,
                &line.text,
                self.wrap_width,
                self.wrap_indentation,
                self.is_wrapped_line(logical),
            )
            .skip(skip)
            .take(remaining);
            self.physical_rows.extend(physical);
            self.frame.push((logical, line));
            if self.physical_rows.len() == height {
                break;
            }
        }
        let focused = self.focused_line_index();
        if focused < self.visible.len() && !self.frame.iter().any(|(index, _)| *index == focused) {
            self.frame.push((focused, self.render_line(focused)));
        }
    }

    fn clamp_wrapped_top(&mut self) {
        self.refresh_frame();
        let height = usize::from(self.dimensions.height).max(1);
        if self.physical_rows.len() < height {
            let missing = height - self.physical_rows.len();
            self.set_top(self.walk(self.top_cursor(), missing, false));
            self.refresh_frame();
        }
    }

    fn reset_frame_geometry(&mut self) {
        self.top_continuation = 0;
        self.frame.clear();
        self.physical_generation = self.physical_generation.wrapping_add(1);
        self.refresh_frame();
    }

    /// The physical projection contains only the current viewport.
    pub fn screen_row(&self, row: usize) -> Option<&PhysicalRow> {
        self.physical_rows.get(row)
    }

    pub fn is_wrapped_line(&self, logical_line: usize) -> bool {
        self.wrapping_enabled && self.wrap_eligible(logical_line)
    }

    /// Update the document geometry used to split rows. Returns whether it
    /// changed the cached geometry and rebuilt the projection.
    pub fn set_wrap_geometry(&mut self, width: usize, indentation: usize) -> bool {
        if self.wrap_width == width && self.wrap_indentation == indentation {
            return false;
        }
        self.wrap_width = width;
        self.wrap_indentation = indentation;
        self.reset_frame_geometry();
        self.ensure_visible();
        true
    }

    /// Toggle wrapping for this interactive session and preserve the logical
    /// line at the top of the viewport.
    pub fn toggle_wrapping(&mut self) {
        self.wrapping_enabled = !self.wrapping_enabled;
        self.reset_frame_geometry();
        self.ensure_visible();
    }

    /// Reveal a display byte range in the currently focused logical line.
    /// Callers resolve source ranges through the line's mapped spans first.
    pub fn reveal_byte_range(&mut self, range: Range<usize>) {
        let logical = self.focused_line_index();
        let line = self.row_text(logical);
        if range.start > line.text.len() || range.end > line.text.len() {
            return;
        }
        let overlaps = |row: &PhysicalRow| {
            range.start < row.bytes.end && row.bytes.start < range.end
                || range.start == range.end
                    && row.bytes.start <= range.start
                    && range.start <= row.bytes.end
        };
        let mut matches = self
            .row_iter(logical, &line)
            .enumerate()
            .filter(|(_, row)| overlaps(row))
            .map(|(index, _)| index);
        let Some(first) = matches.next() else {
            return;
        };
        let last = matches.last().unwrap_or(first);
        drop(line);
        self.focus_byte_range = Some(range);
        let height = usize::from(self.dimensions.height).max(1);
        let padding = usize::from(self.scrolloff_setting).min((height - 1) / 2);
        let target = Cursor {
            logical,
            continuation: first,
        };
        let end = Cursor {
            logical,
            continuation: last,
        };
        if target < self.walk(self.top_cursor(), padding, true)
            || end >= self.walk(self.top_cursor(), height.saturating_sub(padding), true)
        {
            let mut top = self.walk(target, padding, false);
            if last - first < height && end >= self.walk(top, height, true) {
                top = self.walk(end, height - 1, false);
            }
            self.set_top(top);
            if self.wrapping_enabled {
                self.clamp_wrapped_top();
            }
        }
        self.refresh_frame();
    }

    pub fn set_viewport(&mut self, dimensions: TTYDimensions, line_numbers: bool) {
        let reflow = dimensions.width != self.dimensions.width || line_numbers != self.line_numbers;
        let height_changed = dimensions.height != self.dimensions.height;
        self.dimensions = dimensions;
        self.line_numbers = line_numbers;
        if reflow {
            self.rebuild_layout();
        }
        if reflow || height_changed {
            self.ensure_visible();
        }
    }

    fn layout_for_view(
        flat: &FlatJson,
        dimensions: TTYDimensions,
        line_numbers: bool,
        expanded_arrays: &HashSet<usize>,
        roots: &[usize],
    ) -> Layout {
        Layout::new(
            flat,
            roots,
            usize::from(dimensions.width),
            line_numbers,
            expanded_arrays,
        )
    }

    fn rebuild_layout(&mut self) {
        let top_line = self.visible_line(self.top_logical_line());
        let top_node = top_line.map(|line| line.owner);
        let top_document_body = top_line.is_some_and(|line| {
            self.is_document_root(line.owner) && !self.is_document_header(self.top_logical_line())
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
        if focused_document_body && self.is_document_root(focused_node) {
            let body = self.layout.node(&self.flatjson, focused_node).body_line;
            if self.visible.visible_index(body).is_some() {
                self.absolute_anchor_line = body;
            }
        }
        if let Some(top_node) = top_node {
            let top = self.visible_ancestor(top_node);
            let anchor = if top_document_body && self.is_document_root(top) {
                self.layout.node(&self.flatjson, top).body_line
            } else if self.is_document_root(top) {
                self.document_header_absolute(top)
            } else {
                self.layout.node(&self.flatjson, top).line
            };
            if let Some(index) = self.visible.visible_index(anchor) {
                self.top_visible_line = index;
            }
        }
        self.reset_frame_geometry();
        self.layout_generation = self.layout_generation.wrapping_add(1);
    }

    pub fn focused_line_index(&self) -> usize {
        self.visible
            .visible_index(self.absolute_anchor_line)
            .unwrap_or(0)
    }

    fn focused_cursor(&self) -> Cursor {
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

    fn focus(&mut self, node: usize) {
        self.focus_byte_range = None;
        let requested = normalize_node(&self.flatjson, node);
        if !self.contains_node(requested) {
            return;
        }
        self.focused_node = self.visible_ancestor(requested);
        self.frame.clear();
        if self.is_document_root(self.focused_node) {
            let header = self.document_header_absolute(self.focused_node);
            let body_anchor = self.flatjson[self.focused_node].is_collapsed()
                && !self.is_document_root(requested)
                && !self.is_document_collapsed(self.focused_node)
                && self
                    .layout
                    .node(&self.flatjson, self.focused_node)
                    .body_line
                    != header;
            self.absolute_anchor_line = if body_anchor {
                self.layout
                    .node(&self.flatjson, self.focused_node)
                    .body_line
            } else {
                header
            };
            if self.absolute_anchor_line == header {
                self.desired_depth = self.flatjson[self.focused_node].depth;
            }
        } else {
            self.absolute_anchor_line = self.layout.node(&self.flatjson, self.focused_node).line;
        }
    }

    fn reveal(&mut self, node: usize, source: Option<usize>) {
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
        while let OptionIndex::Index(parent) = self.effective_parent(current) {
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

    fn refresh_projection(&mut self) {
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

    fn ensure_visible(&mut self) {
        if self.wrapping_enabled {
            let cursor = self.focused_cursor();
            let height = usize::from(self.dimensions.height).max(1);
            let padding = usize::from(self.scrolloff_setting).min((height - 1) / 2);
            if cursor < self.walk(self.top_cursor(), padding, true) {
                self.set_top(self.walk(cursor, padding, false));
            } else if cursor >= self.walk(self.top_cursor(), height.saturating_sub(padding), true) {
                self.set_top(self.walk(cursor, height.saturating_sub(padding + 1), false));
            }
            self.clamp_wrapped_top();
            return;
        }
        let index = self.focused_line_index();
        let height = usize::from(self.dimensions.height).max(1);
        let padding = usize::from(self.scrolloff_setting).min((height - 1) / 2);
        if index < self.top_visible_line + padding {
            self.top_visible_line = index.saturating_sub(padding);
        } else if index >= self.top_visible_line + height - padding {
            self.top_visible_line = (index + padding + 1)
                .saturating_sub(height)
                .min(self.visible.len().saturating_sub(height));
        }
        self.refresh_frame();
    }

    fn vertical(&mut self, count: usize, down: bool) {
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

    fn focus_line(&mut self, index: usize, retain_field: bool) {
        let owner = self.visible_line(index).unwrap().owner;
        let mut node = owner;
        if retain_field
            && self
                .layout
                .node(&self.flatjson, self.focused_node)
                .table_cell
            && self.layout.node(&self.flatjson, owner).table_row
        {
            // Match the logical column ordinal, including escaped-equivalent keys.
            if let OptionIndex::Index(parent) = self.effective_parent(self.focused_node) {
                if self.flatjson[owner].is_expanded() {
                    let mut ordinal = 0;
                    let mut child = self.flatjson[parent].first_child();
                    while let OptionIndex::Index(candidate) = child {
                        if candidate == self.focused_node {
                            break;
                        }
                        ordinal += 1;
                        child = self.flatjson[candidate].next_sibling;
                    }
                    child = self.flatjson[owner].first_child();
                    for _ in 0..ordinal {
                        if let OptionIndex::Index(candidate) = child {
                            child = self.flatjson[candidate].next_sibling;
                        }
                    }
                    if let OptionIndex::Index(candidate) = child {
                        if self.layout.node(&self.flatjson, candidate).line
                            == self.visible_line(index).unwrap().absolute
                        {
                            node = candidate;
                        }
                    }
                }
            }
        }
        self.focus(node);
        // A sequence root owns both its structural header and its body lines.
        // Keep vertical/body selection on the body line while structural
        // focus continues to resolve to the document header.
        if self.is_document_root(owner) && !self.is_document_header(index) {
            self.absolute_anchor_line = self.visible_line(index).unwrap().absolute;
        }
    }

    fn parent(&mut self) {
        if let OptionIndex::Index(parent) = self.effective_parent(self.focused_node) {
            self.focus(parent);
        }
    }

    fn parent_or_previous_sibling(&mut self) {
        let current = normalize_node(&self.flatjson, self.focused_node);
        let destination = if let OptionIndex::Index(parent) = self.effective_parent(current) {
            OptionIndex::Index(parent)
        } else {
            self.active_root_for(current)
                .and_then(|root| {
                    self.active_roots
                        .iter()
                        .position(|candidate| *candidate == root)
                        .and_then(|position| position.checked_sub(1))
                        .and_then(|position| self.active_roots.get(position).copied())
                })
                .map_or(OptionIndex::Nil, OptionIndex::Index)
        };
        if let OptionIndex::Index(node) = destination {
            self.focus(node);
        }
    }

    fn next_sibling_in_view(&self, node: Index) -> OptionIndex {
        if let Some(position) = self.active_roots.iter().position(|&root| root == node) {
            self.active_roots
                .get(position + 1)
                .copied()
                .map_or(OptionIndex::Nil, OptionIndex::Index)
        } else if self.contains_node(node) {
            self.flatjson[node].next_sibling
        } else {
            OptionIndex::Nil
        }
    }

    fn next_at_parent_level(&mut self) {
        let current = normalize_node(&self.flatjson, self.focused_node);
        let destination = if let OptionIndex::Index(parent) = self.effective_parent(current) {
            let parent_next = self.next_sibling_in_view(parent);
            if let OptionIndex::Index(next) = parent_next {
                if self.contains_node(next) {
                    OptionIndex::Index(next)
                } else {
                    self.next_sibling_in_view(current)
                }
            } else {
                self.next_sibling_in_view(current)
            }
        } else {
            self.next_sibling_in_view(current)
        };
        if let OptionIndex::Index(node) = destination {
            if self.contains_node(node) {
                self.focus(node);
            }
        }
    }

    fn sibling(&mut self, count: usize, next: bool) {
        for _ in 0..count {
            let before = self.focused_node;
            let sibling = if let Some(root) = self.active_root_for(before) {
                if root == normalize_node(&self.flatjson, before) {
                    self.active_roots
                        .iter()
                        .position(|candidate| *candidate == root)
                        .and_then(|position| {
                            if next {
                                self.active_roots.get(position + 1).copied()
                            } else {
                                position
                                    .checked_sub(1)
                                    .and_then(|p| self.active_roots.get(p).copied())
                            }
                        })
                        .map_or(OptionIndex::Nil, OptionIndex::Index)
                } else if next {
                    self.flatjson[before].next_sibling
                } else {
                    self.flatjson[before].prev_sibling
                }
            } else {
                OptionIndex::Nil
            };
            if let OptionIndex::Index(mut node) = sibling {
                if !self.contains_node(node) {
                    break;
                }
                while self.flatjson[node].depth < self.desired_depth
                    && self.flatjson[node].is_expanded()
                    && self.contains_node(node)
                {
                    let child = if next {
                        self.flatjson[node].first_child()
                    } else if let OptionIndex::Index(pair) = self.flatjson[node].pair_index() {
                        self.flatjson[pair].last_child()
                    } else {
                        OptionIndex::Nil
                    };
                    if let OptionIndex::Index(child) = child {
                        if self.contains_node(child) {
                            node = child;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                self.focus(node);
            } else if next {
                break;
            } else {
                self.parent();
            }
            if self.focused_node == before {
                break;
            }
        }
    }

    fn collapse(&mut self, node: usize, collapsed: bool) {
        if self.is_document_root(node) && self.focused_document_header() {
            if collapsed {
                self.document_collapsed.insert(node);
            } else {
                self.document_collapsed.remove(&node);
            }
        } else if self.layout.node(&self.flatjson, node).collapsible {
            if collapsed {
                self.flatjson.collapse(node);
            } else {
                self.flatjson.expand(node);
            }
        }
    }

    fn toggle_collapsed(&mut self, node: usize) {
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

    fn collapse_siblings(&mut self, collapsed: bool, deep: bool) {
        let bulk_document_rows = self
            .active_root_for(self.focused_node)
            .is_some_and(|root| root == normalize_node(&self.flatjson, self.focused_node))
            && self.is_sequence();
        let siblings: Vec<_> = if bulk_document_rows {
            self.active_roots.clone()
        } else if let OptionIndex::Index(parent) = self.effective_parent(self.focused_node) {
            let mut siblings = Vec::new();
            let mut node = self.flatjson[parent].first_child();
            while let OptionIndex::Index(current) = node {
                if self.contains_node(current) {
                    siblings.push(current);
                }
                node = self.flatjson[current].next_sibling;
            }
            siblings
        } else {
            vec![self.focused_node]
        };
        for current in siblings {
            if bulk_document_rows && self.is_document_root(current) {
                if collapsed {
                    self.document_collapsed.insert(current);
                } else {
                    self.document_collapsed.remove(&current);
                }
            } else {
                self.collapse(current, collapsed);
            }
            if deep
                && self.is_document_root(current)
                && self.flatjson[current].is_array()
                && self.layout.node(&self.flatjson, current).entry_count > 0
            {
                if collapsed {
                    self.flatjson.collapse(current);
                } else {
                    self.flatjson.expand(current);
                }
            }
            if deep {
                if let OptionIndex::Index(end) = self.flatjson[current].pair_index() {
                    for descendant in current + 1..end {
                        if self.contains_node(descendant) {
                            self.collapse(descendant, collapsed);
                        }
                    }
                }
            }
        }
        self.refresh_projection();
    }

    fn scroll(&mut self, count: usize, down: bool) {
        if self.wrapping_enabled {
            self.set_top(self.walk(self.top_cursor(), count, down));
            self.clamp_wrapped_top();
            let focused = self.focused_line_index();
            if !self
                .physical_rows
                .iter()
                .any(|row| row.logical_line == focused)
            {
                if let Some(row) = self.physical_rows.first() {
                    self.focus_line(row.logical_line, true);
                }
            }
            self.refresh_frame();
            return;
        }
        self.top_visible_line = if down {
            self.top_visible_line
                .saturating_add(count)
                .min(self.visible.len() - 1)
        } else {
            self.top_visible_line.saturating_sub(count)
        };
        let height = usize::from(self.dimensions.height).max(1);
        let focus = self.focused_line_index();
        let padding = usize::from(self.scrolloff_setting).min((height - 1) / 2);
        let first = (self.top_visible_line + padding).min(self.visible.len() - 1);
        let last = (self.top_visible_line + height - padding - 1).min(self.visible.len() - 1);
        let index = focus.clamp(first, last.max(first));
        let index = (index..self.visible.len())
            .find(|&i| self.is_navigable_line(i))
            .unwrap_or(index);
        if index != focus {
            self.focus_line(index, true);
        }
    }

    fn move_until_depth_change(&mut self, down: bool) {
        let mut depth = self.flatjson[self.focused_node].depth;
        let mut moved = false;
        loop {
            let previous_node = self.focused_node;
            let previous_line = self.absolute_anchor_line;
            self.vertical(1, down);
            if self.absolute_anchor_line == previous_line {
                break;
            }
            let next_depth = self.flatjson[self.focused_node].depth;
            if next_depth != depth {
                if !down && !moved && next_depth > depth {
                    depth = next_depth;
                } else {
                    if moved && (down && next_depth > depth || !down) {
                        self.focus(previous_node);
                        self.absolute_anchor_line = previous_line;
                    }
                    break;
                }
            }
            moved = true;
        }
    }

    fn jump(&mut self, distance: usize, down: bool) {
        if self.wrapping_enabled {
            let previous_top = self.top_cursor();
            let screen_index = self.focused_physical_row();
            self.set_top(self.walk(previous_top, distance, down));
            self.clamp_wrapped_top();
            let focused = self.focused_line_index();
            if !self
                .physical_rows
                .iter()
                .any(|row| row.logical_line == focused)
            {
                if self.top_cursor() == previous_top {
                    self.vertical(distance, down);
                } else {
                    let index = screen_index.min(self.physical_rows.len().saturating_sub(1));
                    self.focus_line(self.physical_rows[index].logical_line, true);
                }
            }
            self.refresh_frame();
            return;
        }
        let previous_top = self.top_visible_line;
        let screen_index = self.focused_line_index().saturating_sub(previous_top);
        let height = usize::from(self.dimensions.height).max(1);
        self.top_visible_line = if down {
            previous_top
                .saturating_add(distance)
                .min(self.visible.len().saturating_sub(height))
                .max(previous_top)
        } else {
            previous_top.saturating_sub(distance)
        };
        if self.top_visible_line == previous_top {
            self.vertical(distance, down);
        } else {
            let index = (self.top_visible_line + screen_index).min(self.visible.len() - 1);
            let index = (index..self.visible.len())
                .find(|&i| self.is_navigable_line(i))
                .unwrap_or(index);
            self.focus_line(index, true);
        }
    }

    pub fn perform_action(&mut self, action: Action) {
        let mut track = true;
        match action {
            Action::NoOp => track = false,
            Action::MoveUp(n) => self.vertical(n, false),
            Action::MoveDown(n) => self.vertical(n, true),
            Action::MoveRight => {
                if self.is_document_root(self.focused_node) && self.focused_document_header() {
                    if self.is_document_collapsed(self.focused_node) {
                        self.collapse(self.focused_node, false);
                        self.refresh_projection();
                    } else if let OptionIndex::Index(child) =
                        self.flatjson[self.focused_node].first_child()
                    {
                        let body_collapsed = self.flatjson[self.focused_node].is_collapsed();
                        if body_collapsed {
                            self.flatjson.expand(self.focused_node);
                        }
                        if body_collapsed
                            && self
                                .layout
                                .node(&self.flatjson, self.focused_node)
                                .inline_array
                        {
                            self.expanded_arrays.insert(self.focused_node);
                            self.rebuild_layout();
                        } else if body_collapsed {
                            self.refresh_projection();
                        } else if self
                            .layout
                            .node(&self.flatjson, self.focused_node)
                            .inline_array
                        {
                            self.expanded_arrays.insert(self.focused_node);
                            self.rebuild_layout();
                        }
                        self.focus(child);
                    }
                } else if self.flatjson[self.focused_node].is_collapsed() {
                    self.collapse(self.focused_node, false);
                    if self
                        .layout
                        .node(&self.flatjson, self.focused_node)
                        .inline_array
                    {
                        self.expanded_arrays.insert(self.focused_node);
                        self.rebuild_layout();
                    } else {
                        self.refresh_projection();
                    }
                } else if self
                    .layout
                    .node(&self.flatjson, self.focused_node)
                    .inline_array
                {
                    self.expanded_arrays.insert(self.focused_node);
                    self.rebuild_layout();
                } else if let OptionIndex::Index(child) =
                    self.flatjson[self.focused_node].first_child()
                {
                    self.focus(child);
                }
            }
            Action::MoveLeft => {
                if self.is_document_root(self.focused_node) && self.focused_document_header() {
                    if !self.is_document_collapsed(self.focused_node) {
                        self.collapse(self.focused_node, true);
                        self.refresh_projection();
                    }
                } else if self.line_is_collapsible(self.focused_line_index())
                    && self
                        .layout
                        .node(&self.flatjson, self.focused_node)
                        .collapsible
                    && self.flatjson[self.focused_node].is_expanded()
                {
                    self.collapse(self.focused_node, true);
                    self.refresh_projection();
                } else {
                    self.parent();
                }
            }
            Action::FocusParent => self.parent(),
            Action::FocusParentOrPreviousSibling => self.parent_or_previous_sibling(),
            Action::FocusNextAtParentLevel => self.next_at_parent_level(),
            Action::FocusPrevSibling(n) => self.sibling(n, false),
            Action::FocusNextSibling(n) => self.sibling(n, true),
            Action::FocusFirstSibling | Action::FocusLastSibling => {
                let last = matches!(action, Action::FocusLastSibling);
                let node = if let Some(root) = self.active_root_for(self.focused_node) {
                    if normalize_node(&self.flatjson, self.focused_node) == root {
                        if last {
                            self.active_roots.last().copied()
                        } else {
                            self.active_roots.first().copied()
                        }
                    } else if let OptionIndex::Index(parent) =
                        self.effective_parent(self.focused_node)
                    {
                        if last {
                            match self.flatjson[parent].pair_index() {
                                OptionIndex::Index(pair) => {
                                    match self.flatjson[pair].last_child() {
                                        OptionIndex::Index(node) => Some(node),
                                        OptionIndex::Nil => None,
                                    }
                                }
                                OptionIndex::Nil => None,
                            }
                        } else {
                            match self.flatjson[parent].first_child() {
                                OptionIndex::Index(node) => Some(node),
                                OptionIndex::Nil => None,
                            }
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(node) = node.filter(|node| self.contains_node(*node)) {
                    self.focus(node);
                }
            }
            Action::FocusTop => {
                let root = self.document_roots().first().copied().unwrap_or(0);
                self.focus(root);
                if self.is_document_root(root) {
                    self.absolute_anchor_line = self.document_header_absolute(root);
                    self.desired_depth = self.flatjson[root].depth;
                }
                self.top_visible_line = 0;
                self.top_continuation = 0;
            }
            Action::FocusBottom => {
                self.focus_line(self.visible.len() - 1, false);
            }
            Action::MoveUpUntilDepthChange | Action::MoveDownUntilDepthChange => {
                self.move_until_depth_change(matches!(action, Action::MoveDownUntilDepthChange));
            }
            Action::JumpTo { line, make_visible } => {
                let line = line.min(self.layout.rows().len() - 1);
                let node = self.layout.rows()[line].owner;
                if make_visible {
                    let visible_index = self.visible.visible_index(line);
                    if visible_index.is_none() {
                        self.reveal(node, None);
                    }
                    if let Some(index) = visible_index.or_else(|| self.visible.visible_index(line))
                    {
                        self.focus_line(index, false);
                    }
                } else {
                    if let Some(index) = self.visible.visible_index(line) {
                        self.focus_line(index, false);
                    } else {
                        self.focus(node);
                    }
                }
            }
            Action::FocusNode { node, source } => self.reveal(node, source),
            Action::FocusNodeAt {
                node,
                source,
                display_range,
            } => {
                self.reveal(node, source);
                self.focus_byte_range = Some(display_range.0..display_range.1);
            }
            Action::ScrollUp(n) => {
                self.scroll(n, false);
                track = false;
            }
            Action::ScrollDown(n) => {
                self.scroll(n, true);
                track = false;
            }
            Action::PageUp(n) => {
                self.scroll(usize::from(self.dimensions.height).saturating_mul(n), false);
                track = false;
            }
            Action::PageDown(n) => {
                self.scroll(usize::from(self.dimensions.height).saturating_mul(n), true);
                track = false;
            }
            Action::JumpUp(n) | Action::JumpDown(n) => {
                self.jump_distance = n.or(self.jump_distance);
                let distance = self
                    .jump_distance
                    .unwrap_or((usize::from(self.dimensions.height) / 2).max(1));
                let down = matches!(action, Action::JumpDown(_));
                self.jump(distance, down);
                track = false;
            }
            Action::MoveFocusedLineToTop
            | Action::MoveFocusedLineToCenter
            | Action::MoveFocusedLineToBottom => {
                let height = usize::from(self.dimensions.height).max(1);
                let padding = match action {
                    Action::MoveFocusedLineToTop => {
                        usize::from(self.scrolloff_setting).min((height - 1) / 2)
                    }
                    Action::MoveFocusedLineToCenter => height / 2,
                    _ => height - 1,
                };
                if self.wrapping_enabled {
                    self.set_top(self.walk(self.focused_cursor(), padding, false));
                    self.clamp_wrapped_top();
                } else {
                    self.top_visible_line = self.focused_line_index().saturating_sub(padding);
                }
                track = false;
            }
            Action::ClickArrow(row) => {
                let Some(physical) = self.screen_row(usize::from(row.saturating_sub(1))) else {
                    return;
                };
                if !physical.first {
                    return;
                }
                if !self.line_is_collapsible(physical.logical_line) {
                    return;
                }
                let is_header = self.is_document_header(physical.logical_line);
                let node = self
                    .document_root_for_line(physical.logical_line)
                    .unwrap_or(self.visible_line(physical.logical_line).unwrap().owner);
                if is_header {
                    self.focus(node);
                } else {
                    self.focus_line(physical.logical_line, false);
                }
                self.toggle_collapsed(node);
            }
            Action::ToggleCollapsed => {
                self.toggle_collapsed(self.focused_node);
            }
            Action::CollapseNodeAndSiblings => self.collapse_siblings(true, false),
            Action::DeepCollapseNodeAndSiblings => self.collapse_siblings(true, true),
            Action::ExpandNodeAndSiblings => self.collapse_siblings(false, false),
            Action::DeepExpandNodeAndSiblings => self.collapse_siblings(false, true),
            Action::ResizeViewerDimensions(dimensions) => {
                self.set_viewport(dimensions, self.line_numbers)
            }
        }
        if !matches!(
            action,
            Action::FocusPrevSibling(_)
                | Action::FocusNextSibling(_)
                | Action::NoOp
                | Action::ScrollUp(_)
                | Action::ScrollDown(_)
                | Action::MoveFocusedLineToTop
                | Action::MoveFocusedLineToCenter
                | Action::MoveFocusedLineToBottom
                | Action::ResizeViewerDimensions(_)
        ) {
            self.desired_depth = self.flatjson[self.focused_node].depth;
        }
        if track {
            self.ensure_visible();
        } else {
            self.refresh_frame();
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Action {
    // Does nothing, for debugging, shouldn't modify any state.
    #[allow(dead_code)]
    NoOp,

    MoveUp(usize),
    MoveDown(usize),
    MoveLeft,
    MoveRight,

    // TODO: Come up with better names for these. Their behavior is
    // a little subtle. When moving down it'll move forward until
    // the depth changes. If the depth increases (because it got to
    // an expanded container) it'll stop on the line of the opening
    // of the container, but if the depth decreases (because we moved
    // past the last child of the current container) it'll focus the
    // line after.
    MoveUpUntilDepthChange,
    MoveDownUntilDepthChange,

    FocusParent,
    FocusParentOrPreviousSibling,
    FocusNextAtParentLevel,

    // The behavior of these is subtle and stateful. These move to the
    // previous/next sibling of the focused element. If we are focused
    // on the first/last child, we will move to the parent, but we
    // will remember what depth we were at when we first performed
    // this action, and move back to that depth the next time we can.
    FocusPrevSibling(usize),
    FocusNextSibling(usize),

    FocusFirstSibling,
    FocusLastSibling,
    FocusTop,
    FocusBottom,

    ScrollUp(usize),
    ScrollDown(usize),

    // By default, these move by half a screen, and move the focus by
    // the same number of lines, so the focus doesn't appear to move
    // on the screen. When jumping down, it will not show lines past
    // the end of the file.
    //
    // When a count is provided, we'll move by that many *lines* (not
    // N half screen sizes). This count is stored in
    // JsonViewer.jump_distance and used for subsequent jumps, rather
    // than half a screen size.
    //
    // vim always moves both the viewing window and the focused line
    // by the appropriate lines, so the location of the focused line
    // on the screen will move when jumping past the end of the file
    // (or before the start).
    //
    // We'll implement a slight variation on this behavior. If the
    // viewing window moves, we'll keep the focused line in the same
    // vertical location, but once we're at the top of the file, and
    // the viewing window doesn't change at all, then we will change
    // the focused line by the expected count.
    //
    // These commands ignore the scrolloff option.
    JumpUp(Option<usize>),
    JumpDown(Option<usize>),

    JumpTo {
        line: Index,
        make_visible: bool,
    },

    FocusNode {
        node: Index,
        source: Option<usize>,
    },
    FocusNodeAt {
        node: Index,
        source: Option<usize>,
        display_range: (usize, usize),
    },

    PageUp(usize),
    PageDown(usize),

    MoveFocusedLineToTop,
    MoveFocusedLineToCenter,
    MoveFocusedLineToBottom,

    ClickArrow(u16),

    ToggleCollapsed,
    CollapseNodeAndSiblings,
    DeepCollapseNodeAndSiblings,
    ExpandNodeAndSiblings,
    DeepExpandNodeAndSiblings,

    ResizeViewerDimensions(TTYDimensions),
}

#[cfg(test)]
impl OptionIndex {
    pub fn as_usize(&self) -> usize {
        match self {
            Self::Nil => crate::flatjson::NIL,
            Self::Index(i) => *i,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flatjson::{PathType, parse_top_level_json, parse_top_level_yaml};
    fn viewer(input: &str) -> JsonViewer {
        JsonViewer::new(parse_top_level_json(input.into()).unwrap())
    }
    fn path(v: &JsonViewer) -> String {
        v.flatjson
            .build_path_to_node(PathType::Dot, v.focused_node)
            .unwrap()
    }

    #[test]
    fn sequence_document_rows_are_structural_navigation_anchors() {
        let mut v = viewer(r#"{"a":1} {"b":2}"#);
        let roots = v.document_roots().to_vec();
        assert_eq!(roots.len(), 2);
        assert_eq!(v.focused_node, roots[0]);
        assert_eq!(v.absolute_anchor_line, v.document_header_absolute(roots[0]));

        // Enter the first document, then move back to its header and across
        // the root sibling. The parsed root identities remain unchanged.
        v.perform_action(Action::MoveRight);
        assert_eq!(path(&v), ".a");
        v.perform_action(Action::FocusParentOrPreviousSibling);
        assert_eq!(v.focused_node, roots[0]);
        assert_eq!(v.absolute_anchor_line, v.document_header_absolute(roots[0]));
        v.perform_action(Action::FocusNextAtParentLevel);
        assert_eq!(v.focused_node, roots[1]);
        assert_eq!(v.absolute_anchor_line, v.document_header_absolute(roots[1]));
    }

    #[test]
    fn sibling_motion_from_a_document_row_does_not_descend_by_stale_depth() {
        let mut v = viewer(r#"{"a":1} {"b":2}"#);
        let roots = v.document_roots().to_vec();

        v.perform_action(Action::MoveRight);
        v.perform_action(Action::FocusPrevSibling(1));
        assert_eq!(v.focused_node, roots[0]);
        v.perform_action(Action::FocusNextSibling(1));
        assert_eq!(v.focused_node, roots[1]);
        assert_eq!(v.absolute_anchor_line, v.document_header_absolute(roots[1]));
    }

    #[test]
    fn sequence_document_collapse_is_independent_of_root_array_body_state() {
        let mut v = viewer("[1,2] {\"tail\":3}");
        let roots = v.document_roots().to_vec();
        let first = roots[0];
        assert!(v.layout.node(&v.flatjson, first).inline_array);

        // Expanding the root array's body is separate from collapsing its
        // sequence row and must not change the document state.
        v.perform_action(Action::MoveRight);
        assert_ne!(v.focused_node, first);
        v.perform_action(Action::FocusParent);
        assert_eq!(v.focused_node, first);
        v.perform_action(Action::MoveDown(1));
        assert_eq!(v.focused_node, first);
        assert!(!v.focused_document_header());
        v.perform_action(Action::ToggleCollapsed);
        assert!(v.flatjson[first].is_collapsed());
        assert!(!v.is_document_collapsed(first));
        v.perform_action(Action::MoveRight);
        assert!(!v.flatjson[first].is_collapsed());
        assert!(!v.focused_document_header());
        assert_eq!(
            v.absolute_anchor_line,
            v.layout.node(&v.flatjson, first).body_line,
            "reopening a root array body preserves its body anchor"
        );
        v.perform_action(Action::FocusTop);
        assert!(!v.is_document_collapsed(first));
        v.perform_action(Action::ToggleCollapsed);
        assert!(v.is_document_collapsed(first));
        assert_eq!(v.focused_node, first);
        assert_eq!(v.focused_line_index(), 0);

        v.perform_action(Action::MoveRight);
        assert!(!v.is_document_collapsed(first));
        assert_eq!(v.focused_node, first);
        v.perform_action(Action::MoveRight);
        assert_eq!(path(&v), "[0]");
    }

    #[test]
    fn scalar_sequence_search_reveal_keeps_body_line_anchor() {
        let mut v = viewer("1 2");
        let roots = v.document_roots().to_vec();
        let second = roots[1];
        v.perform_action(Action::ToggleCollapsed);
        assert!(v.is_document_collapsed(roots[0]));
        v.perform_action(Action::FocusNode {
            node: second,
            source: Some(v.flatjson[second].range.start),
        });
        assert!(!v.is_document_collapsed(second));
        assert_eq!(v.focused_node, second);
        assert_ne!(v.absolute_anchor_line, v.document_header_absolute(second));
        v.perform_action(Action::ResizeViewerDimensions(TTYDimensions {
            width: 24,
            height: 8,
        }));
        assert_eq!(v.focused_node, second);
        assert!(!v.focused_document_header());
        assert_eq!(
            v.absolute_anchor_line,
            v.layout.node(&v.flatjson, second).body_line,
            "resizing preserves a scalar root body anchor"
        );
    }

    #[test]
    fn root_bulk_collapse_works_from_a_scalar_body_anchor() {
        for action in [
            Action::CollapseNodeAndSiblings,
            Action::DeepCollapseNodeAndSiblings,
        ] {
            let mut v = viewer("1 2 3");
            let roots = v.document_roots().to_vec();
            v.perform_action(Action::MoveDown(1));
            assert_eq!(v.focused_node, roots[0]);
            assert!(!v.focused_document_header());
            v.perform_action(action);
            assert!(roots.iter().all(|&root| v.is_document_collapsed(root)));
            assert_eq!(v.focused_line_index(), 0);
            assert_eq!(
                v.absolute_anchor_line,
                v.document_header_absolute(roots[0]),
                "collapsing the document rows recovers the hidden body anchor"
            );
        }
    }

    #[test]
    fn scalar_and_empty_sequence_bodies_do_not_mutate_parsed_collapse() {
        let mut v = viewer("1 {}");
        let first = v.document_roots()[0];
        v.perform_action(Action::MoveDown(1));
        assert!(!v.focused_document_header());

        v.perform_action(Action::ToggleCollapsed);
        v.perform_action(Action::MoveLeft);
        assert!(!v.flatjson[first].is_collapsed());
        assert!(!v.is_document_collapsed(first));
        assert_eq!(v.focused_node, first);
    }

    #[test]
    fn collapsed_header_jump_selects_without_revealing_document() {
        let mut v = viewer("1 2");
        let roots = v.document_roots().to_vec();
        let first_header = v.document_header_absolute(roots[0]);
        v.perform_action(Action::ToggleCollapsed);
        assert!(v.is_document_collapsed(roots[0]));

        // This is the action emitted for a click on header text or its
        // collapsed preview. Search uses FocusNode and still reveals.
        v.perform_action(Action::JumpTo {
            line: first_header,
            make_visible: false,
        });
        assert!(v.is_document_collapsed(roots[0]));
        assert_eq!(v.focused_node, roots[0]);
        assert_eq!(v.focused_line_index(), 0);

        v.perform_action(Action::JumpTo {
            line: first_header,
            make_visible: true,
        });
        assert!(v.is_document_collapsed(roots[0]));
        assert_eq!(v.focused_line_index(), 0);
    }

    #[test]
    fn header_right_reopens_a_collapsed_root_array_body() {
        let mut v = viewer("[1,2] {}");
        let first = v.document_roots()[0];

        v.perform_action(Action::MoveRight);
        v.perform_action(Action::FocusParent);
        v.perform_action(Action::MoveDown(1));
        v.perform_action(Action::ToggleCollapsed);
        assert!(v.flatjson[first].is_collapsed());

        v.perform_action(Action::FocusTop);
        assert!(v.focused_document_header());
        v.perform_action(Action::MoveRight);
        assert!(!v.flatjson[first].is_collapsed());
        assert_eq!(path(&v), "[0]");
    }

    #[test]
    fn structural_root_focus_stays_on_document_header_after_body_collapse() {
        let mut v = viewer("[1,2] {}");
        let root = v.document_roots()[0];

        v.perform_action(Action::MoveRight);
        v.perform_action(Action::FocusParent);
        v.perform_action(Action::MoveDown(1));
        v.perform_action(Action::ToggleCollapsed);
        assert!(v.flatjson[root].is_collapsed());
        assert!(!v.is_document_collapsed(root));

        // Clicking the generated row's arrow must target the document row,
        // while retaining the parsed body collapse state.
        v.perform_action(Action::ClickArrow(1));
        assert!(v.is_document_collapsed(root));
        assert!(v.flatjson[root].is_collapsed());
        assert!(v.focused_document_header());
    }

    #[test]
    fn deep_document_expand_restores_root_body_state() {
        let mut v = viewer("[1,2] {} 3");
        let roots = v.document_roots().to_vec();
        let first = roots[0];

        // Collapse the array body independently, then use a deep operation
        // from the structural header to restore every root's body state.
        v.perform_action(Action::MoveRight);
        v.perform_action(Action::FocusParent);
        v.perform_action(Action::MoveDown(1));
        assert_eq!(v.focused_node, first);
        assert!(!v.focused_document_header());
        v.perform_action(Action::ToggleCollapsed);
        assert!(v.flatjson[first].is_collapsed());
        v.perform_action(Action::FocusTop);
        v.perform_action(Action::DeepExpandNodeAndSiblings);
        assert!(roots.iter().all(|&root| !v.is_document_collapsed(root)));
        assert!(roots.iter().all(|&root| !v.flatjson[root].is_collapsed()));
    }

    #[test]
    fn shallow_document_expand_does_not_leave_object_root_body_collapsed() {
        let mut v = viewer(r#"{"a":1,"nested":{"b":2}} 7"#);
        let roots = v.document_roots().to_vec();
        let root = roots[0];
        let first = v.flatjson[root].first_child().unwrap();
        let nested = v.flatjson[first].next_sibling.unwrap();

        v.perform_action(Action::DeepCollapseNodeAndSiblings);
        assert!(v.is_document_collapsed(root));
        assert!(!v.flatjson[root].is_collapsed());
        assert!(v.flatjson[nested].is_collapsed());

        v.perform_action(Action::ExpandNodeAndSiblings);
        assert!(!v.is_document_collapsed(root));
        v.perform_action(Action::MoveRight);
        assert_eq!(path(&v), ".a");
        assert!(v.flatjson[nested].is_collapsed());
    }
    fn act(v: &mut JsonViewer, actions: &[Action]) {
        for &action in actions {
            v.perform_action(action);
        }
    }

    #[test]
    fn inline_array_controls_expand_and_table_rows_do_not_collapse() {
        for action in [Action::ToggleCollapsed, Action::ClickArrow(1)] {
            let mut v = viewer("[1,2]");
            assert!(v.layout.node(&v.flatjson, 0).inline_array);
            v.perform_action(action);
            assert!(!v.layout.node(&v.flatjson, 0).inline_array);
            assert!(v.flatjson[0].is_expanded());
        }
        let mut v = viewer(r#"[{"a":1},{"a":2}]"#);
        v.perform_action(Action::MoveRight);
        let row = v.focused_node;
        let original = v.render_line(v.focused_line_index()).text;
        v.perform_action(Action::ToggleCollapsed);
        assert!(!v.layout.node(&v.flatjson, row).collapsible);
        assert!(v.flatjson[row].is_expanded());
        assert_eq!(v.render_line(v.focused_line_index()).text, original);
        v.perform_action(Action::FocusParent);
        v.perform_action(Action::ToggleCollapsed);
        assert!(v.flatjson[0].is_collapsed());
    }

    #[test]
    fn right_expands_inline_array_before_entering_elements() {
        let mut v = viewer("[1,2]");
        assert_eq!(v.visible.len(), 1);
        v.perform_action(Action::MoveRight);
        assert_eq!(v.focused_node, 0);
        assert_eq!(
            (0..v.visible.len())
                .map(|line| v.render_line(line).text)
                .collect::<Vec<_>>(),
            vec!["[2]:", "  - 1", "  - 2"]
        );
        v.perform_action(Action::MoveRight);
        assert_eq!(path(&v), "[0]");
    }

    #[test]
    fn arrays_default_to_multiline_above_five_elements() {
        assert_eq!(viewer("[1,2,3,4,5]").visible.len(), 1);
        let v = viewer("[1,2,3,4,5,6]");
        assert_eq!(v.visible.len(), 7);
        assert_eq!(v.render_line(6).text, "  - 6");
    }

    #[test]
    fn initial_geometry_respects_unicode_fit_with_and_without_numbers() {
        for (width, numbers, inline) in [
            (18, true, true),
            (17, true, false),
            (15, false, true),
            (14, false, false),
        ] {
            let flat = parse_top_level_json(r#"{"tags":["界","é"]}"#.into()).unwrap();
            let v = JsonViewer::with_roots(
                flat,
                vec![0],
                TTYDimensions { width, height: 24 },
                numbers,
                false,
            );
            let lines: Vec<_> = (0..v.visible.len())
                .map(|line| v.render_line(line).text)
                .collect();
            let expected = if inline {
                vec!["tags[2]: 界,é"]
            } else {
                vec!["tags[2]:", "  - 界", "  - é"]
            };
            assert_eq!(lines, expected, "width={width}, numbers={numbers}");
        }
    }

    #[test]
    fn inline_array_fit_includes_gutters_and_terminal_cells() {
        let mut v = viewer(r#"{"tags":["界","é"]}"#);
        // tags[2]: 界,é occupies 13 cells, plus five gutter cells.
        v.perform_action(Action::ResizeViewerDimensions(TTYDimensions {
            width: 18,
            height: 24,
        }));
        assert_eq!(v.visible.len(), 1);
        v.perform_action(Action::ResizeViewerDimensions(TTYDimensions {
            width: 17,
            height: 24,
        }));
        assert_eq!(v.visible.len(), 3);
        assert_eq!(v.render_line(1).text, "  - 界");
        v.perform_action(Action::ResizeViewerDimensions(TTYDimensions {
            width: 18,
            height: 24,
        }));
        assert_eq!(v.visible.len(), 1);
    }

    #[test]
    fn explicit_array_expansion_survives_resize_collapse_and_search() {
        let mut v = viewer(r#"{"box":{"tags":["rust","cli"]},"tail":0}"#);
        act(
            &mut v,
            &[Action::MoveRight, Action::MoveRight, Action::MoveRight],
        );
        assert_eq!(path(&v), ".box.tags");
        assert_eq!(v.render_line(2).text, "    - rust");
        let array = v.focused_node;
        act(&mut v, &[Action::MoveLeft, Action::MoveRight]);
        assert_eq!(v.focused_node, array);
        assert!(!v.layout.node(&v.flatjson, array).inline_array);
        let child = v.flatjson[array].first_child().unwrap();
        let second = v.flatjson[child].next_sibling.unwrap();
        v.perform_action(Action::MoveLeft);
        v.perform_action(Action::FocusNode {
            node: second,
            source: Some(v.flatjson[second].range.start),
        });
        assert_eq!(path(&v), ".box.tags[1]");
        assert_eq!(v.render_line(v.focused_line_index()).text, "    - cli");
        v.set_viewport(
            TTYDimensions {
                width: 200,
                height: 24,
            },
            false,
        );
        assert_eq!(v.focused_node, second);
        assert!(!v.layout.node(&v.flatjson, array).inline_array);
        assert_eq!(&v.flatjson.1[v.flatjson[second].range.clone()], "\"cli\"");
    }

    #[test]
    fn array_fit_rechecks_line_number_digits_and_number_visibility() {
        let mut fields = vec!["\"wide\":[1,2,3,4,5,6]".to_string()];
        fields.extend((0..94).map(|i| format!("\"k{i}\":0")));
        fields.push("\"tags\":[\"界\",\"é\"]".into());
        let flat = parse_top_level_json(format!("{{{}}}", fields.join(","))).unwrap();
        let mut v = JsonViewer::with_roots(
            flat,
            vec![0],
            TTYDimensions {
                width: 18,
                height: 24,
            },
            true,
            false,
        );
        assert!(v.layout.rows().len() > 99);
        assert_eq!(v.render_line(v.visible.len() - 1).text, "  - é");
        v.set_viewport(
            TTYDimensions {
                width: 18,
                height: 24,
            },
            false,
        );
        assert_eq!(v.render_line(v.visible.len() - 1).text, "tags[2]: 界,é");
        v.set_viewport(
            TTYDimensions {
                width: 19,
                height: 24,
            },
            true,
        );
        assert_eq!(v.render_line(v.visible.len() - 1).text, "tags[2]: 界,é");
    }

    #[test]
    fn inline_array_warning_width_is_included_without_losing_warnings() {
        let mut v = JsonViewer::new(parse_top_level_yaml("[.inf, 1]".into()).unwrap());
        assert_eq!(v.visible.len(), 1);
        v.set_viewport(
            TTYDimensions {
                width: 25,
                height: 24,
            },
            true,
        );
        assert_eq!(v.visible.len(), 3);
        assert_eq!(
            v.render_line(1).text,
            "  - .inf  # WARN Non-finite number at [0]"
        );
        v.perform_action(Action::MoveLeft);
        assert!(v.render_line(0).text.contains("Contains 1 hidden warnings"));
    }

    #[test]
    fn right_opens_collapsed_inline_arrays_as_multiline() {
        let mut v = viewer("[1,2]");
        act(&mut v, &[Action::MoveLeft, Action::MoveRight]);
        assert_eq!(v.focused_node, 0);
        assert_eq!(v.visible.len(), 3);
        v.perform_action(Action::ClickArrow(1));
        assert_eq!(v.visible.len(), 1);
        v.perform_action(Action::ClickArrow(1));
        assert_eq!(v.visible.len(), 3);
    }

    #[test]
    fn inline_values_retain_individual_identity_and_copy_ranges() {
        let mut v = viewer(r#"{"tags":["rust","cli"],"done":true}"#);
        v.perform_action(Action::MoveRight);
        let child = v.flatjson[v.focused_node].first_child().unwrap();
        v.perform_action(Action::FocusNode {
            node: child,
            source: None,
        });
        assert_eq!(path(&v), ".tags[0]");
        let line = v.absolute_anchor_line;
        v.perform_action(Action::FocusNextSibling(1));
        assert_eq!(path(&v), ".tags[1]");
        assert_eq!(
            &v.flatjson.1[v.flatjson[v.focused_node].range.clone()],
            "\"cli\""
        );
        assert_eq!(line, v.absolute_anchor_line);
        v.perform_action(Action::MoveDown(1));
        assert_eq!(path(&v), ".done");
        assert_eq!(v.absolute_anchor_line, line + 1);
        v.perform_action(Action::MoveUp(1));
        assert_eq!(path(&v), ".tags");
    }

    #[test]
    fn table_cells_keep_columns_and_parent_structure() {
        let mut v = viewer(r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"\u006eame":"Lin"}]}"#);
        act(
            &mut v,
            &[
                Action::MoveRight,
                Action::MoveRight,
                Action::MoveRight,
                Action::FocusNextSibling(1),
            ],
        );
        assert_eq!(path(&v), ".users[0].name");
        v.perform_action(Action::MoveDown(1));
        assert_eq!(
            &v.flatjson.1[v.flatjson[v.focused_node].range.clone()],
            "\"Lin\""
        );
        let line = v.absolute_anchor_line;
        v.perform_action(Action::FocusParent);
        assert_eq!(path(&v), ".users[1]");
        assert_eq!(line, v.absolute_anchor_line);
        v.perform_action(Action::FocusParent);
        assert_eq!(path(&v), ".users");
    }

    #[test]
    fn next_sibling_stops_at_the_last_entry() {
        let mut v = viewer("[1,2]");
        act(&mut v, &[Action::MoveRight, Action::MoveRight]);
        v.perform_action(Action::FocusNextSibling(1));
        assert_eq!(path(&v), "[1]");
        let last = v.focused_node;
        v.perform_action(Action::FocusNextSibling(1));
        assert_eq!(v.focused_node, last);
        v.perform_action(Action::FocusNextSibling(10));
        assert_eq!(v.focused_node, last);
    }

    #[test]
    fn parent_level_motions_select_parent_and_next_parent_entry() {
        let mut v = viewer(r#"{"a":{"value":0},"b":{"x":1},"c":{"value":2}}"#);
        // Preserve the destination's collapse state and select its header.
        v.perform_action(Action::MoveRight);
        v.perform_action(Action::MoveLeft);
        let a = v.focused_node;
        act(&mut v, &[Action::FocusNextSibling(1), Action::MoveRight]);
        assert_eq!(path(&v), ".b.x");
        let x = v.focused_node;
        v.perform_action(Action::FocusParentOrPreviousSibling);
        assert_eq!(path(&v), ".b");
        assert!(v.flatjson[v.focused_node].is_expanded());
        assert!(v.flatjson[a].is_collapsed());
        v.perform_action(Action::FocusNode {
            node: x,
            source: None,
        });
        v.perform_action(Action::FocusNextAtParentLevel);
        assert_eq!(path(&v), ".c");
        assert_eq!(
            v.absolute_anchor_line,
            v.layout.node(&v.flatjson, v.focused_node).line
        );
    }

    #[test]
    fn parent_level_motions_prefer_parent_then_fall_back_to_siblings() {
        let mut v = viewer(r#"{"a":{"x":1,"y":2},"b":{"x":3,"y":4}}"#);
        act(&mut v, &[Action::MoveRight, Action::MoveRight]);
        v.perform_action(Action::FocusNextAtParentLevel);
        assert_eq!(
            path(&v),
            ".b",
            "parent-level entry takes priority over .a.y"
        );
        v.perform_action(Action::MoveRight);
        v.perform_action(Action::FocusNextAtParentLevel);
        assert_eq!(
            path(&v),
            ".b.y",
            "use the next sibling when the parent has no next sibling"
        );
        v.perform_action(Action::FocusParentOrPreviousSibling);
        assert_eq!(path(&v), ".b", "parent takes priority over .b.x");
        v.perform_action(Action::FocusNextAtParentLevel);
        assert_eq!(path(&v), ".b", "no destination leaves focus unchanged");
    }

    #[test]
    fn parent_level_motions_fall_back_between_document_roots() {
        let mut v = viewer("1 2 3");
        let first = v.focused_node;
        v.perform_action(Action::FocusNextAtParentLevel);
        let second = v.focused_node;
        assert_ne!(second, first);
        assert_eq!(&v.flatjson.1[v.flatjson[second].range.clone()], "2");
        v.perform_action(Action::FocusParent);
        assert_eq!(v.focused_node, second, "H remains a strict parent motion");
        v.perform_action(Action::FocusParentOrPreviousSibling);
        assert_eq!(v.focused_node, first);
        v.perform_action(Action::FocusParentOrPreviousSibling);
        assert_eq!(v.focused_node, first);
    }

    #[test]
    fn parent_level_motions_keep_focus_at_document_boundaries() {
        let mut v = viewer(r#"{"only":{"x":1}}"#);
        act(
            &mut v,
            &[
                Action::FocusParentOrPreviousSibling,
                Action::FocusNextAtParentLevel,
            ],
        );
        assert_eq!(v.focused_node, 0);
        act(&mut v, &[Action::MoveRight, Action::MoveRight]);
        assert_eq!(path(&v), ".only.x");
        v.perform_action(Action::FocusNextAtParentLevel);
        assert_eq!(path(&v), ".only.x");
        v.perform_action(Action::FocusParentOrPreviousSibling);
        assert_eq!(path(&v), ".only");
        v.perform_action(Action::FocusParentOrPreviousSibling);
        assert_eq!(v.focused_node, 0);
    }

    #[test]
    fn parent_level_motions_use_table_row_identity() {
        let mut v = viewer(r#"[{"x":1},{"x":2},{"x":3}]"#);
        act(
            &mut v,
            &[
                Action::MoveRight,
                Action::FocusNextSibling(1),
                Action::MoveRight,
            ],
        );
        assert_eq!(path(&v), "[1].x");
        let cell = v.focused_node;
        v.perform_action(Action::FocusParentOrPreviousSibling);
        assert_eq!(path(&v), "[1]");
        v.perform_action(Action::FocusNode {
            node: cell,
            source: None,
        });
        v.perform_action(Action::FocusNextAtParentLevel);
        assert_eq!(path(&v), "[2]");
    }

    #[test]
    fn collapse_preserves_child_state_and_implicit_root() {
        let mut v = viewer(r#"{"box":{"inner":{"value":1}},"last":2}"#);
        v.perform_action(Action::ToggleCollapsed);
        assert!(!v.flatjson[0].is_collapsed());
        act(
            &mut v,
            &[
                Action::MoveRight,
                Action::MoveRight,
                Action::ToggleCollapsed,
            ],
        );
        let child = v.focused_node;
        act(&mut v, &[Action::FocusParent, Action::ToggleCollapsed]);
        assert_eq!(v.visible.len(), 2);
        v.perform_action(Action::MoveRight);
        assert!(v.flatjson[child].is_collapsed());
        v.perform_action(Action::MoveRight);
        assert_eq!(v.focused_node, child);
        v.perform_action(Action::MoveRight);
        assert!(!v.flatjson[child].is_collapsed());
        v.perform_action(Action::MoveRight);
        assert_eq!(path(&v), ".box.inner.value");
    }

    #[test]
    fn absolute_jumps_and_search_reveal_distinguish_nodes_and_lines() {
        let mut v = viewer(r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Lin"}]}"#);
        act(&mut v, &[Action::MoveRight, Action::ToggleCollapsed]);
        v.perform_action(Action::JumpTo {
            line: 2,
            make_visible: false,
        });
        assert_eq!(path(&v), ".users");
        v.perform_action(Action::JumpTo {
            line: 2,
            make_visible: true,
        });
        assert_eq!(path(&v), ".users[1]");
        act(&mut v, &[Action::MoveRight, Action::FocusNextSibling(1)]);
        let node = v.focused_node;
        let key = v.flatjson[node].key_range.as_ref().unwrap().start;
        act(&mut v, &[Action::FocusParent, Action::ToggleCollapsed]);
        v.perform_action(Action::FocusNode {
            node,
            source: Some(key),
        });
        assert_eq!(v.focused_node, node);
        assert_eq!(
            v.absolute_anchor_line, 0,
            "shared key header is the display anchor"
        );
        assert_eq!(path(&v), ".users[1].name");
    }

    #[test]
    fn jump_to_hidden_root_array_content_selects_the_body_control() {
        let mut v = viewer("[1,2] {}");
        let root = v.document_roots()[0];
        v.perform_action(Action::MoveRight);
        v.perform_action(Action::FocusParent);
        v.perform_action(Action::MoveDown(1));
        v.perform_action(Action::ToggleCollapsed);
        assert!(v.flatjson[root].is_collapsed());

        let child_line = v
            .layout
            .node(&v.flatjson, v.flatjson[root].first_child().as_usize())
            .line;
        v.perform_action(Action::JumpTo {
            line: child_line,
            make_visible: false,
        });
        assert_eq!(v.focused_node, root);
        assert!(!v.focused_document_header());
        assert_eq!(
            v.absolute_anchor_line,
            v.layout.node(&v.flatjson, root).body_line
        );
    }

    #[test]
    fn separator_jumps_skip_annotations_and_empty_roots_remain_selectable() {
        let mut v = JsonViewer::new(parse_top_level_yaml("---\n{}\n---\n42\n".into()).unwrap());
        let first = v.focused_node;
        v.perform_action(Action::MoveDown(1));
        assert_eq!(
            first, v.focused_node,
            "the first root body keeps root identity"
        );
        v.perform_action(Action::MoveDown(1));
        assert_ne!(first, v.focused_node);
        assert!(v.is_document_header(v.focused_line_index()));
        v.perform_action(Action::MoveUp(1));
        assert_eq!(v.focused_node, first);
        v.perform_action(Action::JumpTo {
            line: 2,
            make_visible: false,
        });
        assert_ne!(v.focused_node, first);
        let mut empty = viewer("{}");
        act(
            &mut empty,
            &[
                Action::MoveRight,
                Action::ToggleCollapsed,
                Action::FocusBottom,
            ],
        );
        assert_eq!(empty.focused_node, 0);
        assert_eq!(empty.visible.len(), 1);
        assert!(empty.render_line(0).text.is_empty());
    }

    #[test]
    fn mouse_hits_cells_and_resize_preserves_focus() {
        let mut v = viewer(r#"["界","é",3]"#);
        let child = v.flatjson[0].first_child().unwrap();
        let second = v.flatjson[child].next_sibling.unwrap();
        let line = v.render_line(0);
        let span = line.spans.iter().find(|span| span.node == second).unwrap();
        let col = UnicodeWidthStr::width(&line.text[..span.range.start]);
        let (node, source) = crate::lineprinter::hit_test(&line, col);
        v.perform_action(Action::FocusNode { node, source });
        assert_eq!(v.focused_node, second);
        v.perform_action(Action::ResizeViewerDimensions(TTYDimensions {
            width: 2,
            height: 0,
        }));
        assert_eq!(v.focused_node, second);
        assert_eq!(v.absolute_anchor_line, 2);
        v.perform_action(Action::FocusTop);
        v.perform_action(Action::ClickArrow(1));
        assert_eq!(v.focused_node, 0);
        assert!(v.flatjson[0].is_collapsed());
    }

    #[test]
    fn counted_motions_scrolling_and_boundaries() {
        let input = format!(
            "{{{}}}",
            (0..30)
                .map(|i| format!("\"k{i}\":{i}"))
                .collect::<Vec<_>>()
                .join(",")
        );
        let mut v = viewer(&input);
        v.perform_action(Action::ResizeViewerDimensions(TTYDimensions {
            width: 20,
            height: 8,
        }));
        v.perform_action(Action::MoveDown(12));
        assert_eq!(v.absolute_anchor_line, 12);
        assert!(v.index_of_focused_node_on_screen() < 8);
        v.perform_action(Action::MoveFocusedLineToCenter);
        assert_eq!(v.index_of_focused_node_on_screen(), 4);
        v.perform_action(Action::PageDown(1));
        assert!(v.top_visible_line >= 8);
        v.perform_action(Action::JumpDown(Some(3)));
        let before = v.absolute_anchor_line;
        v.perform_action(Action::JumpUp(None));
        assert_eq!(v.absolute_anchor_line, before - 3);
        v.perform_action(Action::MoveDown(usize::MAX));
        assert_eq!(v.absolute_anchor_line, 29);
        v.perform_action(Action::MoveUp(usize::MAX));
        assert_eq!(v.absolute_anchor_line, 0);
        v.perform_action(Action::PageDown(usize::MAX));
        assert!(v.top_visible_line < v.visible.len());
    }
    #[test]
    fn ordinary_fields_and_table_cells_enter_inline_array_owner_vertically() {
        let mut v = viewer(r#"{"x":1,"tags":[2,3]}"#);
        act(&mut v, &[Action::MoveRight, Action::MoveDown(1)]);
        assert_eq!(path(&v), ".tags");
        let mut v = viewer(r#"{"users":[{"a":1}],"tags":[2,3]}"#);
        act(
            &mut v,
            &[
                Action::MoveRight,
                Action::MoveRight,
                Action::MoveRight,
                Action::MoveDown(1),
            ],
        );
        assert_eq!(path(&v), ".tags");
    }

    #[test]
    fn retained_depth_and_first_last_sibling_motions_follow_logical_boundaries() {
        let mut v = viewer(r#"{"a":1,"obj":{"b":2,"c":{"d":3},"e":4},"z":5}"#);
        act(
            &mut v,
            &[Action::MoveRight, Action::MoveDownUntilDepthChange],
        );
        assert_eq!(path(&v), ".obj");
        v.perform_action(Action::MoveDownUntilDepthChange);
        assert_eq!(path(&v), ".obj.b");
        v.perform_action(Action::MoveDownUntilDepthChange);
        assert_eq!(path(&v), ".obj.c");
        act(
            &mut v,
            &[Action::MoveRight, Action::MoveDownUntilDepthChange],
        );
        assert_eq!(path(&v), ".obj.e");
        v.perform_action(Action::MoveUpUntilDepthChange);
        assert_eq!(path(&v), ".obj.c.d");
        v.perform_action(Action::MoveUpUntilDepthChange);
        assert_eq!(path(&v), ".obj.c");
        v.perform_action(Action::FocusFirstSibling);
        assert_eq!(path(&v), ".obj.b");
        v.perform_action(Action::FocusLastSibling);
        assert_eq!(path(&v), ".obj.e");
        act(&mut v, &[Action::FocusParent, Action::FocusLastSibling]);
        assert_eq!(path(&v), ".z");
        v.perform_action(Action::FocusFirstSibling);
        assert_eq!(path(&v), ".a");
    }

    #[test]
    fn deep_collapse_and_expand_preserve_siblings_and_recover_viewport() {
        let mut v = viewer(r#"{"a":{"b":{"x":1}},"c":{"d":{"y":2}},"last":3}"#);
        v.perform_action(Action::ResizeViewerDimensions(TTYDimensions {
            width: 40,
            height: 3,
        }));
        act(
            &mut v,
            &[
                Action::MoveRight,
                Action::FocusNextSibling(1),
                Action::MoveFocusedLineToTop,
                Action::DeepCollapseNodeAndSiblings,
            ],
        );
        assert_eq!(path(&v), ".c");
        assert_eq!(v.visible.len(), 3);
        assert!(v.flatjson[1].is_collapsed());
        assert!(v.flatjson[2].is_collapsed());
        assert!(v.focused_line_index() >= v.top_visible_line);
        v.perform_action(Action::ExpandNodeAndSiblings);
        assert!(!v.flatjson[1].is_collapsed());
        assert!(
            v.flatjson[2].is_collapsed(),
            "shallow expansion restores nested collapse"
        );
        v.perform_action(Action::DeepExpandNodeAndSiblings);
        assert!(
            v.flatjson
                .0
                .iter()
                .filter(|row| row.is_container())
                .all(|row| row.is_expanded())
        );
        assert_eq!(path(&v), ".c");
        assert!(v.index_of_focused_node_on_screen() < 3);
    }

    #[test]
    fn scrolloff_and_half_window_eof_bounds_are_retained() {
        let input = format!(
            "{{{}}}",
            (0..30)
                .map(|i| format!("\"k{i}\":{i}"))
                .collect::<Vec<_>>()
                .join(",")
        );
        let mut v = viewer(&input);
        v.perform_action(Action::ResizeViewerDimensions(TTYDimensions {
            width: 40,
            height: 10,
        }));
        v.scrolloff_setting = 2;
        v.perform_action(Action::ScrollDown(5));
        assert_eq!(v.focused_line_index(), 7);
        v.perform_action(Action::ScrollUp(3));
        assert!(v.focused_line_index() <= v.top_visible_line + 7);
        v.perform_action(Action::FocusBottom);
        for _ in 0..5 {
            v.perform_action(Action::JumpDown(None));
        }
        assert_eq!(v.top_visible_line, 20);
        assert_eq!(v.focused_line_index(), 29);
        v.perform_action(Action::JumpUp(None));
        assert_eq!(v.top_visible_line, 15);
        assert_eq!(v.focused_line_index(), 24);
    }

    #[test]
    fn wrapped_rows_scroll_physically_but_motion_stays_logical() {
        let mut v = viewer(
            r#"{"long":"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdefghijklmnopqrstuvwxyz","next":42}"#,
        );
        v.set_viewport(
            TTYDimensions {
                width: 20,
                height: 4,
            },
            true,
        );
        v.set_wrap_geometry(8, 0);
        v.toggle_wrapping();
        assert!(v.physical_rows.len() > v.visible.len());
        v.perform_action(Action::MoveRight);
        let long = v.focused_node;
        v.perform_action(Action::ScrollDown(1));
        assert_eq!(
            v.focused_node, long,
            "scrolling inside a tall value retains focus"
        );
        assert!(v.top_continuation > 0);
        v.perform_action(Action::MoveDown(1));
        assert_eq!(path(&v), ".next");
        v.perform_action(Action::MoveUp(1));
        assert_eq!(v.focused_node, long);
    }

    #[test]
    fn wrapping_reflow_preserves_logical_focus_and_reveals_display_ranges() {
        let mut v = viewer(r#"{"long":"abcdefghijklmnopqrstuvwxyz0123456789","next":42}"#);
        v.set_viewport(
            TTYDimensions {
                width: 20,
                height: 4,
            },
            true,
        );
        v.set_wrap_geometry(8, 0);
        v.toggle_wrapping();
        v.perform_action(Action::MoveRight);
        let node = v.focused_node;
        let line = v.render_line(v.focused_line_index());
        let span = line
            .spans
            .iter()
            .find(|span| span.node == node && span.source.is_some())
            .unwrap();
        let display = span.matching_ranges(&span.source.clone().unwrap())[0].clone();
        v.reveal_byte_range(display.clone());
        assert_eq!(v.focused_node, node);
        assert!(
            v.physical_rows
                .iter()
                .enumerate()
                .any(|(index, row)| row.logical_line == v.focused_line_index()
                    && row.bytes.start <= display.start
                    && display.start < row.bytes.end
                    && index < usize::from(v.dimensions.height).max(1))
        );
        v.set_wrap_geometry(4, 0);
        assert_eq!(v.focused_node, node);
    }

    #[test]
    fn physical_scroll_keeps_tall_value_focused_at_document_end() {
        let input = format!(
            r#"{{"long":"{}TAIL"}}"#,
            "0123456789abcdefghijklmnopqrstuvwxyz".repeat(8)
        );
        let mut v = viewer(&input);
        v.set_viewport(
            TTYDimensions {
                width: 16,
                height: 6,
            },
            true,
        );
        v.set_wrap_geometry(11, 0);
        v.toggle_wrapping();
        v.perform_action(Action::MoveRight);
        let long = v.focused_node;
        v.perform_action(Action::ScrollDown(3));
        v.perform_action(Action::ScrollDown(3));
        v.perform_action(Action::ScrollUp(3));
        v.perform_action(Action::ScrollDown(1));
        v.perform_action(Action::PageDown(9));
        assert_eq!(v.focused_node, long);
        assert!(
            v.screen_row(0)
                .is_some_and(|row| row.bytes.contains(&row.bytes.end.saturating_sub(1)))
        );
    }
    #[test]
    fn repositioning_uses_a_match_continuation_and_keeps_it_selected() {
        let mut v = viewer(&format!("\"{}\"", "abcdefghij".repeat(30)));
        v.set_viewport(
            TTYDimensions {
                width: 40,
                height: 6,
            },
            true,
        );
        v.scrolloff_setting = 0;
        v.set_wrap_geometry(10, 0);
        v.toggle_wrapping();
        v.reveal_byte_range(55..60);
        v.perform_action(Action::MoveFocusedLineToTop);
        assert_eq!(v.screen_row(0).unwrap().bytes.start, 50);
        v.perform_action(Action::MoveFocusedLineToCenter);
        assert_eq!(v.screen_row(0).unwrap().bytes.start, 20);
        v.perform_action(Action::MoveFocusedLineToBottom);
        assert_eq!(v.screen_row(0).unwrap().bytes.start, 0);
        assert_eq!(v.focused_node, 0);
    }

    #[test]
    fn wrapping_projection_reuses_unchanged_geometry_and_reflows_on_collapse() {
        let mut v = viewer(r#"{"box":{"long":"abcdefghijklmnopqrstuvwxyz"},"tail":1}"#);
        v.set_wrap_geometry(8, 0);
        v.toggle_wrapping();
        let generation = v.physical_generation;
        let rows = v.physical_rows.clone();
        assert!(!v.set_wrap_geometry(8, 0));
        assert_eq!(v.physical_generation, generation);
        assert_eq!(v.physical_rows, rows);
        v.perform_action(Action::MoveRight);
        v.perform_action(Action::ToggleCollapsed);
        assert!(v.physical_generation != generation);
        assert_eq!(
            v.physical_rows
                .iter()
                .filter(|row| row.logical_line == 0)
                .count(),
            1
        );
        assert!(!v.is_wrapped_line(0));
        v.perform_action(Action::MoveRight);
        assert!(v.is_wrapped_line(0));
        assert!(v.set_wrap_geometry(6, 2));
        assert_eq!(path(&v), ".box");
    }

    #[test]
    fn wrapping_reflow_clamps_to_the_last_viewport_origin() {
        let mut v = viewer(&format!(
            r#"{{"long":"{}","tail":42}}"#,
            "0123456789abcdefghijklmnopqrstuvwxyz".repeat(4)
        ));
        v.set_viewport(
            TTYDimensions {
                width: 20,
                height: 4,
            },
            true,
        );
        v.set_wrap_geometry(8, 0);
        v.toggle_wrapping();
        v.perform_action(Action::FocusBottom);

        v.set_wrap_geometry(200, 0);

        assert_eq!(
            v.top_cursor(),
            Cursor {
                logical: 0,
                continuation: 0
            }
        );
    }

    #[test]
    fn wrapped_height_resize_clamps_the_existing_origin() {
        let mut v = viewer(&format!(
            r#"{{"long":"{}","tail":42}}"#,
            "0123456789abcdefghijklmnopqrstuvwxyz".repeat(4)
        ));
        v.set_viewport(
            TTYDimensions {
                width: 20,
                height: 3,
            },
            true,
        );
        v.set_wrap_geometry(8, 0);
        v.toggle_wrapping();
        v.perform_action(Action::FocusBottom);
        assert!(
            v.top_cursor()
                > Cursor {
                    logical: 0,
                    continuation: 0
                }
        );

        v.set_viewport(
            TTYDimensions {
                width: 20,
                height: 20,
            },
            true,
        );

        assert_eq!(
            v.top_cursor(),
            Cursor {
                logical: 0,
                continuation: 0
            }
        );
    }

    #[test]
    fn projection_reflow_preserves_the_top_line_identity() {
        let mut v = viewer(
            r#"{"box":{"long":"0123456789abcdefghijklmnopqrstuvwxyz0123456789"},"tail1":1,"tail2":2,"tail3":3,"tail4":4,"tail5":5,"tail6":6}"#,
        );
        v.set_viewport(
            TTYDimensions {
                width: 20,
                height: 4,
            },
            true,
        );
        v.set_wrap_geometry(8, 0);
        v.toggle_wrapping();

        let box_node = v.flatjson[0].first_child().unwrap();
        let tail_node = v.flatjson[box_node].next_sibling.unwrap();
        let tail_absolute = v.layout.node(&v.flatjson, tail_node).line;
        let tail_line = v.visible.visible_index(tail_absolute).unwrap();
        v.set_top(Cursor {
            logical: tail_line,
            continuation: 0,
        });
        v.collapse(box_node, true);
        v.refresh_projection();

        assert_eq!(
            v.visible_line(v.top_visible_line).unwrap().absolute,
            tail_absolute
        );
    }

    #[test]
    fn projection_reflow_preserves_a_sequence_body_at_the_top() {
        let mut v = viewer("1 2");
        let root = v.document_roots()[0];
        let body = v.layout.node(&v.flatjson, root).body_line;
        v.top_visible_line = v.visible.visible_index(body).unwrap();

        v.rebuild_layout();

        assert_eq!(v.visible_line(v.top_visible_line).unwrap().absolute, body);
    }

    #[test]
    fn wrapped_mouse_focus_keeps_the_clicked_continuation_visible() {
        let mut v = viewer(&format!(
            r#"{{"long":"{}","tail":42}}"#,
            "0123456789abcdefghijklmnopqrstuvwxyz".repeat(4)
        ));
        v.set_viewport(
            TTYDimensions {
                width: 20,
                height: 4,
            },
            true,
        );
        v.set_wrap_geometry(8, 0);
        v.toggle_wrapping();
        let target = v
            .physical_rows
            .iter()
            .rev()
            .find(|row| !row.first)
            .unwrap()
            .clone();
        let line = v.rendered_line(target.logical_line);
        let (node, source) = crate::lineprinter::hit_test_wrapped(line, &target, 0);
        v.perform_action(Action::FocusNodeAt {
            node,
            source,
            display_range: (target.bytes.start, target.bytes.end),
        });

        assert!(
            v.physical_rows
                .iter()
                .any(|row| row.logical_line == target.logical_line && row.bytes == target.bytes)
        );
    }

    #[test]
    fn wrapping_geometry_reflow_keeps_structure_focus_visible() {
        let mut v = viewer(&format!(
            r#"{{"long":"{}","container":{{"value":1}},"tail":2}}"#,
            "0123456789abcdefghijklmnopqrstuvwxyz".repeat(4)
        ));
        v.set_viewport(
            TTYDimensions {
                width: 20,
                height: 4,
            },
            true,
        );
        v.set_wrap_geometry(8, 0);
        v.toggle_wrapping();

        let long_node = v.flatjson[0].first_child().unwrap();
        let container_node = v.flatjson[long_node].next_sibling.unwrap();
        v.focus(container_node);
        v.set_top(Cursor {
            logical: 0,
            continuation: 0,
        });
        v.refresh_frame();

        v.set_wrap_geometry(4, 0);

        let height = usize::from(v.dimensions.height).max(1);
        assert!(
            v.physical_rows
                .iter()
                .any(|row| row.logical_line == v.focused_line_index())
        );
        assert!(v.focused_physical_row() < height);
    }

    fn wrapped_table_viewer() -> JsonViewer {
        let mut v = viewer(
            r#"[{"id":0,"long":"tail"},{"id":1,"long":"tail"},{"id":2,"long":"0123456789abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz"},{"id":3,"long":"tail"},{"id":4,"long":"tail"},{"id":5,"long":"tail"},{"id":6,"long":"tail"},{"id":7,"long":"tail"},{"id":8,"long":"tail"},{"id":9,"long":"tail"}]"#,
        );
        v.set_viewport(
            TTYDimensions {
                width: 20,
                height: 4,
            },
            true,
        );
        v.set_wrap_geometry(8, 0);
        v.toggle_wrapping();
        v.perform_action(Action::MoveRight);
        v.perform_action(Action::FocusNextSibling(2));
        v.perform_action(Action::MoveRight);
        v.perform_action(Action::FocusNextSibling(1));
        v
    }

    #[test]
    fn wrapped_half_page_jumps_retain_selected_cell_while_entry_is_visible() {
        let mut v = wrapped_table_viewer();
        let long = v.focused_node;
        assert_eq!(path(&v), "[2].long");

        v.perform_action(Action::JumpDown(Some(1)));
        assert_eq!(v.focused_node, long);
        assert!(
            v.top_cursor()
                > Cursor {
                    logical: 0,
                    continuation: 0
                }
        );

        v.perform_action(Action::JumpDown(Some(2)));
        assert_eq!(v.focused_node, long);

        v.perform_action(Action::JumpUp(Some(1)));
        assert_eq!(v.focused_node, long);

        v.perform_action(Action::JumpUp(Some(2)));
        assert_eq!(v.focused_node, long);
    }

    #[test]
    fn wrapped_half_page_jumps_select_surrounding_lines_at_viewport_boundaries() {
        let mut v = wrapped_table_viewer();
        let long = v.focused_node;
        let long_line = v.focused_line_index();

        // Move down until the selected entry no longer intersects the screen.
        for _ in 0..32 {
            if v.focused_node != long {
                break;
            }
            v.perform_action(Action::JumpDown(Some(1)));
        }
        assert_ne!(v.focused_node, long);
        assert!(v.focused_line_index() > long_line);

        for _ in 0..64 {
            let previous = v.top_cursor();
            v.perform_action(Action::JumpDown(Some(1)));
            if v.top_cursor() == previous {
                break;
            }
        }
        let lower_boundary_top = v.top_cursor();
        let lower_boundary_node = v.focused_node;
        v.perform_action(Action::JumpDown(Some(1)));
        assert_eq!(v.top_cursor(), lower_boundary_top);
        assert_eq!(v.focused_node, lower_boundary_node);

        let mut v = wrapped_table_viewer();
        let long = v.focused_node;
        let long_line = v.focused_line_index();
        v.perform_action(Action::JumpDown(Some(5)));
        assert_eq!(v.focused_node, long);

        // Move up until the selected entry no longer intersects the screen.
        for _ in 0..32 {
            if v.focused_node != long {
                break;
            }
            v.perform_action(Action::JumpUp(Some(1)));
        }
        assert_ne!(v.focused_node, long);
        assert!(v.focused_line_index() < long_line);

        let upper_boundary_top = v.top_cursor();
        let upper_boundary_node = v.focused_node;
        v.perform_action(Action::JumpUp(Some(1)));
        assert_eq!(v.top_cursor(), upper_boundary_top);
        assert_eq!(v.focused_node, upper_boundary_node);
    }
    #[test]
    fn selected_root_resets_projection_and_blocks_ancestor_navigation() {
        let mut v = viewer(r#"{"outer":{"kept":1},"tail":{"excluded":2}}"#);
        let outer = v.flatjson[0].first_child().unwrap();
        v.set_roots(vec![outer]);
        assert_eq!(v.active_roots(), &[outer]);
        assert!(!v.contains_node(0));
        assert!(v.contains_node(outer));
        assert_eq!(
            (0..v.visible.len())
                .map(|line| v.render_line(line).text)
                .collect::<Vec<_>>(),
            vec!["kept: 1"]
        );
        v.perform_action(Action::MoveRight);
        let kept = v.focused_node;
        v.perform_action(Action::FocusParent);
        assert_eq!(v.focused_node, outer);
        v.perform_action(Action::FocusParentOrPreviousSibling);
        assert_eq!(v.focused_node, outer);
        v.perform_action(Action::FocusNextSibling(1));
        assert_eq!(v.focused_node, outer);
        v.perform_action(Action::FocusNode {
            node: 0,
            source: None,
        });
        assert_eq!(v.focused_node, outer);
        assert!(v.contains_node(kept));
        v.set_roots(vec![0]);
        assert_eq!(v.active_roots(), &[0]);
        assert!(v.contains_node(0));
        assert!((0..v.visible.len()).any(|line| v.render_line(line).text.contains("tail")));
    }

    #[test]
    fn selected_sequence_roots_keep_sibling_motion_without_original_ancestors() {
        let mut v = viewer(r#"{"hits":{"a":1},"outside":0} {"hits":{"b":2},"outside":9}"#);
        let roots = crate::path_filter::PathFilter::parse(".hits")
            .unwrap()
            .resolve(&v.flatjson)
            .unwrap();
        let (first, second) = (roots[0], roots[1]);
        v.set_roots(roots);
        assert_eq!(v.active_roots(), &[first, second]);
        assert_eq!(v.render_line(0).text, "--- (1 of 2)");
        v.perform_action(Action::MoveRight);
        assert_eq!(v.focused_node, v.flatjson[first].first_child().unwrap());
        v.perform_action(Action::FocusNextAtParentLevel);
        assert_eq!(v.focused_node, second);
        v.perform_action(Action::FocusPrevSibling(1));
        assert_eq!(v.focused_node, first);
        v.perform_action(Action::FocusNextSibling(1));
        assert_eq!(v.focused_node, second);
        v.perform_action(Action::FocusNextSibling(1));
        assert_eq!(v.focused_node, second);
        assert!(!v.contains_node(0));
    }

    #[test]
    fn offscreen_final_records_control_table_eligibility_and_warning_totals() {
        let prefix = vec![r#"{"a":1,"b":2}"#; 64].join(",");
        for (last, header) in [
            (r#"{"a":3,"b":4}"#, "[65]{a,b}:"),
            (r#"{"a":3}"#, "[65]:"),
            (r#"{"b":4,"a":3}"#, "[65]:"),
            (r#"{"a":3,"b":{"nested":4}}"#, "[65]:"),
            (r#"{"a":3,"\u0061":4}"#, "[65]:"),
        ] {
            let mut v = viewer(&format!("[{prefix},{last}]"));
            assert_eq!(v.rendered_line(0).text, header);
            if last.contains("\\u0061") {
                v.perform_action(Action::ToggleCollapsed);
                assert!(
                    v.rendered_line(0)
                        .text
                        .contains("Contains 2 hidden warnings")
                );
            }
        }
        let mut v = viewer(r#"{"box":{"\u0001":"\u0001"}}"#);
        v.perform_action(Action::MoveRight);
        v.perform_action(Action::ToggleCollapsed);
        assert!(
            v.rendered_line(0)
                .text
                .contains("Contains 2 hidden warnings")
        );
    }

    #[test]
    fn viewport_work_and_retention_do_not_scale_with_tails_or_visited_rows() {
        const HEIGHT: usize = 8;
        let mut initial_work = None;
        for count in [64, 4096] {
            let input = format!(
                r#"{{"records":[{}]}}"#,
                vec![r#"{"id":0,"nested":{"ok":true,"label":"record"},"values":[1,2,3]}"#; count]
                    .join(",")
            );
            let flat = parse_top_level_json(input).unwrap();
            let before = Layout::formatted_rows();
            let mut v = JsonViewer::with_roots(
                flat,
                vec![0],
                TTYDimensions {
                    width: 80,
                    height: HEIGHT as u16,
                },
                true,
                false,
            );
            let work = Layout::formatted_rows() - before;
            assert!(work <= HEIGHT + 1, "startup formatted {work} rows");
            if let Some(expected) = initial_work {
                assert_eq!(work, expected);
            }
            initial_work = Some(work);
            for visit in 0..32 {
                let last = v.layout.rows().len() - 1;
                let line = [last, 0, last / 2, last / 3][visit % 4];
                let node = v.layout.rows()[line].owner;
                let before = Layout::formatted_rows();
                v.perform_action(Action::FocusNode { node, source: None });
                let work = Layout::formatted_rows() - before;
                assert!(
                    work <= 4 * (HEIGHT + 1),
                    "distant navigation formatted {work} rows"
                );
                assert_eq!(v.focused_node, node);
                assert!(v.frame.len() <= HEIGHT + 1);
                assert!(v.physical_rows.len() <= HEIGHT);
                let spans: usize = v.frame.iter().map(|(_, line)| line.spans.len()).sum();
                let maps: usize = v
                    .frame
                    .iter()
                    .flat_map(|(_, line)| &line.spans)
                    .map(|span| span.source_map.len())
                    .sum();
                assert!(spans <= 16 * (HEIGHT + 1));
                assert!(maps <= 8 * (HEIGHT + 1));
            }
            v.toggle_wrapping();
            let before = Layout::formatted_rows();
            v.perform_action(Action::FocusBottom);
            assert!(Layout::formatted_rows() - before <= 6 * (HEIGHT + 1));
            assert_eq!(v.focused_line_index(), v.visible.len() - 1);
        }
    }

    #[test]
    fn distant_escaped_table_key_highlights_shared_header_without_alias_fanout() {
        use crate::lineprinter::{LineViewport, paint};
        use crate::terminal::Terminal;
        use crate::terminal::test::VisibleEscapesTerminal;
        let input = format!(
            "[{},{{\"\\u006eeedle\":9}}]",
            vec![r#"{"needle":0}"#; 1000].join(",")
        );
        let mut v = viewer(&input);
        let last_row = v.flatjson[v.flatjson[0].pair_index().unwrap()]
            .last_child()
            .unwrap();
        let field = v.flatjson[last_row].first_child().unwrap();
        let key = v.flatjson[field].key_range.clone().unwrap();
        let query = key.start + 1..key.start + 7;
        assert!(
            !v.frame
                .iter()
                .any(|(_, line)| line.spans.iter().any(|span| span.node == field))
        );
        v.highlight_shared_fields(std::slice::from_ref(&query), &(0..0));
        let header = v.rendered_line(0);
        let mut terminal = VisibleEscapesTerminal::new(false, true);
        paint(
            &mut terminal,
            header,
            0..0,
            LineViewport::new(header, 0, 0),
            80,
            std::slice::from_ref(&query),
            &(0..0),
        )
        .unwrap();
        assert!(
            terminal.output().contains("_FG(Yellow)__U_n"),
            "{}",
            terminal.output()
        );
        let before = Layout::formatted_rows();
        v.perform_action(Action::FocusNode {
            node: field,
            source: Some(query.start),
        });
        assert_eq!(v.focused_node, field);
        assert_eq!(v.focused_line_index(), 0);
        assert!(Layout::formatted_rows() - before <= 4 * (usize::from(v.dimensions.height) + 1));
        v.highlight_shared_fields(std::slice::from_ref(&query), &query);
        let header = v.rendered_line(0);
        assert!(header.spans.len() < 12);
        assert!(
            header
                .spans
                .iter()
                .map(|span| span.source_map.len())
                .sum::<usize>()
                < 12
        );
        let alias = header.spans.iter().find(|span| span.node == field).unwrap();
        let range = alias.matching_ranges(&query).pop().unwrap();
        assert_eq!(&header.text[range.clone()], "n");
        let first_field = v.flatjson[v.flatjson[0].first_child().unwrap()]
            .first_child()
            .unwrap();
        assert_eq!(
            crate::lineprinter::hit_test(header, range.start).0,
            first_field
        );
        let mut terminal = VisibleEscapesTerminal::new(false, true);
        paint(
            &mut terminal,
            header,
            field..field + 1,
            LineViewport::new(header, 0, 0),
            80,
            std::slice::from_ref(&query),
            &query,
        )
        .unwrap();
        assert!(
            terminal.output().contains("_FG(LightYellow)__U_n"),
            "{}",
            terminal.output()
        );
    }

    #[test]
    fn distant_filtered_inline_match_and_wrapped_end_skip_intervening_rows() {
        let input = format!(
            r#"{{"keep":[{}],"outside":"excluded"}} {{"keep":{{"tags":["x","\u754cNEEDLE"],"long":"{}TAIL"}},"outside":"excluded"}}"#,
            vec![r#"{"id":0,"nested":{"ok":true}}"#; 1000].join(","),
            "abcdef".repeat(100)
        );
        let flat = parse_top_level_json(input.clone()).unwrap();
        let roots = crate::path_filter::PathFilter::parse(".keep")
            .unwrap()
            .resolve(&flat)
            .unwrap();
        let second = roots[1];
        let array = flat[second].first_child().unwrap();
        let long = flat[array].next_sibling.unwrap();
        let first = flat[array].first_child().unwrap();
        let target = flat[first].next_sibling.unwrap();
        let start =
            flat.1[flat[target].range.clone()].find("NEEDLE").unwrap() + flat[target].range.start;
        let query = start..start + 6;
        let mut v = JsonViewer::with_roots(
            flat,
            roots,
            TTYDimensions {
                width: 26,
                height: 5,
            },
            true,
            true,
        );
        assert!(v.layout.node(&v.flatjson, array).inline_array);
        v.toggle_wrapping();
        let before = Layout::formatted_rows();
        v.perform_action(Action::FocusNode {
            node: target,
            source: Some(start),
        });
        let logical = v.focused_line_index();
        let line = v.rendered_line(logical);
        let range = line
            .spans
            .iter()
            .find(|span| span.node == target && span.source.is_some())
            .unwrap()
            .matching_ranges(&query)
            .pop()
            .unwrap();
        assert_eq!(&line.text[range.clone()], "NEEDLE");
        v.reveal_byte_range(range.clone());
        assert!(Layout::formatted_rows() - before <= 36);
        assert_eq!(v.focused_node, target);
        assert_eq!(v.active_root_for(target), Some(second));
        assert!(!v.contains_node(0));
        assert!(v.physical_rows.iter().any(|row| row.logical_line == logical
            && row.bytes.start < range.end
            && range.start < row.bytes.end));
        let before = Layout::formatted_rows();
        v.perform_action(Action::FocusBottom);
        assert!(Layout::formatted_rows() - before <= 36);
        assert_eq!(v.focused_node, long);
        assert!(v.frame.len() <= 6);
        assert!(v.physical_rows.len() <= 5);

        // Start again so neither the distant logical row nor its continuations
        // has been materialized before the explicit match request.
        let flat = parse_top_level_json(input).unwrap();
        let roots = crate::path_filter::PathFilter::parse(".keep")
            .unwrap()
            .resolve(&flat)
            .unwrap();
        let mut cold = JsonViewer::with_roots(
            flat,
            roots,
            TTYDimensions {
                width: 26,
                height: 5,
            },
            true,
            true,
        );
        cold.toggle_wrapping();
        assert!(
            cold.frame
                .iter()
                .all(|(_, line)| line.spans.iter().all(|span| span.node != long))
        );
        let end = cold.flatjson[long].range.end - 1;
        let query = end - 4..end;
        let before = Layout::formatted_rows();
        cold.perform_action(Action::FocusNode {
            node: long,
            source: Some(query.start),
        });
        let logical = cold.focused_line_index();
        let line = cold.rendered_line(logical);
        let display = line
            .spans
            .iter()
            .flat_map(|span| span.matching_ranges(&query))
            .next()
            .unwrap();
        assert_eq!(&line.text[display.clone()], "TAIL");
        cold.reveal_byte_range(display.clone());
        assert!(Layout::formatted_rows() - before <= 36);
        assert_eq!(cold.focused_node, long);
        assert!(
            cold.physical_rows
                .iter()
                .any(|row| row.logical_line == logical
                    && row.bytes.start < display.end
                    && display.start < row.bytes.end)
        );
        assert!(cold.frame.len() <= 6);
        assert!(cold.physical_rows.len() <= 5);
    }
    #[test]
    fn distant_yaml_values_keep_paths_and_export_after_projection_changes() {
        let mut input = String::from("rows:\n");
        for id in 0..5000 {
            input.push_str(&format!("  - {{id: {id}, label: row}}\n"));
        }
        input.push_str("special:\n  ? [typed, key]\n  : .inf\n  text: \"line\\n界\"\n");
        let flat = parse_top_level_yaml(input).unwrap();
        let target = crate::path_filter::PathFilter::parse(".special.text")
            .unwrap()
            .resolve(&flat)
            .unwrap()[0];
        let special = flat[target].parent.unwrap();
        let mut v = JsonViewer::new(flat);
        v.perform_action(Action::FocusNode {
            node: special,
            source: None,
        });
        v.perform_action(Action::ToggleCollapsed);
        v.perform_action(Action::FocusNode {
            node: target,
            source: None,
        });
        assert_eq!(path(&v), ".special.text");
        assert_eq!(v.flatjson.string_value(target), Some("line\n界"));
        assert_eq!(
            crate::output::serialize_roots(
                &v.flatjson,
                crate::options::OutputFormat::Json,
                &[target]
            )
            .unwrap(),
            "\"line\\n界\"\n"
        );
        v.set_roots(vec![special]);
        v.perform_action(Action::FocusNode {
            node: target,
            source: None,
        });
        assert_eq!(path(&v), ".special.text");
        assert!(
            v.render_line(v.focused_line_index())
                .text
                .contains("line\\n界")
        );
    }

    #[test]
    fn repeated_collapse_through_paired_delimiters_keeps_projection_exact() {
        let mut flat =
            parse_top_level_json(r#"{"a":{"x":1},"b":{"y":2},"tail":3}"#.into()).unwrap();
        let roots = crate::path_filter::document_roots(&flat);
        let a = flat[roots[0]].first_child().unwrap();
        let b = flat[a].next_sibling.unwrap();
        let a_close = flat[a].pair_index().unwrap();
        flat.collapse(a_close);
        flat.collapse(a);
        flat.collapse(b);
        flat.expand(a);
        flat.expand(a_close);
        let mut v = JsonViewer::new(flat);
        assert_eq!(
            (0..v.visible.len())
                .map(|i| v.visible_line(i).unwrap().absolute)
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 4]
        );
        v.flatjson.expand(b);
        v.refresh_projection();
        assert_eq!(
            (0..v.visible.len())
                .map(|i| v.visible_line(i).unwrap().absolute)
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 4]
        );
    }
}
