use super::*;

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
        let line = &viewer.rendered_line(row.logical_line);
        let mut text = TextOnlyTerminal::new();
        LinePainter::new(line, 0..1)
            .paint_wrapped(&mut text, row, 5)
            .unwrap();
        assert!(UnicodeWidthStr::width(text.output()) <= 5);
        assert!(!text.output().contains('…'));
        joined.push_str(text.output());
        let mut styled = VisibleEscapesTerminal::new(false, true);
        LinePainter::new(line, 0..1)
            .with_search(std::slice::from_ref(&query), &query)
            .paint_wrapped(&mut styled, row, 5)
            .unwrap();
        if styled.output().contains("_FG(LightYellow)__U_") {
            highlighted_rows += 1;
        }
    }
    assert_eq!(joined, viewer.rendered_line(0).text);
    assert!(highlighted_rows >= 2, "match spans multiple physical rows");
}

#[test]
fn wrapped_paint_uses_bounded_placeholders_and_zero_width() {
    let mut viewer =
        crate::viewer::JsonViewer::new(parse_top_level_json(r#""界a""#.into()).unwrap());
    viewer.set_wrap_geometry(1, 0);
    viewer.toggle_wrapping();
    let row = viewer.screen_row(0).unwrap();
    let line = &viewer.rendered_line(0);
    let mut terminal = TextOnlyTerminal::new();
    LinePainter::new(line, 0..1)
        .paint_wrapped(&mut terminal, row, 1)
        .unwrap();
    assert_eq!(terminal.output(), "…");
    terminal.clear_output();
    LinePainter::new(line, 0..1)
        .paint_wrapped(&mut terminal, row, 0)
        .unwrap();
    assert_eq!(terminal.output(), "");
}

#[test]
#[cfg(feature = "colorscheme")]
fn wrapped_continuations_keep_theme_background_and_search_styles() {
    use crate::terminal::AnsiTerminal;
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
        let bg_escape = background_escape(theme.row_style(true).bg);
        let mut highlighted_rows = 0;
        for row in &viewer.physical_rows {
            let line = &viewer.rendered_line(row.logical_line);
            let mut terminal = AnsiTerminal::new(String::new());
            LinePainter::new(line, 0..1)
                .with_theme(&theme)
                .paint_wrapped(&mut terminal, row, 5)
                .unwrap();
            assert_background_until_reset(terminal.output(), &bg_escape, name);
            let mut styled = VisibleEscapesTerminal::new(false, true);
            LinePainter::new(line, 0..1)
                .with_theme(&theme)
                .with_search(std::slice::from_ref(&query), &query)
                .paint_wrapped(&mut styled, row, 5)
                .unwrap();
            highlighted_rows += usize::from(styled.output().contains("_U_"));
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
    use crate::terminal::AnsiTerminal;
    use crate::theme::ThemeName;
    let flat =
        parse_top_level_json(r#"{"obj":{"value":"long text with spaces inside"}}"#.into()).unwrap();
    let layout = Fixture::canonical(&flat);
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
        let bg_escape = background_escape(bg);
        for (offset, width) in [(0, 80), (0, 12), (5, 12), (5, 1)] {
            let mut terminal = AnsiTerminal::new(String::new());
            LinePainter::new(line, line.owner..line.owner + 1)
                .with_theme(&theme)
                .paint(&mut terminal, LineViewport::new(line, offset, 0), width)
                .unwrap();
            let output = assert_background_until_reset(terminal.output(), &bg_escape, name);
            assert_eq!(
                output.matches("\x1b[48;").count(),
                1,
                "{name:?}: {output:?}"
            );
        }
    }
}

#[cfg(feature = "colorscheme")]
fn background_escape(color: crate::terminal::Color) -> String {
    use crate::terminal::Color;
    match color {
        Color::Rgb(r, g, b) => format!("\x1b[48;2;{r};{g};{b}m"),
        Color::C256(c) => format!("\x1b[48;5;{c}m"),
        _ => panic!("expected an explicit selection background"),
    }
}

#[cfg(feature = "colorscheme")]
fn assert_background_until_reset<'a>(
    output: &'a str,
    escape: &str,
    theme: crate::theme::ThemeName,
) -> &'a str {
    let body = output.strip_suffix("\x1b[0m").unwrap();
    assert!(body.contains(escape), "{theme:?}: {body:?}");
    assert_absent_styles(body, &["\x1b[49m", "\x1b[0m"]);
    body
}

#[test]
#[cfg(not(feature = "colorscheme"))]
fn selected_row_fallback_background_reaches_document_text() {
    let flat = parse_top_level_json(r#"{"value":"text"}"#.into()).unwrap();
    let layout = Fixture::canonical(&flat);
    let mut terminal = VisibleEscapesTerminal::new(false, true);
    LinePainter::new(&layout.lines[0], 0..1)
        .with_theme(&crate::theme::Theme::default())
        .paint(
            &mut terminal,
            LineViewport::new(&layout.lines[0], 0, 0),
            100,
        )
        .unwrap();
    assert!(terminal.output().contains("_BG(LightBlack)_"));
}

fn assert_quoted_key_styles(line: &DisplayLine, key: &Span) {
    let theme = Theme::default();
    let opening = LinePainter::new(line, usize::MAX..usize::MAX)
        .with_theme(&theme)
        .style_at(key.range.start, 1);
    let interior = LinePainter::new(line, usize::MAX..usize::MAX)
        .with_theme(&theme)
        .style_at(key.range.start + 1, 1);
    let closing = LinePainter::new(line, usize::MAX..usize::MAX)
        .with_theme(&theme)
        .style_at(key.range.end - 1, 1);
    assert_eq!(opening.fg, crate::terminal::DEFAULT);
    assert_eq!(interior.fg, crate::terminal::CYAN);
    assert_eq!(closing.fg, crate::terminal::DEFAULT);
}

#[test]
fn quoted_key_delimiters_use_punctuation_style() {
    let flat = parse_top_level_json(r#"{"needs quotes":1}"#.into()).unwrap();
    let layout = Fixture::canonical(&flat);
    let line = &layout.lines[0];
    let key = line
        .spans
        .iter()
        .find(|span| span.role == TokenRole::Key)
        .unwrap();
    assert_eq!(&line.text[key.range.clone()], r#""needs quotes""#);
    assert_quoted_key_styles(line, key);

    let flat =
        parse_top_level_json(r#"{"rows":[{"needs quotes":1},{"needs quotes":2}]}"#.into()).unwrap();
    let layout = Fixture::canonical(&flat);
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
    let layout = Fixture::canonical(&flat);
    let line = &layout.lines[0];
    let theme = Theme::built_in(crate::theme::ThemeName::Borealis);
    let mut terminal = crate::terminal::AnsiTerminal::new(String::new());
    LinePainter::new(line, usize::MAX..usize::MAX)
        .with_theme(&theme)
        .paint(&mut terminal, LineViewport::new(line, 0, 0), 100)
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
fn expanded_scalars_have_distinct_styles_and_only_annotations_are_dimmed() {
    let flat = parse_top_level_json(r#"[1,true,null,"hello"]"#.into()).unwrap();
    let layout = Fixture::canonical(&flat);
    let mut terminal = VisibleEscapesTerminal::new(false, true);
    LinePainter::new(&layout.lines[0], usize::MAX..usize::MAX)
        .paint(
            &mut terminal,
            LineViewport::new(&layout.lines[0], 0, 0),
            100,
        )
        .unwrap();
    let painted = terminal.output();
    for styled_value in [
        "_FG(Magenta)_1",
        "_FG(Blue)_true",
        "_FG(White)_null",
        "_FG(Green)_hello",
    ] {
        assert!(
            painted.contains(styled_value),
            "missing {styled_value} in {painted}"
        );
    }
    assert_absent_styles(painted, &["_D_"]);
    assert_eq!(text(&layout.lines[0], 100, 0), "[4]: 1,true,null,hello");
}

#[test]
#[cfg(feature = "colorscheme")]
fn default_theme_dims_warning_annotations_without_brightening_them() {
    let flat = parse_top_level_yaml("value: .inf".into()).unwrap();
    let layout = Fixture::canonical(&flat);
    let mut terminal = VisibleEscapesTerminal::new(false, true);
    LinePainter::new(&layout.lines[0], 0..0)
        .with_theme(&Theme::default())
        .paint(
            &mut terminal,
            LineViewport::new(&layout.lines[0], 0, 0),
            200,
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
        let layout = Fixture::canonical(&flat);
        let line = &layout.lines[layout.nodes[focus].line];
        let end = flat[focus].pair_index().unwrap() + 1;
        let mut terminal = VisibleEscapesTerminal::new(false, true);
        LinePainter::new(line, focus..end)
            .paint(&mut terminal, LineViewport::new(line, 0, 0), 100)
            .unwrap();
        let output = terminal.output();
        assert!(output.contains("_FG(LightMagenta)_1"), "{}", output);
        assert!(!output.contains("_B_"), "{}", output);
    }
}

#[test]
fn mapped_search_does_not_highlight_unrelated_string_characters() {
    let flat = parse_top_level_json(r#""aaaaNEEDLEzz""#.into()).unwrap();
    let layout = Fixture::canonical(&flat);
    let source = flat.1.find("NEEDLE").unwrap();
    let query = source..source + 6;
    let mut terminal = VisibleEscapesTerminal::new(false, true);
    LinePainter::new(&layout.lines[0], 0..1)
        .with_search(std::slice::from_ref(&query), &query)
        .paint(
            &mut terminal,
            LineViewport::new(&layout.lines[0], 0, 0),
            100,
        )
        .unwrap();
    let output = terminal.output();
    let before = output.find("aaaa").unwrap();
    assert_absent_styles(&output[..before], &["_U_", "_FG(LightYellow)_"]);
    assert!(output.contains("_!U_zz"), "{}", output);
    assert!(
        output[before..].contains("_FG(LightYellow)__U_NEEDLE"),
        "{}",
        output
    );
    assert_absent_styles(output, &["_INV_", "_BG("]);
    terminal.clear_output();
    LinePainter::new(&layout.lines[0], 0..1)
        .with_search(std::slice::from_ref(&query), &(0..0))
        .paint(
            &mut terminal,
            LineViewport::new(&layout.lines[0], 0, 0),
            100,
        )
        .unwrap();
    let output = terminal.output();
    assert!(output.contains("_FG(Yellow)__U_NEEDLE"), "{}", output);
    assert_absent_styles(output, &["_INV_", "_BG(", "_B_"]);
}

fn assert_absent_styles(output: &str, styles: &[&str]) {
    for style in styles {
        assert!(!output.contains(style), "unexpected {style} in {output:?}");
    }
}
