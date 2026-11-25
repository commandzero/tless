use super::*;

fn wrapped_table_viewer() -> JsonViewer {
    let mut v = viewer(
        r#"[{"id":0,"long":"tail"},{"id":1,"long":"tail"},{"id":2,"long":"0123456789abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz"},{"id":3,"long":"tail"},{"id":4,"long":"tail"},{"id":5,"long":"tail"},{"id":6,"long":"tail"},{"id":7,"long":"tail"},{"id":8,"long":"tail"},{"id":9,"long":"tail"}]"#,
    );
    v.set_viewport(
        TTYDimensions {
            width: 20,
            height: 4,
        },
        true,
    );
    v.set_wrap_geometry(8, 0);
    v.toggle_wrapping();
    v.perform_action(Action::MoveRight);
    v.perform_action(Action::FocusNextSibling(2));
    v.perform_action(Action::MoveRight);
    v.perform_action(Action::FocusNextSibling(1));
    v
}

#[test]
fn wrapped_rows_scroll_physically_but_motion_stays_logical() {
    let mut v = viewer(
        r#"{"long":"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdefghijklmnopqrstuvwxyz","next":42}"#,
    );
    v.set_viewport(
        TTYDimensions {
            width: 20,
            height: 4,
        },
        true,
    );
    v.set_wrap_geometry(8, 0);
    v.toggle_wrapping();
    assert!(v.physical_rows.len() > v.visible.len());
    v.perform_action(Action::MoveRight);
    let long = v.focused_node;
    v.perform_action(Action::ScrollDown(1));
    assert_eq!(
        v.focused_node, long,
        "scrolling inside a tall value retains focus"
    );
    assert!(v.top_continuation > 0);
    v.perform_action(Action::MoveDown(1));
    assert_eq!(path(&v), ".next");
    v.perform_action(Action::MoveUp(1));
    assert_eq!(v.focused_node, long);
}

