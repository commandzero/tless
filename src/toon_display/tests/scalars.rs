use super::*;

fn ordered_values_equal(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    use serde_json::Value;
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|((ak, av), (bk, bv))| ak == bk && ordered_values_equal(av, bv))
        }
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| ordered_values_equal(a, b))
        }
        (Value::Number(a), Value::Number(b)) if a.is_f64() || b.is_f64() => {
            a.as_f64() == b.as_f64()
        }
        _ => a == b,
    }
}

#[test]
fn retained_standard_fixtures() {
    let mut checked = 0;
    for case in crate::toon::fixtures::load_fixtures("encode") {
        let metadata = &case.metadata;
        assert_eq!(metadata["fixture"]["category"], "encode", "{}", case.group);
        if crate::toon::fixtures::uses_encode_profile(&metadata["options"]) {
            assert_fixture_display(&case);
            checked += 1;
        }
    }
    assert_eq!(checked, 139);
}

fn terminal_control(ch: char) -> bool {
    ch >= '\u{7f}' && ch.is_control()
}

fn assert_fixture_display(case: &crate::toon::fixtures::FixtureCase) {
    let name = case.metadata["name"].as_str().unwrap();
    let input = case.metadata["input-json"].as_str().unwrap();
    let actual = text(&json(input));
    let expected = &case.toon;
    let input_value: serde_json::Value = serde_json::from_str(input).unwrap();
    let wire = crate::toon::parse(expected).unwrap();
    let wire_value = serde_json::from_str(&wire.1).unwrap();

    // Keep the UI terminal-safe and in source order even if wire output differs.
    if expected.chars().any(terminal_control) || !ordered_values_equal(&input_value, &wire_value) {
        assert!(!actual.chars().any(terminal_control), "{name}");
        let decoded = crate::toon::parse(&actual).unwrap();
        let displayed = serde_json::from_str(&decoded.1).unwrap();
        assert!(
            ordered_values_equal(&displayed, &input_value),
            "{name}: {actual}"
        );
    } else {
        assert_eq!(&actual, expected, "{name}");
    }
}

