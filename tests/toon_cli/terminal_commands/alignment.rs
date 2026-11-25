use super::pty::session;
use super::pty::session_with_width;
use super::screen::rendered_rows;
use super::screen::strip_styles;
use crate::run;

#[test]
fn alignment_tab_targets_header_row_and_cell_once_and_indicates_focus() {
    let input = r#"{"users":[{"id":1,"name":"Ada"},{"id":200,"name":"Lin"}],"plain":true}"#;
    for keys in ["l\tq", "ll\tq", "lll\tq", "l3\tq"] {
        let rows = rendered_rows(&session(input, keys), 120, 24);
        let text = rows[..22].join("\n");
        assert!(text.contains("users[2]{id  name}:"), "{keys:?}: {rows:?}");
        assert!(text.contains("           1 Ada"), "{keys:?}: {rows:?}");
        assert!(text.contains("         200 Lin"), "{keys:?}: {rows:?}");
        assert!(rows[22].contains("Table aligned"), "{keys:?}: {rows:?}");
    }
    let rows = rendered_rows(&session(input, "l\t\tq"), 120, 24);
    assert!(rows[..22].join("\n").contains("users[2]{id,name}:"));
    assert!(!rows[22].contains("Table aligned"), "{rows:?}");

    let collapsed = rendered_rows(&session(input, "l \tq"), 120, 24);
    assert!(collapsed[0].contains("users[2]"), "{collapsed:?}");
    assert!(!collapsed[..22].join("\n").contains("           1 Ada"));
    assert!(collapsed[22].contains("Table aligned"), "{collapsed:?}");
    let restored = rendered_rows(&session(input, "l \tlq"), 120, 24);
    assert!(restored[..22].join("\n").contains("         200 Lin"));
}

#[test]
fn alignment_is_independent_and_ignores_ineligible_and_prompt_focus() {
    let input = r#"{"users":[{"id":1,"name":"Ada"},{"id":200,"name":"Lin"}],"other":[{"id":3,"name":"One"},{"id":4,"name":"Two"}],"scalar":9,"values":[1,2],"list":[{"a":1},{"b":2}]}"#;
    let first = rendered_rows(&session(input, "l\tJq"), 120, 24);
    assert!(first[..22].join("\n").contains("           1 Ada"));
    assert!(!first[22].contains("Table aligned"), "{first:?}");

    let both = rendered_rows(&session(input, "l\tJ\tq"), 120, 24);
    assert!(both[..22].join("\n").contains("           1 Ada"));
    assert!(both[22].contains("Table aligned"), "{both:?}");
    let only_first = rendered_rows(&session(input, "l\tJ\t\tKq"), 120, 24);
    assert!(only_first[..22].join("\n").contains("           1 Ada"));
    assert!(only_first[..22].join("\n").contains("other[2]{id,name}:"));
    assert!(only_first[22].contains("Table aligned"), "{only_first:?}");

    for keys in ["lJJ\tq", "lJJJ\tq", "lJJJJ\tq"] {
        let rows = rendered_rows(&session(input, keys), 120, 24);
        assert!(!rows[22].contains("Table aligned"), "{keys:?}: {rows:?}");
        assert!(rows[..22].join("\n").contains("users[2]{id,name}:"));
    }
    let sequence = r#"{"users":[{"id":1},{"id":200}]} {"plain":2}"#;
    let separator = rendered_rows(&session(sequence, "\tq"), 120, 24);
    assert!(!separator[22].contains("Table aligned"), "{separator:?}");
    assert!(
        separator[..22].join("\n").contains("users[2]{id}:"),
        "{separator:?}"
    );
    let output = session(input, "l\t:write-j\t\x03/name\t\nq");
    let rows = rendered_rows(&output, 120, 24);
    assert!(rows[22].contains("Table aligned"), "{rows:?}");
    assert!(output.contains("write-json"), "{output:?}");
}

