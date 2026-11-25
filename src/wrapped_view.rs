//! A cell-aware projection from logical TOON lines to physical screen rows.
//!
//! The projection owns no text.  Each row points back into the `DisplayLine`
//! that produced it, so wrapping cannot change copy, search, or serialization
//! coordinates.

use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// One physical terminal row belonging to a logical display line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalRow {
    /// Index into the current `JsonViewer::visible` projection.
    pub logical_line: usize,
    /// Byte range in the materialized logical line's text.
    pub bytes: Range<usize>,
    /// Cell offset in the original logical line, including its indentation.
    pub cell_start: usize,
    /// Whether this is the first physical row for `logical_line`.
    pub first: bool,
    /// The row represents a grapheme wider than the available width.  The
    /// renderer paints an ellipsis instead of the grapheme for this row.
    pub placeholder: bool,
}

#[derive(Clone, Copy)]
pub struct Wrap {
    pub width: usize,
    pub indentation: usize,
}

/// Row-local iteration retains no continuation vector, even for a long value.
pub fn rows(logical_line: usize, text: &str, wrap: Option<Wrap>) -> Rows<'_> {
    let removed = wrap.map_or(0, |wrap| removable_indentation(text, wrap.indentation));
    Rows {
        text,
        logical_line,
        width: wrap.map_or(0, |wrap| wrap.width),
        byte: removed,
        cell: removed,
        first: true,
        done: false,
    }
}

fn removable_indentation(text: &str, indentation: usize) -> usize {
    text.graphemes(true)
        .take(indentation)
        .take_while(|grapheme| grapheme.chars().all(|ch| ch == ' '))
        .map(str::len)
        .sum()
}

pub struct Rows<'a> {
    text: &'a str,
    logical_line: usize,
    width: usize,
    byte: usize,
    cell: usize,
    first: bool,
    done: bool,
}

impl Iterator for Rows<'_> {
    type Item = PhysicalRow;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let start = self.byte;
        let cell_start = self.cell;
        let first = self.first;
        let (length, used, placeholder) = row_extent(&self.text[start..], self.width);
        let end = start + length;
        self.byte = end;
        self.cell += used;
        self.first = false;
        self.done = end == self.text.len();
        Some(PhysicalRow {
            logical_line: self.logical_line,
            bytes: start..end,
            cell_start,
            first,
            placeholder,
        })
    }
}

fn row_extent(text: &str, width: usize) -> (usize, usize, bool) {
    if width == 0 {
        return (text.len(), 0, false);
    }

    let mut used: usize = 0;
    for (offset, grapheme) in text.grapheme_indices(true) {
        let cells = UnicodeWidthStr::width(grapheme);
        if offset == 0 && cells > width {
            return (grapheme.len(), cells, true);
        }
        if used.saturating_add(cells) > width {
            return (offset, used, false);
        }
        used += cells;
    }
    (text.len(), used, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(text: &str, width: usize, indentation: usize) -> Vec<PhysicalRow> {
        super::rows(0, text, Some(Wrap { width, indentation })).collect()
    }

    fn contents<'a>(text: &'a str, rows: &[PhysicalRow]) -> Vec<&'a str> {
        rows.iter().map(|row| &text[row.bytes.clone()]).collect()
    }

    fn geometry(rows: &[PhysicalRow]) -> Vec<(Range<usize>, usize, bool, bool)> {
        rows.iter()
            .map(|row| {
                (
                    row.bytes.clone(),
                    row.cell_start,
                    row.first,
                    row.placeholder,
                )
            })
            .collect()
    }

    #[test]
    fn wraps_greedily_at_grapheme_boundaries() {
        let result = rows("ab界é", 3, 0);
        assert_eq!(
            geometry(&result),
            [(0..2, 0, true, false), (2..8, 2, false, false)]
        );
        assert_eq!(contents("ab界é", &result), ["ab", "界é"]);
    }

    #[test]
    fn exact_fit_stays_on_one_row() {
        for (text, width) in [("abc", 3), ("界", 2)] {
            assert_eq!(geometry(&rows(text, width, 0)), [(0..3, 0, true, false)]);
        }
    }

    #[test]
    fn keeps_combining_sequences_together() {
        let result = rows("xéy", 1, 0);
        assert_eq!(result.len(), 3);
        assert_eq!(&"xéy"[result[1].bytes.clone()], "é");
    }

    #[test]
    fn keeps_emoji_zwj_graphemes_intact() {
        let text = "a👩‍💻b";
        let emoji_width = UnicodeWidthStr::width("👩‍💻");
        let result = rows(text, emoji_width, 0);
        assert_eq!(contents(text, &result), ["a", "👩‍💻", "b"]);
        assert!(result.iter().all(|row| !row.placeholder));
    }

    #[test]
    fn escaped_control_text_wraps_as_literal_display_text() {
        let text = r#"value: \n\t\u001b[31m"#;
        let result = rows(text, 4, 0);
        let source = text;
        let joined: String = result
            .iter()
            .map(|row| &source[row.bytes.clone()])
            .collect();
        assert_eq!(joined, text);
        assert!(joined.chars().all(|ch| !ch.is_control()));
        assert!(result.iter().all(|row| !row.placeholder));
    }

    #[test]
    fn overwide_graphemes_use_placeholder_rows() {
        let result = rows("a界b", 1, 0);
        assert_eq!(
            geometry(&result),
            [
                (0..1, 0, true, false),
                (1..4, 1, false, true),
                (4..5, 3, false, false),
            ]
        );
    }

    #[test]
    fn indentation_is_removed_only_from_the_first_row() {
        let result = rows("    abcdef", 5, 2);
        assert_eq!(
            geometry(&result),
            [(2..7, 2, true, false), (7..10, 7, false, false)]
        );
    }

    #[test]
    fn zero_width_and_empty_lines_have_one_row() {
        let result = rows("abc", 0, 0);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].bytes, 0..3);
        let empty = rows("", 4, 0);
        assert_eq!(empty.len(), 1);
        assert_eq!(empty[0].bytes, 0..0);
    }

    #[test]
    fn large_narrow_lines_make_forward_progress() {
        let text = "x".repeat(4096);
        let result = rows(&text, 1, 0);
        let expected: Vec<_> = (0..text.len())
            .map(|index| (index..index + 1, index, index == 0, false))
            .collect();
        assert_eq!(geometry(&result), expected);
    }

    #[test]
    fn disabled_wrapping_keeps_full_logical_rows() {
        let result: Vec<_> = super::rows(0, "abcdef", None).collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].bytes, 0..6);
    }
}
