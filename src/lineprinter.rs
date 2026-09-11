//! Cell-aware painting of mapped TOON tokens. Generated annotations have no search source.
use crate::terminal::{Style, Terminal};
use crate::theme::{DisplayContext, JsonValueKind, SearchState, StyleRole, StyleState, Theme};
use crate::toon_display::{DisplayLine, TokenRole};
use regex::Regex;
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

lazy_static::lazy_static! {
    pub static ref JS_IDENTIFIER: Regex = Regex::new("^[_$a-zA-Z][_$a-zA-Z0-9]*$").unwrap();
}

/// Indentation suppression and horizontal scrolling have different semantics:
/// only scrolling hides document content and earns a leading ellipsis.
#[derive(Clone, Copy, Debug)]
pub struct LineViewport {
    pub horizontal_offset: usize,
    pub removed_indentation: usize,
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
        }
    }

    pub fn source_column(self, viewport_column: usize) -> usize {
        self.removed_indentation
            + self.horizontal_offset
            + viewport_column.saturating_sub(usize::from(self.horizontal_offset > 0))
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

#[allow(dead_code)]
pub fn paint(
    terminal: &mut impl Terminal,
    line: &DisplayLine,
    focused: Range<usize>,
    viewport: LineViewport,
    width: usize,
    matches: &[Range<usize>],
    current: &Range<usize>,
) -> std::fmt::Result {
    paint_impl(
        terminal, None, line, focused, viewport, width, matches, current,
    )
}

#[allow(dead_code, clippy::too_many_arguments)]
pub fn paint_themed(
    terminal: &mut impl Terminal,
    theme: &Theme,
    line: &DisplayLine,
    focused: Range<usize>,
    viewport: LineViewport,
    width: usize,
    matches: &[Range<usize>],
    current: &Range<usize>,
) -> std::fmt::Result {
    paint_impl(
        terminal,
        Some(theme),
        line,
        focused,
        viewport,
        width,
        matches,
        current,
    )
}

#[allow(clippy::too_many_arguments)]
fn paint_impl(
    terminal: &mut impl Terminal,
    theme: Option<&Theme>,
    line: &DisplayLine,
    focused: Range<usize>,
    viewport: LineViewport,
    width: usize,
    matches: &[Range<usize>],
    current: &Range<usize>,
) -> std::fmt::Result {
    let offset = viewport.removed_indentation + viewport.horizontal_offset;
    let window = viewport.paint_window(line, width);
    let PaintWindow {
        left,
        right,
        available,
    } = window;
    if width == 0 {
        return Ok(());
    }
    let focused_row = !focused.is_empty();
    let ellipsis_style = theme.map_or_else(Style::default, |theme| {
        theme.style_on_row(StyleRole::Ellipsis, StyleState::main(), focused_row)
    });
    if left > 0 {
        terminal.set_style(&ellipsis_style)?;
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
        terminal.set_style(&grapheme_style(
            theme,
            line,
            &focused,
            byte,
            grapheme.len(),
            matches,
            current,
        ))?;
        terminal.write_str(grapheme)?;
        used += cells;
    }
    if right > 0 && left + used < width {
        terminal.set_style(&ellipsis_style)?;
        terminal.write_char('…')?;
    }
    terminal.reset_style()?;
    Ok(())
}

#[allow(dead_code)]
pub fn paint_wrapped(
    terminal: &mut impl Terminal,
    line: &DisplayLine,
    focused: Range<usize>,
    row: &crate::wrapped_view::PhysicalRow,
    width: usize,
    matches: &[Range<usize>],
    current: &Range<usize>,
) -> std::fmt::Result {
    paint_wrapped_impl(terminal, None, line, focused, row, width, matches, current)
}

#[allow(dead_code, clippy::too_many_arguments)]
pub fn paint_wrapped_themed(
    terminal: &mut impl Terminal,
    theme: &Theme,
    line: &DisplayLine,
    focused: Range<usize>,
    row: &crate::wrapped_view::PhysicalRow,
    width: usize,
    matches: &[Range<usize>],
    current: &Range<usize>,
) -> std::fmt::Result {
    paint_wrapped_impl(
        terminal,
        Some(theme),
        line,
        focused,
        row,
        width,
        matches,
        current,
    )
}

#[allow(clippy::too_many_arguments)]
fn paint_wrapped_impl(
    terminal: &mut impl Terminal,
    theme: Option<&Theme>,
    line: &DisplayLine,
    focused: Range<usize>,
    row: &crate::wrapped_view::PhysicalRow,
    width: usize,
    matches: &[Range<usize>],
    current: &Range<usize>,
) -> std::fmt::Result {
    if width == 0 {
        return Ok(());
    }
    if row.placeholder {
        terminal.set_style(&grapheme_style(
            theme,
            line,
            &focused,
            row.bytes.start,
            row.bytes.len(),
            matches,
            current,
        ))?;
        terminal.write_char('…')?;
    } else {
        for (offset, grapheme) in line.text[row.bytes.clone()].grapheme_indices(true) {
            terminal.set_style(&grapheme_style(
                theme,
                line,
                &focused,
                row.bytes.start + offset,
                grapheme.len(),
                matches,
                current,
            ))?;
            terminal.write_str(grapheme)?;
        }
    }
    terminal.reset_style()
}

#[allow(clippy::too_many_arguments)]
fn grapheme_style(
    theme: Option<&Theme>,
    line: &DisplayLine,
    focused: &Range<usize>,
    byte: usize,
    byte_len: usize,
    matches: &[Range<usize>],
    current: &Range<usize>,
) -> Style {
    let span = line
        .spans
        .iter()
        .filter(|span| span.node == focused.start)
        .find(|span| span.range.contains(&byte))
        .or_else(|| line.spans.iter().find(|span| span.range.contains(&byte)));
    let mut style = theme.map_or_else(Style::default, |theme| theme.row_style(!focused.is_empty()));
    if let Some(span) = span {
        let annotation = matches!(
            span.role,
            TokenRole::Preview | TokenRole::Count | TokenRole::Warning
        );
        let quoted_key_delimiter = matches!(span.role, TokenRole::Key | TokenRole::FieldDefinition)
            && line.text[span.range.clone()].starts_with('"')
            && line.text[span.range.clone()].ends_with('"')
            && (byte == span.range.start || byte.saturating_add(byte_len) == span.range.end);
        let role = if quoted_key_delimiter {
            StyleRole::Punctuation
        } else {
            match span.role {
                TokenRole::Key => StyleRole::ObjectKey,
                TokenRole::FieldDefinition => StyleRole::FieldDefinition,
                TokenRole::String => StyleRole::JsonValue(JsonValueKind::String),
                TokenRole::Number => StyleRole::JsonValue(JsonValueKind::Number),
                TokenRole::Boolean => StyleRole::JsonValue(JsonValueKind::Boolean),
                TokenRole::Null => StyleRole::JsonValue(JsonValueKind::Null),
                TokenRole::ArrayIndex => StyleRole::ArrayIndex,
                TokenRole::PrimitiveTrailingComma => StyleRole::PrimitiveTrailingComma,
                TokenRole::ContainerDelimiter => StyleRole::ContainerDelimiter,
                TokenRole::EmptyContainer => StyleRole::JsonValue(JsonValueKind::EmptyObject),
                TokenRole::Punctuation => StyleRole::Punctuation,
                TokenRole::Warning => StyleRole::Message(crate::theme::MessageSeverity::Warn),
                TokenRole::Preview => StyleRole::PreviewText,
                TokenRole::Count => StyleRole::PreviewCount,
            }
        };
        let focus = if focused.contains(&span.node) && !annotation {
            crate::theme::FocusState::Row
        } else {
            crate::theme::FocusState::None
        };
        let overlaps = |query: &Range<usize>| {
            span.matching_ranges(query)
                .iter()
                .any(|range| range.start < byte + byte_len && byte < range.end)
        };
        let search = if overlaps(current) {
            SearchState::CurrentMatch
        } else if matches.iter().any(overlaps) {
            SearchState::Match
        } else {
            SearchState::None
        };
        style = if let Some(theme) = theme {
            theme.style_on_row(
                role,
                StyleState {
                    focus,
                    search,
                    context: if span.role == TokenRole::Preview {
                        DisplayContext::Preview
                    } else {
                        DisplayContext::Main
                    },
                },
                !focused.is_empty(),
            )
        } else {
            let mut style = Style::default();
            style.fg = if quoted_key_delimiter {
                crate::terminal::DEFAULT
            } else {
                match span.role {
                    TokenRole::Key | TokenRole::FieldDefinition => crate::terminal::CYAN,
                    TokenRole::String => crate::terminal::GREEN,
                    TokenRole::Number => crate::terminal::MAGENTA,
                    TokenRole::Boolean => crate::terminal::BLUE,
                    TokenRole::Null => crate::terminal::WHITE,
                    TokenRole::Warning => crate::terminal::YELLOW,
                    TokenRole::Count | TokenRole::Preview => crate::terminal::LIGHT_BLACK,
                    TokenRole::ArrayIndex | TokenRole::ContainerDelimiter => {
                        crate::terminal::LIGHT_BLACK
                    }
                    TokenRole::PrimitiveTrailingComma | TokenRole::Punctuation => {
                        crate::terminal::DEFAULT
                    }
                    TokenRole::EmptyContainer => crate::terminal::WHITE,
                }
            };
            style.dimmed = span.role == TokenRole::Warning;
            if focused.contains(&span.node) && !annotation {
                style.fg = style.fg.bright();
            }
            if matches.iter().any(overlaps) {
                style.fg = crate::terminal::YELLOW;
                style.underlined = true;
            }
            if overlaps(current) {
                style.fg = crate::terminal::LIGHT_YELLOW;
                style.underlined = true;
            }
            style
        };
        if span.role == TokenRole::Warning {
            style.dimmed = true;
        }
    }
    style
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
            for map in &mut span.source_map {
                let start = map
                    .display
                    .start
                    .saturating_sub(range.start)
                    .min(replacement.len());
                let end = map
                    .display
                    .end
                    .saturating_sub(range.start)
                    .min(replacement.len());
                map.display = range.start + start..range.start + end;
            }
            span.source_map
                .retain(|map| map.display.start < map.display.end);
            span.range.end = span.range.start + replacement.len();
        } else if span.range.start >= range.end {
            span.range.start = span.range.start - range.end + range.start + replacement.len();
            span.range.end = span.range.end - range.end + range.start + replacement.len();
        }
    }
    std::borrow::Cow::Owned(clipped)
}

