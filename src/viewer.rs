use crate::flatjson::{FlatJson, Index, OptionIndex};
use crate::toon_display::{normalize_node, Layout, VisibleLine};
use crate::types::TTYDimensions;
use crate::wrapped_view::{build_physical_rows, PhysicalRow};
use std::collections::HashSet;
use std::ops::Range;
#[cfg(test)]
use unicode_width::UnicodeWidthStr;

/// Parsed node focus is independent of its painted line (inline values and table cells).
pub struct JsonViewer {
    pub flatjson: FlatJson,
    pub layout: Layout,
    pub visible: Vec<VisibleLine>,
    /// Index into the visibility projection, not the parsed node list.
    pub top_visible_line: Index,
    /// Index into the physical-row projection used by the screen writer.
    pub top_physical_row: usize,
    /// Cached physical rows. Continuations point into `visible[line].line.text`.
    pub physical_rows: Vec<PhysicalRow>,
    /// Session-only optional wrapping state. It starts disabled.
    pub wrapping_enabled: bool,
    pub focused_node: Index,
    pub absolute_anchor_line: usize,
    jump_distance: Option<usize>,
    desired_depth: usize,
    pub dimensions: TTYDimensions,
    pub scrolloff_setting: u16,
    expanded_arrays: HashSet<usize>,
    line_numbers: bool,
    wrap_width: usize,
    wrap_indentation: usize,
    wrapped_lines: Vec<bool>,
    pub layout_generation: usize,
    pub physical_generation: usize,
    focus_byte_range: Option<Range<usize>>,
}

impl JsonViewer {
    pub fn new(flatjson: FlatJson) -> Self {
        let layout =
            Self::layout_for_view(&flatjson, TTYDimensions::default(), true, &HashSet::new());
        let visible = layout.project(&flatjson);
        let absolute_anchor_line = layout.nodes[0].line;
        let physical_rows = build_physical_rows(
            &visible,
            usize::from(TTYDimensions::default().width),
            0,
            false,
            &[],
        );
        Self {
            flatjson,
            layout,
            visible,
            top_visible_line: 0,
            top_physical_row: 0,
            physical_rows,
            wrapping_enabled: false,
            focused_node: 0,
            absolute_anchor_line,
            jump_distance: None,
            desired_depth: 0,
            dimensions: TTYDimensions::default(),
            scrolloff_setting: 3,
            expanded_arrays: HashSet::new(),
            line_numbers: true,
            wrap_width: usize::from(TTYDimensions::default().width),
            wrap_indentation: 0,
            wrapped_lines: vec![],
            layout_generation: 0,
            physical_generation: 0,
            focus_byte_range: None,
        }
    }

    fn wrap_eligible(&self, logical_line: usize) -> bool {
        let Some(visible) = self.visible.get(logical_line) else {
            return false;
        };
        !visible.line.separator && !self.flatjson[visible.line.owner].is_collapsed()
    }

    fn top_logical_line(&self) -> usize {
        if self.wrapping_enabled {
            self.physical_rows
                .get(self.top_physical_row)
                .map(|row| row.logical_line)
                .unwrap_or(self.top_visible_line)
        } else {
            self.top_visible_line
        }
    }

    fn sync_top_indices(&mut self) {
        if self.wrapping_enabled {
            if let Some(row) = self.physical_rows.get(self.top_physical_row) {
                self.top_visible_line = row.logical_line;
            }
        } else {
            self.top_physical_row = self
                .top_visible_line
                .min(self.physical_rows.len().saturating_sub(1));
        }
    }

