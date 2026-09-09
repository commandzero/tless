//! Cell-aware painting of mapped TOON tokens. Generated annotations have no search source.
use crate::terminal::{self, Style, Terminal};
use crate::toon_display::{DisplayLine, TokenRole};
use regex::Regex;
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

lazy_static::lazy_static! {
    pub static ref JS_IDENTIFIER: Regex = Regex::new("^[_$a-zA-Z][_$a-zA-Z0-9]*$").unwrap();
}

pub fn paint(
    terminal: &mut impl Terminal,
    line: &DisplayLine,
    focused: usize,
    offset: usize,
    width: usize,
    matches: &[Range<usize>],
    current: &Range<usize>,
) -> std::fmt::Result {
    // Inspect only the requested window plus one cell; long inline arrays must
    // not be measured in full on every redraw at their left edge.
    let mut total = 0;
    for grapheme in line.text.graphemes(true) {
        total += UnicodeWidthStr::width(grapheme);
        if total > offset.saturating_add(width) {
            break;
        }
    }
    let focused_spans: Vec<_> = line
        .spans
        .iter()
        .filter(|span| span.node == focused)
        .collect();
    let left = usize::from(offset > 0);
    let right = usize::from(total > offset.saturating_add(width.saturating_sub(left)));
    let available = width.saturating_sub(left + right);
    if width == 0 {
        return Ok(());
    }
    if left > 0 {
        terminal.reset_style()?;
        terminal.write_char('…')?;
    }
    let mut column = 0;
    let mut used = 0;
    for (byte, grapheme) in line.text.grapheme_indices(true) {
        let cells = UnicodeWidthStr::width(grapheme);
        let start = column;
        column += cells;
        if start < offset {
            continue;
        }
        if used + cells > available {
            break;
        }
        let span = focused_spans
            .iter()
            .copied()
            .find(|span| span.range.contains(&byte))
            .or_else(|| line.spans.iter().find(|span| span.range.contains(&byte)));
        let mut style = Style::default();
        if let Some(span) = span {
            style.fg = match span.role {
                TokenRole::Key => terminal::BLUE,
                TokenRole::String => terminal::GREEN,
                TokenRole::Number => terminal::MAGENTA,
                TokenRole::Boolean => terminal::YELLOW,
                TokenRole::Null => terminal::LIGHT_BLACK,
                TokenRole::Warning => terminal::YELLOW,
                TokenRole::Count | TokenRole::Preview => terminal::LIGHT_BLACK,
                TokenRole::Structure => terminal::DEFAULT,
            };
            style.dimmed = matches!(
                span.role,
                TokenRole::Preview | TokenRole::Count | TokenRole::Warning
            );
            style.bold = span.node == focused;
            if let Some(source) = &span.source {
                // For unchanged tokens retain character-level matching; normalized tokens
                // highlight as a unit because byte offsets need not survive normalization.
                let source = if source.len() == span.range.len() {
                    source.start + byte - span.range.start
                        ..source.start + byte - span.range.start + grapheme.len()
                } else {
                    source.clone()
                };
                let overlaps =
                    |range: &Range<usize>| range.start < source.end && source.start < range.end;
                if matches.iter().any(overlaps) {
                    style.inverted = true;
                }
                if overlaps(current) {
                    style.bg = terminal::YELLOW;
                    style.fg = terminal::DEFAULT;
                    style.bold = true;
                }
            }
        }
        terminal.set_style(&style)?;
        terminal.write_str(grapheme)?;
        used += cells;
    }
    terminal.reset_style()?;
    if right > 0 && left + used < width {
        terminal.write_char('…')?;
    }
    Ok(())
}

/// Reserve visible space for counts and the final warning before allocating preview cells.
/// Byte spans are adjusted together with text so mouse ownership remains unchanged.
pub fn fit_annotations(line: &DisplayLine, width: usize) -> std::borrow::Cow<'_, DisplayLine> {
    let Some(preview) = line
        .spans
        .iter()
        .find(|span| span.role == TokenRole::Preview)
    else {
        return std::borrow::Cow::Borrowed(line);
    };
    let total = UnicodeWidthStr::width(line.text.as_str());
    if total <= width {
        return std::borrow::Cow::Borrowed(line);
    }
    let range = preview.range.clone();
    let preview_width = UnicodeWidthStr::width(&line.text[range.clone()]);
    let allowed = width.saturating_sub(total - preview_width);
    let mut replacement = String::new();
    if allowed > 0 {
        let mut used = 0;
        for grapheme in line.text[range.clone()].graphemes(true) {
            let cells = UnicodeWidthStr::width(grapheme);
            if used + cells >= allowed {
                break;
            }
            replacement.push_str(grapheme);
            used += cells;
        }
        replacement.push('…');
    }
    let mut clipped = line.clone();
    clipped.text.replace_range(range.clone(), &replacement);
    for span in &mut clipped.spans {
        if span.range == range {
            span.range.end = span.range.start + replacement.len();
        } else if span.range.start >= range.end {
            span.range.start = span.range.start - range.end + range.start + replacement.len();
            span.range.end = span.range.end - range.end + range.start + replacement.len();
        }
    }
    std::borrow::Cow::Owned(clipped)
}