#[test]
fn standard_shapes_and_order() {
    assert_eq!(
        text(&json(
            r#"{"tags":["rust","cli"],"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Lin"}]}"#
        )),
        "tags[2]: rust,cli\nusers[2]{id,name}:\n  1,Ada\n  2,Lin"
    );
    for (input, expected) in [
        ("null", "null"),
        ("{}", ""),
        ("[]", "[]"),
        ("[{},{}]", "[2]:\n  -\n  -"),
        ("[[1,2],[]]", "[2]:\n  - [2]: 1,2\n  - [0]:"),
        (r#"{"e":{},"a":[]}"#, "e:\na: []"),
        (
            r#"[{"a":1,"b":2},{"b":3,"a":4}]"#,
            "[2]:\n  - a: 1\n    b: 2\n  - b: 3\n    a: 4",
        ),
    ] {
        assert_eq!(text(&json(input)), expected, "{input}");
    }
}
#[test]
fn nested_first_field_grammar() {
    assert_eq!(
        text(&json(r#"[{"rows":[{"id":1},{"id":2}],"x":true}]"#)),
        "[1]:\n  - rows[2]{id}:\n      1\n      2\n    x: true"
    );
}

#[test]
fn structural_tokens_keep_their_theme_roles() {
    let flat = json(r#"{"values":[1,2],"rows":[{"a":1},{"a":2}],"empty":[]}"#);
    let layout = Fixture::canonical(&flat);
    let roles: Vec<_> = layout
        .lines
        .iter()
        .flat_map(|line| line.spans.iter().map(|span| span.role))
        .collect();
    assert!(roles.contains(&TokenRole::ArrayIndex));
    assert!(roles.contains(&TokenRole::PrimitiveTrailingComma));
    assert!(roles.contains(&TokenRole::ContainerDelimiter));
    assert!(roles.contains(&TokenRole::EmptyContainer));
    assert!(roles.contains(&TokenRole::Punctuation));
}

#[test]
fn exact_bounded_decimals() {
    for (input, expected) in [
        ("0.123456789012345678901", "0.123456789012345678901"),
        ("-0.000e99", "0"),
        ("1.2300e2", "123"),
        ("1000e-5", "0.01"),
        ("1e-3", "0.001"),
        ("12.345e1", "123.45"),
    ] {
        assert_eq!(number(input), (expected.into(), None));
    }
    for input in [
        "1e1000000",
        "1e99999999999999999999999999",
        "1e-99999999999999999999",
        "1e4096",
    ] {
        assert_eq!(number(input), (input.into(), None));
    }
    for input in ["1_000", "+1", "01"] {
        assert_eq!(
            number(input),
            (input.into(), Some(WarningKind::NonCanonicalNumber))
        );
    }
    assert_eq!(number("1e4095").0.len(), 4096);
    assert_eq!(number("1e-4094").0.len(), 4096);
}
#[test]
fn typed_recursive_keys_and_multiple_roots() {
    assert_eq!(
        text(&yaml("1: number\n\"1\": string\n")),
        "? 1: number  # WARN Non-string key\n\"1\": string"
    );
    assert_eq!(
        text(&yaml("? [.inf, {a: true}]\n: value\n")),
        "? [.inf,{\"a\":true}]: value  # WARN Non-finite number; Non-string key"
    );
    assert_eq!(
        text(&yaml("---\n{}\n---\n7\n")),
        "--- (1 of 2)\n\n--- (2 of 2)\n7"
    );
}

#[test]
fn typed_complex_keys_preserve_quotes_backslashes_and_nested_values() {
    let flat = yaml("? {'quote\"x': true}\n: value\n");
    assert!(
        flat.1.contains(r#""quote"x": true"#),
        "historical copy/search spelling remains unchanged"
    );
    assert_eq!(
        text(&flat),
        r#"? {"quote\"x":true}: value  # WARN Non-string key"#
    );
    let flat = yaml("? ['quote\"x', '\\literal', {'a:b': [true, null, 1]}]\n: value\n");
    assert_eq!(
        text(&flat),
        r#"? ["quote\"x","\\literal",{"a:b":[true,null,1]}]: value  # WARN Non-string key"#
    );
    assert!(
        !Fixture::canonical(&flat)
            .warnings
            .iter()
            .any(|w| w.kind == WarningKind::NonCanonicalNumber)
    );
}

#[test]
fn collapsed_short_arrays_preserve_inline_value_roles() {
    let mut flat = json(r#"["text",2,true,null,5]"#);
    let layout = Fixture::for_view(&flat, 120, &HashSet::from([0]));
    flat.collapse(0);
    let projected = layout.project(&flat);
    let line = &projected[0].line;
    assert_eq!(line.text, "[5]: text,2,true,null,5");
    assert!(
        !line
            .spans
            .iter()
            .any(|span| span.role == TokenRole::Preview)
    );
    for role in [
        TokenRole::String,
        TokenRole::Number,
        TokenRole::Boolean,
        TokenRole::Null,
    ] {
        assert!(
            line.spans
                .iter()
                .any(|span| span.role == role && span.source.is_some())
        );
    }
    let narrow = Fixture::for_view(&flat, 10, &HashSet::from([0]));
    assert!(
        narrow.project(&flat)[0]
            .line
            .spans
            .iter()
            .any(|span| span.role == TokenRole::Preview)
    );
    let mut long = json("[1,2,3,4,5,6]");
    let layout = Fixture::for_view(&long, 120, &HashSet::new());
    long.collapse(0);
    assert!(
        layout.project(&long)[0]
            .line
            .spans
            .iter()
            .any(|span| span.role == TokenRole::Preview)
    );
}

#[test]
fn control_escapes_and_hash_prefix_are_standard_strings() {
    let flat = json(r##"{"control":"\u0001","literal":"\\u0001","hash":"#tag"}"##);
    assert_eq!(
        text(&flat),
        "control: \"\\u0001\"\nliteral: \"\\\\u0001\"\nhash: \"#tag\""
    );
    assert!(!text(&flat).chars().any(|ch| ch == '\u{1}'));
    let key = json(r#"{"\u0001":1}"#);
    assert_eq!(text(&key), "\"\\u0001\": 1");
    let key = yaml("? [\"\\u0001\"]\n: value\n");
    assert!(text(&key).contains("Non-string key"));
    assert!(!text(&key).contains("Non-standard string escape"));
}
#[test]
fn aligned_nested_leaves_and_keyed_prefixes_share_columns() {
    let flat = json(
        r#"{"short":{"num":2,"group":{"score":"ok"}},"longer":{"num":100,"group":{"score":"界"}}}"#,
    );
    let layout = layout::Layout::new(&flat, &[0], 120, false, &HashSet::new());
    let metrics = alignment::TableMetrics::measure(&flat, &layout.analysis, 0);
    let projection = layout.project_with_documents(&flat, &HashSet::new());
    let rendered: Vec<_> = (0..projection.len())
        .map(|n| {
            layout.render(
                &flat,
                layout.visible_line(&flat, &projection, n).unwrap(),
                0,
                Some(&metrics),
            )
        })
        .collect();
    let starts: Vec<_> = rendered
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .find(|span| match span.role {
                    TokenRole::FieldDefinition => &line.text[span.range.clone()] == "score",
                    TokenRole::String => {
                        &line.text[span.range.clone()] == "ok"
                            || &line.text[span.range.clone()] == "界"
                    }
                    _ => false,
                })
                .unwrap()
                .range
                .start
        })
        .collect();
    assert_eq!(starts, [starts[0]; 3]);
    let right_edges: Vec<_> = rendered[1..]
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .find(|span| span.role == TokenRole::Number)
                .unwrap()
                .range
                .end
        })
        .collect();
    assert_eq!(right_edges[0], right_edges[1]);
    assert!(
        metrics.extent(
            layout
                .geometry
                .table_field_start(&flat, &layout.analysis, 0)
        ) >= unicode_width::UnicodeWidthStr::width(rendered[2].text.as_str())
    );
}
#[test]
fn aligned_nested_numbers_include_group_prefixes_in_column_widths() {
    let flat = json(r#"[{"meta":{"n":1234567890}},{"meta":{"n":1}}]"#);
    let layout = layout::Layout::new(&flat, &[0], 120, false, &HashSet::new());
    let metrics = alignment::TableMetrics::measure(&flat, &layout.analysis, 0);
    let projection = layout.project_with_documents(&flat, &HashSet::new());
    let rows: Vec<_> = (1..=2)
        .map(|line| {
            layout.render(
                &flat,
                layout.visible_line(&flat, &projection, line).unwrap(),
                0,
                Some(&metrics),
            )
        })
        .collect();
    let numbers: Vec<_> = rows
        .iter()
        .map(|row| {
            let span = row
                .spans
                .iter()
                .find(|span| span.role == TokenRole::Number)
                .unwrap();
            (
                &row.text[span.range.clone()],
                unicode_width::UnicodeWidthStr::width(&row.text[..span.range.end]),
            )
        })
        .collect();
    assert_eq!(numbers[0].0, "1234567890");
    assert_eq!(numbers[1].0, "1");
    assert_eq!(numbers[0].1, numbers[1].1);
}

#[test]
fn del_and_c1_controls_use_standard_terminal_safe_unicode_escapes() {
    let flat = json(r#"["a\u007fb","a\u0085b","\u009b31m"]"#);
    let fixture = Fixture::canonical(&flat);
    assert_eq!(
        fixture.lines[0].text,
        r#"[3]: "a\u007fb","a\u0085b","\u009b31m""#
    );
    assert!(fixture.warnings.is_empty());
}