    fn rebuild_physical_rows(&mut self) {
        let previous_top = self.top_logical_line();
        let eligible: Vec<_> = (0..self.visible.len())
            .map(|line| self.wrap_eligible(line))
            .collect();
        let rows = build_physical_rows(
            &self.visible,
            self.wrap_width,
            self.wrap_indentation,
            self.wrapping_enabled,
            &eligible,
        );
        self.physical_rows = rows;
        self.physical_generation = self.physical_generation.wrapping_add(1);
        self.wrapped_lines = eligible;
        self.top_physical_row = self
            .physical_rows
            .iter()
            .position(|row| row.logical_line == previous_top)
            .unwrap_or(0)
            .min(self.physical_rows.len().saturating_sub(1));
        self.sync_top_indices();
    }

    /// Return the physical row painted at a document screen offset.
    pub fn screen_row(&self, row: usize) -> Option<&PhysicalRow> {
        self.physical_rows
            .get(self.top_physical_row.saturating_add(row))
    }

    /// Whether horizontal scrolling is disabled for this expanded logical line.
    pub fn is_wrapped_line(&self, logical_line: usize) -> bool {
        self.wrapping_enabled
            && self
                .wrapped_lines
                .get(logical_line)
                .copied()
                .unwrap_or(false)
    }

    /// Update the document geometry used to split rows. Returns whether it
    /// changed the cached geometry and rebuilt the projection.
    pub fn set_wrap_geometry(&mut self, width: usize, indentation: usize) -> bool {
        if self.wrap_width == width && self.wrap_indentation == indentation {
            return false;
        }
        self.wrap_width = width;
        self.wrap_indentation = indentation;
        self.rebuild_physical_rows();
        true
    }

    /// Toggle wrapping for this interactive session and preserve the logical
    /// line at the top of the viewport.
    pub fn toggle_wrapping(&mut self) {
        self.wrapping_enabled = !self.wrapping_enabled;
        self.rebuild_physical_rows();
        self.ensure_visible();
    }

    /// Reveal a display byte range in the currently focused logical line.
    /// Callers resolve source ranges through the line's mapped spans first.
    pub fn reveal_byte_range(&mut self, range: Range<usize>) {
        self.focus_byte_range = Some(range.clone());
        let logical_line = self.focused_line_index();
        let Some(line) = self.visible.get(logical_line) else {
            return;
        };
        if range.start > line.line.text.len() || range.end > line.line.text.len() {
            return;
        }
        let target_row = self
            .physical_rows
            .iter()
            .enumerate()
            .find(|(_, row)| {
                row.logical_line == logical_line
                    && (range.start < row.bytes.end && row.bytes.start < range.end
                        || range.start == range.end
                            && row.bytes.start <= range.start
                            && range.start <= row.bytes.end)
            })
            .map(|(index, _)| index);
        let Some(target_row) = target_row else {
            return;
        };
        let last_match_row = self
            .physical_rows
            .iter()
            .enumerate()
            .rev()
            .find(|(_, row)| {
                row.logical_line == logical_line
                    && (range.start < row.bytes.end && row.bytes.start < range.end
                        || range.start == range.end
                            && row.bytes.start <= range.start
                            && range.start <= row.bytes.end)
            })
            .map(|(index, _)| index)
            .unwrap_or(target_row);
        let height = usize::from(self.dimensions.height).max(1);
        let padding = usize::from(self.scrolloff_setting).min((height - 1) / 2);
        let last_top = self.physical_rows.len().saturating_sub(height);
        if target_row < self.top_physical_row.saturating_add(padding)
            || last_match_row >= self.top_physical_row + height.saturating_sub(padding)
        {
            self.top_physical_row = target_row.saturating_sub(padding).min(last_top);
            let match_height = last_match_row.saturating_sub(target_row).saturating_add(1);
            if match_height <= height && last_match_row >= self.top_physical_row + height {
                self.top_physical_row = last_match_row
                    .saturating_add(1)
                    .saturating_sub(height)
                    .min(last_top);
            }
        }
        self.top_visible_line = logical_line;
        self.sync_top_indices();
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
    ) -> Layout {
        // Start with the smallest gutter and only grow it: expanding arrays can
        // increase the number of digits, which can force another array onto lines.
        let mut number_width = if line_numbers { 3 } else { 0 };
        loop {
            let width = usize::from(dimensions.width).saturating_sub(number_width + 2);
            let layout = Layout::for_view(flat, width, expanded_arrays);
            let required = if line_numbers {
                layout.lines.len().to_string().len().max(2) + 1
            } else {
                0
            };
            if required <= number_width {
                return layout;
            }
            number_width = required;
        }
    }

