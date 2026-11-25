use super::*;

#[test]
fn duplicate_decoded_keys_keep_identity() {
    let flat = json(r#"{"box":{"a":1,"\u0061":2}}"#);
    let layout = Fixture::canonical(&flat);
    assert_eq!(
        text(&flat),
        "box:\n  a: 1  # WARN Duplicate key\n  a: 2  # WARN Duplicate key"
    );
    assert_eq!(layout.nodes[1].entry_count, 2);
    assert_eq!(layout.nodes[1].descendant_warnings, 2);
    assert_eq!(layout.nodes[2].occurrence, Some(1));
    assert_eq!(layout.nodes[3].occurrence, Some(2));
    assert_eq!(layout.lines[1].owner, 2);
    assert_eq!(layout.lines[2].owner, 3);
}
#[test]
fn warnings_on_shared_lines_and_source_text() {
    assert_eq!(
        text(&yaml("[1, .inf, .nan]")),
        "[3]: 1,.inf,.nan  # WARN Non-finite number at [1]; Non-finite number at [2]"
    );
    assert_eq!(
        text(&json(
            r##"["# WARN Duplicate key",".inf","true","05","a,b","-x"]"##
        )),
        r##"[6]: "# WARN Duplicate key",".inf","true","05","a,b","-x""##
    );
    let flat = yaml("- a: .inf\n- a: .nan\n");
    assert_eq!(
        text(&flat),
        "[2]{a}:\n  .inf  # WARN Non-finite number at field \"a\"\n  .nan  # WARN Non-finite number at field \"a\""
    );
}
#[test]
fn multiline_sequence_root_array_warnings_keep_element_locators() {
    let flat = yaml("---\n[.inf, 1, 2, 3, 4, 5]\n---\n7\n");
    let layout = Fixture::for_view(&flat, 20, &HashSet::new());
    let warning = layout
        .lines
        .iter()
        .find(|line| line.text.contains("Non-finite number"))
        .unwrap();
    assert!(warning.text.contains("at [0]"));
}

#[test]
fn escaped_locators_and_semantic_warning_counts() {
    let flat = yaml("- \"a\\nb\": .inf\n- \"a\\nb\": .nan\n");
    let layout = Fixture::canonical(&flat);
    assert!(layout.lines[1].text.ends_with("at field \"a\\nb\""));
    assert_eq!(layout.nodes[0].descendant_warnings, 2);
    assert_eq!(layout.warnings.len(), 2);
    let mut flat = json(r#"{"a":{"x":1,"x":2},"a":0}"#);
    let layout = Fixture::canonical(&flat);
    flat.collapse(1);
    let visible = layout.project(&flat);
    assert!(
        visible[0]
            .line
            .text
            .ends_with("# WARN Duplicate key; Contains 2 hidden warnings")
    );
    assert_eq!(layout.nodes[0].descendant_warnings, 4);
}
#[test]
fn list_first_field_inline_warnings_keep_element_locators() {
    let flat = yaml("- vals: [.inf, .nan]\n  nested: {}\n");
    assert_eq!(
        text(&flat),
        "[1]:\n  - vals[2]: .inf,.nan  # WARN Non-finite number at [0]; Non-finite number at [1]\n    nested:"
    );
}

#[test]
fn mixed_warning_kinds_follow_node_order_and_hidden_summary_is_last() {
    let flat = yaml(r#"["\u0001", .inf, 1e1000000]"#);
    let layout = Fixture::canonical(&flat);
    assert_eq!(
        layout.lines[0].text,
        r#"[3]: "\u0001",.inf,1e1000000  # WARN Non-finite number at [1]"#
    );
    assert_eq!(
        layout
            .warnings
            .iter()
            .map(|warning| warning.kind)
            .collect::<Vec<_>>(),
        vec![WarningKind::NonFiniteNumber]
    );
    let mut flat = yaml(".inf: {a: .inf}");
    let layout = Fixture::canonical(&flat);
    flat.collapse(1);
    assert!(
        layout.project(&flat)[0]
            .line
            .text
            .ends_with("# WARN Non-finite number; Non-string key; Contains 1 hidden warnings")
    );
}
#[test]
fn repeated_key_shapes_recheck_changed_keys_and_scalar_warnings() {
    let mut input = format!("[{},", vec![r#"{"a":1,"b":2}"#; 128].join(","));
    input.push_str(r#"{"a":1},{"a":2,"b":3,"a":4},"#);
    input.push_str(r#"{"\u0061":1e1000000,"b":3},"#);
    input.push_str(r#"{"a":4,"\u0061":5},"#);
    input.push_str(r#"{"\u0001":6,"b":7},"#);
    input.push_str(r#"{"\u0001":8,"b":9}]"#);
    let flat = json(&input);
    let layout = Fixture::for_view(&flat, 120, &HashSet::new());
    assert_eq!(layout.lines[0].text, "[134]:");
    assert_eq!(layout.nodes[0].descendant_warnings, 4);
    let kinds: Vec<_> = layout.warnings.iter().map(|warning| warning.kind).collect();
    assert_eq!(
        kinds,
        [
            WarningKind::DuplicateKey,
            WarningKind::DuplicateKey,
            WarningKind::DuplicateKey,
            WarningKind::DuplicateKey,
        ]
    );
}
