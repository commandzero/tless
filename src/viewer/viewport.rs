use super::{Cursor, JsonViewer};
use std::ops::Range;

impl JsonViewer {
    pub(super) fn walk(&self, cursor: Cursor, count: usize, down: bool) -> Cursor {
        if !self.wrapping_enabled {
            let logical = if down {
                cursor
                    .logical
                    .saturating_add(count)
                    .min(self.visible.len().saturating_sub(1))
            } else {
                cursor.logical.saturating_sub(count)
            };
            return Cursor {
                logical,
                continuation: 0,
            };
        }
        if down {
            self.walk_wrapped_down(cursor, count)
        } else {
            self.walk_wrapped_up(cursor, count)
        }
    }

    fn walk_wrapped_up(&self, mut cursor: Cursor, mut count: usize) -> Cursor {
        while count > cursor.continuation {
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
        }
        cursor.continuation -= count;
        cursor
    }

    fn walk_wrapped_down(&self, mut cursor: Cursor, mut count: usize) -> Cursor {
        while count > 0 {
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
        cursor
    }

    pub(super) fn clamp_wrapped_top(&mut self) {
        self.refresh_frame();
        let height = usize::from(self.dimensions.height).max(1);
        if self.physical_rows.len() < height {
            let missing = height - self.physical_rows.len();
            self.set_top(self.walk(self.top_cursor(), missing, false));
            self.refresh_frame();
        }
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
        let Some((first, last)) = self.rows_covering_range(logical, &range) else {
            return;
        };
        self.focus_byte_range = Some(range);
        self.reveal_rows(first, last);
        self.refresh_frame();
    }

    fn rows_covering_range(
        &self,
        logical: usize,
        range: &Range<usize>,
    ) -> Option<(Cursor, Cursor)> {
        let line = self.row_text(logical);
        if range.start > line.text.len() || range.end > line.text.len() {
            return None;
        }
        let mut matches = self
            .row_iter(logical, &line)
            .enumerate()
            .filter(|(_, row)| row_covers_range(&row.bytes, range))
            .map(|(index, _)| Cursor {
                logical,
                continuation: index,
            });
        let first = matches.next()?;
        let last = matches.last().unwrap_or(first);
        Some((first, last))
    }

    fn reveal_rows(&mut self, first: Cursor, last: Cursor) {
        let height = usize::from(self.dimensions.height).max(1);
        let padding = usize::from(self.scrolloff_setting).min((height - 1) / 2);
        let top = self.top_cursor();
        if first >= self.walk(top, padding, true)
            && last < self.walk(top, height.saturating_sub(padding), true)
        {
            return;
        }
        let mut top = self.walk(first, padding, false);
        if last.continuation - first.continuation < height && last >= self.walk(top, height, true) {
            top = self.walk(last, height - 1, false);
        }
        self.set_top(top);
        if self.wrapping_enabled {
            self.clamp_wrapped_top();
        }
    }

    pub(super) fn ensure_visible(&mut self) {
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

    pub(super) fn scroll(&mut self, count: usize, down: bool) {
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

    pub(super) fn jump(&mut self, distance: usize, down: bool) {
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
}

fn row_covers_range(row: &Range<usize>, range: &Range<usize>) -> bool {
    if range.start == range.end {
        row.start <= range.start && range.start <= row.end
    } else {
        range.start < row.end && row.start < range.end
    }
}
