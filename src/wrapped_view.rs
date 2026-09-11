//! A cell-aware projection from logical TOON lines to physical screen rows.
//!
//! The projection owns no text.  Each row points back into the `DisplayLine`
//! that produced it, so wrapping cannot change copy, search, or serialization
//! coordinates.

use crate::toon_display::VisibleLine;
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// One physical terminal row belonging to a logical display line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalRow {
    /// Index into the current `JsonViewer::visible` projection.
    pub logical_line: usize,
    /// Byte range in `visible[logical_line].line.text`.
    pub bytes: Range<usize>,
    /// Cell offset in the original logical line, including its indentation.
    pub cell_start: usize,
    /// Whether this is the first physical row for `logical_line`.
    pub first: bool,
    /// The row represents a grapheme wider than the available width.  The
    /// renderer paints an ellipsis instead of the grapheme for this row.
    pub placeholder: bool,
}

/// Build physical rows for a logical visibility projection.
///
/// `width` is the document width after gutters. `indentation` is the number
/// of leading indentation cells already removed by the screen writer. The
/// corresponding leading spaces are omitted from the first wrapped row, and
/// continuation rows begin at the document area's left edge. `eligible`
/// identifies expanded lines; collapsed previews and separators remain one
/// physical row even when wrapping is enabled.
pub fn build_physical_rows(
    visible: &[VisibleLine],
    width: usize,
    indentation: usize,
    wrapping_enabled: bool,
    eligible: &[bool],
) -> Vec<PhysicalRow> {
    let mut rows = Vec::new();
    for (logical_line, visible_line) in visible.iter().enumerate() {
        let line = &visible_line.line;
        let wrap = wrapping_enabled
            && eligible.get(logical_line).copied().unwrap_or(false)
            && !line.separator;
        if !wrap {
            rows.push(PhysicalRow {
                logical_line,
                bytes: 0..line.text.len(),
                cell_start: 0,
                first: true,
                placeholder: false,
            });
            continue;
        }
        rows.extend(wrap_line(
            logical_line,
            line.text.as_str(),
            width,
            indentation,
        ));
    }
    rows
}

