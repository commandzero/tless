use super::pty::session_with_width;
use super::screen::rendered_rows;

#[test]
fn wrapping_is_opt_in_and_numeric_prefix_toggles_once() {
    let input = r#"{"long":"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdefghijklmnopqrstuvwxyz","next":42}"#;

    let off = rendered_rows(&session_with_width(input, "q", None, 35), 35, 24);
    let unwrapped = off.iter().find(|row| row.contains("long:")).unwrap();
    assert!(unwrapped.contains('…'), "unwrapped rows: {:?}", off);

    for commands in ["\x0cq", "3\x0cq"] {
        let output = session_with_width(input, commands, None, 35);
        let rows = rendered_rows(&output, 35, 24);
        let first = rows.iter().position(|row| row.contains("long:")).unwrap();
        let next = rows.iter().position(|row| row.contains("next:")).unwrap();
        assert!(
            next > first + 1,
            "wrapped rows for {:?}: {:?}",
            commands,
            rows
        );
        assert!(
            rows.iter().any(|row| row.contains("Line wrapping on")),
            "{:?}",
            rows
        );
    }

    let output = session_with_width(input, "\x0c\x0cq", None, 35);
    let rows = rendered_rows(&output, 35, 24);
    let unwrapped_again = rows.iter().find(|row| row.contains("long:")).unwrap();
    assert!(
        unwrapped_again.contains('…'),
        "rows after disabling: {:?}",
        rows
    );
    assert!(
        rows.iter().any(|row| row.contains("Line wrapping off")),
        "{:?}",
        rows
    );
}

#[test]
fn continuation_rows_keep_number_and_arrow_gutters_blank() {
    let input = r#"{"long":"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdefghijklmnopqrstuvwxyz","next":42}"#;
    let output = session_with_width(input, ":set number\n\x0cq", None, 35);
    let rows = rendered_rows(&output, 35, 24);
    let first = rows.iter().position(|row| row.contains("long:")).unwrap();
    let continuation = rows
        .iter()
        .skip(first + 1)
        .take_while(|row| !row.contains("next:"))
        .find(|row| !row.is_empty())
        .unwrap();
    let next = rows.iter().find(|row| row.contains("next:")).unwrap();

    assert!(
        rows[first].starts_with(" 1 "),
        "first row: {:?}",
        rows[first]
    );
    assert!(
        continuation.len() >= 5,
        "continuation row: {:?}",
        continuation
    );
    assert!(
        continuation[..3].chars().all(|cell| cell == ' '),
        "continuation gutter: {:?}",
        continuation
    );
    assert!(
        continuation.chars().nth(3) == Some(' '),
        "continuation arrow gutter: {:?}",
        continuation
    );
    assert!(next.starts_with(" 2 "), "next row: {:?}", next);
}

#[test]
fn logical_motion_skips_wrapped_continuations() {
    let input = r#"{"long":"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdefghijklmnopqrstuvwxyz","next":42}"#;

    let down = session_with_width(input, "l\x0cjq", None, 35);
    assert!(
        rendered_rows(&down, 35, 24)
            .iter()
            .any(|row| row.contains(".next")),
        "{}",
        down
    );

    let up = session_with_width(input, "l\x0cjkq", None, 35);
    assert!(
        rendered_rows(&up, 35, 24)
            .iter()
            .any(|row| row.contains(".long")),
        "{}",
        up
    );
}

