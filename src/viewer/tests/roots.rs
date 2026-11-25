use super::*;

#[test]
fn selected_root_resets_projection_and_blocks_ancestor_navigation() {
    let mut v = viewer(r#"{"outer":{"kept":1},"tail":{"excluded":2}}"#);
    let outer = v.flatjson[0].first_child().unwrap();
    v.set_roots(vec![outer]);
    assert_eq!(v.active_roots(), &[outer]);
    assert!(!v.contains_node(0));
    assert!(v.contains_node(outer));
    assert_eq!(
        (0..v.visible.len())
            .map(|line| v.render_line(line).text)
            .collect::<Vec<_>>(),
        vec!["kept: 1"]
    );
    v.perform_action(Action::MoveRight);
    let kept = v.focused_node;
    v.perform_action(Action::FocusParent);
    assert_eq!(v.focused_node, outer);
    v.perform_action(Action::FocusParentOrPreviousSibling);
    assert_eq!(v.focused_node, outer);
    v.perform_action(Action::FocusNextSibling(1));
    assert_eq!(v.focused_node, outer);
    v.perform_action(Action::FocusNode {
        node: 0,
        source: None,
    });
    assert_eq!(v.focused_node, outer);
    assert!(v.contains_node(kept));
    v.set_roots(vec![0]);
    assert_eq!(v.active_roots(), &[0]);
    assert!(v.contains_node(0));
    assert!((0..v.visible.len()).any(|line| v.render_line(line).text.contains("tail")));
}

#[test]
fn selected_sequence_roots_keep_sibling_motion_without_original_ancestors() {
    let mut v = viewer(r#"{"hits":{"a":1},"outside":0} {"hits":{"b":2},"outside":9}"#);
    let roots = crate::path_filter::PathFilter::parse(".hits")
        .unwrap()
        .resolve(&v.flatjson)
        .unwrap();
    let (first, second) = (roots[0], roots[1]);
    v.set_roots(roots);
    assert_eq!(v.active_roots(), &[first, second]);
    assert_eq!(v.render_line(0).text, "--- (1 of 2)");
    v.perform_action(Action::MoveRight);
    assert_eq!(v.focused_node, v.flatjson[first].first_child().unwrap());
    v.perform_action(Action::FocusNextAtParentLevel);
    assert_eq!(v.focused_node, second);
    v.perform_action(Action::FocusPrevSibling(1));
    assert_eq!(v.focused_node, first);
    v.perform_action(Action::FocusNextSibling(1));
    assert_eq!(v.focused_node, second);
    v.perform_action(Action::FocusNextSibling(1));
    assert_eq!(v.focused_node, second);
    assert!(!v.contains_node(0));
}
