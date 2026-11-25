use super::*;

#[test]
fn collapsed_preview_maps_searches_to_generated_text() {
    let mut flat = json(r#"{"obj":{"needle":"value","other":1}}"#);
    let layout = Fixture::canonical(&flat);
    let object = children(&flat, 0)
        .into_iter()
        .find(|&node| flat[node].key_range.is_some())
        .unwrap();
    flat.collapse(object);
    let line = layout
        .project(&flat)
        .into_iter()
        .find(|line| line.line.owner == object)
        .unwrap()
        .line;
    let preview = line
        .spans
        .iter()
        .find(|span| span.role == TokenRole::Preview)
        .unwrap();
    let source_start = flat.1.find("needle").unwrap();
    assert!(
        !preview
            .matching_ranges(&(source_start..source_start + 6))
            .is_empty()
    );
}

#[test]
fn bounded_preview_marks_truncation_with_little_room_left() {
    let mut preview = Preview::default();
    preview_append(&mut preview, &"x".repeat(254), None);
    preview_append(&mut preview, "longer", None);
    assert_eq!(preview.text.len(), 256);
    assert!(preview.text.ends_with('…'));
}
#[test]
fn string_matches_map_only_the_rendered_substring_after_quotes_and_escapes() {
    for input in [
        r#"{"value":"aaaaaaaaNEEDLE"}"#,
        r#"{"value":"\u754c\n\"\\\ud83d\ude00NEEDLE"}"#,
    ] {
        let flat = json(input);
        let layout = Fixture::canonical(&flat);
        let source = flat.1.find("NEEDLE").unwrap();
        let span = layout.lines[0]
            .spans
            .iter()
            .find(|span| span.role == TokenRole::String)
            .unwrap();
        let ranges = span.matching_ranges(&(source..source + 6));
        assert_eq!(
            ranges
                .iter()
                .map(|range| &layout.lines[0].text[range.clone()])
                .collect::<String>(),
            "NEEDLE"
        );
    }
    let flat = json(r#""\u0061b\u0063""#);
    let layout = Fixture::canonical(&flat);
    let span = &layout.lines[0].spans[0];
    assert_eq!(span.matching_ranges(&(1..7)), vec![0..1]);
    assert_eq!(layout.lines[0].text, "abc");
}

#[test]
fn large_plain_string_mapping_coalesces_without_character_storage() {
    let input = format!("\"{}NEEDLE\"", "a".repeat(1_000_000));
    // Exercise display allocation directly, independently of parser throughput.
    let mut flat = json(r#""""#);
    flat.1 = input;
    flat.0[0].range = 0..flat.1.len();
    let layout = Fixture::canonical(&flat);
    let span = &layout.lines[0].spans[0];
    assert_eq!(span.source_map.len(), 1);
    assert_eq!(
        span.matching_ranges(&(1_000_001..1_000_007)),
        vec![1_000_000..1_000_006]
    );
}
