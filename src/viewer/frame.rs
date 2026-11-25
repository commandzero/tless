use super::Cursor;
use super::JsonViewer;
use crate::toon_display::DisplayLine;
use crate::types::TTYDimensions;
use crate::wrapped_view::PhysicalRow;
use crate::wrapped_view::rows;
use std::ops::Range;

impl JsonViewer {
    pub(super) fn wrap_eligible(&self, logical_line: usize) -> bool {
        let Some(visible) = self.visible_line(logical_line) else {
            return false;
        };
        !self.is_document_header(logical_line) && !self.effective_collapsed(visible.owner)
    }

    pub(super) fn top_logical_line(&self) -> usize {
        self.top_visible_line
    }

    pub(super) fn top_cursor(&self) -> Cursor {
        Cursor {
            logical: self.top_visible_line,
            continuation: self.top_continuation,
        }
    }

    pub(super) fn set_top(&mut self, cursor: Cursor) {
        self.top_visible_line = cursor.logical;
        self.top_continuation = cursor.continuation;
    }

    pub fn render_line(&self, logical: usize) -> DisplayLine {
        let visible = self.visible_line(logical).unwrap();
        let alignment = self
            .aligned_table_for_line(logical)
            .and_then(|table| self.aligned_tables.get(&table));
        self.layout
            .render(&self.flatjson, visible, self.focused_node, alignment)
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

    pub(super) fn row_text(&self, logical: usize) -> std::borrow::Cow<'_, DisplayLine> {
        match self.frame.iter().find(|(index, _)| *index == logical) {
            Some((_, line)) => std::borrow::Cow::Borrowed(line),
            None => std::borrow::Cow::Owned(self.render_line(logical)),
        }
    }

    pub(super) fn row_iter<'a>(
        &self,
        logical: usize,
        line: &'a DisplayLine,
    ) -> crate::wrapped_view::Rows<'a> {
        rows(
            logical,
            &line.text,
            self.wrap_width,
            self.wrap_indentation,
            self.is_wrapped_line(logical),
        )
    }

    pub(super) fn refresh_frame(&mut self) {
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

    pub(super) fn reset_frame_geometry(&mut self) {
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
        self.wrapping_enabled
            && self.aligned_table_for_line(logical_line).is_none()
            && self.wrap_eligible(logical_line)
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
}