/// Resolve a terminal cell to its parsed owner and optional source-token anchor.
pub fn hit_test(line: &DisplayLine, column: usize) -> (usize, Option<usize>) {
    let mut cells = 0;
    let byte = line
        .text
        .grapheme_indices(true)
        .find_map(|(byte, grapheme)| {
            cells += UnicodeWidthStr::width(grapheme);
            if cells > column {
                Some(byte)
            } else {
                None
            }
        })
        .unwrap_or(line.text.len());
    let span = line.spans.iter().find(|span| span.range.contains(&byte));
    (
        span.map_or(line.owner, |span| span.node),
        span.and_then(|span| span.source.as_ref().map(|source| source.start)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flatjson::{parse_top_level_json, parse_top_level_yaml};
    use crate::terminal::test::{TextOnlyTerminal, VisibleEscapesTerminal};
    use crate::toon_display::Layout;
    fn text(line: &DisplayLine, width: usize, offset: usize) -> String {
        let mut terminal = TextOnlyTerminal::new();
        paint(&mut terminal, line, usize::MAX, offset, width, &[], &(0..0)).unwrap();
        terminal.output().to_string()
    }
    #[test]
    fn clipping_uses_grapheme_cell_boundaries_and_horizontal_scrolling() {
        let flat = parse_top_level_json(r#"["界","é",3]"#.into()).unwrap();
        let layout = Layout::new(&flat);
        let line = &layout.lines[0];
        for width in 0..20 {
            for offset in 0..16 {
                let painted = text(line, width, offset);
                assert!(
                    UnicodeWidthStr::width(painted.as_str()) <= width,
                    "{}/{}: {}",
                    width,
                    offset,
                    painted
                );
                assert!(!painted.contains('\u{fffd}'));
                if painted.contains('\u{301}') {
                    assert!(painted.contains("é"));
                }
            }
        }
        assert!(text(line, 4, 10).contains('3'));
    }
    #[test]
    fn counts_and_final_warnings_take_space_before_previews() {
        let mut flat = parse_top_level_yaml("box:\n  a: .inf\n  b: ordinary\n".into()).unwrap();
        let layout = Layout::new(&flat);
        flat.collapse(1);
        let projection = layout.project(&flat);
        let line = &projection[0].line;
        let fitted = fit_annotations(line, 50);
        assert!(fitted.text.contains("2 entries"));
        assert!(fitted.text.contains("# WARN"));
        for span in &fitted.spans {
            assert!(span.range.end <= fitted.text.len());
            assert!(fitted.text.is_char_boundary(span.range.start));
        }
        assert!(
            UnicodeWidthStr::width(fitted.text.as_str())
                <= UnicodeWidthStr::width(line.text.as_str())
        );
        let warning = fitted
            .spans
            .iter()
            .find(|span| span.role == TokenRole::Warning)
            .unwrap();
        assert!(
            fitted.text[warning.range.clone()].contains("WARN")
                || fitted.text[warning.range.clone()].contains("warning")
        );
    }
    #[test]
    fn expanded_scalars_have_distinct_styles_and_only_annotations_are_dimmed() {
        let flat = parse_top_level_json(r#"[1,true,null,"hello"]"#.into()).unwrap();
        let layout = Layout::new(&flat);
        let mut terminal = VisibleEscapesTerminal::new(false, true);
        paint(
            &mut terminal,
            &layout.lines[0],
            usize::MAX,
            0,
            100,
            &[],
            &(0..0),
        )
        .unwrap();
        let painted = terminal.output();
        assert!(painted.contains("1"));
        assert!(painted.contains("true"));
        assert!(painted.contains("hello"));
        assert!(!painted.contains("_D_"));
        assert!(painted.contains("_FG(Magenta)_1"));
        assert!(painted.contains("_FG(Yellow)_true"));
        assert!(painted.contains("_FG(LightBlack)_null"));
        assert!(painted.contains("_FG(Green)_hello"));
        assert_eq!(text(&layout.lines[0], 100, 0), "[4]: 1,true,null,hello");
    }
    #[test]
    fn generated_warning_text_is_not_highlighted_as_a_search_match() {
        let flat = parse_top_level_yaml("value: .inf".into()).unwrap();
        let layout = Layout::new(&flat);
        let line = &layout.lines[0];
        assert!(line
            .spans
            .iter()
            .filter(|span| span.role == TokenRole::Warning)
            .all(|span| span.source.is_none()));
        assert!(line.spans.iter().any(|span| span.source.is_some()));
        assert!(text(line, 200, 0).ends_with("# WARN Non-finite number"));
    }
}
