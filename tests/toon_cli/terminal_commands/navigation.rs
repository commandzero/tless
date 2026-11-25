use super::pty::NEXT_SESSION;
use super::pty::session;
use super::pty::session_with_width;
use super::screen::rendered_rows;
use super::screen::strip_styles;

#[test]
fn brackets_move_to_entries_at_the_parent_level() {
    for (motion, expected) in [("[", "\"x\": 20"), ("]", "30\r\n")] {
        let output = session(
            r#"{"a":10,"b":{"x":20},"c":30}"#,
            &format!("ljl{}pp q", motion),
        );
        assert!(output.contains(expected), "{}", output);
    }
}

#[test]
fn bracket_fallbacks_select_siblings_when_parent_targets_are_missing() {
    for (input, commands, expected) in [
        (r#"{"a":{"x":1,"y":2},"b":3}"#, "ll]pp q", "3"),
        (r#"{"only":{"x":1,"y":2}}"#, "ll]]pp q", "2"),
        ("1 2 3", "]pp q", "2"),
        ("1 2 3", "][pp q", "1"),
    ] {
        let output = session(input, commands);
        assert!(output.contains(&format!("{}\r\n", expected)), "{}", output);
    }
}

#[test]
fn sequence_headers_show_positions_and_collapse_only_their_document() {
    let input = "{\"name\":\"Ada\",\"active\":true}\n[10,20]\n7\n{}\n[]";
    for commands in ["q", " q", "  q"] {
        let output = session(input, commands);
        assert!(!output.contains("Multiple document roots"));
        let rows = rendered_rows(&output, 120, 24);
        let headers: Vec<_> = rows.iter().filter(|row| row.contains("--- (")).collect();
        assert_eq!(headers.len(), 5, "{:?}", rows);
        for (index, header) in headers.iter().enumerate() {
            assert!(header.contains(&format!("({} of 5)", index + 1)));
        }
        if commands == " q" {
            assert!(headers[0].contains("▸ --- (1 of 5) name: Ada; active: true"));
            assert!(
                !rows
                    .iter()
                    .any(|row| row.trim_end().ends_with("active: true") && !row.contains("---"))
            );
        } else {
            assert!(headers[0].ends_with("▾ --- (1 of 5)"), "{:?}", rows);
            let field = rows.iter().find(|row| row.contains("name: Ada")).unwrap();
            assert_eq!(
                headers[0][..headers[0].find("---").unwrap()]
                    .chars()
                    .count(),
                field[..field.find("name:").unwrap()].chars().count()
            );
        }
        assert!(headers[1].ends_with("▾ --- (2 of 5)"));
        assert!(rows.iter().any(|row| row.contains("[2]: 10,20")));
        assert_eq!(
            rows.iter()
                .filter(|row| row.contains('▾') || row.contains('▸'))
                .count(),
            6,
            "Only the five document headers and nonempty array body have arrows: {:?}",
            rows
        );
    }
}

#[test]
fn sequence_navigation_search_and_mouse_keep_value_targets() {
    let input = "{\"name\":\"Ada\",\"details\":{\"needle\":\"MATCH\"}}\n[10,20]\n7\n{}\n[]";
    for (commands, expected) in [
        ("l]Jpp q", "7\r\n"),
        (
            " pp q",
            "{\n  \"name\": \"Ada\",\n  \"details\": {\n    \"needle\": \"MATCH\"\n  }\n}",
        ),
        (" /MATCH\npp q", "\"MATCH\"\r\n"),
        ("4Gpp q", "\"MATCH\"\r\n"),
    ] {
        let output = session(input, commands);
        assert!(
            output
                .replace("\r\n", "\n")
                .contains(&expected.replace("\r\n", "\n")),
            "{}",
            output
        );
    }
    // Disable numbers so the arrow is at column one. Collapse the first
    // document by mouse, then select the scalar document's header by click.
    let commands = ":set nonumber\n\x1b[<0;1;1M\x1b[<0;7;4Mpp q";
    let output = session(input, commands);
    assert!(strip_styles(&output).contains("▸ --- (1 of 5)"));
    assert!(output.contains("7\r\n"), "{}", output);
}

#[test]
fn sequence_array_body_retains_inline_array_controls() {
    let output = session("[10,20] 7", "jlpp q");
    assert!(
        output.replace("\r\n", "\n").contains("[\n  10,\n  20\n]"),
        "{}",
        output
    );
    let rows = rendered_rows(&session("[10,20] 7", "j q"), 120, 24);
    assert!(rows.iter().any(|row| row.ends_with("▾ --- (1 of 2)")));
    assert!(rows.iter().any(|row| row.contains("  - 10")), "{:?}", rows);
}

#[test]
fn sequence_search_ignores_positions_and_preserves_duplicate_identity() {
    let input = "{\"status\":\"queued\",\"status\":\"done\"} 7";
    let output = session(input, " /done\npp q");
    assert!(output.contains("\"done\"\r\n"));
    assert!(strip_styles(&output).contains("occurrence 2 of 2"));
    let output = strip_styles(&session(input, " /1 of 2\nq"));
    assert!(output.contains("Pattern not found: 1 of 2"), "{}", output);
}

#[test]
fn sequence_document_collapse_preserves_whole_document_json_write() {
    let target = std::env::temp_dir().join(format!(
        "tless-sequence-export-{}-{}.json",
        std::process::id(),
        NEXT_SESSION.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let output = session(
        "{\"a\":1} [2,3] 7 {} []",
        &format!("c:write-json {}\nq", target.display()),
    );
    assert!(output.contains("written"), "{}", output);
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        "{\n  \"a\": 1\n}\n[\n  2,\n  3\n]\n7\n{}\n[]\n"
    );
    std::fs::remove_file(target).unwrap();
}

#[test]
fn sequence_bulk_collapse_from_body_and_header_click_keep_document_state() {
    let output = session("1 2 3", "jcq");
    let rows = rendered_rows(&output, 120, 24);
    for index in 1..=3 {
        assert!(
            rows.iter()
                .any(|row| row.contains(&format!("▸ --- ({} of 3) {}", index, index))),
            "{:?}",
            rows
        );
    }
    let output = session("{\"name\":\"Ada\"} 7", ":set nonumber\n \x1b[<0;7;1Mq");
    let rows = rendered_rows(&output, 120, 24);
    assert!(rows[0].contains("▸ --- (1 of 2) name: Ada"), "{:?}", rows);
    assert!(rows[1].contains("▾ --- (2 of 2)"), "{:?}", rows);
}

#[test]
fn right_expands_inline_arrays_into_navigable_lines() {
    let output = session(r#"["alpha","beta"]"#, "ljpp q");
    let clean = strip_styles(&output);
    assert!(clean.contains("[2]: alpha,beta"), "{}", clean);
    assert!(clean.contains("  - alpha"), "{}", clean);
    assert!(clean.contains("  - beta"), "{}", clean);
    assert!(output.contains("\"alpha\"\r\n"));
}

#[test]
fn terminal_resize_reflows_arrays_before_another_keypress() {
    // The helper resizes on ^R without sending that byte to the app.
    // If resize waits for input, q exits before a multiline redraw occurs.
    let output = strip_styles(&session(r#"["alpha","beta"]"#, "\x12q"));
    assert!(output.contains("[2]: alpha,beta"), "{}", output);
    assert!(output.contains("  - alpha"), "{}", output);
    assert!(output.contains("  - beta"), "{}", output);
}

#[test]
fn arrays_start_multiline_when_large_or_too_wide() {
    for (input, width, last) in [
        ("[1,2,3,4,5,6]", 120, "  - 6"),
        (r#"{"items":["alpha","beta"]}"#, 20, "  - beta"),
    ] {
        let output = strip_styles(&session_with_width(input, "q", None, width));
        assert!(output.contains(last), "{}", output);
    }
}

#[test]
fn hidden_search_reveals_and_prints_the_cell_without_annotations() {
    let output = session(
        r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Lin"}]}"#,
        "l /Lin\npp q",
    );
    assert!(output.contains("\"Lin\"\r\n"), "{}", output);
    assert!(strip_styles(&output).contains("users[1].name"));
}

#[test]
fn resize_and_column_motion_preserve_the_parsed_selection() {
    let output = session(
        r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Lin"}]}"#,
        "lllJj\x12pp q",
    );
    assert!(output.contains("\"Lin\"\r\n"), "{}", output);
}

#[test]
fn duplicate_warnings_and_occurrence_status_preserve_copy_identity() {
    let output = session(r#"{"status":"queued","status":"done"}"#, "lJpp q");
    let clean = strip_styles(&output);
    assert!(clean.contains("# WARN Duplicate key"));
    assert!(clean.contains("occurrence 2 of 2"));
    assert!(output.contains("\"done\"\r\n"));
}

#[test]
fn search_in_the_last_fully_visible_column_does_not_scroll() {
    let value = format!("{}Z", "a".repeat(26));
    let input = format!("{{\"x\":\"{}\"}}", value);
    let output = strip_styles(&session_with_width(&input, "/Z\nq", None, 35));
    let line = format!("x: {}", value);
    assert!(output.matches(&line).count() >= 2, "{}", output);
    assert!(!output.contains("…Z"), "{}", output);
}

#[test]
fn long_string_search_reveals_the_match_inside_its_token() {
    let input = format!("{{\"value\":\"{}NEEDLE\"}}", "a".repeat(150));
    let output = strip_styles(&session_with_width(&input, "/NEEDLE\nq", None, 35));
    assert!(output.contains("…NEEDLE"), "{}", output);
}

#[test]
fn horizontal_keys_move_in_ten_cell_increments() {
    let input = r#""0123456789abcdefghijKLMNOPQRSTUVWXYZ0123456789abcdefghij""#;
    for (commands, expected) in [
        (".q", "…abcdefghij"),
        ("2.q", "…KLMNOPQRST"),
        ("2.,q", "…abcdefghij"),
    ] {
        let output = strip_styles(&session_with_width(input, commands, None, 35));
        assert!(output.contains(expected), "{}", output);
    }
}

#[test]
fn semicolon_reaches_the_end_from_an_intermediate_horizontal_offset() {
    let input = format!("\"{}TAIL\"", "a".repeat(150));
    let output = strip_styles(&session_with_width(&input, "10.;q", None, 35));
    assert!(output.contains("TAIL"), "{}", output);
}
