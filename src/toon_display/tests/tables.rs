use super::*;

#[test]
fn shared_header_mapping_and_table_rows_remain_visible() {
    let mut flat = yaml("- a: .inf\n  b: 2\n- a: .nan\n  b: 4\n");
    let layout = Fixture::canonical(&flat);
    let rows = children(&flat, 0);
    let fields = children(&flat, rows[1]);
    let visible = layout.layout.project_with_documents(&flat, &HashSet::new());
    let header = layout.layout.render(
        &flat,
        layout.layout.visible_line(&flat, &visible, 0).unwrap(),
        fields[0],
        None,
    );
    let key_span = header
        .spans
        .iter()
        .find(|span| {
            span.node == fields[0]
                && matches!(span.role, TokenRole::Key | TokenRole::FieldDefinition)
        })
        .unwrap();
    assert_eq!(key_span.source, flat[fields[0]].key_range);
    assert_eq!(&header.text[key_span.range.clone()], "a");
    assert!(
        layout.lines[2]
            .spans
            .iter()
            .any(|span| span.node == fields[0] && span.role == TokenRole::Warning)
    );
    flat.collapse(rows[1]);
    let collapsed = layout.project(&flat);
    assert_eq!(collapsed[2].absolute, 2);
    assert!(!layout.nodes[rows[1]].collapsible);
    assert!(layout.nodes[0].collapsible);
    assert_eq!(collapsed[2].line.text, layout.lines[2].text);
    assert!(
        collapsed[2]
            .line
            .spans
            .iter()
            .any(|span| span.node == fields[0])
    );
    flat.expand(rows[1]);
    assert_eq!(layout.project(&flat)[2].line.text, layout.lines[2].text);
}
#[test]
fn collapsed_table_header_retains_field_style_source_and_identity() {
    let mut flat = json(r#"[{"a":1},{"a":2}]"#);
    let layout = Fixture::canonical(&flat);
    let rows = children(&flat, 0);
    let first = children(&flat, rows[0])[0];
    let selected = children(&flat, rows[1])[0];
    flat.collapse(0);
    let projected = layout.layout.project_with_documents(&flat, &HashSet::new());
    let header = layout.layout.render(
        &flat,
        layout.layout.visible_line(&flat, &projected, 0).unwrap(),
        selected,
        None,
    );
    let key = header
        .spans
        .iter()
        .find(|span| span.node == selected && span.role == TokenRole::FieldDefinition)
        .unwrap();
    assert_eq!(key.source, flat[selected].key_range);
    assert_eq!(&header.text[key.range.clone()], "a");
    let (node, source) = crate::lineprinter::hit_test(&header, key.range.start);
    assert_eq!(
        node, first,
        "mouse targeting is independent of the selected alias"
    );
    assert_eq!(
        source,
        flat[first].key_range.as_ref().map(|range| range.start)
    );
}

#[test]
fn nested_uniform_tables_preserve_field_identity_and_row_positions() {
    let flat = json(r#"[{"id":1,"meta":{"score":2}},{"id":3,"meta":{"score":4}}]"#);
    assert_eq!(text(&flat), "[2]{id,meta{score}}:\n  1,2\n  3,4");
    let rows = children(&flat, 0);
    let group = children(&flat, rows[1])[1];
    let leaf = children(&flat, group)[0];
    let layout = Fixture::canonical(&flat);
    let analysis = &layout.layout.analysis;
    assert_eq!(analysis.table_owner(&flat, leaf), Some(0));
    assert_eq!(analysis.table_row_owner(&flat, leaf), Some(rows[1]));
    assert_eq!(
        analysis.table_cells(&flat, rows[1]).collect::<Vec<_>>(),
        vec![children(&flat, rows[1])[0], leaf]
    );
    assert!(layout.nodes[group].body_extent == layout.nodes[leaf].body_extent);
    assert!(!layout.nodes[group].collapsible);
    assert_eq!(
        layout
            .layout
            .source_line(&flat, leaf, flat[leaf].key_range.as_ref().map(|r| r.start)),
        0
    );
    assert_eq!(
        layout
            .layout
            .source_line(&flat, leaf, Some(flat[leaf].range.start)),
        2
    );
    let first_leaf = children(&flat, children(&flat, rows[0])[1])[0];
    let definition = layout.lines[0]
        .spans
        .iter()
        .find(|span| span.node == first_leaf && span.role == TokenRole::FieldDefinition)
        .unwrap();
    assert_eq!(&layout.lines[0].text[definition.range.clone()], "score");
    let visible = layout
        .layout
        .visible_line(
            &flat,
            &layout.layout.project_with_documents(&flat, &HashSet::new()),
            0,
        )
        .unwrap();
    let focused_header = layout.layout.render(&flat, visible, leaf, None);
    assert!(focused_header.spans.iter().any(|span| {
        span.node == leaf
            && span.role == TokenRole::FieldDefinition
            && span.source == flat[leaf].key_range
    }));
    let mut shared = layout.lines[0].clone();
    let source = flat[leaf].key_range.clone().unwrap();
    layout
        .layout
        .highlight_shared_fields(&flat, &mut shared, &[source], &(0..0));
    assert!(
        shared
            .shared_matches
            .iter()
            .any(|(range, _)| { &shared.text[range.clone()] == "score" })
    );
}

#[test]
fn keyed_tables_obey_position_and_order_restrictions() {
    assert_eq!(
        text(&json(r#"{"alice":{"age":30},"bob":{"age":25}}"#)),
        "[2:]{age}:\n  alice: 30\n  bob: 25"
    );
    assert_eq!(
        text(&json(r#"{"server":{"a":{"x":1},"b":{"x":2}}}"#)),
        "server[2:]{x}:\n  a: 1\n  b: 2"
    );
    let anonymous = json(r#"[{"a":{"x":1},"b":{"x":2}},0]"#);
    assert_eq!(
        text(&anonymous),
        "[2]:\n  - a:\n      x: 1\n    b:\n      x: 2\n  - 0"
    );
    assert!(
        !Fixture::canonical(&anonymous)
            .layout
            .analysis
            .node(&anonymous, children(&anonymous, 0)[0])
            .table
    );
    let reordered = json(r#"[{"a":1,"b":{"x":2,"y":3}},{"a":4,"b":{"y":5,"x":6}}]"#);
    assert!(
        !Fixture::canonical(&reordered)
            .layout
            .analysis
            .node(&reordered, 0)
            .table
    );
}

#[test]
fn deep_uniform_groups_keep_table_text_without_recursive_stack_growth() {
    let depth = 512;
    let prefix = r#"{"group":"#.repeat(depth);
    let suffix = "}".repeat(depth);
    // Constrain only the display traversal, not the existing input parser.
    let flat = json(&format!("[{prefix}1{suffix},{prefix}2{suffix}]"));
    std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(move || {
            let layout = layout::Layout::new(&flat, &[0], 120, false, &HashSet::new());
            let projection = layout.project_with_documents(&flat, &HashSet::new());
            let header = layout.render(
                &flat,
                layout.visible_line(&flat, &projection, 0).unwrap(),
                0,
                None,
            );
            assert_eq!(
                header.text,
                format!(
                    "[2]{{{}group{}}}:",
                    "group{".repeat(depth - 1),
                    "}".repeat(depth - 1)
                )
            );
            for (line, expected) in [(1, "  1"), (2, "  2")] {
                let row = layout.render(
                    &flat,
                    layout.visible_line(&flat, &projection, line).unwrap(),
                    0,
                    None,
                );
                assert_eq!(row.text, expected);
            }
            let metrics = alignment::TableMetrics::measure(&flat, &layout.analysis, 0);
            assert_eq!(metrics.widths, [depth * 7 - 2]);
        })
        .unwrap()
        .join()
        .unwrap();
}
