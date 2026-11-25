use super::*;

#[test]
fn next_sibling_stops_at_the_last_entry() {
    let mut v = viewer("[1,2]");
    act(&mut v, &[Action::MoveRight, Action::MoveRight]);
    v.perform_action(Action::FocusNextSibling(1));
    assert_eq!(path(&v), "[1]");
    let last = v.focused_node;
    v.perform_action(Action::FocusNextSibling(1));
    assert_eq!(v.focused_node, last);
    v.perform_action(Action::FocusNextSibling(10));
    assert_eq!(v.focused_node, last);
}

#[test]
fn parent_level_motions_select_parent_and_next_parent_entry() {
    let mut v = viewer(r#"{"a":{"value":0},"b":{"x":1},"c":{"value":2}}"#);
    // Preserve the destination's collapse state and select its header.
    v.perform_action(Action::MoveRight);
    v.perform_action(Action::MoveLeft);
    let a = v.focused_node;
    act(&mut v, &[Action::FocusNextSibling(1), Action::MoveRight]);
    assert_eq!(path(&v), ".b.x");
    let x = v.focused_node;
    v.perform_action(Action::FocusParentOrPreviousSibling);
    assert_eq!(path(&v), ".b");
    assert!(v.flatjson[v.focused_node].is_expanded());
    assert!(v.flatjson[a].is_collapsed());
    v.perform_action(Action::FocusNode {
        node: x,
        source: None,
    });
    v.perform_action(Action::FocusNextAtParentLevel);
    assert_eq!(path(&v), ".c");
    assert_eq!(
        v.absolute_anchor_line,
        v.layout.node(&v.flatjson, v.focused_node).line
    );
}

#[test]
fn parent_level_motions_prefer_parent_then_fall_back_to_siblings() {
    let mut v = viewer(r#"{"a":{"x":1,"y":2},"b":{"x":3,"y":4}}"#);
    act(&mut v, &[Action::MoveRight, Action::MoveRight]);
    v.perform_action(Action::FocusNextAtParentLevel);
    assert_eq!(
        path(&v),
        ".b",
        "parent-level entry takes priority over .a.y"
    );
    v.perform_action(Action::MoveRight);
    v.perform_action(Action::FocusNextAtParentLevel);
    assert_eq!(
        path(&v),
        ".b.y",
        "use the next sibling when the parent has no next sibling"
    );
    v.perform_action(Action::FocusParentOrPreviousSibling);
    assert_eq!(path(&v), ".b", "parent takes priority over .b.x");
    v.perform_action(Action::FocusNextAtParentLevel);
    assert_eq!(path(&v), ".b", "no destination leaves focus unchanged");
}

#[test]
fn parent_level_motions_fall_back_between_document_roots() {
    let mut v = viewer("1 2 3");
    let first = v.focused_node;
    v.perform_action(Action::FocusNextAtParentLevel);
    let second = v.focused_node;
    assert_ne!(second, first);
    assert_eq!(&v.flatjson.1[v.flatjson[second].range.clone()], "2");
    v.perform_action(Action::FocusParent);
    assert_eq!(v.focused_node, second, "H remains a strict parent motion");
    v.perform_action(Action::FocusParentOrPreviousSibling);
    assert_eq!(v.focused_node, first);
    v.perform_action(Action::FocusParentOrPreviousSibling);
    assert_eq!(v.focused_node, first);
}

#[test]
fn parent_level_motions_keep_focus_at_document_boundaries() {
    let mut v = viewer(r#"{"only":{"x":1}}"#);
    act(
        &mut v,
        &[
            Action::FocusParentOrPreviousSibling,
            Action::FocusNextAtParentLevel,
        ],
    );
    assert_eq!(v.focused_node, 0);
    act(&mut v, &[Action::MoveRight, Action::MoveRight]);
    assert_eq!(path(&v), ".only.x");
    v.perform_action(Action::FocusNextAtParentLevel);
    assert_eq!(path(&v), ".only.x");
    v.perform_action(Action::FocusParentOrPreviousSibling);
    assert_eq!(path(&v), ".only");
    v.perform_action(Action::FocusParentOrPreviousSibling);
    assert_eq!(v.focused_node, 0);
}

#[test]
fn parent_level_motions_use_table_row_identity() {
    let mut v = viewer(r#"[{"x":1},{"x":2},{"x":3}]"#);
    act(
        &mut v,
        &[
            Action::MoveRight,
            Action::FocusNextSibling(1),
            Action::MoveRight,
        ],
    );
    assert_eq!(path(&v), "[1].x");
    let cell = v.focused_node;
    v.perform_action(Action::FocusParentOrPreviousSibling);
    assert_eq!(path(&v), "[1]");
    v.perform_action(Action::FocusNode {
        node: cell,
        source: None,
    });
    v.perform_action(Action::FocusNextAtParentLevel);
    assert_eq!(path(&v), "[2]");
}

#[test]
fn collapse_preserves_child_state_and_implicit_root() {
    let mut v = viewer(r#"{"box":{"inner":{"value":1}},"last":2}"#);
    v.perform_action(Action::ToggleCollapsed);
    assert!(!v.flatjson[0].is_collapsed());
    act(
        &mut v,
        &[
            Action::MoveRight,
            Action::MoveRight,
            Action::ToggleCollapsed,
        ],
    );
    let child = v.focused_node;
    act(&mut v, &[Action::FocusParent, Action::ToggleCollapsed]);
    assert_eq!(v.visible.len(), 2);
    v.perform_action(Action::MoveRight);
    assert!(v.flatjson[child].is_collapsed());
    v.perform_action(Action::MoveRight);
    assert_eq!(v.focused_node, child);
    v.perform_action(Action::MoveRight);
    assert!(!v.flatjson[child].is_collapsed());
    v.perform_action(Action::MoveRight);
    assert_eq!(path(&v), ".box.inner.value");
}

#[test]
fn absolute_jumps_and_search_reveal_distinguish_nodes_and_lines() {
    let mut v = viewer(r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Lin"}]}"#);
    act(&mut v, &[Action::MoveRight, Action::ToggleCollapsed]);
    v.perform_action(Action::JumpTo {
        line: 2,
        make_visible: false,
    });
    assert_eq!(path(&v), ".users");
    v.perform_action(Action::JumpTo {
        line: 2,
        make_visible: true,
    });
    assert_eq!(path(&v), ".users[1]");
    act(&mut v, &[Action::MoveRight, Action::FocusNextSibling(1)]);
    let node = v.focused_node;
    let key = v.flatjson[node].key_range.as_ref().unwrap().start;
    act(&mut v, &[Action::FocusParent, Action::ToggleCollapsed]);
    v.perform_action(Action::FocusNode {
        node,
        source: Some(key),
    });
    assert_eq!(v.focused_node, node);
    assert_eq!(
        v.absolute_anchor_line, 0,
        "shared key header is the display anchor"
    );
    assert_eq!(path(&v), ".users[1].name");
}

#[test]
fn jump_to_hidden_root_array_content_selects_the_body_control() {
    let mut v = viewer("[1,2] {}");
    let root = v.document_roots()[0];
    v.perform_action(Action::MoveRight);
    v.perform_action(Action::FocusParent);
    v.perform_action(Action::MoveDown(1));
    v.perform_action(Action::ToggleCollapsed);
    assert!(v.flatjson[root].is_collapsed());

    let child_line = v
        .layout
        .node(&v.flatjson, v.flatjson[root].first_child().as_usize())
        .line;
    v.perform_action(Action::JumpTo {
        line: child_line,
        make_visible: false,
    });
    assert_eq!(v.focused_node, root);
    assert!(!v.focused_document_header());
    assert_eq!(
        v.absolute_anchor_line,
        v.layout.node(&v.flatjson, root).body_line
    );
}

#[test]
fn separator_jumps_skip_annotations_and_empty_roots_remain_selectable() {
    let mut v = JsonViewer::new(parse_top_level_yaml("---\n{}\n---\n42\n".into()).unwrap());
    let first = v.focused_node;
    v.perform_action(Action::MoveDown(1));
    assert_eq!(
        first, v.focused_node,
        "the first root body keeps root identity"
    );
    v.perform_action(Action::MoveDown(1));
    assert_ne!(first, v.focused_node);
    assert!(v.is_document_header(v.focused_line_index()));
    v.perform_action(Action::MoveUp(1));
    assert_eq!(v.focused_node, first);
    v.perform_action(Action::JumpTo {
        line: 2,
        make_visible: false,
    });
    assert_ne!(v.focused_node, first);
    let mut empty = viewer("{}");
    act(
        &mut empty,
        &[
            Action::MoveRight,
            Action::ToggleCollapsed,
            Action::FocusBottom,
        ],
    );
    assert_eq!(empty.focused_node, 0);
    assert_eq!(empty.visible.len(), 1);
    assert!(empty.render_line(0).text.is_empty());
}

#[test]
fn mouse_hits_cells_and_resize_preserves_focus() {
    let mut v = viewer(r#"["界","é",3]"#);
    let child = v.flatjson[0].first_child().unwrap();
    let second = v.flatjson[child].next_sibling.unwrap();
    let line = v.render_line(0);
    let span = line.spans.iter().find(|span| span.node == second).unwrap();
    let col = UnicodeWidthStr::width(&line.text[..span.range.start]);
    let (node, source) = crate::lineprinter::hit_test(&line, col);
    v.perform_action(Action::FocusNode { node, source });
    assert_eq!(v.focused_node, second);
    v.perform_action(Action::ResizeViewerDimensions(TTYDimensions {
        width: 2,
        height: 0,
    }));
    assert_eq!(v.focused_node, second);
    assert_eq!(v.absolute_anchor_line, 2);
    v.perform_action(Action::FocusTop);
    v.perform_action(Action::ClickArrow(1));
    assert_eq!(v.focused_node, 0);
    assert!(v.flatjson[0].is_collapsed());
}

#[test]
fn counted_motions_scrolling_and_boundaries() {
    let input = format!(
        "{{{}}}",
        (0..30)
            .map(|i| format!("\"k{i}\":{i}"))
            .collect::<Vec<_>>()
            .join(",")
    );
    let mut v = viewer(&input);
    v.perform_action(Action::ResizeViewerDimensions(TTYDimensions {
        width: 20,
        height: 8,
    }));
    v.perform_action(Action::MoveDown(12));
    assert_eq!(v.absolute_anchor_line, 12);
    assert!(v.index_of_focused_node_on_screen() < 8);
    v.perform_action(Action::MoveFocusedLineToCenter);
    assert_eq!(v.index_of_focused_node_on_screen(), 4);
    v.perform_action(Action::PageDown(1));
    assert!(v.top_visible_line >= 8);
    v.perform_action(Action::JumpDown(Some(3)));
    let before = v.absolute_anchor_line;
    v.perform_action(Action::JumpUp(None));
    assert_eq!(v.absolute_anchor_line, before - 3);
    v.perform_action(Action::MoveDown(usize::MAX));
    assert_eq!(v.absolute_anchor_line, 29);
    v.perform_action(Action::MoveUp(usize::MAX));
    assert_eq!(v.absolute_anchor_line, 0);
    v.perform_action(Action::PageDown(usize::MAX));
    assert!(v.top_visible_line < v.visible.len());
}

#[test]
fn ordinary_fields_and_table_cells_enter_inline_array_owner_vertically() {
    let mut v = viewer(r#"{"x":1,"tags":[2,3]}"#);
    act(&mut v, &[Action::MoveRight, Action::MoveDown(1)]);
    assert_eq!(path(&v), ".tags");
    let mut v = viewer(r#"{"users":[{"a":1}],"tags":[2,3]}"#);
    act(
        &mut v,
        &[
            Action::MoveRight,
            Action::MoveRight,
            Action::MoveRight,
            Action::MoveDown(1),
        ],
    );
    assert_eq!(path(&v), ".tags");
}

#[test]
fn retained_depth_and_first_last_sibling_motions_follow_logical_boundaries() {
    let mut v = viewer(r#"{"a":1,"obj":{"b":2,"c":{"d":3},"e":4},"z":5}"#);
    act(
        &mut v,
        &[Action::MoveRight, Action::MoveDownUntilDepthChange],
    );
    assert_eq!(path(&v), ".obj");
    v.perform_action(Action::MoveDownUntilDepthChange);
    assert_eq!(path(&v), ".obj.b");
    v.perform_action(Action::MoveDownUntilDepthChange);
    assert_eq!(path(&v), ".obj.c");
    act(
        &mut v,
        &[Action::MoveRight, Action::MoveDownUntilDepthChange],
    );
    assert_eq!(path(&v), ".obj.e");
    v.perform_action(Action::MoveUpUntilDepthChange);
    assert_eq!(path(&v), ".obj.c.d");
    v.perform_action(Action::MoveUpUntilDepthChange);
    assert_eq!(path(&v), ".obj.c");
    v.perform_action(Action::FocusFirstSibling);
    assert_eq!(path(&v), ".obj.b");
    v.perform_action(Action::FocusLastSibling);
    assert_eq!(path(&v), ".obj.e");
    act(&mut v, &[Action::FocusParent, Action::FocusLastSibling]);
    assert_eq!(path(&v), ".z");
    v.perform_action(Action::FocusFirstSibling);
    assert_eq!(path(&v), ".a");
}

#[test]
fn deep_collapse_and_expand_preserve_siblings_and_recover_viewport() {
    let mut v = viewer(r#"{"a":{"b":{"x":1}},"c":{"d":{"y":2}},"last":3}"#);
    v.perform_action(Action::ResizeViewerDimensions(TTYDimensions {
        width: 40,
        height: 3,
    }));
    act(
        &mut v,
        &[
            Action::MoveRight,
            Action::FocusNextSibling(1),
            Action::MoveFocusedLineToTop,
            Action::DeepCollapseNodeAndSiblings,
        ],
    );
    assert_eq!(path(&v), ".c");
    assert_eq!(v.visible.len(), 3);
    assert!(v.flatjson[1].is_collapsed());
    assert!(v.flatjson[2].is_collapsed());
    assert!(v.focused_line_index() >= v.top_visible_line);
    v.perform_action(Action::ExpandNodeAndSiblings);
    assert!(!v.flatjson[1].is_collapsed());
    assert!(
        v.flatjson[2].is_collapsed(),
        "shallow expansion restores nested collapse"
    );
    v.perform_action(Action::DeepExpandNodeAndSiblings);
    assert!(
        v.flatjson
            .0
            .iter()
            .filter(|row| row.is_container())
            .all(|row| row.is_expanded())
    );
    assert_eq!(path(&v), ".c");
    assert!(v.index_of_focused_node_on_screen() < 3);
}

#[test]
fn scrolloff_and_half_window_eof_bounds_are_retained() {
    let input = format!(
        "{{{}}}",
        (0..30)
            .map(|i| format!("\"k{i}\":{i}"))
            .collect::<Vec<_>>()
            .join(",")
    );
    let mut v = viewer(&input);
    v.perform_action(Action::ResizeViewerDimensions(TTYDimensions {
        width: 40,
        height: 10,
    }));
    v.scrolloff_setting = 2;
    v.perform_action(Action::ScrollDown(5));
    assert_eq!(v.focused_line_index(), 7);
    v.perform_action(Action::ScrollUp(3));
    assert!(v.focused_line_index() <= v.top_visible_line + 7);
    v.perform_action(Action::FocusBottom);
    for _ in 0..5 {
        v.perform_action(Action::JumpDown(None));
    }
    assert_eq!(v.top_visible_line, 20);
    assert_eq!(v.focused_line_index(), 29);
    v.perform_action(Action::JumpUp(None));
    assert_eq!(v.top_visible_line, 15);
    assert_eq!(v.focused_line_index(), 24);
}
