use super::*;

#[test]
fn root_anchors_and_bounded_preview() {
    let mut flat = json("{\"a\":{\"b\":1},\"c\":2}");
    let layout = Fixture::canonical(&flat);
    assert_eq!(layout.nodes[0].line, 0);
    assert!(!layout.nodes[0].collapsible);
    flat.collapse(1);
    let visible = layout.project(&flat);
    assert_eq!(
        visible.iter().map(|line| line.absolute).collect::<Vec<_>>(),
        vec![0, 2]
    );
    // Exercise the parsed-model boundary without involving the tokenizer's
    // unrelated large non-ASCII string behavior.
    let mut flat = json(r#"["x"]"#);
    flat.1 = format!("[\"{}\"]", "界".repeat(100_000));
    let length = flat.1.len();
    flat[0].range = 0..length;
    flat[1].range = 1..length - 1;
    flat[2].range = length - 1..length;
    let layout = Fixture::for_view(&flat, 120, &HashSet::new());
    flat.collapse(0);
    let visible = layout.project(&flat);
    assert!(visible[0].line.text.len() < 280);
    assert!(visible[0].line.text.ends_with('…'));
}

#[test]
fn mappings_and_collapse_restore() {
    let mut flat = json(r#"{"tags":[1,2],"rows":[{"a":3},{"a":4}],"obj":{"x":{"y":5}}}"#);
    let layout = Fixture::canonical(&flat);
    assert_eq!(layout.nodes[2].line, layout.nodes[3].line);
    flat.collapse(1);
    let visible = layout.project(&flat);
    assert_eq!(visible[0].line.text, "tags[2]: 1,2");
    assert!(
        visible[0]
            .line
            .spans
            .iter()
            .any(|s| s.role == TokenRole::Number)
    );
    assert!(
        !visible[0]
            .line
            .spans
            .iter()
            .any(|s| s.role == TokenRole::Preview)
    );
    flat.expand(1);
    assert_eq!(layout.project(&flat)[0].line.text, layout.lines[0].text);
    let mut dup = json(r#"{"a":{"b":{"x":1,"x":2}}}"#);
    let l = Fixture::canonical(&dup);
    dup.collapse(2);
    dup.collapse(1);
    assert!(
        l.project(&dup)[0]
            .line
            .text
            .contains("Contains 2 hidden warnings")
    );
    dup.expand(1);
    assert!(dup[2].is_collapsed());
    assert_eq!(l.project(&dup).len(), 2);
}
#[test]
fn selected_nested_root_renders_at_baseline_without_excluded_lines() {
    let flat = json(r#"{"outer":{"kept":1},"tail":{"excluded":2}}"#);
    let selected = children(&flat, 0)[0];
    let layout = Fixture::for_view_with_roots(&flat, 120, &HashSet::new(), &[selected]);
    let text = layout
        .lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(text, vec!["kept: 1"]);
    assert_eq!(layout.nodes[selected].line, 0);
    assert!(
        layout
            .lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .all(|span| { layout.active_root_for(span.node) == Some(selected) })
    );
}

#[test]
fn selected_root_key_control_is_not_a_warning() {
    let mut flat = json(r#"{"k\u0001":[1,2,3,4,5,6,7]}"#);
    let root = children(&flat, 0)[0];
    let layout = Fixture::for_view_with_roots(&flat, 120, &HashSet::new(), &[root]);
    assert!(layout.warnings.is_empty());
    flat.collapse(root);
    assert!(!layout.project(&flat)[0].line.text.contains("# WARN"));

    let flat = json(r#"{"k\u0001":"v\u0001"}"#);
    let root = children(&flat, 0)[0];
    let layout = Fixture::for_view_with_roots(&flat, 120, &HashSet::new(), &[root]);
    assert!(layout.warnings.is_empty());
    assert_eq!(layout.lines[0].text, "\"v\\u0001\"");
}

#[test]
fn selected_array_element_warning_has_no_excluded_parent_locator() {
    let flat = yaml("- .inf\n- 1\n");
    let root = children(&flat, 0)[0];
    let layout = Fixture::for_view_with_roots(&flat, 120, &HashSet::new(), &[root]);
    assert_eq!(layout.warnings[0].kind, WarningKind::NonFiniteNumber);
    assert!(!layout.lines[0].text.contains("at [0]"));
}

#[test]
fn selected_root_warnings_and_sequence_numbers_are_filtered() {
    let flat = yaml("- a: .inf\n- b: .nan\n");
    let rows = children(&flat, 0);
    let layout = Fixture::for_view_with_roots(&flat, 120, &HashSet::new(), &[rows[0]]);
    assert_eq!(layout.lines.len(), 1);
    assert!(layout.lines[0].text.contains("Non-finite number"));
    assert!(!layout.lines[0].text.contains("b:"));
    let stream = json(r#"{"a":1} {"b":2} {"c":3}"#);
    let roots = crate::path_filter::document_roots(&stream);
    let filtered =
        Fixture::for_view_with_roots(&stream, 120, &HashSet::new(), &[roots[2], roots[0]]);
    assert_eq!(filtered.lines[0].text, "--- (1 of 2)");
    assert_eq!(filtered.lines[2].text, "--- (2 of 2)");
}

#[test]
fn selected_first_field_is_not_merged_with_its_excluded_parent() {
    for (input, header) in [
        (r#"{"items":[{"a":1},{"a":2}]}"#, "[2]{a}:"),
        (r#"{"value":1}"#, "1"),
    ] {
        let flat = json(input);
        let root = children(&flat, 0)[0];
        let layout = layout::Layout::new(&flat, &[root], 120, true, &HashSet::new());
        let projection = layout.project_with_documents(&flat, &HashSet::new());
        let row = layout.visible_line(&flat, &projection, 0).unwrap();
        assert_eq!(row.owner, root);
        assert_eq!(layout.render(&flat, row, root, None).text, header);
    }
}

#[test]
fn filtered_out_of_order_subtrees_keep_exact_addresses_across_reflow_and_collapse() {
    let input = format!(
        r#"{{"skip":[{}],"first":{{"x":[1,2],"after":7}},"last":[{{"id":1}},{{"id":2}}]}}"#,
        vec!["0"; 512].join(",")
    );
    let mut flat = json(&input);
    let fields = children(&flat, 0);
    let roots = [fields[2], fields[1]];
    let x = children(&flat, fields[1])[0];
    let after = children(&flat, fields[1])[1];
    let mut layout = layout::Layout::new(&flat, &roots, 120, true, &HashSet::new());
    let projection = layout.project_with_documents(&flat, &HashSet::new());
    let lines: Vec<_> = (0..projection.len())
        .map(|i| {
            let row = layout.visible_line(&flat, &projection, i).unwrap();
            layout.render(&flat, row, row.owner, None).text
        })
        .collect();
    assert_eq!(
        lines,
        [
            "--- (1 of 2)",
            "[2]{id}:",
            "  1",
            "  2",
            "--- (2 of 2)",
            "x[2]: 1,2",
            "after: 7",
        ]
    );
    assert_eq!(layout.node(&flat, after).line, 6);
    layout.reflow(&flat, 9, true, &HashSet::new());
    assert_eq!(layout.line_count(), 9);
    assert_eq!(layout.node(&flat, after).line, 8);
    flat.collapse(0); // An excluded ancestor must not collapse selected roots.
    flat.collapse(x);
    let projected = layout.project_with_documents(&flat, &HashSet::from([roots[0]]));
    let addresses: Vec<_> = (0..projected.len())
        .map(|i| layout.visible_line(&flat, &projected, i).unwrap().absolute)
        .collect();
    assert_eq!(addresses, [0, 4, 5, 8]);
    assert_eq!(layout.row(&flat, 8).unwrap().owner, after);
    layout.reflow(&flat, 120, true, &HashSet::new());
    assert_eq!(layout.node(&flat, after).line, 6);
}
