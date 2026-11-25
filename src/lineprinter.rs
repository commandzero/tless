//! Cell-aware painting of mapped TOON tokens. Generated annotations have no search source.
use crate::toon_display::DisplayLine;
use regex::Regex;
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

mod annotations;
mod paint;
mod style;

pub use annotations::fit_annotations;
pub use paint::LinePainter;

lazy_static::lazy_static! {
    pub static ref JS_IDENTIFIER: Regex = Regex::new("^[_$a-zA-Z][_$a-zA-Z0-9]*$").unwrap();
}

/// Indentation suppression and horizontal scrolling have different semantics:
/// only scrolling hides document content and earns a leading ellipsis.
#[derive(Clone, Copy, Debug)]
pub struct LineViewport {
    pub horizontal_offset: usize,
    pub removed_indentation: usize,
    mode: OffsetMode,
}

/// A shared table offset is a cell coordinate, never snapped separately for
/// each row. A cut wide grapheme occupies blank cells in its original position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OffsetMode {
    Line,
    Table,
}

impl LineViewport {
    pub fn new(line: &DisplayLine, horizontal_offset: usize, indentation_reduction: usize) -> Self {
        let removed_indentation = line
            .text
            .bytes()
            .take(indentation_reduction)
            .take_while(|byte| *byte == b' ')
            .count();
        // Painting skips a grapheme when a requested cell offset cuts through it.
        // Resolve that effective boundary once so mouse and reveal coordinates
        // use the same first visible cell, including after indentation removal.
        let horizontal_offset = if horizontal_offset == 0 {
            0
        } else {
            let requested_start = removed_indentation.saturating_add(horizontal_offset);
            let mut aligned_start = 0;
            for grapheme in line.text.graphemes(true) {
                if aligned_start >= requested_start {
                    break;
                }
                aligned_start += UnicodeWidthStr::width(grapheme);
            }
            aligned_start
                .max(requested_start)
                .saturating_sub(removed_indentation)
        };
        Self {
            horizontal_offset,
            removed_indentation,
            mode: OffsetMode::Line,
        }
    }

    pub fn table(
        line: &DisplayLine,
        horizontal_offset: usize,
        reduction: usize,
        indentation: usize,
    ) -> Self {
        let removed_indentation = line
            .text
            .bytes()
            .take(reduction.min(indentation))
            .take_while(|byte| *byte == b' ')
            .count();
        Self {
            horizontal_offset,
            removed_indentation,
            mode: OffsetMode::Table,
        }
    }

    pub fn source_column(self, viewport_column: usize) -> usize {
        self.removed_indentation
            + self.horizontal_offset
            + viewport_column.saturating_sub(usize::from(self.horizontal_offset > 0))
    }

    /// Resolve only painted cells; clipping markers and the remainder of a
    /// grapheme cut by a shared table offset have no source token.
    pub fn hit_test(
        self,
        line: &DisplayLine,
        column: usize,
        width: usize,
    ) -> (usize, Option<usize>) {
        let window = self.paint_window(line, width);
        if column < window.left || column >= window.left + window.available {
            return (line.owner, None);
        }
        let hit = grapheme_at_column(&line.text, self.source_column(column));
        let offset = self.removed_indentation + self.horizontal_offset;
        if self.mode == OffsetMode::Table && hit.is_some_and(|(_, start)| start < offset) {
            return (line.owner, None);
        }
        let byte = hit.map_or(line.text.len(), |(byte, _)| byte);
        hit_at_byte(line, Some(byte))
    }

    pub fn reduced_column(self, source_column: usize) -> usize {
        source_column.saturating_sub(self.removed_indentation)
    }

    fn paint_window(self, line: &DisplayLine, width: usize) -> PaintWindow {
        let offset = self.removed_indentation + self.horizontal_offset;
        // Inspect only the requested window plus one cell; long inline arrays must
        // not be measured in full on every redraw at their left edge.
        let mut total = 0;
        for grapheme in line.text.graphemes(true) {
            total += UnicodeWidthStr::width(grapheme);
            if total > offset.saturating_add(width) {
                break;
            }
        }
        let left = usize::from(self.horizontal_offset > 0);
        let right = usize::from(total > offset.saturating_add(width.saturating_sub(left)));
        let available = width.saturating_sub(left + right);
        PaintWindow {
            left,
            right,
            available,
        }
    }

    pub fn visible_columns(self, line: &DisplayLine, width: usize) -> Range<usize> {
        self.horizontal_offset
            ..self
                .horizontal_offset
                .saturating_add(self.paint_window(line, width).available)
    }

    pub fn content_width(self, line: &DisplayLine) -> usize {
        UnicodeWidthStr::width(line.text.as_str()).saturating_sub(self.removed_indentation)
    }
}

/// Document cells remaining after the clipping markers actually needed by a line.
struct PaintWindow {
    left: usize,
    right: usize,
    available: usize,
}

pub fn hit_test_wrapped(
    line: &DisplayLine,
    row: &crate::wrapped_view::PhysicalRow,
    column: usize,
) -> (usize, Option<usize>) {
    let byte = if row.placeholder {
        (column == 0).then_some(row.bytes.start)
    } else {
        grapheme_at_column(&line.text[row.bytes.clone()], column)
            .map(|(byte, _)| row.bytes.start + byte)
    };
    hit_at_byte(line, byte)
}

/// Resolve a terminal cell to its parsed owner and optional source-token anchor.
#[cfg(test)]
pub fn hit_test(line: &DisplayLine, column: usize) -> (usize, Option<usize>) {
    let byte = grapheme_at_column(&line.text, column).map_or(line.text.len(), |(byte, _)| byte);
    hit_at_byte(line, Some(byte))
}

fn grapheme_at_column(text: &str, column: usize) -> Option<(usize, usize)> {
    let mut start = 0;
    for (byte, grapheme) in text.grapheme_indices(true) {
        let end = start + UnicodeWidthStr::width(grapheme);
        if end > column {
            return Some((byte, start));
        }
        start = end;
    }
    None
}

fn hit_at_byte(line: &DisplayLine, byte: Option<usize>) -> (usize, Option<usize>) {
    let span = byte.and_then(|byte| line.spans.iter().find(|span| span.range.contains(&byte)));
    (
        span.map_or(line.owner, |span| span.node),
        span.and_then(|span| span.source.as_ref().map(|source| source.start)),
    )
}

#[cfg(test)]
mod tests;