pub fn hit_test_wrapped(
    line: &DisplayLine,
    row: &crate::wrapped_view::PhysicalRow,
    column: usize,
) -> (usize, Option<usize>) {
    let mut cells = 0;
    let byte = if row.placeholder && column == 0 {
        Some(row.bytes.start)
    } else if row.placeholder {
        None
    } else {
        line.text[row.bytes.clone()]
            .grapheme_indices(true)
            .find_map(|(byte, grapheme)| {
                cells += UnicodeWidthStr::width(grapheme);
                (cells > column).then_some(row.bytes.start + byte)
            })
    };
    let span = byte.and_then(|byte| line.spans.iter().find(|span| span.range.contains(&byte)));
    (
        span.map_or(line.owner, |span| span.node),
        span.and_then(|span| span.source.as_ref().map(|source| source.start)),
    )
}

/// Resolve a terminal cell to its parsed owner and optional source-token anchor.
pub fn hit_test(line: &DisplayLine, column: usize) -> (usize, Option<usize>) {
    let mut cells = 0;
    let byte = line
        .text
        .grapheme_indices(true)
        .find_map(|(byte, grapheme)| {
            cells += UnicodeWidthStr::width(grapheme);
            if cells > column { Some(byte) } else { None }
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
    use crate::theme::Theme;
    use crate::toon_display::{Layout, Span};
    fn text(line: &DisplayLine, width: usize, offset: usize) -> String {
        let mut terminal = TextOnlyTerminal::new();
        paint(
            &mut terminal,
            line,
            usize::MAX..usize::MAX,
            LineViewport::new(line, offset, 0),
            width,
            &[],
            &(0..0),
        )
        .unwrap();
        terminal.output().to_string()
    }
    #[test]
    fn wrapped_paint_preserves_graphemes_text_and_search_styles() {
        let flat = parse_top_level_json(r#""ab界éNEEDLEzz""#.into()).unwrap();
        let source = flat.1.find("NEEDLE").unwrap();
        let query = source..source + 6;
        let mut viewer = crate::viewer::JsonViewer::new(flat);
        viewer.set_wrap_geometry(5, 0);
        viewer.toggle_wrapping();
        let mut joined = String::new();
        let mut highlighted_rows = 0;
        for index in 0..100 {
            let Some(row) = viewer.screen_row(index) else {
                break;
            };
            let line = &viewer.visible[row.logical_line].line;
            let mut text = TextOnlyTerminal::new();
            paint_wrapped(&mut text, line, 0..1, row, 5, &[], &(0..0)).unwrap();
            assert!(UnicodeWidthStr::width(text.output()) <= 5);
            assert!(!text.output().contains('…'));
            joined.push_str(text.output());
            let mut styled = VisibleEscapesTerminal::new(false, true);
            paint_wrapped(
                &mut styled,
                line,
                0..1,
                row,
                5,
                std::slice::from_ref(&query),
                &query,
            )
            .unwrap();
            if styled.output().contains("_FG(LightYellow)__U_") {
                highlighted_rows += 1;
            }
        }
        assert_eq!(joined, viewer.visible[0].line.text);
        assert!(highlighted_rows >= 2, "match spans multiple physical rows");
    }

    #[test]
    fn wrapped_paint_uses_bounded_placeholders_and_zero_width() {
        let mut viewer =
            crate::viewer::JsonViewer::new(parse_top_level_json(r#""界a""#.into()).unwrap());
        viewer.set_wrap_geometry(1, 0);
        viewer.toggle_wrapping();
        let row = viewer.screen_row(0).unwrap();
        let line = &viewer.visible[0].line;
        let mut terminal = TextOnlyTerminal::new();
        paint_wrapped(&mut terminal, line, 0..1, row, 1, &[], &(0..0)).unwrap();
        assert_eq!(terminal.output(), "…");
        terminal.clear_output();
        paint_wrapped(&mut terminal, line, 0..1, row, 0, &[], &(0..0)).unwrap();
        assert_eq!(terminal.output(), "");
    }

    #[test]
    fn wrap_padding_does_not_select_a_cell_on_the_next_row() {
        let mut viewer = crate::viewer::JsonViewer::new(
            parse_top_level_json(r#"[{"a":"abc","b":"界"}]"#.into()).unwrap(),
        );
        viewer.set_wrap_geometry(7, 0);
        viewer.toggle_wrapping();
        let row = viewer
            .physical_rows
            .iter()
            .find(|row| row.first && viewer.visible[row.logical_line].line.text == "  abc,界")
            .unwrap();
        let line = &viewer.visible[row.logical_line].line;
        assert_ne!(hit_test_wrapped(line, row, 2).0, line.owner);
        assert_eq!(hit_test_wrapped(line, row, 6), (line.owner, None));
        let continuation = viewer
            .physical_rows
            .iter()
            .find(|next| next.logical_line == row.logical_line && !next.first)
            .unwrap();
        assert_ne!(hit_test_wrapped(line, continuation, 0).0, line.owner);
    }

    #[test]
    #[cfg(feature = "colorscheme")]
    fn wrapped_continuations_keep_theme_background_and_search_styles() {
        use crate::terminal::{AnsiTerminal, Color};
        use crate::theme::ThemeName;
        let flat = parse_top_level_json(r#""abc NEEDLE tail with spaces""#.into()).unwrap();
        let start = flat.1.find("NEEDLE").unwrap();
        let query = start..start + 6;
        let mut viewer = crate::viewer::JsonViewer::new(flat);
        viewer.set_wrap_geometry(5, 0);
        viewer.toggle_wrapping();
        for name in [
            ThemeName::Borealis,
            ThemeName::VimDesert,
            ThemeName::VimDefault,
        ] {
            let theme = Theme::built_in(name);
            let bg_escape = match theme.row_style(true).bg {
                Color::Rgb(r, g, b) => format!("\x1b[48;2;{r};{g};{b}m"),
                Color::C256(c) => format!("\x1b[48;5;{c}m"),
                _ => panic!("expected selection background"),
            };
            let mut highlighted_rows = 0;
            for row in &viewer.physical_rows {
                let line = &viewer.visible[row.logical_line].line;
                let mut terminal = AnsiTerminal::new(String::new());
                paint_wrapped_themed(&mut terminal, &theme, line, 0..1, row, 5, &[], &(0..0))
                    .unwrap();
                let output = terminal.output().strip_suffix("\x1b[0m").unwrap();
                assert!(output.contains(&bg_escape), "{name:?}: {output:?}");
                assert!(!output.contains("\x1b[49m"));
                assert!(!output.contains("\x1b[0m"));
                let mut styled = VisibleEscapesTerminal::new(false, true);
                paint_wrapped_themed(
                    &mut styled,
                    &theme,
                    line,
                    0..1,
                    row,
                    5,
                    std::slice::from_ref(&query),
                    &query,
                )
                .unwrap();
                if styled.output().contains("_U_") {
                    highlighted_rows += 1;
                }
            }
            assert!(
                highlighted_rows >= 2,
                "search style must cross wrap boundaries for {name:?}"
            );
        }
    }

    #[test]
    #[cfg(feature = "colorscheme")]
    fn selected_row_keeps_background_on_indentation_spaces_and_clipping() {
        use crate::terminal::{AnsiTerminal, Color};
        use crate::theme::ThemeName;
        let flat =
            parse_top_level_json(r#"{"obj":{"value":"long text with spaces inside"}}"#.into())
                .unwrap();
        let layout = Layout::canonical(&flat);
        let line = &layout.lines[1];
        assert!(line.text.starts_with("  "));
        for name in [
            ThemeName::Borealis,
            ThemeName::VimDesert,
            ThemeName::VimDefault,
        ] {
            let theme = Theme::built_in(name);
            let bg = theme.row_style(true).bg;
            assert_ne!(bg, theme.row_style(false).bg);
            let bg_escape = match bg {
                Color::Rgb(r, g, b) => format!("\x1b[48;2;{r};{g};{b}m"),
                Color::C256(c) => format!("\x1b[48;5;{c}m"),
                _ => panic!("expected explicit selection background"),
            };
            for (offset, width) in [(0, 80), (0, 12), (5, 12), (5, 1)] {
                let mut terminal = AnsiTerminal::new(String::new());
                paint_themed(
                    &mut terminal,
                    &theme,
                    line,
                    line.owner..line.owner + 1,
                    LineViewport::new(line, offset, 0),
                    width,
                    &[],
                    &(0..0),
                )
                .unwrap();
                let output = terminal.output().strip_suffix("\x1b[0m").unwrap();
                assert!(output.contains(&bg_escape), "{:?}: {:?}", name, output);
                assert_eq!(
                    output.matches("\x1b[48;").count(),
                    1,
                    "{name:?}: {output:?}"
                );
                assert!(!output.contains("\x1b[49m"));
                assert!(!output.contains("\x1b[0m"));
            }
        }
    }

    #[test]
    #[cfg(not(feature = "colorscheme"))]
    fn selected_row_fallback_background_reaches_document_text() {
        let flat = parse_top_level_json(r#"{"value":"text"}"#.into()).unwrap();
        let layout = Layout::canonical(&flat);
        let mut terminal = VisibleEscapesTerminal::new(false, true);
        paint_themed(
            &mut terminal,
            &crate::theme::Theme::default(),
            &layout.lines[0],
            0..1,
            LineViewport::new(&layout.lines[0], 0, 0),
            100,
            &[],
            &(0..0),
        )
        .unwrap();
        assert!(terminal.output().contains("_BG(LightBlack)_"));
    }

    fn assert_quoted_key_styles(line: &DisplayLine, key: &Span) {
        let theme = Theme::default();
        let opening = grapheme_style(
            Some(&theme),
            line,
            &(usize::MAX..usize::MAX),
            key.range.start,
            1,
            &[],
            &(0..0),
        );
        let interior = grapheme_style(
            Some(&theme),
            line,
            &(usize::MAX..usize::MAX),
            key.range.start + 1,
            1,
            &[],
            &(0..0),
        );
        let closing = grapheme_style(
            Some(&theme),
            line,
            &(usize::MAX..usize::MAX),
            key.range.end - 1,
            1,
            &[],
            &(0..0),
        );
        assert_eq!(opening.fg, crate::terminal::DEFAULT);
        assert_eq!(interior.fg, crate::terminal::CYAN);
        assert_eq!(closing.fg, crate::terminal::DEFAULT);
    }

    #[test]
    fn quoted_key_delimiters_use_punctuation_style() {
        let flat = parse_top_level_json(r#"{"needs quotes":1}"#.into()).unwrap();
        let layout = Layout::canonical(&flat);
        let line = &layout.lines[0];
        let key = line
            .spans
            .iter()
            .find(|span| span.role == TokenRole::Key)
            .unwrap();
        assert_eq!(&line.text[key.range.clone()], r#""needs quotes""#);
        assert_quoted_key_styles(line, key);

        let flat =
            parse_top_level_json(r#"{"rows":[{"needs quotes":1},{"needs quotes":2}]}"#.into())
                .unwrap();
        let layout = Layout::canonical(&flat);
        let line = &layout.lines[0];
        let field = line
            .spans
            .iter()
            .find(|span| span.role == TokenRole::FieldDefinition)
            .unwrap();
        assert_eq!(&line.text[field.range.clone()], r#""needs quotes""#);
        assert_quoted_key_styles(line, field);
    }

    #[test]
    #[cfg(feature = "colorscheme")]
    fn borealis_table_fields_and_punctuation_use_plain_text() {
        let flat = parse_top_level_json(r#"{"rows":[{"field":1,"name":true}]}"#.into()).unwrap();
        let layout = Layout::canonical(&flat);
        let line = &layout.lines[0];
        let theme = Theme::built_in(crate::theme::ThemeName::Borealis);
        let mut terminal = crate::terminal::AnsiTerminal::new(String::new());
        paint_impl(
            &mut terminal,
            Some(&theme),
            line,
            usize::MAX..usize::MAX,
            LineViewport::new(line, 0, 0),
            100,
            &[],
            &(0..0),
        )
        .unwrap();
        assert!(
            terminal
                .output()
                .contains("\x1b[38;2;202;211;226m[1]{field,name}:"),
            "{}",
            terminal.output()
        );
        assert!(!terminal.output().contains("\x1b[1m"));
    }

    #[test]
    fn reveal_window_reserves_only_actual_clipping_markers() {
        let flat = parse_top_level_json(r#"{"x":"aaaaaaaaaaaaaaaaaaaaaaaaaaZ"}"#.into()).unwrap();
        let layout = Layout::canonical(&flat);
        let line = &layout.lines[0];
        for (width, offset, visible, expected) in [
            (30, 0, 0..30, "x: aaaaaaaaaaaaaaaaaaaaaaaaaaZ"),
            (29, 0, 0..28, "x: aaaaaaaaaaaaaaaaaaaaaaaaa…"),
            (30, 1, 1..30, "…: aaaaaaaaaaaaaaaaaaaaaaaaaaZ"),
            (28, 1, 1..27, "…: aaaaaaaaaaaaaaaaaaaaaaaa…"),
            (0, 0, 0..0, ""),
        ] {
            let viewport = LineViewport::new(line, offset, 0);
            assert_eq!(viewport.visible_columns(line, width), visible);
            assert_eq!(text(line, width, offset), expected);
        }
    }

    #[test]
    fn clipping_uses_grapheme_cell_boundaries_and_horizontal_scrolling() {
        let flat = parse_top_level_json(r#"["界","é",3]"#.into()).unwrap();
        let layout = Layout::canonical(&flat);
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
        let layout = Layout::canonical(&flat);
        flat.collapse(1);
        let projection = layout.project(&flat);
        let line = &projection[0].line;
        let fitted = fit_annotations(line, 50);
        assert!(fitted.text.contains("(2)"));
        assert!(fitted.text.contains("# WARN"));
        let render = |focus| {
            let mut terminal = VisibleEscapesTerminal::new(false, true);
            paint(
                &mut terminal,
                &fitted,
                focus,
                LineViewport::new(&fitted, 0, 0),
                200,
                &[],
                &(0..0),
            )
            .unwrap();
            terminal.output().to_string()
        };
        let selected = render(1..flat[1].pair_index().unwrap() + 1);
        let unselected = render(0..0);
        let preview = selected.split("_FG(Yellow)_").next().unwrap();
        assert!(preview.contains("_FG(LightBlack)_"));
        assert!(!preview.contains("_D_"), "{}", selected);
        assert!(!preview.contains("_B_"), "{}", selected);
        // Count, preview, and warning styles stay identical when their owner is focused.
        let hints = |text: String| text.split_once("_FG(LightBlack)_").unwrap().1.to_string();
        assert_eq!(hints(selected), hints(unselected));
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
        let layout = Layout::canonical(&flat);
        let mut terminal = VisibleEscapesTerminal::new(false, true);
        paint(
            &mut terminal,
            &layout.lines[0],
            usize::MAX..usize::MAX,
            LineViewport::new(&layout.lines[0], 0, 0),
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
        assert!(painted.contains("_FG(Blue)_true"));
        assert!(painted.contains("_FG(White)_null"));
        assert!(painted.contains("_FG(Green)_hello"));
        assert_eq!(text(&layout.lines[0], 100, 0), "[4]: 1,true,null,hello");
    }
    #[test]
    fn generated_warning_text_is_not_highlighted_as_a_search_match() {
        let flat = parse_top_level_yaml("value: .inf".into()).unwrap();
        let layout = Layout::canonical(&flat);
        let line = &layout.lines[0];
        assert!(
            line.spans
                .iter()
                .filter(|span| span.role == TokenRole::Warning)
                .all(|span| span.source.is_none())
        );
        assert!(line.spans.iter().any(|span| span.source.is_some()));
        assert!(text(line, 200, 0).ends_with("# WARN Non-finite number"));
    }

    #[test]
    #[cfg(feature = "colorscheme")]
    fn default_theme_dims_warning_annotations_without_brightening_them() {
        let flat = parse_top_level_yaml("value: .inf".into()).unwrap();
        let layout = Layout::canonical(&flat);
        let mut terminal = VisibleEscapesTerminal::new(false, true);
        paint_themed(
            &mut terminal,
            &Theme::default(),
            &layout.lines[0],
            0..0,
            LineViewport::new(&layout.lines[0], 0, 0),
            200,
            &[],
            &(0..0),
        )
        .unwrap();
        let output = terminal.output();
        assert!(output.contains("_FG(Yellow)_"), "{}", output);
        assert!(output.contains("_D_"), "{}", output);
        assert!(!output.contains("_FG(LightYellow)_"), "{}", output);
    }
    #[test]
    fn container_focus_brightens_row_values_and_implicit_root_fields() {
        for (input, focus) in [(r#"[{"a":1}]"#, 1), (r#"{"a":1}"#, 0)] {
            let flat = parse_top_level_json(input.into()).unwrap();
            let layout = Layout::canonical(&flat);
            let line = &layout.lines[layout.nodes[focus].line];
            let end = flat[focus].pair_index().unwrap() + 1;
            let mut terminal = VisibleEscapesTerminal::new(false, true);
            paint(
                &mut terminal,
                line,
                focus..end,
                LineViewport::new(line, 0, 0),
                100,
                &[],
                &(0..0),
            )
            .unwrap();
            let output = terminal.output();
            assert!(output.contains("_FG(LightMagenta)_1"), "{}", output);
            assert!(!output.contains("_B_"), "{}", output);
        }
    }

    #[test]
    fn mapped_search_does_not_highlight_unrelated_string_characters() {
        let flat = parse_top_level_json(r#""aaaaNEEDLEzz""#.into()).unwrap();
        let layout = Layout::canonical(&flat);
        let source = flat.1.find("NEEDLE").unwrap();
        let query = source..source + 6;
        let mut terminal = VisibleEscapesTerminal::new(false, true);
        paint(
            &mut terminal,
            &layout.lines[0],
            0..1,
            LineViewport::new(&layout.lines[0], 0, 0),
            100,
            std::slice::from_ref(&query),
            &query,
        )
        .unwrap();
        let output = terminal.output();
        let before = output.find("aaaa").unwrap();
        assert!(!output[..before].contains("_U_"), "{}", output);
        assert!(output.contains("_!U_zz"), "{}", output);
        assert!(
            !output[..before].contains("_FG(LightYellow)_"),
            "{}",
            output
        );
        assert!(
            output[before..].contains("_FG(LightYellow)__U_NEEDLE"),
            "{}",
            output
        );
        assert!(!output.contains("_INV_"), "{}", output);
        assert!(!output.contains("_BG("), "{}", output);
        terminal.clear_output();
        paint(
            &mut terminal,
            &layout.lines[0],
            0..1,
            LineViewport::new(&layout.lines[0], 0, 0),
            100,
            std::slice::from_ref(&query),
            &(0..0),
        )
        .unwrap();
        let output = terminal.output();
        assert!(output.contains("_FG(Yellow)__U_NEEDLE"), "{}", output);
        assert!(!output.contains("_INV_"), "{}", output);
        assert!(!output.contains("_BG("), "{}", output);
        assert!(!output.contains("_B_"), "{}", output);
    }
    fn reduced_text(line: &DisplayLine, viewport: LineViewport, width: usize) -> String {
        let mut terminal = TextOnlyTerminal::new();
        paint(
            &mut terminal,
            line,
            usize::MAX..usize::MAX,
            viewport,
            width,
            &[],
            &(0..0),
        )
        .unwrap();
        terminal.output().to_string()
    }

    #[test]
    fn indentation_reduction_removes_only_layout_spaces_without_scroll_ellipsis() {
        let flat = parse_top_level_json(r#"{"root":{"nested":{"value":1}}}"#.into()).unwrap();
        let layout = Layout::canonical(&flat);
        let root = &layout.lines[0];
        let nested = &layout.lines[2];
        assert_eq!(
            reduced_text(root, LineViewport::new(root, 0, 100), 100),
            "root:"
        );
        assert_eq!(
            reduced_text(nested, LineViewport::new(nested, 0, 2), 100),
            "  value: 1"
        );
        assert_eq!(
            reduced_text(nested, LineViewport::new(nested, 0, 100), 100),
            "value: 1"
        );
        assert_eq!(
            reduced_text(nested, LineViewport::new(nested, 0, 0), 100),
            "    value: 1"
        );
        assert_eq!(nested.text, "    value: 1", "cached layout is unchanged");
        let flat = parse_top_level_json(r#"[{"values":[1],"other":{}}]"#.into()).unwrap();
        let layout = Layout::canonical(&flat);
        let list = &layout.lines[1];
        assert!(reduced_text(list, LineViewport::new(list, 0, 100), 100).starts_with("- values"));
        let roots = parse_top_level_json("1 2".into()).unwrap();
        let layout = Layout::canonical(&roots);
        let separator = &layout.lines[0];
        assert_eq!(
            reduced_text(separator, LineViewport::new(separator, 0, 100), 100),
            separator.text
        );
    }

    #[test]
    fn reduced_indentation_keeps_unicode_cell_hits_and_search_coordinates() {
        let flat = parse_top_level_json(r#"{"root":{"rows":[{"id":1,"name":"界NEEDLE"}]}}"#.into())
            .unwrap();
        let layout = Layout::canonical(&flat);
        let source_start = flat.1.find("NEEDLE").unwrap();
        let query = source_start..source_start + 6;
        let line = layout
            .lines
            .iter()
            .find(|line| line.text.contains("界NEEDLE"))
            .unwrap();
        let span = line
            .spans
            .iter()
            .find(|span| !span.matching_ranges(&query).is_empty())
            .unwrap();
        let cell_column = UnicodeWidthStr::width(&line.text[..span.range.start]);
        let viewport = LineViewport::new(line, 0, 2);
        let displayed_cell_column = viewport.reduced_column(cell_column);
        assert_eq!(
            hit_test(line, viewport.source_column(displayed_cell_column)).0,
            span.node
        );
        let matched = span.matching_ranges(&query)[0].clone();
        let source_column = UnicodeWidthStr::width(&line.text[..matched.start]);
        let search_offset = viewport.reduced_column(source_column);
        let searched = LineViewport::new(line, search_offset, 2);
        assert_eq!(reduced_text(line, searched, 10), "…NEEDLE");
        assert_eq!(hit_test(line, searched.source_column(1)).0, span.node);
        assert_eq!(
            searched.content_width(line),
            UnicodeWidthStr::width(line.text.as_str()) - 2
        );
        // Indentation suppression is independent of a real one-cell horizontal scroll.
        let scrolled = LineViewport::new(line, 1, 100);
        assert!(reduced_text(line, scrolled, 10).starts_with('…'));
    }
    #[test]
    fn wide_grapheme_scroll_boundary_uses_the_same_paint_and_mouse_offset() {
        for (input, reduction) in [
            (r#"{"nested":["界",2]}"#, 0),
            (r#"{"outer":{"nested":["界",2]}}"#, 2),
        ] {
            let flat = parse_top_level_json(input.into()).unwrap();
            let layout = Layout::canonical(&flat);
            let line = layout
                .lines
                .iter()
                .find(|line| line.text.contains('界'))
                .unwrap();
            let viewport = LineViewport::new(line, 12, reduction);
            assert_eq!(viewport.horizontal_offset, 13);
            assert_eq!(reduced_text(line, viewport, 30), "…,2");
            let selected = hit_test(line, viewport.source_column(2)).0;
            assert_eq!(&flat.1[flat[selected].range.clone()], "2");
            let matching = line
                .spans
                .iter()
                .find(|span| span.node == selected && span.source.is_some())
                .unwrap();
            let source = matching.source.clone().unwrap();
            let painted_match = matching.matching_ranges(&source)[0].clone();
            let column = UnicodeWidthStr::width(&line.text[..painted_match.start]);
            assert_eq!(
                viewport.reduced_column(column) - viewport.horizontal_offset + 1,
                2
            );
        }
    }
}
