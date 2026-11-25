use super::*;

#[test]
fn sequence_root_body_preview_keeps_source_mapping() {
    let mut flat = json(r#"[{"needle":1},{"other":2}] {}"#);
    let root = flat
        .0
        .iter()
        .enumerate()
        .find(|(_, row)| row.parent.is_nil() && !row.is_closing_of_container())
        .map(|(node, _)| node)
        .unwrap();
    let layout = Fixture::canonical(&flat);
    flat.collapse(root);
    let line = layout
        .project_with_documents(&flat, &HashSet::new())
        .into_iter()
        .find(|line| line.absolute == layout.nodes[root].body_line)
        .unwrap()
        .line;
    let preview = line
        .spans
        .iter()
        .find(|span| span.role == TokenRole::Preview)
        .unwrap();
    let source_start = flat.1.find("needle").unwrap();
    assert!(preview.source.is_some());
    assert!(
        !preview
            .matching_ranges(&(source_start..source_start + 6))
            .is_empty()
    );
}

#[test]
fn sequence_headers_preserve_roots_and_collapse_each_shape() {
    let flat = json(r#"{"name":"Ada"} 7 {} []"#);
    let roots: Vec<_> = flat
        .0
        .iter()
        .enumerate()
        .filter(|(_, row)| row.parent.is_nil() && !row.is_closing_of_container())
        .map(|(node, _)| node)
        .collect();
    assert_eq!(roots.len(), 4);
    let layout = Fixture::canonical(&flat);
    assert_eq!(layout.lines[0].text, "--- (1 of 4)");
    assert_eq!(layout.lines[1].text, "name: Ada");
    assert_eq!(layout.lines[2].text, "--- (2 of 4)");
    assert_eq!(layout.lines[3].text, "7");
    assert_eq!(layout.lines[4].text, "--- (3 of 4)");
    assert_eq!(layout.lines[5].text, "");
    assert_eq!(layout.lines[6].text, "--- (4 of 4)");
    assert_eq!(layout.lines[7].text, "[]");
    assert_eq!(layout.nodes[roots[0]].line, 0);
    assert_eq!(layout.nodes[roots[1]].line, 2);
    assert_eq!(layout.nodes[roots[2]].line, 4);
    assert_eq!(layout.nodes[roots[3]].line, 6);
    assert_eq!(layout.nodes[roots[3]].body_line, 7);
    assert_eq!(layout.nodes[roots[3]].body_extent, 7..8);
    assert!(layout.warnings.is_empty());

    let document_collapsed = roots.iter().copied().collect::<HashSet<_>>();
    let visible = layout.project_with_documents(&flat, &document_collapsed);
    assert_eq!(
        visible
            .iter()
            .map(|line| line.line.text.as_str())
            .collect::<Vec<_>>(),
        vec![
            "--- (1 of 4) name: Ada",
            "--- (2 of 4) 7",
            "--- (3 of 4) {}",
            "--- (4 of 4) []",
        ]
    );
    assert!(
        visible
            .iter()
            .all(|line| line.line.separator && line.line.owner < flat.0.len())
    );
    assert!(
        visible
            .iter()
            .flat_map(|line| line.line.spans.iter())
            .filter(|span| span.role == TokenRole::Preview)
            .all(|span| span.source.is_none())
    );
}

#[test]
fn sequence_root_warning_moves_to_collapsed_header() {
    let flat = yaml("---\n.inf\n---\n7\n");
    let roots: Vec<_> = flat
        .0
        .iter()
        .enumerate()
        .filter(|(_, row)| row.parent.is_nil() && !row.is_closing_of_container())
        .map(|(node, _)| node)
        .collect();
    let layout = Fixture::canonical(&flat);
    assert_eq!(layout.warnings.len(), 1);
    assert_eq!(layout.warnings[0].kind, WarningKind::NonFiniteNumber);
    let document_collapsed = HashSet::from([roots[0]]);
    let header = &layout.project_with_documents(&flat, &document_collapsed)[0].line;
    assert_eq!(header.text, "--- (1 of 2) .inf  # WARN Non-finite number");
    assert!(
        header
            .spans
            .iter()
            .all(|span| span.source.is_none() || span.role != TokenRole::Preview)
    );
}

#[test]
fn sequence_root_array_warnings_keep_element_locators() {
    let flat = yaml("---\n[.inf, 7]\n---\n7\n");
    assert_eq!(
        text(&flat),
        "--- (1 of 2)\n[2]: .inf,7  # WARN Non-finite number at [0]\n--- (2 of 2)\n7"
    );
}

#[test]
fn root_array_body_collapse_keeps_document_header_expanded() {
    let mut flat = json(r#"[{"a":1},{"a":2}] {}"#);
    let roots: Vec<_> = flat
        .0
        .iter()
        .enumerate()
        .filter(|(_, row)| row.parent.is_nil() && !row.is_closing_of_container())
        .map(|(node, _)| node)
        .collect();
    let layout = Fixture::canonical(&flat);
    assert_ne!(
        layout.nodes[roots[0]].body_line,
        layout.nodes[roots[0]].line
    );
    flat.collapse(roots[0]);
    let visible = layout.project_with_documents(&flat, &HashSet::new());
    assert_eq!(visible[0].line.text, "--- (1 of 2)");
    assert_eq!(visible[1].absolute, layout.nodes[roots[0]].body_line);
    assert_eq!(visible[2].absolute, layout.nodes[roots[1]].line);
}

#[test]
fn collapsed_sequence_header_aggregates_nested_duplicate_warnings() {
    let flat = json(r#"{"nested":{"a":1,"a":2}} 7"#);
    let roots: Vec<_> = flat
        .0
        .iter()
        .enumerate()
        .filter(|(_, row)| row.parent.is_nil() && !row.is_closing_of_container())
        .map(|(node, _)| node)
        .collect();
    let nested = children(&flat, roots[0])[0];
    let layout = Fixture::canonical(&flat);
    assert_eq!(layout.nodes[nested].descendant_warnings, 2);
    let visible = layout.project_with_documents(&flat, &HashSet::from([roots[0]]));
    assert_eq!(visible[0].line.owner, roots[0]);
    assert!(
        visible[0]
            .line
            .text
            .ends_with("# WARN Contains 2 hidden warnings")
    );
    assert!(!visible[0].line.text.contains("Multiple document roots"));
}

#[test]
fn json_and_yaml_sequences_have_equivalent_document_layouts() {
    let json_layout = Fixture::canonical(&json(r#"{"a":1} {"b":2}"#));
    let yaml_layout = Fixture::canonical(&yaml("---\na: 1\n---\nb: 2\n"));
    let json_lines: Vec<_> = json_layout
        .lines
        .iter()
        .map(|line| line.text.as_str())
        .collect();
    let yaml_lines: Vec<_> = yaml_layout
        .lines
        .iter()
        .map(|line| line.text.as_str())
        .collect();
    assert_eq!(json_lines, yaml_lines);
    assert_eq!(
        json_lines,
        vec!["--- (1 of 2)", "a: 1", "--- (2 of 2)", "b: 2"]
    );
    assert!(
        json_layout
            .warnings
            .iter()
            .all(|warning| warning.kind != WarningKind::NonStringKey)
    );
    assert!(
        yaml_layout
            .warnings
            .iter()
            .all(|warning| warning.kind != WarningKind::NonStringKey)
    );
}

#[test]
fn expanding_a_document_restores_descendant_collapse_state() {
    let mut flat = json(r#"{"outer":{"x":1,"y":2},"tail":0} {}"#);
    let roots: Vec<_> = flat
        .0
        .iter()
        .enumerate()
        .filter(|(_, row)| row.parent.is_nil() && !row.is_closing_of_container())
        .map(|(node, _)| node)
        .collect();
    let outer = children(&flat, roots[0])[0];
    let layout = Fixture::canonical(&flat);
    flat.collapse(outer);
    let collapsed_document = HashSet::from([roots[0]]);
    let hidden = layout.project_with_documents(&flat, &collapsed_document);
    assert_eq!(hidden[0].line.text, "--- (1 of 2) outer: …; tail: 0");
    assert_eq!(hidden[1].line.text, "--- (2 of 2)");
    let reopened = layout.project_with_documents(&flat, &HashSet::new());
    assert!(
        reopened.iter().any(|visible| {
            visible.line.owner == outer && visible.line.text.starts_with("outer:")
        })
    );
    let outer_line = reopened
        .iter()
        .find(|visible| visible.line.owner == outer)
        .unwrap();
    assert!(outer_line.line.text.contains("x: 1; y: 2"));
}

fn collapsed_first_document(input: &str) -> String {
    let flat = json(input);
    let layout = Fixture::canonical(&flat);
    let visible = layout.project_with_documents(&flat, &HashSet::from([0]));
    visible[0].line.text.clone()
}

fn assert_preview_is_bounded(input: &str) {
    let line = collapsed_first_document(input);
    assert!(line.len() <= "--- (1 of 2) ".len() + 256);
    assert!(line.ends_with('…'));
}

#[test]
fn collapsed_scalar_document_preview_is_bounded() {
    let input = format!("\"{}\" 0", "x".repeat(10_000));
    assert_preview_is_bounded(&input);
}

#[test]
fn collapsed_array_document_preview_keeps_toon_header() {
    assert_eq!(
        collapsed_first_document("[10,20] {}"),
        "--- (1 of 2) [2]: 10,20"
    );
}

#[test]
fn collapsed_array_document_preview_stays_bounded_after_header() {
    let input = format!(
        "[{}] 0",
        (0..200)
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    assert_preview_is_bounded(&input);
}
