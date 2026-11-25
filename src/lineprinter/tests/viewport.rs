use super::*;

#[test]
fn table_viewport_keeps_cell_columns_when_a_wide_grapheme_is_cut() {
    let line = |text: &str, owner: usize| {
        let start = text.find('Z').unwrap();
        DisplayLine {
            text: text.to_owned(),
            spans: vec![Span {
                range: start..start + 1,
                node: owner + 1,
                role: TokenRole::String,
                source: Some(40..41),
                source_map: vec![],
            }],
            owner,
            separator: false,
            shared_matches: vec![],
        }
    };
    let wide = line("  界Z", 10);
    let narrow = line("  abZ", 20);
    let wide_view = LineViewport::table(&wide, 1, 2, 2);
    let narrow_view = LineViewport::table(&narrow, 1, 2, 2);
    let draw = |line: &DisplayLine, viewport| {
        let mut terminal = TextOnlyTerminal::new();
        LinePainter::new(line, 0..0)
            .paint(&mut terminal, viewport, 6)
            .unwrap();
        terminal.output().to_owned()
    };
    assert_eq!(draw(&wide, wide_view), "… Z");
    assert_eq!(draw(&narrow, narrow_view), "…bZ");
    assert_eq!(wide_view.hit_test(&wide, 1, 6), (10, None));
    assert_eq!(wide_view.hit_test(&wide, 2, 6), (11, Some(40)));
    assert_eq!(narrow_view.hit_test(&narrow, 2, 6), (21, Some(40)));
    assert_eq!(wide_view.source_column(2), 4);
    assert_eq!(narrow_view.source_column(2), 4);
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
        .find(|row| row.first && viewer.rendered_line(row.logical_line).text == "  abc,界")
        .unwrap();
    let line = &viewer.rendered_line(row.logical_line);
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
fn reveal_window_reserves_only_actual_clipping_markers() {
    let flat = parse_top_level_json(r#"{"x":"aaaaaaaaaaaaaaaaaaaaaaaaaaZ"}"#.into()).unwrap();
    let layout = Fixture::canonical(&flat);
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
    let layout = Fixture::canonical(&flat);
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

fn reduced_text(line: &DisplayLine, viewport: LineViewport, width: usize) -> String {
    let mut terminal = TextOnlyTerminal::new();
    LinePainter::new(line, usize::MAX..usize::MAX)
        .paint(&mut terminal, viewport, width)
        .unwrap();
    terminal.output().to_string()
}

#[test]
fn indentation_reduction_removes_only_layout_spaces_without_scroll_ellipsis() {
    let flat = parse_top_level_json(r#"{"root":{"nested":{"value":1}}}"#.into()).unwrap();
    let layout = Fixture::canonical(&flat);
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
    let layout = Fixture::canonical(&flat);
    let list = &layout.lines[1];
    assert!(reduced_text(list, LineViewport::new(list, 0, 100), 100).starts_with("- values"));
    let roots = parse_top_level_json("1 2".into()).unwrap();
    let layout = Fixture::canonical(&roots);
    let separator = &layout.lines[0];
    assert_eq!(
        reduced_text(separator, LineViewport::new(separator, 0, 100), 100),
        separator.text
    );
}

#[test]
fn reduced_indentation_keeps_unicode_cell_hits_and_search_coordinates() {
    let flat =
        parse_top_level_json(r#"{"root":{"rows":[{"id":1,"name":"界NEEDLE"}]}}"#.into()).unwrap();
    let layout = Fixture::canonical(&flat);
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
        let layout = Fixture::canonical(&flat);
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
