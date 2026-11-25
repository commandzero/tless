use super::*;

#[test]
fn offscreen_final_records_control_table_eligibility_and_warning_totals() {
    let prefix = vec![r#"{"a":1,"b":2}"#; 64].join(",");
    for (last, header) in [
        (r#"{"a":3,"b":4}"#, "[65]{a,b}:"),
        (r#"{"a":3}"#, "[65]:"),
        (r#"{"b":4,"a":3}"#, "[65]:"),
        (r#"{"a":3,"b":{"nested":4}}"#, "[65]:"),
        (r#"{"a":3,"\u0061":4}"#, "[65]:"),
    ] {
        let mut v = viewer(&format!("[{prefix},{last}]"));
        assert_eq!(v.rendered_line(0).text, header);
        if last.contains("\\u0061") {
            v.perform_action(Action::ToggleCollapsed);
            assert!(
                v.rendered_line(0)
                    .text
                    .contains("Contains 2 hidden warnings")
            );
        }
    }
    let mut v = viewer(r#"{"box":{"\u0001":"\u0001"}}"#);
    v.perform_action(Action::MoveRight);
    v.perform_action(Action::ToggleCollapsed);
    assert!(!v.rendered_line(0).text.contains("# WARN"));
}

#[test]
fn viewport_work_and_retention_do_not_scale_with_tails_or_visited_rows() {
    const HEIGHT: usize = 8;
    let mut initial_work = None;
    for count in [64, 4096] {
        let input = format!(
            r#"{{"records":[{}]}}"#,
            vec![r#"{"id":0,"nested":{"ok":true,"label":"record"},"values":[1,2,3]}"#; count]
                .join(",")
        );
        let flat = parse_top_level_json(input).unwrap();
        let before = Layout::formatted_rows();
        let mut v = JsonViewer::with_roots(
            flat,
            vec![0],
            TTYDimensions {
                width: 80,
                height: HEIGHT as u16,
            },
            true,
            false,
        );
        let work = Layout::formatted_rows() - before;
        assert!(work <= HEIGHT + 1, "startup formatted {work} rows");
        if let Some(expected) = initial_work {
            assert_eq!(work, expected);
        }
        initial_work = Some(work);
        for visit in 0..32 {
            let last = v.layout.line_count() - 1;
            let line = [last, 0, last / 2, last / 3][visit % 4];
            let node = v.layout.row(&v.flatjson, line).unwrap().owner;
            let before = Layout::formatted_rows();
            v.perform_action(Action::FocusNode { node, source: None });
            let work = Layout::formatted_rows() - before;
            assert!(
                work <= 4 * (HEIGHT + 1),
                "distant navigation formatted {work} rows"
            );
            assert_eq!(v.focused_node, node);
            assert_frame_row_budget(&v, HEIGHT);
            let spans: usize = v.frame.iter().map(|(_, line)| line.spans.len()).sum();
            let maps: usize = v
                .frame
                .iter()
                .flat_map(|(_, line)| &line.spans)
                .map(|span| span.source_map.len())
                .sum();
            assert!(spans <= 16 * (HEIGHT + 1));
            assert!(maps <= 8 * (HEIGHT + 1));
        }
        v.toggle_wrapping();
        let before = Layout::formatted_rows();
        v.perform_action(Action::FocusBottom);
        assert!(Layout::formatted_rows() - before <= 6 * (HEIGHT + 1));
        assert_eq!(v.focused_line_index(), v.visible.len() - 1);
    }
}

#[test]
fn distant_escaped_table_key_highlights_shared_header_without_alias_fanout() {
    let input = format!(
        "[{},{{\"\\u006eeedle\":9}}]",
        vec![r#"{"needle":0}"#; 1000].join(",")
    );
    let mut v = viewer(&input);
    let last_row = v.flatjson[v.flatjson[0].pair_index().unwrap()]
        .last_child()
        .unwrap();
    let field = v.flatjson[last_row].first_child().unwrap();
    let key = v.flatjson[field].key_range.clone().unwrap();
    let query = key.start + 1..key.start + 7;
    assert!(
        !v.frame
            .iter()
            .any(|(_, line)| line.spans.iter().any(|span| span.node == field))
    );
    v.highlight_shared_fields(std::slice::from_ref(&query), &(0..0));
    let header = v.rendered_line(0);
    assert_escaped_header_highlight(header, &query, None);
    let before = Layout::formatted_rows();
    v.perform_action(Action::FocusNode {
        node: field,
        source: Some(query.start),
    });
    assert_eq!(v.focused_node, field);
    assert_eq!(v.focused_line_index(), 0);
    assert!(Layout::formatted_rows() - before <= 4 * (usize::from(v.dimensions.height) + 1));
    v.highlight_shared_fields(std::slice::from_ref(&query), &query);
    let header = v.rendered_line(0);
    assert!(header.spans.len() < 12);
    assert!(
        header
            .spans
            .iter()
            .map(|span| span.source_map.len())
            .sum::<usize>()
            < 12
    );
    let alias = header.spans.iter().find(|span| span.node == field).unwrap();
    let range = alias.matching_ranges(&query).pop().unwrap();
    assert_eq!(&header.text[range.clone()], "n");
    let first_field = v.flatjson[v.flatjson[0].first_child().unwrap()]
        .first_child()
        .unwrap();
    assert_eq!(
        crate::lineprinter::hit_test(header, range.start).0,
        first_field
    );
    assert_escaped_header_highlight(header, &query, Some(field));
}

fn assert_escaped_header_highlight(
    header: &DisplayLine,
    query: &Range<usize>,
    focused_field: Option<Index>,
) {
    use crate::lineprinter::{LinePainter, LineViewport};
    use crate::terminal::Terminal;
    use crate::terminal::test::VisibleEscapesTerminal;

    let (selection, current_match, highlight) = match focused_field {
        Some(field) => (field..field + 1, query.clone(), "_FG(LightYellow)__U_n"),
        None => (0..0, 0..0, "_FG(Yellow)__U_n"),
    };
    let mut terminal = VisibleEscapesTerminal::new(false, true);
    LinePainter::new(header, selection)
        .with_search(std::slice::from_ref(query), &current_match)
        .paint(&mut terminal, LineViewport::new(header, 0, 0), 80)
        .unwrap();
    assert!(
        terminal.output().contains(highlight),
        "{}",
        terminal.output()
    );
}

#[test]
fn distant_filtered_inline_match_and_wrapped_end_skip_intervening_rows() {
    let input = format!(
        r#"{{"keep":[{}],"outside":"excluded"}} {{"keep":{{"tags":["x","\u754cNEEDLE"],"long":"{}TAIL"}},"outside":"excluded"}}"#,
        vec![r#"{"id":0,"nested":{"ok":true}}"#; 1000].join(","),
        "abcdef".repeat(100)
    );
    let flat = parse_top_level_json(input.clone()).unwrap();
    let roots = keep_roots(&flat);
    let second = roots[1];
    let array = flat[second].first_child().unwrap();
    let long = flat[array].next_sibling.unwrap();
    let first = flat[array].first_child().unwrap();
    let target = flat[first].next_sibling.unwrap();
    let start =
        flat.1[flat[target].range.clone()].find("NEEDLE").unwrap() + flat[target].range.start;
    let query = start..start + 6;
    let mut v = narrow_filtered_viewer(flat, roots);
    assert!(v.layout.node(&v.flatjson, array).inline_array);
    v.toggle_wrapping();
    let before = Layout::formatted_rows();
    v.perform_action(Action::FocusNode {
        node: target,
        source: Some(start),
    });
    let logical = v.focused_line_index();
    let line = v.rendered_line(logical);
    let range = line
        .spans
        .iter()
        .find(|span| span.node == target && span.source.is_some())
        .unwrap()
        .matching_ranges(&query)
        .pop()
        .unwrap();
    assert_eq!(&line.text[range.clone()], "NEEDLE");
    v.reveal_byte_range(range.clone());
    assert!(Layout::formatted_rows() - before <= 36);
    assert_eq!(
        (
            v.focused_node,
            v.active_root_for(target),
            v.contains_node(0)
        ),
        (target, Some(second), false),
        "focus stays inside the selected roots"
    );
    assert!(v.physical_rows.iter().any(|row| row.logical_line == logical
        && row.bytes.start < range.end
        && range.start < row.bytes.end));
    let before = Layout::formatted_rows();
    v.perform_action(Action::FocusBottom);
    assert!(Layout::formatted_rows() - before <= 36);
    assert_eq!(v.focused_node, long);
    assert_frame_row_budget(&v, 5);

    assert_cold_wrapped_tail(&input, long);
}

fn assert_cold_wrapped_tail(input: &str, long: Index) {
    // Start fresh so the distant line and its continuations have not been rendered.
    let flat = parse_top_level_json(input.into()).unwrap();
    let roots = keep_roots(&flat);
    let mut cold = narrow_filtered_viewer(flat, roots);
    cold.toggle_wrapping();
    assert!(
        cold.frame
            .iter()
            .all(|(_, line)| line.spans.iter().all(|span| span.node != long))
    );
    let end = cold.flatjson[long].range.end - 1;
    let query = end - 4..end;
    let before = Layout::formatted_rows();
    cold.perform_action(Action::FocusNode {
        node: long,
        source: Some(query.start),
    });
    let logical = cold.focused_line_index();
    let line = cold.rendered_line(logical);
    let display = line
        .spans
        .iter()
        .flat_map(|span| span.matching_ranges(&query))
        .next()
        .unwrap();
    assert_eq!(&line.text[display.clone()], "TAIL");
    cold.reveal_byte_range(display.clone());
    assert!(Layout::formatted_rows() - before <= 36);
    assert_eq!(cold.focused_node, long);
    assert!(
        cold.physical_rows
            .iter()
            .any(|row| row.logical_line == logical
                && row.bytes.start < display.end
                && display.start < row.bytes.end)
    );
    assert_frame_row_budget(&cold, 5);
}

fn assert_frame_row_budget(viewer: &JsonViewer, height: usize) {
    let logical = viewer.frame.len();
    let physical = viewer.physical_rows.len();
    assert!(
        logical <= height + 1 && physical <= height,
        "retained {logical} logical rows and {physical} physical rows for height {height}"
    );
}

fn keep_roots(flat: &FlatJson) -> Vec<Index> {
    crate::path_filter::PathFilter::parse(".keep")
        .unwrap()
        .resolve(flat)
        .unwrap()
}

fn narrow_filtered_viewer(flat: FlatJson, roots: Vec<Index>) -> JsonViewer {
    JsonViewer::with_roots(
        flat,
        roots,
        TTYDimensions {
            width: 26,
            height: 5,
        },
        true,
        true,
    )
}

#[test]
fn distant_yaml_values_keep_paths_and_export_after_projection_changes() {
    let mut input = String::from("rows:\n");
    for id in 0..5000 {
        input.push_str(&format!("  - {{id: {id}, label: row}}\n"));
    }
    input.push_str("special:\n  ? [typed, key]\n  : .inf\n  text: \"line\\n界\"\n");
    let flat = parse_top_level_yaml(input).unwrap();
    let target = crate::path_filter::PathFilter::parse(".special.text")
        .unwrap()
        .resolve(&flat)
        .unwrap()[0];
    let special = flat[target].parent.unwrap();
    let mut v = JsonViewer::new(flat);
    v.perform_action(Action::FocusNode {
        node: special,
        source: None,
    });
    v.perform_action(Action::ToggleCollapsed);
    v.perform_action(Action::FocusNode {
        node: target,
        source: None,
    });
    assert_eq!(path(&v), ".special.text");
    assert_eq!(v.flatjson.string_value(target), Some("line\n界"));
    assert_eq!(
        crate::output::serialize_roots(&v.flatjson, crate::options::OutputFormat::Json, &[target])
            .unwrap(),
        "\"line\\n界\"\n"
    );
    v.set_roots(vec![special]);
    v.perform_action(Action::FocusNode {
        node: target,
        source: None,
    });
    assert_eq!(path(&v), ".special.text");
    assert!(
        v.render_line(v.focused_line_index())
            .text
            .contains("line\\n界")
    );
}

#[test]
fn repeated_collapse_through_paired_delimiters_keeps_projection_exact() {
    let mut flat = parse_top_level_json(r#"{"a":{"x":1},"b":{"y":2},"tail":3}"#.into()).unwrap();
    let roots = crate::path_filter::document_roots(&flat);
    let a = flat[roots[0]].first_child().unwrap();
    let b = flat[a].next_sibling.unwrap();
    let a_close = flat[a].pair_index().unwrap();
    flat.collapse(a_close);
    flat.collapse(a);
    flat.collapse(b);
    flat.expand(a);
    flat.expand(a_close);
    let mut v = JsonViewer::new(flat);
    assert_eq!(
        (0..v.visible.len())
            .map(|i| v.visible_line(i).unwrap().absolute)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 4]
    );
    v.flatjson.expand(b);
    v.refresh_projection();
    assert_eq!(
        (0..v.visible.len())
            .map(|i| v.visible_line(i).unwrap().absolute)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4]
    );
}