fn wrap_line(
    logical_line: usize,
    text: &str,
    width: usize,
    indentation: usize,
) -> Vec<PhysicalRow> {
    let mut start_byte = 0;
    let mut start_cell = 0;
    let mut removed = 0;
    for (byte, grapheme) in text.grapheme_indices(true) {
        if removed >= indentation || !grapheme.chars().all(|ch| ch == ' ') {
            start_byte = byte;
            start_cell = removed;
            break;
        }
        removed += UnicodeWidthStr::width(grapheme);
        start_byte = byte + grapheme.len();
        start_cell = removed;
    }
    if text.is_empty() {
        start_byte = 0;
        start_cell = 0;
    }

    // A zero-cell document area cannot display text, but still needs a finite
    // row for every logical line so the viewer can select and scroll it.
    if width == 0 {
        return vec![PhysicalRow {
            logical_line,
            bytes: start_byte..text.len(),
            cell_start: start_cell,
            first: true,
            placeholder: false,
        }];
    }

    let mut rows = Vec::new();
    let mut row_start = start_byte;
    let mut row_cell_start = start_cell;
    let mut used = 0;
    let mut first = true;
    let mut had_grapheme = false;
    for (relative_byte, grapheme) in text[start_byte..].grapheme_indices(true) {
        let byte = start_byte + relative_byte;
        let end = byte + grapheme.len();
        let cells = UnicodeWidthStr::width(grapheme);
        had_grapheme = true;

        if cells > width {
            if row_start < byte || used > 0 {
                rows.push(PhysicalRow {
                    logical_line,
                    bytes: row_start..byte,
                    cell_start: row_cell_start,
                    first,
                    placeholder: false,
                });
                first = false;
            }
            rows.push(PhysicalRow {
                logical_line,
                bytes: byte..end,
                cell_start: row_cell_start + used,
                first,
                placeholder: true,
            });
            first = false;
            row_start = end;
            row_cell_start += used + cells;
            used = 0;
        } else if used.saturating_add(cells) <= width {
            used += cells;
        } else {
            rows.push(PhysicalRow {
                logical_line,
                bytes: row_start..byte,
                cell_start: row_cell_start,
                first,
                placeholder: false,
            });
            first = false;
            row_start = byte;
            row_cell_start += used;
            used = cells;
        }
    }

    if row_start < text.len() || used > 0 || !had_grapheme || rows.is_empty() {
        rows.push(PhysicalRow {
            logical_line,
            bytes: row_start..text.len(),
            cell_start: row_cell_start,
            first,
            placeholder: false,
        });
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toon_display::{DisplayLine, VisibleLine};

    fn line(text: &str) -> Vec<VisibleLine> {
        vec![VisibleLine {
            absolute: 0,
            line: DisplayLine {
                text: text.into(),
                spans: vec![],
                owner: 0,
                separator: false,
            },
        }]
    }

    fn rows(text: &str, width: usize, indentation: usize) -> Vec<PhysicalRow> {
        build_physical_rows(&line(text), width, indentation, true, &[true])
    }

    #[test]
    fn wraps_greedily_at_grapheme_boundaries() {
        let result = rows("ab界é", 3, 0);
        assert_eq!(result.len(), 2);
        assert_eq!(&line("ab界é")[0].line.text[result[0].bytes.clone()], "ab");
        assert_eq!(&line("ab界é")[0].line.text[result[1].bytes.clone()], "界é");
        assert_eq!(result[0].cell_start, 0);
        assert_eq!(result[1].cell_start, 2);
        assert!(result[0].first);
        assert!(!result[1].first);
    }

    #[test]
    fn exact_fit_stays_on_one_row() {
        let result = rows("abc", 3, 0);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].bytes, 0..3);
        assert_eq!(result[0].cell_start, 0);
        assert!(!result[0].placeholder);

        let wide = rows("界", 2, 0);
        assert_eq!(wide.len(), 1);
        assert_eq!(wide[0].bytes, 0..3);
        assert!(!wide[0].placeholder);
    }

    #[test]
    fn keeps_combining_sequences_together() {
        let result = rows("xéy", 1, 0);
        assert_eq!(result.len(), 3);
        assert_eq!(&line("xéy")[0].line.text[result[1].bytes.clone()], "é");
    }

    #[test]
    fn keeps_emoji_zwj_graphemes_intact() {
        let text = "a👩‍💻b";
        let emoji_width = UnicodeWidthStr::width("👩‍💻");
        let result = rows(text, emoji_width, 0);
        assert_eq!(result.len(), 3);
        let source = &line(text)[0].line.text;
        assert_eq!(&source[result[0].bytes.clone()], "a");
        assert_eq!(&source[result[1].bytes.clone()], "👩‍💻");
        assert_eq!(&source[result[2].bytes.clone()], "b");
        assert!(result.iter().all(|row| !row.placeholder));
    }

    #[test]
    fn escaped_control_text_wraps_as_literal_display_text() {
        let text = r#"value: \n\t\u001b[31m"#;
        let result = rows(text, 4, 0);
        let source = &line(text)[0].line.text;
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
        assert_eq!(result.len(), 3);
        assert!(!result[0].placeholder);
        assert!(result[1].placeholder);
        assert_eq!(result[1].bytes, 1..4);
        assert_eq!(result[2].bytes, 4..5);
        assert_eq!(result[2].cell_start, 3);
    }

    #[test]
    fn indentation_is_removed_only_from_the_first_row() {
        let result = rows("    abcdef", 5, 2);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].bytes, 2..7);
        assert_eq!(result[1].bytes, 7..10);
        assert_eq!(result[0].cell_start, 2);
        assert_eq!(result[1].cell_start, 7);
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
        assert_eq!(result.len(), text.len());
        assert!(
            result
                .iter()
                .enumerate()
                .all(|(index, row)| { row.bytes == (index..index + 1) && row.cell_start == index })
        );
        assert!(result.first().is_some_and(|row| row.first));
        assert!(result.iter().skip(1).all(|row| !row.first));
    }

    #[test]
    fn collapsed_and_separator_lines_stay_single_rows() {
        let mut visible = line("abcdef");
        visible.push(VisibleLine {
            absolute: 1,
            line: DisplayLine {
                text: "ghijkl".into(),
                spans: vec![],
                owner: 0,
                separator: true,
            },
        });
        let result = build_physical_rows(&visible, 2, 0, true, &[false, true]);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].bytes, 0..6);
        assert_eq!(result[1].bytes, 0..6);
    }

    #[test]
    fn disabled_wrapping_keeps_full_logical_rows() {
        let result = build_physical_rows(&line("abcdef"), 2, 0, false, &[true]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].bytes, 0..6);
    }
}
