use super::*;
use crate::toon_display::TokenRole;

#[test]
fn table_alignment_tracks_parsed_identity_across_focus_filters_and_collapse() {
    let mut v = viewer(
        r#"{"a":[{"id":1,"name":"Ada"},{"id":200,"name":"Lin"}],"b":[{"x":1},{"x":2}],"other":3}"#,
    );
    let a = v.flatjson[0].first_child().unwrap();
    let b = v.flatjson[a].next_sibling.unwrap();
    let other = v.flatjson[b].next_sibling.unwrap();
    let row = v.flatjson[a].first_child().unwrap();
    let cell = v.flatjson[row].first_child().unwrap();
    for node in [a, row, cell] {
        v.perform_action(Action::FocusNode { node, source: None });
        assert_eq!(v.focused_table(), Some(a));
    }
    let anchor = v.absolute_anchor_line;
    assert_eq!(v.toggle_table_alignment(), Some(a));
    assert_eq!(
        (
            v.table_alignment_enabled(a),
            v.table_alignment_enabled(b),
            v.focused_node,
            v.absolute_anchor_line
        ),
        (true, false, cell, anchor),
        "alignment applies to the selected table and preserves cell focus"
    );
    v.perform_action(Action::FocusNode {
        node: other,
        source: None,
    });
    assert_eq!(v.focused_table(), None);
    assert_eq!(v.toggle_table_alignment(), None);
    v.perform_action(Action::FocusNode {
        node: b,
        source: None,
    });
    assert_eq!(v.toggle_table_alignment(), Some(b));
    assert!(v.table_alignment_enabled(a));
    v.perform_action(Action::FocusNode {
        node: a,
        source: None,
    });
    assert_alignment_survives_collapse(&mut v, a);
    v.set_roots(vec![cell]);
    assert_eq!(v.focused_table(), None);
    assert!(v.table_alignment_enabled(a));
    v.set_roots(vec![row]);
    assert_eq!(v.focused_table(), None);
    v.set_roots(vec![0]);
    v.perform_action(Action::FocusNode {
        node: cell,
        source: None,
    });
    assert_eq!(v.focused_table(), Some(a));
    assert!(v.table_alignment_enabled(a));
    let mut sequence = viewer(r#"[{"x":1}] {"tail":0}"#);
    assert_eq!(sequence.focused_table(), None);
    assert_eq!(sequence.toggle_table_alignment(), None);
    sequence.perform_action(Action::MoveDown(1));
    assert_eq!(sequence.focused_table(), Some(0));
}

fn assert_alignment_survives_collapse(viewer: &mut JsonViewer, table: Index) {
    viewer.perform_action(Action::ToggleCollapsed);
    assert!(viewer.flatjson[table].is_collapsed());
    assert_eq!(viewer.focused_table(), Some(table));
    assert_eq!(
        viewer.aligned_table_for_line(viewer.focused_line_index()),
        None
    );
    for _ in 0..2 {
        assert_eq!(viewer.toggle_table_alignment(), Some(table));
        assert!(viewer.flatjson[table].is_collapsed());
    }
    viewer.perform_action(Action::ToggleCollapsed);
    assert!(
        viewer
            .render_line(viewer.focused_line_index())
            .text
            .contains("id ")
    );
}

#[test]
fn aligned_tokens_keep_terminal_columns_and_padding_outside_source_spans() {
    let mut v = viewer(r#"{"users":[{"id":1,"name":"Ada"},{"id":200,"name":"Lin"}]}"#);
    let table = v.flatjson[0].first_child().unwrap();
    v.perform_action(Action::FocusNode {
        node: table,
        source: None,
    });
    assert_eq!(v.toggle_table_alignment(), Some(table));
    let lines: Vec<_> = (0..v.visible.len()).map(|i| v.render_line(i)).collect();
    assert_eq!(
        lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>(),
        [
            "users[2]{id  name}:",
            "           1 Ada",
            "         200 Lin"
        ]
    );
    assert_eq!(v.table_indentation(table), 0);
    assert_eq!(v.aligned_table_width(table), 19);
    let header = &lines[0];
    let first_row = &lines[1];
    let id = first_row
        .spans
        .iter()
        .find(|span| span.role == crate::toon_display::TokenRole::Number)
        .unwrap();
    let name = first_row
        .spans
        .iter()
        .find(|span| span.role == crate::toon_display::TokenRole::String)
        .unwrap();
    assert_eq!(&first_row.text[id.range.end..name.range.start], " ");
    for (line, byte) in [
        (first_row, 10),
        (first_row, 9),
        (header, 11),
        (first_row, 12),
    ] {
        assert_eq!(
            crate::lineprinter::hit_test(line, byte),
            (line.owner, None),
            "padding at byte {byte}"
        );
    }
    assert_eq!(crate::lineprinter::hit_test(first_row, 13).0, name.node);
    let source = id.source.clone().unwrap();
    assert_eq!(id.matching_ranges(&source), vec![id.range.clone()]);
    assert_eq!(crate::lineprinter::hit_test(first_row, 11).0, id.node);
}

#[test]
fn aligned_numbers_use_right_edges_while_numeric_strings_stay_left_aligned() {
    let mut v = viewer(
        r#"{"users":[{"amount":1,"label":"7","n":2},{"amount":-12.5,"label":"009","n":300},{"amount":2.75,"label":"Ada","n":null},{"amount":0,"label":"Lin","n":"123456789"}]}"#,
    );
    let table = v.flatjson[0].first_child().unwrap();
    v.perform_action(Action::FocusNode {
        node: table,
        source: None,
    });
    v.toggle_table_alignment().unwrap();
    let lines: Vec<_> = (0..v.visible.len()).map(|i| v.render_line(i)).collect();
    assert_numeric_right_edges(&lines);
    assert_string_and_null_columns(&lines);
    assert_eq!(v.aligned_table_width(table), widest_line(&lines));
}

fn token_ranges(line: &DisplayLine, role: TokenRole) -> Vec<Range<usize>> {
    line.spans
        .iter()
        .filter(|span| span.role == role)
        .map(|span| span.range.clone())
        .collect()
}

fn assert_numeric_right_edges(lines: &[DisplayLine]) {
    let first = token_ranges(&lines[1], TokenRole::Number);
    let second = token_ranges(&lines[2], TokenRole::Number);
    let third = token_ranges(&lines[3], TokenRole::Number);
    assert_eq!(
        [second[0].end, third[0].end, second[1].end],
        [first[0].end, first[0].end, first[1].end],
        "numeric tokens share right edges"
    );
    let last_strings = token_ranges(&lines[4], TokenRole::String);
    assert_eq!(first[1].end, last_strings[1].end);
}

fn assert_string_and_null_columns(lines: &[DisplayLine]) {
    let header = token_ranges(&lines[0], TokenRole::FieldDefinition);
    let first = &token_ranges(&lines[1], TokenRole::String)[0];
    let second = &token_ranges(&lines[2], TokenRole::String)[0];
    let last = &token_ranges(&lines[4], TokenRole::String)[1];
    let null = &token_ranges(&lines[3], TokenRole::Null)[0];
    assert_eq!(
        [first.start, second.start, last.start, null.start],
        [
            header[1].start,
            header[1].start,
            header[2].start,
            header[2].start
        ],
        "strings and nulls align with their field starts"
    );
    assert_eq!(
        [
            &lines[1].text[first.clone()],
            &lines[2].text[second.clone()]
        ],
        ["\"7\"", "\"009\""]
    );
}

fn widest_line(lines: &[DisplayLine]) -> usize {
    lines
        .iter()
        .map(|line| UnicodeWidthStr::width(line.text.as_str()))
        .max()
        .unwrap()
}

#[test]
fn aligned_final_numbers_include_leading_padding_in_warning_bounds() {
    let mut v = JsonViewer::new(
        parse_top_level_yaml("rows:\n  - n: .inf\n  - n: abcdefghijklmnopqrstuvwxyz\n".into())
            .unwrap(),
    );
    let table = v.flatjson[0].first_child().unwrap();
    v.perform_action(Action::FocusNode {
        node: table,
        source: None,
    });
    v.toggle_table_alignment().unwrap();
    let lines: Vec<_> = (0..v.visible.len()).map(|i| v.render_line(i)).collect();
    let number = lines[1]
        .spans
        .iter()
        .find(|span| span.role == TokenRole::Number)
        .unwrap();
    let string = lines[2]
        .spans
        .iter()
        .find(|span| span.role == TokenRole::String)
        .unwrap();
    assert_eq!(number.range.end, string.range.end);
    assert_eq!(v.aligned_table_width(table), widest_line(&lines));
}

#[test]
fn aligned_unicode_controls_and_lazy_offscreen_measurements_survive_reflow() {
    const COUNT: usize = 512;
    let mut values = vec![r#"{"id":1,"text":"e\u0301"}"#; COUNT];
    values[COUNT - 1] = r#"{"id":999,"text":"界🦊,\\\u0001"}"#;
    let input = format!(r#"{{"box":{{"rows":[{}]}}}}"#, values.join(","));
    let mut v = viewer(&input);
    v.set_viewport(
        TTYDimensions {
            width: 28,
            height: 4,
        },
        true,
    );
    let box_node = v.flatjson[0].first_child().unwrap();
    let table = v.flatjson[box_node].first_child().unwrap();
    let last = v.flatjson[v.flatjson[table].pair_index().unwrap()]
        .last_child()
        .unwrap();
    v.perform_action(Action::FocusNode {
        node: table,
        source: None,
    });
    let before = Layout::formatted_rows();
    assert_eq!(v.toggle_table_alignment(), Some(table));
    assert!(Layout::formatted_rows() - before <= 12);
    assert!(v.frame.len() <= 5);
    let first = v.render_line(v.focused_line_index() + 1);
    let last_line = v.layout.node(&v.flatjson, last).line;
    let last_line = v.visible.visible_index(last_line).unwrap();
    let widest = v.render_line(last_line);
    assert_aligned_unicode_cells(&first, &widest);

    assert_eq!(
        v.aligned_table_width(table),
        UnicodeWidthStr::width(widest.text.as_str())
    );
    let width = v.aligned_table_width(table);
    v.set_wrap_geometry(14, v.table_indentation(table));
    let before = Layout::formatted_rows();
    for _ in 0..3 {
        v.set_wrap_geometry(14, v.table_indentation(table));
        assert_eq!(v.aligned_table_width(table), width);
        assert!(v.frame.len() <= 5);
    }
    assert_eq!(Layout::formatted_rows() - before, 0);
    v.toggle_wrapping();
    assert!(!v.is_wrapped_line(v.focused_line_index()));
    assert!(!v.is_wrapped_line(last_line));
    v.perform_action(Action::FocusNode {
        node: box_node,
        source: None,
    });
    assert!(v.is_wrapped_line(v.focused_line_index()));
    v.perform_action(Action::FocusNode {
        node: table,
        source: None,
    });
    v.toggle_table_alignment();
    assert!(v.is_wrapped_line(v.focused_line_index()));
    assert_eq!(v.focused_node, table);
    assert!(
        v.render_line(v.focused_line_index())
            .text
            .contains("{id,text}")
    );
}

fn assert_aligned_unicode_cells(first: &DisplayLine, widest: &DisplayLine) {
    assert!(widest.text.contains("界🦊,"));
    let token_column = |line: &DisplayLine, role| {
        let span = line.spans.iter().find(|span| span.role == role).unwrap();
        UnicodeWidthStr::width(&line.text[..span.range.end])
    };
    assert_eq!(
        token_column(widest, TokenRole::Number),
        token_column(first, TokenRole::Number)
    );
    let text_column = |line: &DisplayLine| {
        let span = line
            .spans
            .iter()
            .find(|span| span.role == TokenRole::String)
            .unwrap();
        UnicodeWidthStr::width(&line.text[..span.range.start])
    };
    assert_eq!(text_column(first), text_column(widest));
    assert!(!widest.text.contains("# WARN"));
}

#[test]
fn nested_grid_retains_columns_across_gutters_and_indentation_reduction() {
    let mut v = viewer(
        r#"{"outer":[{"rows":[{"id":1,"text":"e\u0301"},{"id":200,"text":"界🦊"}],"flag":true}]}"#,
    );
    let outer = v.flatjson[0].first_child().unwrap();
    let object = v.flatjson[outer].first_child().unwrap();
    let table = v.flatjson[object].first_child().unwrap();
    v.perform_action(Action::FocusNode {
        node: table,
        source: None,
    });
    v.toggle_table_alignment();
    for numbers in [true, false, true] {
        v.set_viewport(
            TTYDimensions {
                width: 24,
                height: 8,
            },
            numbers,
        );
        let logical = v.focused_line_index();
        let lines = [
            v.render_line(logical),
            v.render_line(logical + 1),
            v.render_line(logical + 2),
        ];
        let indentation = v.table_indentation(table);
        assert_nested_grid_columns(&lines, indentation);
        assert_eq!(
            [
                v.aligned_table_for_line(logical),
                v.aligned_table_for_line(logical + 1)
            ],
            [Some(table); 2]
        );
        assert_eq!(v.aligned_table_width(table), widest_line(&lines));
        v.set_wrap_geometry(10, indentation);
        assert!(!v.is_wrapped_line(logical));
    }
}

fn assert_nested_grid_columns(lines: &[DisplayLine; 3], indentation: usize) {
    let [header, row, long] = lines;
    let column = |line: &DisplayLine, role, ordinal: usize| {
        let range = &token_ranges(line, role)[ordinal];
        let edge = if role == TokenRole::Number {
            range.end
        } else {
            range.start
        };
        UnicodeWidthStr::width(&line.text[..edge]) - indentation
    };
    let text_column = column(row, TokenRole::String, 0);
    assert_eq!(
        [
            column(long, TokenRole::Number, 0),
            column(header, TokenRole::FieldDefinition, 1),
            column(long, TokenRole::String, 0),
        ],
        [column(row, TokenRole::Number, 0), text_column, text_column],
        "number and string columns agree with the first row"
    );
}
