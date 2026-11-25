use super::*;

#[test]
fn table_cells_keep_columns_and_parent_structure() {
    let mut v = viewer(r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"\u006eame":"Lin"}]}"#);
    act(
        &mut v,
        &[
            Action::MoveRight,
            Action::MoveRight,
            Action::MoveRight,
            Action::FocusNextSibling(1),
        ],
    );
    assert_eq!(path(&v), ".users[0].name");
    v.perform_action(Action::MoveDown(1));
    assert_eq!(
        &v.flatjson.1[v.flatjson[v.focused_node].range.clone()],
        "\"Lin\""
    );
    let line = v.absolute_anchor_line;
    v.perform_action(Action::FocusParent);
    assert_eq!(path(&v), ".users[1]");
    assert_eq!(line, v.absolute_anchor_line);
    v.perform_action(Action::FocusParent);
    assert_eq!(path(&v), ".users");
}

#[test]
fn nested_table_leaves_retain_columns_and_groups_remain_selectable() {
    let mut v = viewer(
        r#"{"items":[{"id":1,"nested":{"left":"Ada","right":2}},{"id":3,"nested":{"left":"Lin","right":4}}]}"#,
    );
    let table = v.flatjson[0].first_child().unwrap();
    let first = v.flatjson[table].first_child().unwrap();
    let id = v.flatjson[first].first_child().unwrap();
    let group = v.flatjson[id].next_sibling.unwrap();
    let left = v.flatjson[group].first_child().unwrap();
    let right = v.flatjson[left].next_sibling.unwrap();
    let second = v.flatjson[first].next_sibling.unwrap();
    let second_id = v.flatjson[second].first_child().unwrap();
    let second_group = v.flatjson[second_id].next_sibling.unwrap();
    let second_left = v.flatjson[second_group].first_child().unwrap();
    let second_right = v.flatjson[second_left].next_sibling.unwrap();

    for node in [table, first, group, left, right] {
        v.perform_action(Action::FocusNode { node, source: None });
        assert_eq!(v.focused_table(), Some(table), "{node}");
    }
    assert_eq!(v.toggle_table_alignment(), Some(table));
    v.perform_action(Action::FocusNode {
        node: right,
        source: None,
    });
    v.perform_action(Action::MoveDown(1));
    assert_eq!(v.focused_node, second_right);
    v.perform_action(Action::MoveUp(1));
    assert_eq!(v.focused_node, right);
    v.perform_action(Action::FocusParent);
    assert_eq!(v.focused_node, group);
    assert_eq!(
        v.absolute_anchor_line,
        v.layout.node(&v.flatjson, first).line
    );
    v.perform_action(Action::ToggleCollapsed);
    assert_eq!(v.focused_node, group);
    assert!(!v.flatjson[group].is_collapsed());
    v.perform_action(Action::MoveRight);
    assert_eq!(v.focused_node, left);
    v.perform_action(Action::MoveDown(1));
    assert_eq!(v.focused_node, second_left);
    v.perform_action(Action::FocusParentOrPreviousSibling);
    assert_eq!(v.focused_node, second_group);
    v.perform_action(Action::FocusParent);
    assert_eq!(v.focused_node, second);
    v.perform_action(Action::FocusParent);
    assert_eq!(v.focused_node, table);
}

#[test]
fn keyed_table_header_and_nested_entries_keep_identity_across_filters() {
    let mut v = viewer(
        r#"{"scores":{"alice":{"score":1,"meta":{"rank":2}},"bob":{"score":3,"meta":{"rank":4}}},"tail":0}"#,
    );
    let table = v.flatjson[0].first_child().unwrap();
    let alice = v.flatjson[table].first_child().unwrap();
    let bob = v.flatjson[alice].next_sibling.unwrap();
    let score = v.flatjson[alice].first_child().unwrap();
    let meta = v.flatjson[score].next_sibling.unwrap();
    let rank = v.flatjson[meta].first_child().unwrap();
    let bob_score = v.flatjson[bob].first_child().unwrap();
    let bob_meta = v.flatjson[bob_score].next_sibling.unwrap();
    let bob_rank = v.flatjson[bob_meta].first_child().unwrap();
    for node in [table, alice, meta, rank, bob, bob_meta, bob_rank] {
        v.perform_action(Action::FocusNode { node, source: None });
        assert_eq!(v.focused_table(), Some(table));
    }
    v.perform_action(Action::FocusNode {
        node: table,
        source: None,
    });
    let header = v.focused_line_index();
    assert!(v.line_is_collapsible(header));
    assert_eq!(v.toggle_table_alignment(), Some(table));
    v.perform_action(Action::FocusNode {
        node: rank,
        source: None,
    });
    v.perform_action(Action::MoveDown(1));
    assert_eq!(v.focused_node, bob_rank);
    v.perform_action(Action::FocusParent);
    assert_eq!(v.focused_node, bob_meta);
    v.perform_action(Action::FocusNode {
        node: table,
        source: None,
    });
    v.perform_action(Action::ToggleCollapsed);
    assert!(v.flatjson[table].is_collapsed());
    assert_eq!(v.aligned_table_for_line(v.focused_line_index()), None);
    v.perform_action(Action::ToggleCollapsed);
    assert_eq!(
        v.aligned_table_for_line(v.focused_line_index()),
        Some(table)
    );
    v.set_roots(vec![rank]);
    assert_eq!(v.focused_table(), None);
    v.set_roots(vec![bob]);
    assert_eq!(v.focused_table(), None);
    v.set_roots(vec![0]);
    v.perform_action(Action::FocusNode {
        node: bob_rank,
        source: None,
    });
    assert_eq!(v.focused_table(), Some(table));
    assert!(v.table_alignment_enabled(table));
}

#[test]
fn nested_table_key_search_anchors_header_without_losing_leaf_focus() {
    let mut v =
        viewer(r#"{"items":[{"id":1,"meta":{"label":"Ada"}},{"id":2,"meta":{"label":"Lin"}}]}"#);
    let table = v.flatjson[0].first_child().unwrap();
    let first = v.flatjson[table].first_child().unwrap();
    let id = v.flatjson[first].first_child().unwrap();
    let group = v.flatjson[id].next_sibling.unwrap();
    let label = v.flatjson[group].first_child().unwrap();
    let key = v.flatjson.1.find("\"label\"").unwrap() + 1;
    v.perform_action(Action::FocusNode {
        node: label,
        source: Some(key),
    });
    assert_eq!(v.focused_node, label);
    assert_eq!(
        v.absolute_anchor_line,
        v.layout.node(&v.flatjson, table).body_line
    );
    let value = v.flatjson[label].range.start;
    v.perform_action(Action::FocusNode {
        node: label,
        source: Some(value),
    });
    assert_eq!(v.focused_node, label);
    assert_eq!(
        v.absolute_anchor_line,
        v.layout.node(&v.flatjson, first).line
    );
}
