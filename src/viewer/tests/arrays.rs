use super::*;

#[test]
fn inline_array_controls_expand_and_table_rows_do_not_collapse() {
    for action in [Action::ToggleCollapsed, Action::ClickArrow(1)] {
        let mut v = viewer("[1,2]");
        assert!(v.layout.node(&v.flatjson, 0).inline_array);
        v.perform_action(action);
        assert!(!v.layout.node(&v.flatjson, 0).inline_array);
        assert!(v.flatjson[0].is_expanded());
    }
    let mut v = viewer(r#"[{"a":1},{"a":2}]"#);
    v.perform_action(Action::MoveRight);
    let row = v.focused_node;
    let original = v.render_line(v.focused_line_index()).text;
    v.perform_action(Action::ToggleCollapsed);
    assert!(!v.layout.node(&v.flatjson, row).collapsible);
    assert!(v.flatjson[row].is_expanded());
    assert_eq!(v.render_line(v.focused_line_index()).text, original);
    v.perform_action(Action::FocusParent);
    v.perform_action(Action::ToggleCollapsed);
    assert!(v.flatjson[0].is_collapsed());
}

#[test]
fn right_expands_inline_array_before_entering_elements() {
    let mut v = viewer("[1,2]");
    assert_eq!(v.visible.len(), 1);
    v.perform_action(Action::MoveRight);
    assert_eq!(v.focused_node, 0);
    assert_eq!(
        (0..v.visible.len())
            .map(|line| v.render_line(line).text)
            .collect::<Vec<_>>(),
        vec!["[2]:", "  - 1", "  - 2"]
    );
    v.perform_action(Action::MoveRight);
    assert_eq!(path(&v), "[0]");
}

#[test]
fn arrays_default_to_multiline_above_five_elements() {
    assert_eq!(viewer("[1,2,3,4,5]").visible.len(), 1);
    let v = viewer("[1,2,3,4,5,6]");
    assert_eq!(v.visible.len(), 7);
    assert_eq!(v.render_line(6).text, "  - 6");
}

#[test]
fn initial_geometry_respects_unicode_fit_with_and_without_numbers() {
    for (width, numbers, inline) in [
        (18, true, true),
        (17, true, false),
        (15, false, true),
        (14, false, false),
    ] {
        let flat = parse_top_level_json(r#"{"tags":["界","é"]}"#.into()).unwrap();
        let v = JsonViewer::with_roots(
            flat,
            vec![0],
            TTYDimensions { width, height: 24 },
            numbers,
            false,
        );
        let lines: Vec<_> = (0..v.visible.len())
            .map(|line| v.render_line(line).text)
            .collect();
        let expected = if inline {
            vec!["tags[2]: 界,é"]
        } else {
            vec!["tags[2]:", "  - 界", "  - é"]
        };
        assert_eq!(lines, expected, "width={width}, numbers={numbers}");
    }
}

#[test]
fn inline_array_fit_includes_gutters_and_terminal_cells() {
    let mut v = viewer(r#"{"tags":["界","é"]}"#);
    // tags[2]: 界,é occupies 13 cells, plus five gutter cells.
    v.perform_action(Action::ResizeViewerDimensions(TTYDimensions {
        width: 18,
        height: 24,
    }));
    assert_eq!(v.visible.len(), 1);
    v.perform_action(Action::ResizeViewerDimensions(TTYDimensions {
        width: 17,
        height: 24,
    }));
    assert_eq!(v.visible.len(), 3);
    assert_eq!(v.render_line(1).text, "  - 界");
    v.perform_action(Action::ResizeViewerDimensions(TTYDimensions {
        width: 18,
        height: 24,
    }));
    assert_eq!(v.visible.len(), 1);
}

#[test]
fn explicit_array_expansion_survives_resize_collapse_and_search() {
    let mut v = viewer(r#"{"box":{"tags":["rust","cli"]},"tail":0}"#);
    act(
        &mut v,
        &[Action::MoveRight, Action::MoveRight, Action::MoveRight],
    );
    assert_eq!(path(&v), ".box.tags");
    assert_eq!(v.render_line(2).text, "    - rust");
    let array = v.focused_node;
    act(&mut v, &[Action::MoveLeft, Action::MoveRight]);
    assert_eq!(v.focused_node, array);
    assert!(!v.layout.node(&v.flatjson, array).inline_array);
    let child = v.flatjson[array].first_child().unwrap();
    let second = v.flatjson[child].next_sibling.unwrap();
    v.perform_action(Action::MoveLeft);
    v.perform_action(Action::FocusNode {
        node: second,
        source: Some(v.flatjson[second].range.start),
    });
    assert_eq!(path(&v), ".box.tags[1]");
    assert_eq!(v.render_line(v.focused_line_index()).text, "    - cli");
    v.set_viewport(
        TTYDimensions {
            width: 200,
            height: 24,
        },
        false,
    );
    assert_eq!(v.focused_node, second);
    assert!(!v.layout.node(&v.flatjson, array).inline_array);
    assert_eq!(&v.flatjson.1[v.flatjson[second].range.clone()], "\"cli\"");
}

#[test]
fn array_fit_rechecks_line_number_digits_and_number_visibility() {
    let mut fields = vec!["\"wide\":[1,2,3,4,5,6]".to_string()];
    fields.extend((0..94).map(|i| format!("\"k{i}\":0")));
    fields.push("\"tags\":[\"界\",\"é\"]".into());
    let flat = parse_top_level_json(format!("{{{}}}", fields.join(","))).unwrap();
    let mut v = JsonViewer::with_roots(
        flat,
        vec![0],
        TTYDimensions {
            width: 18,
            height: 24,
        },
        true,
        false,
    );
    assert!(v.layout.line_count() > 99);
    assert_eq!(v.render_line(v.visible.len() - 1).text, "  - é");
    v.set_viewport(
        TTYDimensions {
            width: 18,
            height: 24,
        },
        false,
    );
    assert_eq!(v.render_line(v.visible.len() - 1).text, "tags[2]: 界,é");
    v.set_viewport(
        TTYDimensions {
            width: 19,
            height: 24,
        },
        true,
    );
    assert_eq!(v.render_line(v.visible.len() - 1).text, "tags[2]: 界,é");
}

#[test]
fn inline_array_warning_width_is_included_without_losing_warnings() {
    let mut v = JsonViewer::new(parse_top_level_yaml("[.inf, 1]".into()).unwrap());
    assert_eq!(v.visible.len(), 1);
    v.set_viewport(
        TTYDimensions {
            width: 25,
            height: 24,
        },
        true,
    );
    assert_eq!(v.visible.len(), 3);
    assert_eq!(
        v.render_line(1).text,
        "  - .inf  # WARN Non-finite number at [0]"
    );
    v.perform_action(Action::MoveLeft);
    assert!(v.render_line(0).text.contains("Contains 1 hidden warnings"));
}

#[test]
fn right_opens_collapsed_inline_arrays_as_multiline() {
    let mut v = viewer("[1,2]");
    act(&mut v, &[Action::MoveLeft, Action::MoveRight]);
    assert_eq!(v.focused_node, 0);
    assert_eq!(v.visible.len(), 3);
    v.perform_action(Action::ClickArrow(1));
    assert_eq!(v.visible.len(), 1);
    v.perform_action(Action::ClickArrow(1));
    assert_eq!(v.visible.len(), 3);
}

#[test]
fn inline_values_retain_individual_identity_and_copy_ranges() {
    let mut v = viewer(r#"{"tags":["rust","cli"],"done":true}"#);
    v.perform_action(Action::MoveRight);
    let child = v.flatjson[v.focused_node].first_child().unwrap();
    v.perform_action(Action::FocusNode {
        node: child,
        source: None,
    });
    assert_eq!(path(&v), ".tags[0]");
    let line = v.absolute_anchor_line;
    v.perform_action(Action::FocusNextSibling(1));
    assert_eq!(path(&v), ".tags[1]");
    assert_eq!(
        &v.flatjson.1[v.flatjson[v.focused_node].range.clone()],
        "\"cli\""
    );
    assert_eq!(line, v.absolute_anchor_line);
    v.perform_action(Action::MoveDown(1));
    assert_eq!(path(&v), ".done");
    assert_eq!(v.absolute_anchor_line, line + 1);
    v.perform_action(Action::MoveUp(1));
    assert_eq!(path(&v), ".tags");
}
