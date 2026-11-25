use super::{LineViewport, OffsetMode, PaintWindow};
use crate::terminal::{Style, Terminal};
use crate::theme::{StyleRole, StyleState, Theme};
use crate::toon_display::DisplayLine;
use crate::wrapped_view::PhysicalRow;
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Paint one logical line or a wrapped part of it using the same focus and search styles.
pub struct LinePainter<'a> {
    pub(super) line: &'a DisplayLine,
    pub(super) focused: Range<usize>,
    pub(super) theme: Option<&'a Theme>,
    pub(super) matches: &'a [Range<usize>],
    pub(super) current: &'a Range<usize>,
}

impl<'a> LinePainter<'a> {
    pub fn new(line: &'a DisplayLine, focused: Range<usize>) -> Self {
        Self {
            line,
            focused,
            theme: None,
            matches: &[],
            current: &(0..0),
        }
    }

    pub fn with_theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    pub fn with_search(mut self, matches: &'a [Range<usize>], current: &'a Range<usize>) -> Self {
        self.matches = matches;
        self.current = current;
        self
    }

    pub fn paint(
        &self,
        terminal: &mut impl Terminal,
        viewport: LineViewport,
        width: usize,
    ) -> std::fmt::Result {
        if width == 0 {
            return Ok(());
        }
        let PaintWindow {
            left,
            right,
            available,
        } = viewport.paint_window(self.line, width);
        let ellipsis_style = self.theme.map_or_else(Style::default, |theme| {
            theme.style_on_row(
                StyleRole::Ellipsis,
                StyleState::main(),
                !self.focused.is_empty(),
            )
        });
        if left > 0 {
            terminal.set_style(&ellipsis_style)?;
            terminal.write_char('…')?;
        }
        let used = self.paint_visible_graphemes(terminal, viewport, available)?;
        if right > 0 && left + used < width {
            terminal.set_style(&ellipsis_style)?;
            terminal.write_char('…')?;
        }
        terminal.reset_style()
    }

    fn paint_visible_graphemes(
        &self,
        terminal: &mut impl Terminal,
        viewport: LineViewport,
        available: usize,
    ) -> Result<usize, std::fmt::Error> {
        let offset = viewport.removed_indentation + viewport.horizontal_offset;
        let mut column = 0;
        let mut used = 0;
        for (byte, grapheme) in self.line.text.grapheme_indices(true) {
            let cells = UnicodeWidthStr::width(grapheme);
            let start = column;
            column += cells;
            if start < offset {
                let padding = match viewport.mode {
                    OffsetMode::Line => 0,
                    OffsetMode::Table => column
                        .saturating_sub(offset)
                        .min(available.saturating_sub(used)),
                };
                write_padding(terminal, padding)?;
                used += padding;
                continue;
            }
            if used + cells > available {
                break;
            }
            self.paint_grapheme(terminal, byte, grapheme)?;
            used += cells;
        }
        Ok(used)
    }

    pub fn paint_wrapped(
        &self,
        terminal: &mut impl Terminal,
        row: &PhysicalRow,
        width: usize,
    ) -> std::fmt::Result {
        if width == 0 {
            return Ok(());
        }
        if row.placeholder {
            terminal.set_style(&self.style_at(row.bytes.start, row.bytes.len()))?;
            terminal.write_char('…')?;
        } else {
            for (offset, grapheme) in self.line.text[row.bytes.clone()].grapheme_indices(true) {
                self.paint_grapheme(terminal, row.bytes.start + offset, grapheme)?;
            }
        }
        terminal.reset_style()
    }

    fn paint_grapheme(
        &self,
        terminal: &mut impl Terminal,
        byte: usize,
        grapheme: &str,
    ) -> std::fmt::Result {
        terminal.set_style(&self.style_at(byte, grapheme.len()))?;
        terminal.write_str(grapheme)
    }
}

fn write_padding(terminal: &mut impl Terminal, cells: usize) -> std::fmt::Result {
    for _ in 0..cells {
        terminal.write_char(' ')?;
    }
    Ok(())
}
