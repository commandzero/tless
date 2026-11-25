use super::*;

#[test]
fn counts_and_final_warnings_take_space_before_previews() {
    let mut flat = parse_top_level_yaml("box:\n  a: .inf\n  b: ordinary\n".into()).unwrap();
    let layout = Fixture::canonical(&flat);
    flat.collapse(1);
    let projection = layout.project(&flat);
    let line = &projection[0].line;
    let fitted = fit_annotations(line, 50);
    assert!(fitted.text.contains("(2)"));
    assert!(fitted.text.contains("# WARN"));
    let render = |focus| {
        let mut terminal = VisibleEscapesTerminal::new(false, true);
        LinePainter::new(&fitted, focus)
            .paint(&mut terminal, LineViewport::new(&fitted, 0, 0), 200)
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
        UnicodeWidthStr::width(fitted.text.as_str()) <= UnicodeWidthStr::width(line.text.as_str())
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
fn sequence_position_is_kept_before_a_fitted_document_preview() {
    let mut flat = parse_top_level_json(r#"{"name":"Ada","active":true} 7"#.into()).unwrap();
    let roots: Vec<_> = flat
        .0
        .iter()
        .enumerate()
        .filter(|(_, row)| row.parent.is_nil() && !row.is_closing_of_container())
        .map(|(node, _)| node)
        .collect();
    let layout = Fixture::canonical(&flat);
    let projected =
        layout.project_with_documents(&flat, &std::collections::HashSet::from([roots[0]]));
    let fitted = fit_annotations(&projected[0].line, 24);
    assert!(fitted.text.starts_with("--- (1 of 2)"));
    assert!(fitted.text.contains('…'));
    assert_eq!(
        fitted.spans[1].role,
        TokenRole::DocumentPosition,
        "the sequence position is its own annotation rather than preview text"
    );
    assert!(
        fitted
            .spans
            .iter()
            .filter(|span| span.role == TokenRole::Preview)
            .all(|span| span.source.is_none())
    );
    flat.expand(roots[0]);
    assert_eq!(layout.project(&flat)[0].line.text, "--- (1 of 2)");
}

#[test]
fn generated_warning_text_is_not_highlighted_as_a_search_match() {
    let flat = parse_top_level_yaml("value: .inf".into()).unwrap();
    let layout = Fixture::canonical(&flat);
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