#[test]
fn alignment_preserves_complete_column_widths_and_source_spelling() {
    use unicode_width::UnicodeWidthStr;

    let mut input = String::from(r#"{"users":["#);
    for index in 0..28 {
        if index != 0 {
            input.push(',');
        }
        let value = if index == 27 {
            r#""é界👩‍💻\\n\\\"END""#
        } else {
            r#""Ada""#
        };
        input.push_str(&format!(r#"{{"id":{index},"name":{value},"flag":true}}"#));
    }
    input.push_str("]}");
    let output = session(&input, "l\tq");
    let rows = rendered_rows(&output, 120, 24);
    let header = rows.iter().find(|row| row.contains("users[28]{")).unwrap();
    let first = rows.iter().find(|row| row.contains("Ada")).unwrap();
    let header_col = UnicodeWidthStr::width(header.split("name").next().unwrap());
    let value_col = UnicodeWidthStr::width(first.split("Ada").next().unwrap());
    assert_eq!(header_col, value_col, "{rows:?}");
    let flag_header_col = UnicodeWidthStr::width(header.split("flag").next().unwrap());
    let flag_value_col = UnicodeWidthStr::width(first.split("true").next().unwrap());
    assert_eq!(flag_header_col, flag_value_col, "{rows:?}");
    let last = session(&input, "l\t/END\npp q");
    let clean = strip_styles(&last);
    assert!(clean.contains("users[27].name"), "{last:?}");
    assert!(clean.contains(r#"\\n\\\"END"#), "{last:?}");
}

#[test]
fn wrapping_preserves_scrolled_aligned_header_rows_and_cell_focus() {
    let input = format!(
        r#"{{"users":[{{"identifier":"{}","name":"Ada"}},{{"identifier":"{}","name":"Lin"}}],"other":"{}"}}"#,
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
        "0123456789".repeat(8),
    );
    for (focus, path) in [
        ("", ".users"),
        ("j", ".users[0]"),
        ("jl", ".users[0].identifier"),
    ] {
        let before = rendered_rows(
            &session_with_width(&input, &format!("l\t{focus}.q"), None, 35),
            35,
            24,
        );
        assert!(before[22].contains(path), "{focus}: {before:?}");
        let wrapped = rendered_rows(
            &session_with_width(&input, &format!("l\t{focus}.\x0cq"), None, 35),
            35,
            24,
        );
        assert_eq!(&before[..3], &wrapped[..3], "{focus}: {wrapped:?}");
        assert!(wrapped[22].contains(path), "{focus}: {wrapped:?}");
        let unwrapped = rendered_rows(
            &session_with_width(&input, &format!("l\t{focus}.\x0c\x0cq"), None, 35),
            35,
            24,
        );
        assert_eq!(&before[..3], &unwrapped[..3], "{focus}: {unwrapped:?}");
        let count = |rows: &[String]| {
            rows[..22]
                .iter()
                .take_while(|row| row.as_str() != "~")
                .count()
        };
        assert!(count(&wrapped) > count(&before), "{focus}: {wrapped:?}");
    }
}

#[test]
fn aligned_offset_survives_vertical_motion_leave_return_and_collapse() {
    let input = format!(
        r#"{{"users":[{{"identifier":"{}","name":"Ada"}},{{"identifier":"{}","name":"Lin"}}],"other":0}}"#,
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789", "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
    );
    let viewport = |keys: &str| {
        let rows = rendered_rows(&session_with_width(&input, keys, None, 35), 35, 24);
        rows[..4].to_vec()
    };
    let cell = viewport("l\tlll.q");
    assert_ne!(
        cell,
        viewport("l\tlllq"),
        "counted scrolling must move the shared viewport"
    );
    for keys in ["l\tlll.jq", "l\tlll.jkkq", "l\tlll.jkkkjq", "l\tlll.jjkq"] {
        assert_eq!(viewport(keys), cell, "{keys}");
    }

    let header = viewport("l\t.q");
    assert_ne!(header, viewport("l\tq"), "header must scroll");
    for keys in ["l\t.  q", "l\t.jhq"] {
        assert_eq!(viewport(keys), header, "{keys}");
    }
    let selected = viewport("l\t.jlq");
    assert_ne!(
        selected, header,
        "horizontal selection must reveal its cell"
    );
    assert!(
        selected.iter().any(|row| row.contains("ABCDEFGHIJ")),
        "{selected:?}"
    );

    let nested = format!(r#"{{"nest":{input},"outside":0}}"#);
    let nested_view = |keys: &str| {
        let rows = rendered_rows(&session_with_width(&nested, keys, None, 35), 35, 24);
        rows[..5].to_vec()
    };
    let before = nested_view("ll\t.q");
    assert!(before.iter().any(|row| row.contains('…')), "{before:?}");
    assert_eq!(nested_view("ll\t.k  jq"), before);
}

#[test]
fn horizontal_cell_selection_reveals_without_vertical_cell_reveal() {
    let input = r#"{"users":[{"identifier":"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789","name":"Ada"},{"identifier":"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789","name":"Lin"}]}"#;
    let viewport = |keys: &str| {
        let rows = rendered_rows(&session_with_width(input, keys, None, 35), 35, 24);
        rows[..3].to_vec()
    };
    let id = viewport("l\tlllq");
    let scrolled = viewport("l\tlll.q");
    let name = viewport("l\tlll.Jq");
    assert_ne!(
        name, scrolled,
        "sibling selection must reveal the name cell"
    );
    assert!(name.iter().any(|row| row.contains("Ada")), "{name:?}");
    assert_eq!(viewport("l\tlll.$q"), name);
    for keys in ["l\tlll.JKq", "l\tlll.J0q", "l\tlll.J^q"] {
        assert_eq!(viewport(keys), id, "{keys}");
    }

    let uneven = r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"}]}"#;
    let first = rendered_rows(&session_with_width(uneven, "l\tllJq", None, 35), 35, 24);
    let moved = rendered_rows(&session_with_width(uneven, "l\tllJjq", None, 35), 35, 24);
    assert_eq!(&moved[..3], &first[..3]);
    assert!(moved[22].contains(".users[1].name"), "{moved:?}");
}

#[test]
fn resize_preserves_scrolled_header_and_reveals_newly_right_clipped_cell() {
    let input = r#"{"users":[{"identifier":"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789","name":"Ada"},{"identifier":"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789","name":"Lin"}]}"#;
    let before = rendered_rows(&session_with_width(input, "l\t.q", None, 35), 35, 24);
    let height = rendered_rows(&session_with_width(input, "l\t.\x11q", None, 35), 35, 8);
    assert_eq!(
        &height[..3],
        &before[..3],
        "height-only resize moved the table"
    );
    let narrow = rendered_rows(&session_with_width(input, "l\t.\x12q", None, 35), 16, 8);
    let already_narrow = rendered_rows(&session_with_width(input, "l\t.q", None, 16), 16, 24);
    assert_eq!(
        &narrow[..3],
        &already_narrow[..3],
        "width resize lost a valid offset"
    );

    let cell = r#"{"users":[{"id":"12345678","name":"Ada"},{"id":"87654321","name":"Lin"}]}"#;
    let resized = rendered_rows(&session_with_width(cell, "l\tjlJ\x12q", None, 35), 16, 8);
    assert!(resized[..6].join("\n").contains("Ada"), "{resized:?}");
}

#[test]
fn hidden_aligned_table_restores_saturated_offset_after_filtering() {
    let input = r#"{"table_with_a_long_name":[{"identifier":"0123456789ABCDEFGHIJ","name":"Ada"},{"identifier":"x","name":"Lin"}],"other":"outside"}"#;
    let viewport = |keys: &str| {
        let rows = rendered_rows(&session_with_width(input, keys, None, 50), 50, 24);
        rows[..4].to_vec()
    };
    let before = viewport("l\t999999999.q");
    assert!(before.iter().any(|row| row.contains("…:")), "{before:?}");
    assert_eq!(viewport("l\t999999999.:.other\n:.\nq"), before);

    let nested = r#"{"nest":{"users":[{"identifier":"0123456789ABCDEFGHIJ","name":"Ada"},{"identifier":"x","name":"Lin"}]},"other":0}"#;
    let nested_view = |keys: &str| {
        let rows = rendered_rows(&session_with_width(nested, keys, None, 50), 50, 24);
        rows[..5].to_vec()
    };
    let ancestor = nested_view("ll\t999999999.q");
    assert_eq!(nested_view("ll\t999999999.k  jq"), ancestor);

    let sequence = r#"{"users":[{"identifier":"0123456789ABCDEFGHIJ","name":"Ada"},{"identifier":"x","name":"Lin"}],"a":1} {"a":2}"#;
    let original = rendered_rows(
        &session_with_width(sequence, "l\t999999999.q", None, 50),
        50,
        24,
    );
    let rows = rendered_rows(
        &session_with_width(sequence, "l\t999999999.:.a\n:.\nq", None, 50),
        50,
        24,
    );
    assert_eq!(&rows[..7], &original[..7]);
    assert!(rows[..22].join("\n").contains("--- (2 of 2)"), "{rows:?}");
}

#[test]
fn alignment_toggle_clears_ordinary_offsets_on_shared_list_object_headers() {
    let input = r#"{"outer":[{"rows":[{"identifier":1,"description":"Lin"}],"flag":true}]}"#;
    let viewport = |keys: &str| {
        let rows = rendered_rows(&session_with_width(input, keys, None, 35), 35, 24);
        rows[..4].to_vec()
    };
    let ordinary = viewport("lllq");
    assert_ne!(viewport("lll.q")[1], ordinary[1]);
    assert_eq!(viewport("lll.\t\tq"), ordinary);
}

#[test]
fn list_object_table_full_reduction_keeps_columns_and_mouse_identity() {
    use unicode_width::UnicodeWidthStr;
    let input = r#"{"list":[{"users":[{"id":1,"name":"Ada"},{"id":200,"name":"Lin"}]},{"x":1}]}"#;
    let keys = "lll\t<<";
    let rows = rendered_rows(&session(input, &format!("{keys}q")), 120, 24);
    let header = rows.iter().find(|row| row.contains("users[2]{")).unwrap();
    let first = rows.iter().find(|row| row.contains("Ada")).unwrap();
    let second = rows.iter().find(|row| row.contains("Lin")).unwrap();
    let column = UnicodeWidthStr::width(header.split("name").next().unwrap());
    assert_eq!(
        column,
        UnicodeWidthStr::width(first.split("Ada").next().unwrap()),
        "{rows:?}"
    );
    assert_eq!(
        column,
        UnicodeWidthStr::width(second.split("Lin").next().unwrap()),
        "{rows:?}"
    );
    let header_row = rows.iter().position(|row| row == header).unwrap() + 1;
    let value_row = rows.iter().position(|row| row == second).unwrap() + 1;
    for (row, expected) in [
        (header_row, ".list[0].users[0].name"),
        (value_row, ".list[0].users[1].name"),
    ] {
        let click = format!("\x1b[<0;{};{row}MpP q", column + 1);
        let output = session(input, &format!("{keys}{click}"));
        assert!(
            strip_styles(&output).contains(&format!("{expected}\r\n")),
            "{output:?}"
        );
    }
}

#[test]
fn nested_header_and_row_mouse_hits_keep_their_source_identity() {
    assert_aligned_mouse_paths(AlignedMouseCase {
        input: r#"{"items":[{"id":1,"nested":{"left":"Ada","right":2}},{"id":3,"nested":{"left":"Lin","right":4}}]}"#,
        keys: "l\t",
        header: "nested{left",
        field: "left",
        expected_paths: [".items[0].nested.left", ".items[1].nested.left"],
    });
}

#[test]
fn keyed_table_mouse_reaches_nested_header_and_second_entry() {
    assert_aligned_mouse_paths(AlignedMouseCase {
        input: r#"{"alice":{"rank":1,"meta":{"label":"Ada"}},"bob":{"rank":2,"meta":{"label":"Lin"}}}"#,
        keys: "\t",
        header: "meta{label",
        field: "label",
        expected_paths: [".alice.meta.label", ".bob.meta.label"],
    });
}

struct AlignedMouseCase {
    input: &'static str,
    keys: &'static str,
    header: &'static str,
    field: &'static str,
    expected_paths: [&'static str; 2],
}

fn assert_aligned_mouse_paths(case: AlignedMouseCase) {
    use unicode_width::UnicodeWidthStr;

    let AlignedMouseCase {
        input,
        keys,
        header,
        field,
        expected_paths,
    } = case;
    let rows = rendered_rows(&session(input, &format!("{keys}q")), 120, 24);
    let header_row = rows.iter().position(|row| row.contains(header)).unwrap();
    let value_row = rows.iter().position(|row| row.contains("Lin")).unwrap();
    let header_column = UnicodeWidthStr::width(rows[header_row].split(field).next().unwrap());
    let value_column = UnicodeWidthStr::width(rows[value_row].split("Lin").next().unwrap());
    assert_eq!(header_column, value_column, "{rows:?}");

    let column = header_column + 1;
    for (row, expected) in [header_row + 1, value_row + 1]
        .into_iter()
        .zip(expected_paths)
    {
        let output = session(input, &format!("{keys}\x1b[<0;{column};{row}MpP q"));
        assert!(output.contains(&format!("{expected}\r\n")), "{output:?}");
    }
}

#[test]
fn aligned_table_scrolling_and_wrapping_are_local() {
    let input = format!(
        r#"{{"users":[{{"id":1,"name":"Ada"}},{{"id":200,"name":"{}END"}}],"other":"{}"}}"#,
        "abcdefghijklmnopqrstuvwxyz".repeat(3),
        "0123456789".repeat(8),
    );
    let start = rendered_rows(&session_with_width(&input, "l\tq", None, 35), 35, 24);
    assert!(start[..22].join("\n").contains("users[2]{"), "{start:?}");
    let right = rendered_rows(&session_with_width(&input, "l\t2.q", None, 35), 35, 24);
    assert!(!right[..22].join("\n").contains("users[2]{"), "{right:?}");
    assert!(!right[..22].join("\n").contains(",Ada"), "{right:?}");
    let left = rendered_rows(&session_with_width(&input, "l\t2.,,q", None, 35), 35, 24);
    assert!(left[..22].join("\n").contains("users[2]{"), "{left:?}");
    let end = rendered_rows(&session_with_width(&input, "l\t;q", None, 35), 35, 24);
    assert!(end[..22].join("\n").contains("END"), "{end:?}");
    let beginning = rendered_rows(&session_with_width(&input, "l\t;;q", None, 35), 35, 24);
    assert!(
        beginning[..22].join("\n").contains("users[2]{"),
        "{beginning:?}"
    );
    let wrapped = rendered_rows(&session_with_width(&input, "l\x0c\t.q", None, 35), 35, 24);
    assert!(
        !wrapped[..22].join("\n").contains("users[2]{"),
        "{wrapped:?}"
    );
    assert!(wrapped[22].contains("Table aligned"), "{wrapped:?}");
    let reverted = rendered_rows(&session_with_width(&input, "l\x0c\t\tq", None, 35), 35, 24);
    assert!(!reverted[22].contains("Table aligned"), "{reverted:?}");
    let data_lines = |rows: &[String]| {
        rows[..22]
            .iter()
            .take_while(|row| row.as_str() != "~")
            .count()
    };
    assert!(data_lines(&reverted) > data_lines(&start), "{reverted:?}");
}

#[test]
fn aligned_table_survives_filtering_collapse_gutters_and_resize() {
    let input = r#"{"nest":{"users":[{"id":1,"name":"Ada"},{"id":200,"name":"Lin"}]},"last":0}"#;
    let output = session(input, "ll\t:set nonumber\n:set relativenumber\n<\x12q");
    let rows = rendered_rows(&output, 16, 8);
    assert!(rows[6].contains("Align"), "{rows:?}");
    let restored = session(input, "ll\t:.nest.users[0]\n:.\nllq");
    let rows = rendered_rows(&restored, 120, 24);
    assert!(
        rows[..22].join("\n").contains("         200 Lin"),
        "{rows:?}"
    );
    assert!(rows[22].contains("Table aligned"), "{rows:?}");
    let filtered = rendered_rows(&session(input, "ll\t:.nest.users[0]\nq"), 120, 24);
    assert!(
        !filtered[..22].join("\n").contains("users[2]{"),
        "{filtered:?}"
    );
    assert!(!filtered[22].contains("Table aligned"), "{filtered:?}");
}

#[test]
fn aligned_search_reveals_clipped_cell_and_shared_field_key_without_losing_identity() {
    let id = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let input =
        format!(r#"{{"users":[{{"id":"{id}","name":"Ada"}},{{"id":"{id}","name":"Lin"}}]}}"#);
    for (pattern, expected_path, expected_value) in [
        ("Lin", ".users[1].name", "\"Lin\""),
        ("name", ".users[0].name", "\"Ada\""),
    ] {
        let output = session_with_width(&input, &format!("l\t/{pattern}\npP pp q"), None, 35);
        assert!(
            strip_styles(&output).contains(&format!("{expected_path}\r\n")),
            "{pattern}: {output:?}"
        );
        assert!(
            output.contains(&format!("{expected_value}\r\n")),
            "{pattern}: {output:?}"
        );
        let rows = rendered_rows(&output, 35, 24);
        assert!(
            rows[..22].join("\n").contains(pattern),
            "{pattern}: {rows:?}"
        );
    }
}

#[test]
fn aligned_scroll_bounds_include_row_warnings_and_saturate_counts() {
    let input = "users:\n  - id: 1\n    name: Ada\n  - id: 2\n    name: .inf\n";
    let format = Some("--input-format=yaml");
    let at_end = rendered_rows(&session_with_width(input, "l\t;q", format, 35), 35, 24);
    let saturated = rendered_rows(
        &session_with_width(input, "l\t999999999.q", format, 35),
        35,
        24,
    );
    let repeated = rendered_rows(
        &session_with_width(input, "l\t999999999.999999999.q", format, 35),
        35,
        24,
    );
    // `;` fits the end in the viewport; counted scrolling can instead
    // place the very last cell at the left edge. Both must reach the
    // warning's last cell, and repeated oversized counts stop there.
    assert_eq!(
        saturated[..22],
        repeated[..22],
        "{saturated:?}\n{repeated:?}"
    );
    assert!(saturated[2].ends_with('"'), "{saturated:?}");
    assert!(at_end[..22].join("\n").contains("number"), "{at_end:?}");
    let back = rendered_rows(
        &session_with_width(input, "l\t;999999999,q", format, 35),
        35,
        24,
    );
    assert!(back[..22].join("\n").contains("users[2]{"), "{back:?}");
    let reset = rendered_rows(&session_with_width(input, "l\t;;q", format, 35), 35, 24);
    assert!(reset[..22].join("\n").contains("users[2]{"), "{reset:?}");
}

#[test]
fn aligned_mouse_targets_data_and_padding_after_shared_scroll() {
    let input = r#"{"users":[{"id":"ABCDEFGHIJKLMNO","name":"Ada"},{"id":"ABCDEFGHIJKLMNOP","name":"Lin"}]}"#;
    // With no number gutter, the second row's wider id leaves one
    // padding cell after the first row's id. At offset ten that cell is
    // terminal column 18 (including the arrow, spacer and ellipsis).
    let selected = session_with_width(input, ":set nonumber\nl\t.\x1b[<0;20;3MpP q", None, 35);
    assert!(
        strip_styles(&selected).contains(".users[1].name\r\n"),
        "{selected:?}"
    );
    let copied = session_with_width(input, ":set nonumber\nl\t.\x1b[<0;20;3Mpp q", None, 35);
    assert!(copied.contains("\"Lin\"\r\n"), "{copied:?}");
    let row_padding = session_with_width(input, ":set nonumber\nl\t.\x1b[<0;18;2MpP q", None, 35);
    assert!(
        strip_styles(&row_padding).contains(".users[0]\r\n"),
        "{row_padding:?}"
    );
    let header_padding =
        session_with_width(input, ":set nonumber\nl\t.\x1b[<0;10;1MpP q", None, 35);
    assert!(
        strip_styles(&header_padding).contains(".users\r\n"),
        "{header_padding:?}"
    );
}

#[test]
fn aligned_copy_write_and_redirected_output_ignore_presentation_padding() {
    let input = r#"{"users":[{"id":1,"name":"Ada"},{"id":200,"name":"Lin"}]}"#;
    let cell = session(input, "lll\tjpP q");
    assert!(strip_styles(&cell).contains(".users[1].id\r\n"), "{cell:?}");
    let value = session(input, "lll\tjJpp q");
    assert!(value.contains("\"Lin\"\r\n"), "{value:?}");
    let toon = std::env::temp_dir().join(format!("tless-aligned-{}.toon", std::process::id()));
    let json = std::env::temp_dir().join(format!("tless-aligned-{}.json", std::process::id()));
    session(
        input,
        &format!(
            "l:.users\n:write {}\n:write-json {}\nq",
            toon.display(),
            json.display()
        ),
    );
    let baseline_toon = std::fs::read(&toon).unwrap();
    let baseline_json = std::fs::read(&json).unwrap();
    session(
        input,
        &format!(
            "l\t:.users\n:write! {}\n:write-json! {}\nq",
            toon.display(),
            json.display()
        ),
    );
    assert_eq!(std::fs::read(&toon).unwrap(), baseline_toon);
    assert_eq!(std::fs::read(&json).unwrap(), baseline_json);
    let json_data: serde_json::Value = serde_json::from_slice(&baseline_json).unwrap();
    assert_eq!(json_data[1]["name"], "Lin");
    std::fs::remove_file(toon).unwrap();
    std::fs::remove_file(json).unwrap();
    let redirected = run(&["-i", "json"], input.as_bytes());
    assert!(redirected.status.success());
    assert!(!String::from_utf8_lossy(&redirected.stdout).contains("Table aligned"));
    assert_eq!(redirected.stdout, b"users[2]{id,name}:\n  1,Ada\n  200,Lin");
}
