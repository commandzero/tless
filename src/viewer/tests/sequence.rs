use super::*;

#[test]
fn sequence_document_rows_are_structural_navigation_anchors() {
    let mut v = viewer(r#"{"a":1} {"b":2}"#);
    let roots = v.document_roots().to_vec();
    assert_eq!(roots.len(), 2);
    assert_eq!(v.focused_node, roots[0]);
    assert_eq!(v.absolute_anchor_line, v.document_header_absolute(roots[0]));

    // Enter the first document, then move back to its header and across
    // the root sibling. The parsed root identities remain unchanged.
    v.perform_action(Action::MoveRight);
    assert_eq!(path(&v), ".a");
    v.perform_action(Action::FocusParentOrPreviousSibling);
    assert_eq!(v.focused_node, roots[0]);
    assert_eq!(v.absolute_anchor_line, v.document_header_absolute(roots[0]));
    v.perform_action(Action::FocusNextAtParentLevel);
    assert_eq!(v.focused_node, roots[1]);
    assert_eq!(v.absolute_anchor_line, v.document_header_absolute(roots[1]));
}

#[test]
fn sibling_motion_from_a_document_row_does_not_descend_by_stale_depth() {
    let mut v = viewer(r#"{"a":1} {"b":2}"#);
    let roots = v.document_roots().to_vec();

    v.perform_action(Action::MoveRight);
    v.perform_action(Action::FocusPrevSibling(1));
    assert_eq!(v.focused_node, roots[0]);
    v.perform_action(Action::FocusNextSibling(1));
    assert_eq!(v.focused_node, roots[1]);
    assert_eq!(v.absolute_anchor_line, v.document_header_absolute(roots[1]));
}

#[test]
fn sequence_document_collapse_is_independent_of_root_array_body_state() {
    let mut v = viewer("[1,2] {\"tail\":3}");
    let roots = v.document_roots().to_vec();
    let first = roots[0];
    assert!(v.layout.node(&v.flatjson, first).inline_array);

    // Expanding the root array's body is separate from collapsing its
    // sequence row and must not change the document state.
    v.perform_action(Action::MoveRight);
    assert_ne!(v.focused_node, first);
    v.perform_action(Action::FocusParent);
    assert_eq!(v.focused_node, first);
    v.perform_action(Action::MoveDown(1));
    assert_eq!(v.focused_node, first);
    assert!(!v.focused_document_header());
    v.perform_action(Action::ToggleCollapsed);
    assert!(v.flatjson[first].is_collapsed());
    assert!(!v.is_document_collapsed(first));
    v.perform_action(Action::MoveRight);
    assert!(!v.flatjson[first].is_collapsed());
    assert!(!v.focused_document_header());
    assert_eq!(
        v.absolute_anchor_line,
        v.layout.node(&v.flatjson, first).body_line,
        "reopening a root array body preserves its body anchor"
    );
    v.perform_action(Action::FocusTop);
    assert!(!v.is_document_collapsed(first));
    v.perform_action(Action::ToggleCollapsed);
    assert!(v.is_document_collapsed(first));
    assert_eq!(v.focused_node, first);
    assert_eq!(v.focused_line_index(), 0);

    v.perform_action(Action::MoveRight);
    assert!(!v.is_document_collapsed(first));
    assert_eq!(v.focused_node, first);
    v.perform_action(Action::MoveRight);
    assert_eq!(path(&v), "[0]");
}

#[test]
fn scalar_sequence_search_reveal_keeps_body_line_anchor() {
    let mut v = viewer("1 2");
    let roots = v.document_roots().to_vec();
    let second = roots[1];
    v.perform_action(Action::ToggleCollapsed);
    assert!(v.is_document_collapsed(roots[0]));
    v.perform_action(Action::FocusNode {
        node: second,
        source: Some(v.flatjson[second].range.start),
    });
    assert!(!v.is_document_collapsed(second));
    assert_eq!(v.focused_node, second);
    assert_ne!(v.absolute_anchor_line, v.document_header_absolute(second));
    v.perform_action(Action::ResizeViewerDimensions(TTYDimensions {
        width: 24,
        height: 8,
    }));
    assert_eq!(v.focused_node, second);
    assert!(!v.focused_document_header());
    assert_eq!(
        v.absolute_anchor_line,
        v.layout.node(&v.flatjson, second).body_line,
        "resizing preserves a scalar root body anchor"
    );
}

#[test]
fn root_bulk_collapse_works_from_a_scalar_body_anchor() {
    for action in [
        Action::CollapseNodeAndSiblings,
        Action::DeepCollapseNodeAndSiblings,
    ] {
        let mut v = viewer("1 2 3");
        let roots = v.document_roots().to_vec();
        v.perform_action(Action::MoveDown(1));
        assert_eq!(v.focused_node, roots[0]);
        assert!(!v.focused_document_header());
        v.perform_action(action);
        assert!(roots.iter().all(|&root| v.is_document_collapsed(root)));
        assert_eq!(v.focused_line_index(), 0);
        assert_eq!(
            v.absolute_anchor_line,
            v.document_header_absolute(roots[0]),
            "collapsing the document rows recovers the hidden body anchor"
        );
    }
}

#[test]
fn scalar_and_empty_sequence_bodies_do_not_mutate_parsed_collapse() {
    let mut v = viewer("1 {}");
    let first = v.document_roots()[0];
    v.perform_action(Action::MoveDown(1));
    assert!(!v.focused_document_header());

    v.perform_action(Action::ToggleCollapsed);
    v.perform_action(Action::MoveLeft);
    assert!(!v.flatjson[first].is_collapsed());
    assert!(!v.is_document_collapsed(first));
    assert_eq!(v.focused_node, first);
}

#[test]
fn collapsed_header_jump_selects_without_revealing_document() {
    let mut v = viewer("1 2");
    let roots = v.document_roots().to_vec();
    let first_header = v.document_header_absolute(roots[0]);
    v.perform_action(Action::ToggleCollapsed);
    assert!(v.is_document_collapsed(roots[0]));

    // This is the action emitted for a click on header text or its
    // collapsed preview. Search uses FocusNode and still reveals.
    v.perform_action(Action::JumpTo {
        line: first_header,
        make_visible: false,
    });
    assert!(v.is_document_collapsed(roots[0]));
    assert_eq!(v.focused_node, roots[0]);
    assert_eq!(v.focused_line_index(), 0);

    v.perform_action(Action::JumpTo {
        line: first_header,
        make_visible: true,
    });
    assert!(v.is_document_collapsed(roots[0]));
    assert_eq!(v.focused_line_index(), 0);
}

#[test]
fn header_right_reopens_a_collapsed_root_array_body() {
    let mut v = viewer("[1,2] {}");
    let first = v.document_roots()[0];

    v.perform_action(Action::MoveRight);
    v.perform_action(Action::FocusParent);
    v.perform_action(Action::MoveDown(1));
    v.perform_action(Action::ToggleCollapsed);
    assert!(v.flatjson[first].is_collapsed());

    v.perform_action(Action::FocusTop);
    assert!(v.focused_document_header());
    v.perform_action(Action::MoveRight);
    assert!(!v.flatjson[first].is_collapsed());
    assert_eq!(path(&v), "[0]");
}

#[test]
fn structural_root_focus_stays_on_document_header_after_body_collapse() {
    let mut v = viewer("[1,2] {}");
    let root = v.document_roots()[0];

    v.perform_action(Action::MoveRight);
    v.perform_action(Action::FocusParent);
    v.perform_action(Action::MoveDown(1));
    v.perform_action(Action::ToggleCollapsed);
    assert!(v.flatjson[root].is_collapsed());
    assert!(!v.is_document_collapsed(root));

    // Clicking the generated row's arrow must target the document row,
    // while retaining the parsed body collapse state.
    v.perform_action(Action::ClickArrow(1));
    assert!(v.is_document_collapsed(root));
    assert!(v.flatjson[root].is_collapsed());
    assert!(v.focused_document_header());
}

#[test]
fn sequence_keyed_root_keeps_body_and_document_collapse_independent() {
    let mut v = viewer(r#"{"alice":{"score":1},"bob":{"score":2}} 7"#);
    let table = v.document_roots()[0];
    assert!(v.focused_document_header());
    assert_eq!(v.focused_table(), None);
    v.perform_action(Action::MoveDown(1));
    assert_eq!(v.focused_node, table);
    assert!(!v.focused_document_header());
    assert!(v.line_is_collapsible(v.focused_line_index()));
    assert_eq!(v.focused_table(), Some(table));
    assert_eq!(v.toggle_table_alignment(), Some(table));
    v.perform_action(Action::ToggleCollapsed);
    assert!(v.flatjson[table].is_collapsed());
    assert!(!v.is_document_collapsed(table));
    v.perform_action(Action::FocusTop);
    assert_eq!(v.focused_table(), None);
    v.perform_action(Action::ToggleCollapsed);
    assert!(v.is_document_collapsed(table));
    assert!(v.flatjson[table].is_collapsed());
    v.perform_action(Action::DeepExpandNodeAndSiblings);
    assert!(!v.is_document_collapsed(table));
    assert!(!v.flatjson[table].is_collapsed());
}

#[test]
fn deep_document_expand_restores_root_body_state() {
    let mut v = viewer("[1,2] {} 3");
    let roots = v.document_roots().to_vec();
    let first = roots[0];

    // Collapse the array body independently, then use a deep operation
    // from the structural header to restore every root's body state.
    v.perform_action(Action::MoveRight);
    v.perform_action(Action::FocusParent);
    v.perform_action(Action::MoveDown(1));
    assert_eq!(v.focused_node, first);
    assert!(!v.focused_document_header());
    v.perform_action(Action::ToggleCollapsed);
    assert!(v.flatjson[first].is_collapsed());
    v.perform_action(Action::FocusTop);
    v.perform_action(Action::DeepExpandNodeAndSiblings);
    assert!(roots.iter().all(|&root| !v.is_document_collapsed(root)));
    assert!(roots.iter().all(|&root| !v.flatjson[root].is_collapsed()));
}

#[test]
fn shallow_document_expand_does_not_leave_object_root_body_collapsed() {
    let mut v = viewer(r#"{"a":1,"nested":{"b":2}} 7"#);
    let roots = v.document_roots().to_vec();
    let root = roots[0];
    let first = v.flatjson[root].first_child().unwrap();
    let nested = v.flatjson[first].next_sibling.unwrap();

    v.perform_action(Action::DeepCollapseNodeAndSiblings);
    assert!(v.is_document_collapsed(root));
    assert!(!v.flatjson[root].is_collapsed());
    assert!(v.flatjson[nested].is_collapsed());

    v.perform_action(Action::ExpandNodeAndSiblings);
    assert!(!v.is_document_collapsed(root));
    v.perform_action(Action::MoveRight);
    assert_eq!(path(&v), ".a");
    assert!(v.flatjson[nested].is_collapsed());
}