    fn rebuild_layout(&mut self) {
        let top_node = self
            .visible
            .get(self.top_logical_line())
            .map(|line| line.line.owner);
        self.layout = Self::layout_for_view(
            &self.flatjson,
            self.dimensions,
            self.line_numbers,
            &self.expanded_arrays,
        );
        self.refresh_projection();
        self.focus(self.focused_node);
        if let Some(top_node) = top_node {
            let top = self.flatjson.first_visible_ancestor(top_node);
            if let Some(index) = self
                .visible
                .iter()
                .position(|line| line.absolute == self.layout.nodes[top].line)
            {
                self.top_visible_line = index;
            }
        }
        self.rebuild_physical_rows();
        self.layout_generation = self.layout_generation.wrapping_add(1);
    }

    pub fn focused_line_index(&self) -> usize {
        self.visible
            .iter()
            .position(|v| v.absolute == self.absolute_anchor_line)
            .unwrap_or(0)
    }

    /// Physical row containing the focused logical line, preferring its first
    /// row because logical focus has no continuation coordinate of its own.
    pub fn focused_physical_row(&self) -> usize {
        let logical = self.focused_line_index();
        let byte = self
            .focus_byte_range
            .as_ref()
            .map(|range| range.start)
            .or_else(|| {
                self.visible[logical]
                    .line
                    .spans
                    .iter()
                    .find(|span| span.node == self.focused_node && span.source.is_some())
                    .map(|span| span.range.start)
            });
        let first = self
            .physical_rows
            .iter()
            .position(|row| row.logical_line == logical)
            .unwrap_or(0);
        byte.and_then(|byte| {
            self.physical_rows
                .iter()
                .enumerate()
                .skip(first)
                .take_while(|(_, row)| row.logical_line == logical)
                .find(|(_, row)| row.bytes.contains(&byte))
                .map(|(index, _)| index)
        })
        .unwrap_or(first)
    }

    #[cfg(test)]
    pub fn index_of_focused_node_on_screen(&self) -> u16 {
        self.focused_line_index()
            .saturating_sub(self.top_visible_line) as u16
    }

    fn focus(&mut self, node: usize) {
        self.focus_byte_range = None;
        self.focused_node = normalize_node(&self.flatjson, node);
        self.focused_node = self.flatjson.first_visible_ancestor(self.focused_node);
        self.absolute_anchor_line = self.layout.nodes[self.focused_node].line;
    }

    fn reveal(&mut self, node: usize, source: Option<usize>) {
        let node = normalize_node(&self.flatjson, node);
        let mut current = node;
        let mut changed = false;
        while let OptionIndex::Index(parent) = self.flatjson[current].parent {
            changed |= self.flatjson[parent].is_collapsed();
            self.flatjson.expand(parent);
            current = parent;
        }
        if changed {
            self.refresh_projection();
        }
        self.focus(node);
        if let Some(source) = source {
            if let Some((line, _)) = self.layout.lines.iter().enumerate().find(|(_, line)| {
                line.spans.iter().any(|span| {
                    span.node == node
                        && span
                            .source
                            .as_ref()
                            .is_some_and(|range| range.contains(&source))
                })
            }) {
                self.absolute_anchor_line = line;
            }
        }
    }