#[test]
fn active_scalar_search_stays_visible_after_wrapping_and_resize() {
    let value = format!("{}NEEDLE", "a".repeat(120));
    let input = format!(r#"{{"long":"{}"}}"#, value);
    let output = session_with_width(&input, "/NEEDLE\n\x0c\x12pp q", None, 120);
    let rows = rendered_rows(&output, 16, 8);

    assert!(
        rows[..6].iter().any(|row| row.contains("NEEDLE")),
        "{:?}",
        rows
    );
    assert!(rows.iter().any(|row| row.contains(".long")), "{:?}", rows);
    assert!(output.contains(&format!("\"{}\"\r\n", value)), "{}", output);
}

#[test]
fn active_shared_header_search_stays_visible_after_wrapping_and_resize() {
    let key = format!("field{}NEEDLE", "x".repeat(120));
    let input = format!(r#"{{"rows":[{{"{}":"A"}},{{"{}":"B"}}]}}"#, key, key);
    let output = session_with_width(&input, "/NEEDLE\n\x0c\x12pp q", None, 120);
    let rows = rendered_rows(&output, 16, 8);

    assert!(
        rows[..6].iter().any(|row| row.contains("NEEDLE")),
        "{:?}",
        rows
    );
    assert!(output.contains("\"A\"\r\n"), "{}", output);
}

#[test]
fn horizontal_scroll_is_inert_while_wrapped_and_returns_after_toggle_off() {
    let input = r#""0123456789abcdefghijKLMNOPQRSTUVWXYZ0123456789abcdefghij""#;
    let wrapped = rendered_rows(&session_with_width(input, "\x0c.q", None, 35), 35, 24);
    assert!(
        !wrapped[..22].iter().any(|row| row.contains('…')),
        "wrapped rows: {:?}",
        wrapped
    );

    let unwrapped = rendered_rows(&session_with_width(input, "\x0c.\x0c.q", None, 35), 35, 24);
    assert!(
        unwrapped[..22]
            .iter()
            .any(|row| row.contains("…abcdefghij")),
        "unwrapped rows: {:?}",
        unwrapped
    );
}

#[test]
fn collapsed_preview_stays_on_one_row_when_wrapping_is_enabled() {
    let input =
        r#"{"container":{"alpha":"abcdefghijklmnopqrstuvwxyz","beta":"0123456789"},"after":3}"#;
    let rows = rendered_rows(&session_with_width(input, "l\x0c q", None, 35), 35, 24);
    let container_rows = rows[..22]
        .iter()
        .filter(|row| row.contains("container"))
        .count();
    assert_eq!(container_rows, 1, "collapsed rows: {:?}", rows);
    assert!(rows.iter().any(|row| row.contains("after:")), "{:?}", rows);
}

#[test]
fn physical_scrolling_reaches_a_tall_wrapped_value() {
    let input = format!(
        r#"{{"long":"{}TAIL"}}"#,
        "0123456789abcdefghijklmnopqrstuvwxyz".repeat(8)
    );
    // ^R resizes the isolated PTY to 16x8. The command sequence exercises
    // both single-row scroll commands and full-page scrolling while the
    // same logical value remains focused.
    let wheel_down = "\x1b[<65;1;4M";
    let wheel_up = "\x1b[<64;1;4M";
    let commands = format!("l\x0c\x12{wheel_down}{wheel_down}{wheel_up}\x059\x06q");
    let output = session_with_width(&input, &commands, None, 35);
    let rows = rendered_rows(&output, 16, 8);
    let tail_visible = rows
        .windows(2)
        .any(|pair| pair[0].ends_with("TAI") && pair[1].trim_start() == "L");
    assert!(tail_visible, "{:?}", rows);
    assert!(rows.iter().any(|row| row.contains(".long")), "{:?}", rows);
}

#[test]
fn mouse_continuation_cell_keeps_the_value_copy_target() {
    let value = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let input = format!(r#"{{"first":1,"long":"{}"}}"#, value);
    // Row four is a continuation after the 16-column resize. Column eight
    // is in its document text, beyond the reserved gutter.
    let click = "\x1b[<0;8;4M";
    let output = session_with_width(&input, &format!("l\x0c\x12{click}pp q"), None, 35);
    assert!(output.contains(&format!("\"{}\"\r\n", value)), "{}", output);
}

#[test]
fn deep_mouse_continuation_stays_visible_after_redraw() {
    let value = format!("{}CLICKED", "abcdefghijklmnopqrstuvwxyz".repeat(8));
    let input = format!(r#"{{"first":1,"long":"{}"}}"#, value);
    // Scroll to the end, then click the bottom viewer row on a continuation.
    let wheel_down = "\x1b[<65;1;4M";
    let click = "\x1b[<0;8;6M";
    let output = session_with_width(
        &input,
        &format!("l\x0c\x12{}{click}q", wheel_down.repeat(12)),
        None,
        35,
    );
    let rows = rendered_rows(&output, 16, 8);
    assert!(
        rows.windows(2)
            .any(|pair| pair[0].contains("CLICKE") && pair[1].contains('D')),
        "{:?}",
        rows
    );
}