#[test]
fn wrapping_reflow_preserves_logical_focus_and_reveals_display_ranges() {
    let mut v = viewer(r#"{"long":"abcdefghijklmnopqrstuvwxyz0123456789","next":42}"#);
    v.set_viewport(
        TTYDimensions {
            width: 20,
            height: 4,
        },
        true,
    );
    v.set_wrap_geometry(8, 0);
    v.toggle_wrapping();
    v.perform_action(Action::MoveRight);
    let node = v.focused_node;
    let line = v.render_line(v.focused_line_index());
    let span = line
        .spans
        .iter()
        .find(|span| span.node == node && span.source.is_some())
        .unwrap();
    let display = span.matching_ranges(&span.source.clone().unwrap())[0].clone();
    v.reveal_byte_range(display.clone());
    assert_eq!(v.focused_node, node);
    assert!(
        v.physical_rows
            .iter()
            .enumerate()
            .any(|(index, row)| row.logical_line == v.focused_line_index()
                && row.bytes.start <= display.start
                && display.start < row.bytes.end
                && index < usize::from(v.dimensions.height).max(1))
    );
    v.set_wrap_geometry(4, 0);
    assert_eq!(v.focused_node, node);
}

#[test]
fn physical_scroll_keeps_tall_value_focused_at_document_end() {
    let input = format!(
        r#"{{"long":"{}TAIL"}}"#,
        "0123456789abcdefghijklmnopqrstuvwxyz".repeat(8)
    );
    let mut v = viewer(&input);
    v.set_viewport(
        TTYDimensions {
            width: 16,
            height: 6,
        },
        true,
    );
    v.set_wrap_geometry(11, 0);
    v.toggle_wrapping();
    v.perform_action(Action::MoveRight);
    let long = v.focused_node;
    v.perform_action(Action::ScrollDown(3));
    v.perform_action(Action::ScrollDown(3));
    v.perform_action(Action::ScrollUp(3));
    v.perform_action(Action::ScrollDown(1));
    v.perform_action(Action::PageDown(9));
    assert_eq!(v.focused_node, long);
    assert!(
        v.screen_row(0)
            .is_some_and(|row| row.bytes.contains(&row.bytes.end.saturating_sub(1)))
    );
}

#[test]
fn repositioning_uses_a_match_continuation_and_keeps_it_selected() {
    let mut v = viewer(&format!("\"{}\"", "abcdefghij".repeat(30)));
    v.set_viewport(
        TTYDimensions {
            width: 40,
            height: 6,
        },
        true,
    );
    v.scrolloff_setting = 0;
    v.set_wrap_geometry(10, 0);
    v.toggle_wrapping();
    v.reveal_byte_range(55..60);
    v.perform_action(Action::MoveFocusedLineToTop);
    assert_eq!(v.screen_row(0).unwrap().bytes.start, 50);
    v.perform_action(Action::MoveFocusedLineToCenter);
    assert_eq!(v.screen_row(0).unwrap().bytes.start, 20);
    v.perform_action(Action::MoveFocusedLineToBottom);
    assert_eq!(v.screen_row(0).unwrap().bytes.start, 0);
    assert_eq!(v.focused_node, 0);
}

#[test]
fn wrapping_projection_reuses_unchanged_geometry_and_reflows_on_collapse() {
    let mut v = viewer(r#"{"box":{"long":"abcdefghijklmnopqrstuvwxyz"},"tail":1}"#);
    v.set_wrap_geometry(8, 0);
    v.toggle_wrapping();
    let generation = v.physical_generation;
    let rows = v.physical_rows.clone();
    assert!(!v.set_wrap_geometry(8, 0));
    assert_eq!(v.physical_generation, generation);
    assert_eq!(v.physical_rows, rows);
    v.perform_action(Action::MoveRight);
    v.perform_action(Action::ToggleCollapsed);
    assert!(v.physical_generation != generation);
    assert_eq!(
        v.physical_rows
            .iter()
            .filter(|row| row.logical_line == 0)
            .count(),
        1
    );
    assert!(!v.is_wrapped_line(0));
    v.perform_action(Action::MoveRight);
    assert!(v.is_wrapped_line(0));
    assert!(v.set_wrap_geometry(6, 2));
    assert_eq!(path(&v), ".box");
}

#[test]
fn wrapping_reflow_clamps_to_the_last_viewport_origin() {
    let mut v = viewer(&format!(
        r#"{{"long":"{}","tail":42}}"#,
        "0123456789abcdefghijklmnopqrstuvwxyz".repeat(4)
    ));
    v.set_viewport(
        TTYDimensions {
            width: 20,
            height: 4,
        },
        true,
    );
    v.set_wrap_geometry(8, 0);
    v.toggle_wrapping();
    v.perform_action(Action::FocusBottom);

    v.set_wrap_geometry(200, 0);

    assert_eq!(
        v.top_cursor(),
        Cursor {
            logical: 0,
            continuation: 0
        }
    );
}

#[test]
fn wrapped_height_resize_clamps_the_existing_origin() {
    let mut v = viewer(&format!(
        r#"{{"long":"{}","tail":42}}"#,
        "0123456789abcdefghijklmnopqrstuvwxyz".repeat(4)
    ));
    v.set_viewport(
        TTYDimensions {
            width: 20,
            height: 3,
        },
        true,
    );
    v.set_wrap_geometry(8, 0);
    v.toggle_wrapping();
    v.perform_action(Action::FocusBottom);
    assert!(
        v.top_cursor()
            > Cursor {
                logical: 0,
                continuation: 0
            }
    );

    v.set_viewport(
        TTYDimensions {
            width: 20,
            height: 20,
        },
        true,
    );

    assert_eq!(
        v.top_cursor(),
        Cursor {
            logical: 0,
            continuation: 0
        }
    );
}

#[test]
fn projection_reflow_preserves_the_top_line_identity() {
    let mut v = viewer(
        r#"{"box":{"long":"0123456789abcdefghijklmnopqrstuvwxyz0123456789"},"tail1":1,"tail2":2,"tail3":3,"tail4":4,"tail5":5,"tail6":6}"#,
    );
    v.set_viewport(
        TTYDimensions {
            width: 20,
            height: 4,
        },
        true,
    );
    v.set_wrap_geometry(8, 0);
    v.toggle_wrapping();

    let box_node = v.flatjson[0].first_child().unwrap();
    let tail_node = v.flatjson[box_node].next_sibling.unwrap();
    let tail_absolute = v.layout.node(&v.flatjson, tail_node).line;
    let tail_line = v.visible.visible_index(tail_absolute).unwrap();
    v.set_top(Cursor {
        logical: tail_line,
        continuation: 0,
    });
    v.collapse(box_node, true);
    v.refresh_projection();

    assert_eq!(
        v.visible_line(v.top_visible_line).unwrap().absolute,
        tail_absolute
    );
}

#[test]
fn projection_reflow_preserves_a_sequence_body_at_the_top() {
    let mut v = viewer("1 2");
    let root = v.document_roots()[0];
    let body = v.layout.node(&v.flatjson, root).body_line;
    v.top_visible_line = v.visible.visible_index(body).unwrap();

    v.rebuild_layout();

    assert_eq!(v.visible_line(v.top_visible_line).unwrap().absolute, body);
}

#[test]
fn wrapped_mouse_focus_keeps_the_clicked_continuation_visible() {
    let mut v = viewer(&format!(
        r#"{{"long":"{}","tail":42}}"#,
        "0123456789abcdefghijklmnopqrstuvwxyz".repeat(4)
    ));
    v.set_viewport(
        TTYDimensions {
            width: 20,
            height: 4,
        },
        true,
    );
    v.set_wrap_geometry(8, 0);
    v.toggle_wrapping();
    let target = v
        .physical_rows
        .iter()
        .rev()
        .find(|row| !row.first)
        .unwrap()
        .clone();
    let line = v.rendered_line(target.logical_line);
    let (node, source) = crate::lineprinter::hit_test_wrapped(line, &target, 0);
    v.perform_action(Action::FocusNodeAt {
        node,
        source,
        display_range: (target.bytes.start, target.bytes.end),
    });

    assert!(
        v.physical_rows
            .iter()
            .any(|row| row.logical_line == target.logical_line && row.bytes == target.bytes)
    );
}

#[test]
fn wrapping_geometry_reflow_keeps_structure_focus_visible() {
    let mut v = viewer(&format!(
        r#"{{"long":"{}","container":{{"value":1}},"tail":2}}"#,
        "0123456789abcdefghijklmnopqrstuvwxyz".repeat(4)
    ));
    v.set_viewport(
        TTYDimensions {
            width: 20,
            height: 4,
        },
        true,
    );
    v.set_wrap_geometry(8, 0);
    v.toggle_wrapping();

    let long_node = v.flatjson[0].first_child().unwrap();
    let container_node = v.flatjson[long_node].next_sibling.unwrap();
    v.focus(container_node);
    v.set_top(Cursor {
        logical: 0,
        continuation: 0,
    });
    v.refresh_frame();

    v.set_wrap_geometry(4, 0);

    let height = usize::from(v.dimensions.height).max(1);
    assert!(
        v.physical_rows
            .iter()
            .any(|row| row.logical_line == v.focused_line_index())
    );
    assert!(v.focused_physical_row() < height);
}

#[test]
fn wrapped_half_page_jumps_retain_selected_cell_while_entry_is_visible() {
    let mut v = wrapped_table_viewer();
    let long = v.focused_node;
    assert_eq!(path(&v), "[2].long");

    v.perform_action(Action::JumpDown(Some(1)));
    assert_eq!(v.focused_node, long);
    assert!(
        v.top_cursor()
            > Cursor {
                logical: 0,
                continuation: 0
            }
    );

    v.perform_action(Action::JumpDown(Some(2)));
    assert_eq!(v.focused_node, long);

    v.perform_action(Action::JumpUp(Some(1)));
    assert_eq!(v.focused_node, long);

    v.perform_action(Action::JumpUp(Some(2)));
    assert_eq!(v.focused_node, long);
}

#[test]
fn wrapped_half_page_jumps_select_surrounding_lines_at_viewport_boundaries() {
    let mut v = wrapped_table_viewer();
    let long = v.focused_node;
    let long_line = v.focused_line_index();

    jump_until_focus_changes(&mut v, Action::JumpDown(Some(1)));
    assert_ne!(v.focused_node, long);
    assert!(v.focused_line_index() > long_line);

    for _ in 0..64 {
        let previous = v.top_cursor();
        v.perform_action(Action::JumpDown(Some(1)));
        if v.top_cursor() == previous {
            break;
        }
    }
    let lower_boundary_top = v.top_cursor();
    let lower_boundary_node = v.focused_node;
    v.perform_action(Action::JumpDown(Some(1)));
    assert_eq!(v.top_cursor(), lower_boundary_top);
    assert_eq!(v.focused_node, lower_boundary_node);

    let mut v = wrapped_table_viewer();
    let long = v.focused_node;
    let long_line = v.focused_line_index();
    v.perform_action(Action::JumpDown(Some(5)));
    assert_eq!(v.focused_node, long);

    jump_until_focus_changes(&mut v, Action::JumpUp(Some(1)));
    assert_ne!(v.focused_node, long);
    assert!(v.focused_line_index() < long_line);

    let upper_boundary_top = v.top_cursor();
    let upper_boundary_node = v.focused_node;
    v.perform_action(Action::JumpUp(Some(1)));
    assert_eq!(v.top_cursor(), upper_boundary_top);
    assert_eq!(v.focused_node, upper_boundary_node);
}

// Step until the selected entry no longer intersects the viewport, with a bound
// so a stuck focus produces an assertion failure.
fn jump_until_focus_changes(viewer: &mut JsonViewer, action: Action) {
    let focus = viewer.focused_node;
    for _ in 0..32 {
        if viewer.focused_node != focus {
            break;
        }
        viewer.perform_action(action);
    }
}
