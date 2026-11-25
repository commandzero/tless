use super::pty::session;
use super::pty::session_with_width;
use super::screen::rendered_rows;
use super::screen::strip_styles;

#[test]
fn output_selection_leaves_terminal_view_and_json_print_unchanged() {
    for option in [
        "--output-format=json",
        "--output-format=yaml",
        "--output-format=toon",
    ] {
        let output = session_with_width(r#"{"a":1}"#, "lpp q", Some(option), 120);
        assert!(strip_styles(&output).contains("a: 1"));
        assert!(output.contains("1\r\n"));
    }
}

#[test]
fn every_input_uses_the_same_toon_document_lines() {
    let json = r#"{"tags":["rust","cli"],"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Lin"}]}"#;
    let yaml = "tags: [rust, cli]\nusers:\n  - {id: 1, name: Ada}\n  - {id: 2, name: Lin}\n";
    let cases = vec![
        (json, None),
        (yaml, Some("--input-format=yaml")),
        (
            "tags[2]: rust,cli\nusers[2]{id,name}:\n  1,Ada\n  2,Lin",
            Some("--input-format=toon"),
        ),
    ];
    for (input, argument) in cases {
        let output = strip_styles(&session_with_width(input, "q", argument, 120));
        for expected in ["tags[2]: rust,cli", "users[2]{id,name}:", "1,Ada", "2,Lin"] {
            assert!(
                output.contains(expected),
                "missing {}: {}",
                expected,
                output
            );
        }
    }
}

#[test]
fn toon_44_input_renders_nested_keyed_and_empty_shapes_like_json() {
    for (toon, json, header) in [
        (
            "items[2]{id,nested{x}}:\n  1,2\n  3,4",
            r#"{"items":[{"id":1,"nested":{"x":2}},{"id":3,"nested":{"x":4}}]}"#,
            Some("items[2]{id,nested{x}}:"),
        ),
        (
            "scores[2:]{score}:\n  alice: 1\n  bob: 2",
            r#"{"scores":{"alice":{"score":1},"bob":{"score":2}}}"#,
            Some("scores[2:]{score}:"),
        ),
        ("items: []", r#"{"items":[]}"#, Some("items: []")),
        ("[2]:\n  -\n  -", "[{},{}]", None),
        ("\"name\"   : Ada", r#"{"name":"Ada"}"#, None),
        (
            "items[2]:\n  -   a\n  -   c",
            r#"{"items":["a","c"]}"#,
            None,
        ),
        (
            "value: \u{a0}Ada\u{a0}",
            "{\"value\":\"\u{a0}Ada\u{a0}\"}",
            None,
        ),
        ("value: 1e1025", r#"{"value":"1e1025"}"#, None),
    ] {
        for width in [120, 30] {
            let actual = rendered_rows(
                &session_with_width(toon, "q", Some("--input-format=toon"), width),
                width,
                24,
            );
            let reference = rendered_rows(&session_with_width(json, "q", None, width), width, 24);
            assert_eq!(&actual[..22], &reference[..22], "{toon}, width {width}");
            if let Some(header) = header.filter(|_| width == 120) {
                assert!(actual[..22].join("\n").contains(header), "{actual:?}");
            }
        }
    }
    for (toon, path, value) in [
        (
            "items[2]{id,nested{x}}:\n  1,2\n  3,4",
            ".items[1].nested.x",
            "4",
        ),
        (
            "scores[2:]{score}:\n  alice: 1\n  bob: 2",
            ".scores.bob.score",
            "2",
        ),
    ] {
        let output = session_with_width(
            toon,
            &format!(":{path}\npt q"),
            Some("--input-format=toon"),
            120,
        );
        assert!(output.contains(&format!("{value}\r\n")), "{output}");
    }
}

#[test]
fn nested_columns_preserve_focus_search_alignment_and_paths() {
    let input = r#"{"items":[{"id":1,"nested":{"left":"Ada","right":2}},{"id":3,"nested":{"left":"Lin","right":4}}],"outside":true}"#;
    for width in [120, 35] {
        let rows = rendered_rows(&session_with_width(input, "l\tq", None, width), width, 24);
        assert!(rows[..22].join("\n").contains("nested{left"), "{rows:?}");
        assert!(rows[22].contains("Table aligned"), "{rows:?}");
        let group = session_with_width(input, "lllJ\tlpP pp q", None, width);
        assert!(group.contains(".items[0].nested.left\r\n"), "{group:?}");
        assert!(group.contains("\"Ada\"\r\n"), "{group:?}");
        let second = session_with_width(input, "lllJljpP pp q", None, width);
        assert!(second.contains(".items[1].nested.left\r\n"), "{second:?}");
        assert!(second.contains("\"Lin\"\r\n"), "{second:?}");
        let search = session_with_width(input, "l\t/left\npP pp q", None, width);
        assert!(search.contains(".items[0].nested.left\r\n"), "{search:?}");
        assert!(search.contains("\"Ada\"\r\n"), "{search:?}");
    }
    let ordinary = rendered_rows(&session(input, "J\tq"), 120, 24);
    assert!(!ordinary[22].contains("Table aligned"), "{ordinary:?}");
}

#[test]
fn keyed_root_table_header_rows_groups_and_filter_restore() {
    let input = r#"{"alice":{"score":1,"meta":{"rank":2}},"bob":{"score":3,"meta":{"rank":4}}}"#;
    for width in [120, 35] {
        let rows = rendered_rows(&session_with_width(input, "\tq", None, width), width, 24);
        assert!(rows[..22].join("\n").contains("[2:]{"), "{rows:?}");
        assert!(rows[22].contains("Table aligned"), "{rows:?}");
        let nested = session_with_width(input, "llJlpP pp q", None, width);
        assert!(nested.contains(".alice.meta.rank\r\n"), "{nested:?}");
        assert!(nested.contains("2\r\n"), "{nested:?}");
        let next = session_with_width(input, "llJljpP pp q", None, width);
        assert!(next.contains(".bob.meta.rank\r\n"), "{next:?}");
        let searched = session_with_width(input, "\t/rank\npP pp q", None, width);
        assert!(searched.contains(".alice.meta.rank\r\n"), "{searched:?}");
    }
    for width in [120, 30] {
        let collapsed = rendered_rows(&session_with_width(input, " q", None, width), width, 24);
        assert!(collapsed[0].contains("▸ [2:]"), "{collapsed:?}");
        assert_eq!(collapsed[1].trim(), "~", "{collapsed:?}");
    }
    let restored = rendered_rows(&session(input, "\t:.bob.meta.rank\n:.\nq"), 120, 24);
    assert!(restored[..22].join("\n").contains("[2:]{"), "{restored:?}");
    assert!(restored[22].contains("Table aligned"), "{restored:?}");
    let filtered = rendered_rows(&session(input, "\t:.bob.meta.rank\nq"), 120, 24);
    assert!(!filtered[22].contains("Table aligned"), "{filtered:?}");
}

#[test]
fn keyed_table_focused_print_and_filtered_write_keep_nested_source_values() {
    let input = r#"{"alice":{"score":1,"meta":{"rank":2}},"bob":{"score":3,"meta":{"rank":4}}}"#;
    let copied = session(input, "llJlpP pp q");
    assert!(copied.contains(".alice.meta.rank\r\n"), "{copied:?}");
    assert!(copied.contains("2\r\n"), "{copied:?}");
    let target =
        std::env::temp_dir().join(format!("tless-keyed-nested-{}.json", std::process::id()));
    session(
        input,
        &format!(":.bob.meta.rank\n:write-json! {}\nq", target.display()),
    );
    assert_eq!(std::fs::read(&target).unwrap(), b"4\n");
    session(input, &format!(":.\n:write-json! {}\nq", target.display()));
    assert_eq!(
            std::fs::read(&target).unwrap(),
            b"{\n  \"alice\": {\n    \"score\": 1,\n    \"meta\": {\n      \"rank\": 2\n    }\n  },\n  \"bob\": {\n    \"score\": 3,\n    \"meta\": {\n      \"rank\": 4\n    }\n  }\n}\n"
        );
    std::fs::remove_file(target).unwrap();
}