    fn refresh_projection(&mut self) {
        self.focus_byte_range = None;
        self.visible = self.layout.project(&self.flatjson);
        let recovered = self.flatjson.first_visible_ancestor(self.focused_node);
        if recovered != self.focused_node {
            self.focus(recovered);
        }
        self.top_visible_line = self
            .top_visible_line
            .min(self.visible.len().saturating_sub(1));
        self.rebuild_physical_rows();
    }

    fn ensure_visible(&mut self) {
        if self.wrapping_enabled {
            let index = self.focused_physical_row();
            let height = usize::from(self.dimensions.height).max(1);
            let padding = usize::from(self.scrolloff_setting).min((height - 1) / 2);
            if index < self.top_physical_row.saturating_add(padding) {
                self.top_physical_row = index.saturating_sub(padding);
            } else if index >= self.top_physical_row + height.saturating_sub(padding) {
                self.top_physical_row = index
                    .saturating_add(padding + 1)
                    .saturating_sub(height)
                    .min(self.physical_rows.len().saturating_sub(height));
            }
            self.sync_top_indices();
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
    }

    fn vertical(&mut self, count: usize, down: bool) {
        let mut index = self.focused_line_index();
        for _ in 0..count {
            let next = if down {
                ((index + 1)..self.visible.len()).find(|&i| !self.visible[i].line.separator)
            } else {
                (0..index).rev().find(|&i| !self.visible[i].line.separator)
            };
            let Some(next) = next else { break };
            index = next;
        }
        self.focus_line(index, true);
    }

    fn focus_line(&mut self, index: usize, retain_field: bool) {
        let owner = self.visible[index].line.owner;
        let mut node = owner;
        if retain_field
            && self.layout.nodes[self.focused_node].table_cell
            && self.layout.nodes[owner].table_row
        {
            // Match the logical column ordinal, including escaped-equivalent keys.
            if let OptionIndex::Index(parent) = self.flatjson[self.focused_node].parent {
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
                        if self.layout.nodes[candidate].line == self.visible[index].absolute {
                            node = candidate;
                        }
                    }
                }
            }
        }
        self.focus(node);
    }

    fn parent(&mut self) {
        if let OptionIndex::Index(parent) = self.flatjson[self.focused_node].parent {
            self.focus(parent);
        }
    }

    fn parent_or_previous_sibling(&mut self) {
        let current = &self.flatjson[self.focused_node];
        let destination = if current.parent.is_nil() {
            current.prev_sibling
        } else {
            current.parent
        };
        if let OptionIndex::Index(node) = destination {
            self.focus(node);
        }
    }

    fn next_at_parent_level(&mut self) {
        let current = &self.flatjson[self.focused_node];
        let parent_next = match current.parent {
            OptionIndex::Index(parent) => self.flatjson[parent].next_sibling,
            OptionIndex::Nil => OptionIndex::Nil,
        };
        let destination = if parent_next.is_nil() {
            current.next_sibling
        } else {
            parent_next
        };
        if let OptionIndex::Index(node) = destination {
            self.focus(node);
        }
    }

    fn sibling(&mut self, count: usize, next: bool) {
        for _ in 0..count {
            let before = self.focused_node;
            let sibling = if next {
                self.flatjson[before].next_sibling
            } else {
                self.flatjson[before].prev_sibling
            };
            if let OptionIndex::Index(mut node) = sibling {
                while self.flatjson[node].depth < self.desired_depth
                    && self.flatjson[node].is_expanded()
                {
                    let child = if next {
                        self.flatjson[node].first_child()
                    } else if let OptionIndex::Index(pair) = self.flatjson[node].pair_index() {
                        self.flatjson[pair].last_child()
                    } else {
                        OptionIndex::Nil
                    };
                    if let OptionIndex::Index(child) = child {
                        node = child;
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
        if self.layout.nodes[node].collapsible {
            if collapsed {
                self.flatjson.collapse(node);
            } else {
                self.flatjson.expand(node);
            }
        }
    }

    fn toggle_collapsed(&mut self, node: usize) {
        if self.layout.nodes[node].inline_array {
            self.flatjson.expand(node);
            self.expanded_arrays.insert(node);
            self.rebuild_layout();
        } else {
            self.collapse(node, !self.flatjson[node].is_collapsed());
            self.refresh_projection();
        }
    }

    fn collapse_siblings(&mut self, collapsed: bool, deep: bool) {
        let mut node = match self.flatjson[self.focused_node].parent {
            OptionIndex::Index(parent) => self.flatjson[parent].first_child(),
            OptionIndex::Nil => OptionIndex::Index(0),
        };
        while let OptionIndex::Index(current) = node {
            self.collapse(current, collapsed);
            if deep {
                if let OptionIndex::Index(end) = self.flatjson[current].pair_index() {
                    for descendant in current + 1..end {
                        self.collapse(descendant, collapsed);
                    }
                }
            }
            node = self.flatjson[current].next_sibling;
        }
        self.refresh_projection();
    }

    fn scroll(&mut self, count: usize, down: bool) {
        if self.wrapping_enabled {
            let height = usize::from(self.dimensions.height).max(1);
            let last_top = self.physical_rows.len().saturating_sub(height);
            self.top_physical_row = if down {
                self.top_physical_row.saturating_add(count).min(last_top)
            } else {
                self.top_physical_row.saturating_sub(count)
            };
            let focused = self.focused_line_index();
            let focused_first = self
                .physical_rows
                .iter()
                .position(|row| row.logical_line == focused)
                .unwrap_or(0);
            let focused_last = self
                .physical_rows
                .iter()
                .rposition(|row| row.logical_line == focused)
                .unwrap_or(focused_first);
            if focused_last < self.top_physical_row
                || focused_first >= self.top_physical_row.saturating_add(height)
            {
                let target = self
                    .physical_rows
                    .iter()
                    .enumerate()
                    .skip(self.top_physical_row)
                    .take(height)
                    .find(|(_, row)| !self.visible[row.logical_line].line.separator)
                    .map(|(index, _)| index)
                    .unwrap_or(self.top_physical_row);
                self.focus_line(self.physical_rows[target].logical_line, true);
            }
            self.sync_top_indices();
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
            .find(|&i| !self.visible[i].line.separator)
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
            let previous_top = self.top_physical_row;
            let screen_index = self.focused_physical_row().saturating_sub(previous_top);
            let height = usize::from(self.dimensions.height).max(1);
            let last_top = self.physical_rows.len().saturating_sub(height);
            self.top_physical_row = if down {
                previous_top
                    .saturating_add(distance)
                    .min(last_top)
                    .max(previous_top)
            } else {
                previous_top.saturating_sub(distance)
            };
            if self.top_physical_row == previous_top {
                self.vertical(distance, down);
            } else {
                let index = (self.top_physical_row + screen_index)
                    .min(self.physical_rows.len().saturating_sub(1));
                let index = (index..self.physical_rows.len())
                    .find(|&i| {
                        !self.visible[self.physical_rows[i].logical_line]
                            .line
                            .separator
                    })
                    .unwrap_or(index);
                self.focus_line(self.physical_rows[index].logical_line, true);
            }
            self.sync_top_indices();
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
                .find(|&i| !self.visible[i].line.separator)
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
                if self.flatjson[self.focused_node].is_collapsed() {
                    self.collapse(self.focused_node, false);
                    if self.layout.nodes[self.focused_node].inline_array {
                        self.expanded_arrays.insert(self.focused_node);
                        self.rebuild_layout();
                    } else {
                        self.refresh_projection();
                    }
                } else if self.layout.nodes[self.focused_node].inline_array {
                    self.expanded_arrays.insert(self.focused_node);
                    self.rebuild_layout();
                } else if let OptionIndex::Index(child) =
                    self.flatjson[self.focused_node].first_child()
                {
                    self.focus(child);
                }
            }
            Action::MoveLeft => {
                if self.layout.nodes[self.focused_node].collapsible
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
                let mut node = match self.flatjson[self.focused_node].parent {
                    OptionIndex::Index(parent) => self.flatjson[parent].first_child().unwrap(),
                    OptionIndex::Nil => 0,
                };
                if last {
                    if let OptionIndex::Index(parent) = self.flatjson[node].parent {
                        node = self.flatjson[self.flatjson[parent].pair_index().unwrap()]
                            .last_child()
                            .unwrap();
                    } else {
                        while let OptionIndex::Index(next) = self.flatjson[node].next_sibling {
                            node = next;
                        }
                    }
                }
                self.focus(node);
            }
            Action::FocusTop => {
                self.focus(0);
                self.top_visible_line = 0;
                self.top_physical_row = 0;
            }
            Action::FocusBottom => {
                self.focus_line(self.visible.len() - 1, false);
            }
            Action::MoveUpUntilDepthChange | Action::MoveDownUntilDepthChange => {
                self.move_until_depth_change(matches!(action, Action::MoveDownUntilDepthChange));
            }
            Action::JumpTo { line, make_visible } => {
                let line = line.min(self.layout.lines.len() - 1);
                let node = self.layout.lines[line].owner;
                if make_visible {
                    self.reveal(node, None);
                } else {
                    self.focus(node);
                }
            }
            Action::FocusNode { node, source } => self.reveal(node, source),
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
                    self.top_physical_row = self
                        .focused_physical_row()
                        .saturating_sub(padding)
                        .min(self.physical_rows.len().saturating_sub(height));
                    self.sync_top_indices();
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
                let node = self.visible[physical.logical_line].line.owner;
                self.focus(node);
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
        }
        self.sync_top_indices();
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
    use crate::flatjson::{parse_top_level_json, parse_top_level_yaml, PathType};
    fn viewer(input: &str) -> JsonViewer {
        JsonViewer::new(parse_top_level_json(input.into()).unwrap())
    }
    fn path(v: &JsonViewer) -> String {
        v.flatjson
            .build_path_to_node(PathType::Dot, v.focused_node)
            .unwrap()
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
            assert!(v.layout.nodes[0].inline_array);
            v.perform_action(action);
            assert!(!v.layout.nodes[0].inline_array);
            assert!(v.flatjson[0].is_expanded());
        }
        let mut v = viewer(r#"[{"a":1},{"a":2}]"#);
        v.perform_action(Action::MoveRight);
        let row = v.focused_node;
        let original = v.visible[v.focused_line_index()].line.text.clone();
        v.perform_action(Action::ToggleCollapsed);
        assert!(!v.layout.nodes[row].collapsible);
        assert!(v.flatjson[row].is_expanded());
        assert_eq!(v.visible[v.focused_line_index()].line.text, original);
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
            v.visible
                .iter()
                .map(|line| line.line.text.as_str())
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
        assert_eq!(v.visible[6].line.text, "  - 6");
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
        assert_eq!(v.visible[1].line.text, "  - 界");
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
        assert_eq!(v.visible[2].line.text, "    - rust");
        let array = v.focused_node;
        act(&mut v, &[Action::MoveLeft, Action::MoveRight]);
        assert_eq!(v.focused_node, array);
        assert!(!v.layout.nodes[array].inline_array);
        let child = v.flatjson[array].first_child().unwrap();
        let second = v.flatjson[child].next_sibling.unwrap();
        v.perform_action(Action::MoveLeft);
        v.perform_action(Action::FocusNode {
            node: second,
            source: Some(v.flatjson[second].range.start),
        });
        assert_eq!(path(&v), ".box.tags[1]");
        assert_eq!(v.visible[v.focused_line_index()].line.text, "    - cli");
        v.set_viewport(
            TTYDimensions {
                width: 200,
                height: 24,
            },
            false,
        );
        assert_eq!(v.focused_node, second);
        assert!(!v.layout.nodes[array].inline_array);
        assert_eq!(&v.flatjson.1[v.flatjson[second].range.clone()], "\"cli\"");
    }

    #[test]
    fn array_fit_rechecks_line_number_digits_and_number_visibility() {
        let mut fields = vec!["\"wide\":[1,2,3,4,5,6]".to_string()];
        fields.extend((0..94).map(|i| format!("\"k{i}\":0")));
        fields.push("\"tags\":[\"界\",\"é\"]".into());
        let mut v = viewer(&format!("{{{}}}", fields.join(",")));
        v.set_viewport(
            TTYDimensions {
                width: 18,
                height: 24,
            },
            true,
        );
        assert!(v.layout.lines.len() > 99);
        assert_eq!(v.visible.last().unwrap().line.text, "  - é");
        v.set_viewport(
            TTYDimensions {
                width: 18,
                height: 24,
            },
            false,
        );
        assert_eq!(v.visible.last().unwrap().line.text, "tags[2]: 界,é");
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
        assert_eq!(v.visible[1].line.text, "  - .inf  # WARN Non-finite number");
        assert_eq!(v.layout.warnings.len(), 1);
        v.perform_action(Action::MoveLeft);
        assert!(v.visible[0]
            .line
            .text
            .contains("Contains 1 hidden warnings"));
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
        assert_eq!(v.absolute_anchor_line, v.layout.nodes[v.focused_node].line);
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
    fn separator_jumps_skip_annotations_and_empty_roots_remain_selectable() {
        let mut v = JsonViewer::new(parse_top_level_yaml("---\n{}\n---\n42\n".into()).unwrap());
        let first = v.focused_node;
        v.perform_action(Action::MoveDown(1));
        assert_ne!(first, v.focused_node);
        assert!(!v.visible[v.focused_line_index()].line.separator);
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
        assert!(empty.visible[0].line.text.is_empty());
    }

    #[test]
    fn mouse_hits_cells_and_resize_preserves_focus() {
        let mut v = viewer(r#"["界","é",3]"#);
        let child = v.flatjson[0].first_child().unwrap();
        let second = v.flatjson[child].next_sibling.unwrap();
        let span = v.layout.lines[0]
            .spans
            .iter()
            .find(|span| span.node == second)
            .unwrap();
        let col = UnicodeWidthStr::width(&v.layout.lines[0].text[..span.range.start]);
        let (node, source) = crate::lineprinter::hit_test(&v.visible[0].line, col);
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
        assert!(v
            .flatjson
            .0
            .iter()
            .filter(|row| row.is_container())
            .all(|row| row.is_expanded()));
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
        assert!(v.top_physical_row > 0);
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
        let span = v.visible[v.focused_line_index()]
            .line
            .spans
            .iter()
            .find(|span| span.node == node && span.source.is_some())
            .unwrap();
        let display = span.matching_ranges(&span.source.clone().unwrap())[0].clone();
        v.reveal_byte_range(display.clone());
        assert_eq!(v.focused_node, node);
        assert!(v
            .physical_rows
            .iter()
            .enumerate()
            .any(|(index, row)| row.logical_line == v.focused_line_index()
                && row.bytes.start <= display.start
                && display.start < row.bytes.end
                && index >= v.top_physical_row
                && index < v.top_physical_row + usize::from(v.dimensions.height).max(1)));
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
        assert!(v
            .screen_row(0)
            .is_some_and(|row| row.bytes.contains(&row.bytes.end.saturating_sub(1))));
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
        assert_eq!(v.top_physical_row, 5);
        v.perform_action(Action::MoveFocusedLineToCenter);
        assert_eq!(v.top_physical_row, 2);
        v.perform_action(Action::MoveFocusedLineToBottom);
        assert_eq!(v.top_physical_row, 0);
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
}
