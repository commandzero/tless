//! TOON document layout over parsed row identities, independent of the optional codec.
use crate::flatjson::{FlatJson, KeyValue, Value};
#[cfg(test)]
use std::collections::HashSet;
use std::ops::Range;

#[cfg(test)]
pub mod fixture;
mod format;
mod geometry;
mod index;
pub mod layout;
mod line_index;
#[cfg(test)]
use fixture::Fixture;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenRole {
    Key,
    FieldDefinition,
    String,
    Number,
    Boolean,
    Null,
    ArrayIndex,
    PrimitiveTrailingComma,
    ContainerDelimiter,
    EmptyContainer,
    Punctuation,
    Warning,
    Preview,
    Count,
    DocumentPosition,
}
#[derive(Clone, Debug)]
pub struct Span {
    pub range: Range<usize>,
    pub node: usize,
    pub role: TokenRole,
    pub source: Option<Range<usize>>,
    pub source_map: Vec<SourceMap>,
}
#[derive(Clone, Debug)]
pub struct SourceMap {
    pub source: Range<usize>,
    pub display: Range<usize>,
}

#[derive(Clone, Debug, Default)]
struct Preview {
    text: String,
    source: Option<Range<usize>>,
    source_map: Vec<SourceMap>,
}

impl Span {
    pub fn matching_ranges(&self, query: &Range<usize>) -> Vec<Range<usize>> {
        let overlaps = |range: &Range<usize>| range.start < query.end && query.start < range.end;
        if !self.source.as_ref().is_some_and(overlaps) {
            return vec![];
        }
        if self.source_map.is_empty() {
            return vec![self.range.clone()];
        }
        self.source_map
            .iter()
            .filter(|map| overlaps(&map.source))
            .map(|map| {
                if map.source.len() == map.display.len() {
                    map.display.start + query.start.max(map.source.start) - map.source.start
                        ..map.display.start + query.end.min(map.source.end) - map.source.start
                } else {
                    map.display.clone()
                }
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct DisplayLine {
    pub text: String,
    pub spans: Vec<Span>,
    pub owner: usize,
    pub separator: bool,
    /// Coalesced shared-header display highlights; bool marks the current match.
    pub shared_matches: Vec<(Range<usize>, bool)>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WarningKind {
    DuplicateKey,
    NonFiniteNumber,
    NonCanonicalNumber,
    NonStringKey,
    NonStandardStringEscape,
}
impl WarningKind {
    fn message(self) -> &'static str {
        match self {
            Self::DuplicateKey => "Duplicate key",
            Self::NonFiniteNumber => "Non-finite number",
            Self::NonCanonicalNumber => "Non-canonical number",
            Self::NonStringKey => "Non-string key",
            Self::NonStandardStringEscape => "Non-standard string escape",
        }
    }
}

pub fn normalize_node(flat: &FlatJson, node: usize) -> usize {
    if flat[node].is_closing_of_container() {
        flat[node].pair_index().unwrap()
    } else {
        node
    }
}

#[cfg(test)]
fn children(flat: &FlatJson, node: usize) -> Vec<usize> {
    let mut result = Vec::new();
    let mut next = flat[node].first_child();
    while let Some(i) = next.as_option() {
        result.push(i);
        next = flat[i].next_sibling;
    }
    result
}

fn scalar(flat: &FlatJson, node: usize) -> bool {
    matches!(
        flat[node].value,
        Value::String | Value::Number | Value::Boolean | Value::Null
    )
}

impl DisplayLine {
    fn new(owner: usize, prefix: String) -> Self {
        Self {
            text: prefix,
            spans: vec![],
            owner,
            separator: false,
            shared_matches: Vec::new(),
        }
    }
    fn token(&mut self, text: &str, node: usize, role: TokenRole, source: Option<Range<usize>>) {
        let start = self.text.len();
        self.text.push_str(text);
        self.spans.push(Span {
            range: start..self.text.len(),
            node,
            role,
            source,
            source_map: vec![],
        });
    }
}

fn quote_json(value: &str) -> String {
    use std::fmt::Write;
    let mut text = String::with_capacity(value.len() + 2);
    text.push('"');
    for ch in value.chars() {
        match ch {
            '"' => text.push_str("\\\""),
            '\\' => text.push_str("\\\\"),
            '\n' => text.push_str("\\n"),
            '\r' => text.push_str("\\r"),
            '\t' => text.push_str("\\t"),
            ch if ch.is_control() => {
                write!(text, "\\u{:04x}", ch as u32).unwrap();
            }
            _ => text.push(ch),
        }
    }
    text.push('"');
    text
}
fn decode_string(raw: &str) -> String {
    let inner = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(raw);
    // YAML's existing flattened strings can contain literal backslashes. Only
    // call the JSON unescaper when its syntactic precondition is satisfied.
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('u') => {
                    for _ in 0..4 {
                        if !chars.next().is_some_and(|c| c.is_ascii_hexdigit()) {
                            return inner.to_owned();
                        }
                    }
                }
                Some('"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't') => {}
                _ => return inner.to_owned(),
            }
        }
    }
    crate::jsonstringunescaper::unsafe_unescape_json_string(inner)
        .unwrap_or_else(|_| inner.to_owned())
}
fn quote_key(key: &str) -> String {
    let mut chars = key.chars();
    if chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    {
        key.to_owned()
    } else {
        quote_json(key)
    }
}
fn quote_value(value: &str) -> String {
    lazy_static::lazy_static! { static ref NUMERIC: regex::Regex=regex::Regex::new(r"^-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?$").unwrap(); }
    if value.is_empty()
        || value.trim() != value
        || matches!(
            value,
            "true" | "false" | "null" | ".inf" | "+.inf" | "-.inf" | ".nan"
        )
        || value.starts_with('-')
        || value.contains('#')
        || NUMERIC.is_match(value)
        || value
            .chars()
            .any(|c| c.is_control() || ":\"\\[]{},".contains(c))
    {
        quote_json(value)
    } else {
        value.to_owned()
    }
}
/// Exact decimal normalization, with the output length checked before allocating.
fn number(raw: &str) -> (String, Option<WarningKind>) {
    match raw.to_ascii_lowercase().as_str() {
        ".inf" | "+.inf" => return (".inf".into(), Some(WarningKind::NonFiniteNumber)),
        "-.inf" => return ("-.inf".into(), Some(WarningKind::NonFiniteNumber)),
        ".nan" | "+.nan" | "-.nan" => return (".nan".into(), Some(WarningKind::NonFiniteNumber)),
        _ => {}
    }
    let fallback = || (raw.to_owned(), Some(WarningKind::NonCanonicalNumber));
    lazy_static::lazy_static! {static ref DECIMAL:regex::Regex=regex::Regex::new(r"^(-?)(0|[1-9][0-9]*)(?:\.([0-9]+))?(?:[eE]([+-]?[0-9]+))?$").unwrap();}
    let Some(parts) = DECIMAL.captures(raw) else {
        return fallback();
    };
    let integer = &parts[2];
    let fraction = parts.get(3).map_or("", |m| m.as_str());
    let digits = format!("{integer}{fraction}");
    let trimmed = digits.trim_start_matches('0');
    if trimmed.is_empty() {
        return ("0".into(), None);
    }
    let leading = digits.len() - trimmed.len();
    let significant = trimmed.trim_end_matches('0');
    let exponent = match parts.get(4) {
        Some(exp) => match exp.as_str().parse::<i64>() {
            Ok(n) => n,
            Err(_) => return fallback(),
        },
        None => 0,
    };
    let point = match (integer.len() as i64)
        .checked_add(exponent)
        .and_then(|n| n.checked_sub(leading as i64))
    {
        Some(n) => n,
        None => return fallback(),
    };
    let negative = &parts[1] == "-";
    let size = if point <= 0 {
        point
            .checked_neg()
            .and_then(|n| n.checked_add(2))
            .and_then(|n| n.checked_add(significant.len() as i64))
    } else {
        Some(point.max(significant.len() as i64) + i64::from(point < (significant.len() as i64)))
    };
    let Some(size) = size.and_then(|n| n.checked_add(i64::from(negative))) else {
        return fallback();
    };
    if size > 4096 {
        return fallback();
    }
    let mut text = String::with_capacity(size as usize);
    if negative {
        text.push('-');
    }
    if point <= 0 {
        text.push_str("0.");
        text.extend(std::iter::repeat_n('0', (-point) as usize));
        text.push_str(significant);
    } else if point as usize >= significant.len() {
        text.push_str(significant);
        text.extend(std::iter::repeat_n('0', point as usize - significant.len()));
    } else {
        let point = point as usize;
        text.push_str(&significant[..point]);
        text.push('.');
        text.push_str(&significant[point..]);
    }
    (text, None)
}

/// Map decoded characters back to the matcher spelling, retaining quote removal
/// and escapes. Stream characters into coalesced runs without per-character storage.
fn string_source_map(
    raw: &str,
    parsed: Option<&str>,
    rendered: &str,
    source: usize,
    display: usize,
) -> Vec<SourceMap> {
    let quoted = rendered.starts_with('"') && rendered.ends_with('"');
    let mut mappings = Vec::<SourceMap>::new();
    let mut rendered_bytes = 0;
    let mut valid = true;
    let mut append = |text: &str, range: Range<usize>| {
        valid &= rendered.get(rendered_bytes..rendered_bytes + text.len()) == Some(text);
        let map = SourceMap {
            source: source + range.start..source + range.end,
            display: display + rendered_bytes..display + rendered_bytes + text.len(),
        };
        if let Some(previous) = mappings.last_mut().filter(|previous| {
            previous.source.len() == previous.display.len()
                && map.source.len() == map.display.len()
                && previous.source.end == map.source.start
                && previous.display.end == map.display.start
        }) {
            previous.source.end = map.source.end;
            previous.display.end = map.display.end;
        } else {
            mappings.push(map);
        }
        rendered_bytes += text.len();
    };
    if quoted {
        append("\"", 0..1);
    }
    let mut character = |ch: char, range: Range<usize>| {
        let mut buffer = [0; 4];
        if quoted {
            match ch {
                '"' => append("\\\"", range),
                '\\' => append("\\\\", range),
                '\n' => append("\\n", range),
                '\r' => append("\\r", range),
                '\t' => append("\\t", range),
                ch if ch.is_control() => append(&format!("\\u{:04x}", ch as u32), range),
                ch => append(ch.encode_utf8(&mut buffer), range),
            }
        } else {
            append(ch.encode_utf8(&mut buffer), range);
        }
    };
    if let Some(parsed) = parsed {
        let mut offset = 1;
        for ch in parsed.chars() {
            let length = if ch == '\n' { 2 } else { ch.len_utf8() };
            character(ch, offset..offset + length);
            offset += length;
        }
    } else {
        let mut offset = 1;
        let end = raw.len().saturating_sub(1);
        while offset < end {
            let start = offset;
            let ch = raw[offset..].chars().next().unwrap();
            offset += ch.len_utf8();
            if ch == '\\' && offset < end {
                let escaped = raw.as_bytes()[offset];
                offset += 1;
                if escaped == b'u' {
                    offset = (offset + 4).min(end);
                    if raw
                        .get(start + 2..offset)
                        .and_then(|s| u16::from_str_radix(s, 16).ok())
                        .is_some_and(|code| (0xd800..=0xdbff).contains(&code))
                        && raw[offset..].starts_with("\\u")
                    {
                        offset = (offset + 6).min(end);
                    }
                }
                for ch in decode_string(&format!("\"{}\"", &raw[start..offset])).chars() {
                    character(ch, start..offset);
                }
            } else {
                character(ch, start..offset);
            }
        }
    }
    if quoted {
        append("\"", raw.len() - 1..raw.len());
    }
    if valid && rendered_bytes == rendered.len() {
        mappings
    } else {
        vec![]
    }
}

fn unsupported_controls(text: &str) -> bool {
    text.chars()
        .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
}

fn compact_key(key: &KeyValue) -> (String, Vec<WarningKind>) {
    fn render(key: &KeyValue, warnings: &mut Vec<WarningKind>) -> String {
        match key {
            KeyValue::String(text) => {
                if unsupported_controls(text) {
                    warnings.push(WarningKind::NonStandardStringEscape);
                }
                quote_json(text)
            }
            KeyValue::Number(token) => {
                let (text, warning) = number(token);
                warnings.extend(warning);
                text
            }
            KeyValue::Boolean(value) => value.to_string(),
            KeyValue::Null => "null".into(),
            KeyValue::Array(values) => format!(
                "[{}]",
                values
                    .iter()
                    .map(|value| render(value, warnings))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            KeyValue::Object(entries) => format!(
                "{{{}}}",
                entries
                    .iter()
                    .map(|(key, value)| format!(
                        "{}:{}",
                        render(key, warnings),
                        render(value, warnings)
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
    let mut warnings = vec![];
    let text = render(key, &mut warnings);
    (text, warnings)
}

fn bounded_prefix(text: &str, limit: usize) -> &str {
    let mut end = text.len().min(limit);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn truncate_preview(preview: &mut Preview, limit: usize) {
    let end = bounded_prefix(&preview.text, limit).len();
    preview.text.truncate(end);
    for map in &mut preview.source_map {
        map.display.end = map.display.end.min(end);
    }
    preview
        .source_map
        .retain(|map| map.display.start < map.display.end);
}

fn append_ellipsis(preview: &mut Preview, source: Option<Range<usize>>) {
    const LIMIT: usize = 256;
    const ELLIPSIS_LEN: usize = '…'.len_utf8();
    truncate_preview(preview, LIMIT - ELLIPSIS_LEN);
    let start = preview.text.len();
    preview.text.push('…');
    if let Some(source) = source {
        preview.source_map.push(SourceMap {
            source,
            display: start..preview.text.len(),
        });
    }
}

fn preview_append(preview: &mut Preview, text: &str, source: Option<Range<usize>>) {
    const LIMIT: usize = 256;
    const ELLIPSIS_LEN: usize = '…'.len_utf8();
    let remaining = LIMIT.saturating_sub(preview.text.len());
    if remaining == 0 {
        if !text.is_empty() {
            append_ellipsis(preview, source);
        }
        return;
    }
    let start = preview.text.len();
    let limit = if text.len() > remaining {
        (LIMIT - ELLIPSIS_LEN).saturating_sub(start)
    } else {
        remaining
    };
    let part = bounded_prefix(text, limit);
    preview.text.push_str(part);
    if part.len() < text.len() {
        append_ellipsis(preview, None);
        if let Some(source) = source {
            preview.source_map.push(SourceMap {
                source,
                display: start.min(LIMIT - ELLIPSIS_LEN)..preview.text.len(),
            });
        }
    } else if let Some(source) = source {
        preview.source_map.push(SourceMap {
            source,
            display: start..preview.text.len(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn json(input: &str) -> FlatJson {
        crate::flatjson::parse_top_level_json(input.into()).unwrap()
    }
    fn yaml(input: &str) -> FlatJson {
        crate::flatjson::parse_top_level_yaml(input.into()).unwrap()
    }
    fn text(flat: &FlatJson) -> String {
        Fixture::canonical(flat)
            .lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
    #[test]
    fn retained_standard_fixtures() {
        let fixtures = [
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/primitives.json"),
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/objects.json"),
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/arrays-primitive.json"),
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/arrays-nested.json"),
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/arrays-objects.json"),
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/arrays-tabular.json"),
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/whitespace.json"),
        ];
        let mut checked = 0;
        for fixture in fixtures {
            let fixture = json(fixture);
            let field = |object, name: &str| {
                children(&fixture, object).into_iter().find(|&i| {
                    fixture[i]
                        .key_range
                        .as_ref()
                        .is_some_and(|r| decode_string(&fixture.1[r.clone()]) == name)
                })
            };
            for test in children(&fixture, field(0, "tests").unwrap()) {
                if field(test, "options").is_some() {
                    continue;
                }
                let name = field(test, "name").unwrap();
                let name = decode_string(&fixture.1[fixture[name].range.clone()]);
                // The viewer's explicit order-preservation contract selects lists here.
                if name == "uses field order from first object for tabular headers" {
                    continue;
                }
                let input = field(test, "input").unwrap();
                let expected = field(test, "expected").unwrap();
                assert_eq!(
                    text(&json(&fixture.1[fixture[input].range.clone()])),
                    decode_string(&fixture.1[fixture[expected].range.clone()]),
                    "{name}"
                );
                checked += 1;
            }
        }
        assert!(checked > 80);
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
            ("[]", "[0]:"),
            ("[{},{}]", "[2]:\n  -\n  -"),
            ("[[1,2],[]]", "[2]:\n  - [2]: 1,2\n  - [0]:"),
            (r#"{"e":{},"a":[]}"#, "e:\na[0]:"),
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
    fn sequence_root_body_preview_keeps_source_mapping() {
        let mut flat = json(r#"[{"needle":1},{"other":2}] {}"#);
        let root = flat
            .0
            .iter()
            .enumerate()
            .find(|(_, row)| row.parent.is_nil() && !row.is_closing_of_container())
            .map(|(node, _)| node)
            .unwrap();
        let layout = Fixture::canonical(&flat);
        flat.collapse(root);
        let line = layout
            .project_with_documents(&flat, &HashSet::new())
            .into_iter()
            .find(|line| line.absolute == layout.nodes[root].body_line)
            .unwrap()
            .line;
        let preview = line
            .spans
            .iter()
            .find(|span| span.role == TokenRole::Preview)
            .unwrap();
        let source_start = flat.1.find("needle").unwrap();
        assert!(preview.source.is_some());
        assert!(
            !preview
                .matching_ranges(&(source_start..source_start + 6))
                .is_empty()
        );
    }

    #[test]
    fn duplicate_decoded_keys_keep_identity() {
        let flat = json(r#"{"box":{"a":1,"\u0061":2}}"#);
        let layout = Fixture::canonical(&flat);
        assert_eq!(
            text(&flat),
            "box:\n  a: 1  # WARN Duplicate key\n  a: 2  # WARN Duplicate key"
        );
        assert_eq!(layout.nodes[1].entry_count, 2);
        assert_eq!(layout.nodes[1].descendant_warnings, 2);
        assert_eq!(layout.nodes[2].occurrence, Some(1));
        assert_eq!(layout.nodes[3].occurrence, Some(2));
        assert_eq!(layout.lines[1].owner, 2);
        assert_eq!(layout.lines[2].owner, 3);
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
            "1_000",
            "+1",
            "01",
            "1e4096",
        ] {
            assert_eq!(
                number(input),
                (input.into(), Some(WarningKind::NonCanonicalNumber))
            );
        }
        assert_eq!(number("1e4095").0.len(), 4096);
        assert_eq!(number("1e-4094").0.len(), 4096);
    }
    #[test]
    fn warnings_on_shared_lines_and_source_text() {
        assert_eq!(
            text(&yaml("[1, .inf, .nan]")),
            "[3]: 1,.inf,.nan  # WARN Non-finite number at [1]; Non-finite number at [2]"
        );
        assert_eq!(
            text(&json(
                r##"["# WARN Duplicate key",".inf","true","05","a,b","-x"]"##
            )),
            r##"[6]: "# WARN Duplicate key",".inf","true","05","a,b","-x""##
        );
        let flat = yaml("- a: .inf\n- a: .nan\n");
        assert_eq!(
            text(&flat),
            "[2]{a}:\n  .inf  # WARN Non-finite number at field \"a\"\n  .nan  # WARN Non-finite number at field \"a\""
        );
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
    fn sequence_headers_preserve_roots_and_collapse_each_shape() {
        let flat = json(r#"{"name":"Ada"} 7 {} []"#);
        let roots: Vec<_> = flat
            .0
            .iter()
            .enumerate()
            .filter(|(_, row)| row.parent.is_nil() && !row.is_closing_of_container())
            .map(|(node, _)| node)
            .collect();
        assert_eq!(roots.len(), 4);
        let layout = Fixture::canonical(&flat);
        assert_eq!(layout.lines[0].text, "--- (1 of 4)");
        assert_eq!(layout.lines[1].text, "name: Ada");
        assert_eq!(layout.lines[2].text, "--- (2 of 4)");
        assert_eq!(layout.lines[3].text, "7");
        assert_eq!(layout.lines[4].text, "--- (3 of 4)");
        assert_eq!(layout.lines[5].text, "");
        assert_eq!(layout.lines[6].text, "--- (4 of 4)");
        assert_eq!(layout.lines[7].text, "[0]:");
        assert_eq!(layout.nodes[roots[0]].line, 0);
        assert_eq!(layout.nodes[roots[1]].line, 2);
        assert_eq!(layout.nodes[roots[2]].line, 4);
        assert_eq!(layout.nodes[roots[3]].line, 6);
        assert_eq!(layout.nodes[roots[3]].body_line, 7);
        assert_eq!(layout.nodes[roots[3]].body_extent, 7..8);
        assert!(layout.warnings.is_empty());

        let document_collapsed = roots.iter().copied().collect::<HashSet<_>>();
        let visible = layout.project_with_documents(&flat, &document_collapsed);
        assert_eq!(
            visible
                .iter()
                .map(|line| line.line.text.as_str())
                .collect::<Vec<_>>(),
            vec![
                "--- (1 of 4) name: Ada",
                "--- (2 of 4) 7",
                "--- (3 of 4) {}",
                "--- (4 of 4) []",
            ]
        );
        assert!(
            visible
                .iter()
                .all(|line| line.line.separator && line.line.owner < flat.0.len())
        );
        assert!(
            visible
                .iter()
                .flat_map(|line| line.line.spans.iter())
                .filter(|span| span.role == TokenRole::Preview)
                .all(|span| span.source.is_none())
        );
    }

    #[test]
    fn sequence_root_warning_moves_to_collapsed_header() {
        let flat = yaml("---\n.inf\n---\n7\n");
        let roots: Vec<_> = flat
            .0
            .iter()
            .enumerate()
            .filter(|(_, row)| row.parent.is_nil() && !row.is_closing_of_container())
            .map(|(node, _)| node)
            .collect();
        let layout = Fixture::canonical(&flat);
        assert_eq!(layout.warnings.len(), 1);
        assert_eq!(layout.warnings[0].kind, WarningKind::NonFiniteNumber);
        let document_collapsed = HashSet::from([roots[0]]);
        let header = &layout.project_with_documents(&flat, &document_collapsed)[0].line;
        assert_eq!(header.text, "--- (1 of 2) .inf  # WARN Non-finite number");
        assert!(
            header
                .spans
                .iter()
                .all(|span| span.source.is_none() || span.role != TokenRole::Preview)
        );
    }

    #[test]
    fn sequence_root_array_warnings_keep_element_locators() {
        let flat = yaml("---\n[.inf, 7]\n---\n7\n");
        assert_eq!(
            text(&flat),
            "--- (1 of 2)\n[2]: .inf,7  # WARN Non-finite number at [0]\n--- (2 of 2)\n7"
        );
    }

    #[test]
    fn multiline_sequence_root_array_warnings_keep_element_locators() {
        let flat = yaml("---\n[.inf, 1, 2, 3, 4, 5]\n---\n7\n");
        let layout = Fixture::for_view(&flat, 20, &HashSet::new());
        let warning = layout
            .lines
            .iter()
            .find(|line| line.text.contains("Non-finite number"))
            .unwrap();
        assert!(warning.text.contains("at [0]"));
    }

    #[test]
    fn root_array_body_collapse_keeps_document_header_expanded() {
        let mut flat = json(r#"[{"a":1},{"a":2}] {}"#);
        let roots: Vec<_> = flat
            .0
            .iter()
            .enumerate()
            .filter(|(_, row)| row.parent.is_nil() && !row.is_closing_of_container())
            .map(|(node, _)| node)
            .collect();
        let layout = Fixture::canonical(&flat);
        assert_ne!(
            layout.nodes[roots[0]].body_line,
            layout.nodes[roots[0]].line
        );
        flat.collapse(roots[0]);
        let visible = layout.project_with_documents(&flat, &HashSet::new());
        assert_eq!(visible[0].line.text, "--- (1 of 2)");
        assert_eq!(visible[1].absolute, layout.nodes[roots[0]].body_line);
        assert_eq!(visible[2].absolute, layout.nodes[roots[1]].line);
    }

    #[test]
    fn collapsed_sequence_header_aggregates_nested_duplicate_warnings() {
        let flat = json(r#"{"nested":{"a":1,"a":2}} 7"#);
        let roots: Vec<_> = flat
            .0
            .iter()
            .enumerate()
            .filter(|(_, row)| row.parent.is_nil() && !row.is_closing_of_container())
            .map(|(node, _)| node)
            .collect();
        let nested = children(&flat, roots[0])[0];
        let layout = Fixture::canonical(&flat);
        assert_eq!(layout.nodes[nested].descendant_warnings, 2);
        let visible = layout.project_with_documents(&flat, &HashSet::from([roots[0]]));
        assert_eq!(visible[0].line.owner, roots[0]);
        assert!(
            visible[0]
                .line
                .text
                .ends_with("# WARN Contains 2 hidden warnings")
        );
        assert!(!visible[0].line.text.contains("Multiple document roots"));
    }

    #[test]
    fn json_and_yaml_sequences_have_equivalent_document_layouts() {
        let json_layout = Fixture::canonical(&json(r#"{"a":1} {"b":2}"#));
        let yaml_layout = Fixture::canonical(&yaml("---\na: 1\n---\nb: 2\n"));
        let json_lines: Vec<_> = json_layout
            .lines
            .iter()
            .map(|line| line.text.as_str())
            .collect();
        let yaml_lines: Vec<_> = yaml_layout
            .lines
            .iter()
            .map(|line| line.text.as_str())
            .collect();
        assert_eq!(json_lines, yaml_lines);
        assert_eq!(
            json_lines,
            vec!["--- (1 of 2)", "a: 1", "--- (2 of 2)", "b: 2"]
        );
        assert!(
            json_layout
                .warnings
                .iter()
                .all(|warning| warning.kind != WarningKind::NonStringKey)
        );
        assert!(
            yaml_layout
                .warnings
                .iter()
                .all(|warning| warning.kind != WarningKind::NonStringKey)
        );
    }

    #[test]
    fn expanding_a_document_restores_descendant_collapse_state() {
        let mut flat = json(r#"{"outer":{"x":1,"y":2},"tail":0} {}"#);
        let roots: Vec<_> = flat
            .0
            .iter()
            .enumerate()
            .filter(|(_, row)| row.parent.is_nil() && !row.is_closing_of_container())
            .map(|(node, _)| node)
            .collect();
        let outer = children(&flat, roots[0])[0];
        let layout = Fixture::canonical(&flat);
        flat.collapse(outer);
        let collapsed_document = HashSet::from([roots[0]]);
        let hidden = layout.project_with_documents(&flat, &collapsed_document);
        assert_eq!(hidden[0].line.text, "--- (1 of 2) outer: …; tail: 0");
        assert_eq!(hidden[1].line.text, "--- (2 of 2)");
        let reopened = layout.project_with_documents(&flat, &HashSet::new());
        assert!(reopened.iter().any(|visible| {
            visible.line.owner == outer && visible.line.text.starts_with("outer:")
        }));
        let outer_line = reopened
            .iter()
            .find(|visible| visible.line.owner == outer)
            .unwrap();
        assert!(outer_line.line.text.contains("x: 1; y: 2"));
    }

    #[test]
    fn collapsed_scalar_document_preview_is_bounded() {
        let input = format!("\"{}\" 0", "x".repeat(10_000));
        let flat = json(&input);
        let roots: Vec<_> = flat
            .0
            .iter()
            .enumerate()
            .filter(|(_, row)| row.parent.is_nil() && !row.is_closing_of_container())
            .map(|(node, _)| node)
            .collect();
        let layout = Fixture::canonical(&flat);
        let visible = layout.project_with_documents(&flat, &HashSet::from([roots[0]]));
        assert!(visible[0].line.text.len() <= "--- (1 of 2) ".len() + 256);
        assert!(visible[0].line.text.ends_with('…'));
    }

    #[test]
    fn collapsed_array_document_preview_keeps_toon_header() {
        let flat = json("[10,20] {}");
        let roots: Vec<_> = flat
            .0
            .iter()
            .enumerate()
            .filter(|(_, row)| row.parent.is_nil() && !row.is_closing_of_container())
            .map(|(node, _)| node)
            .collect();
        let layout = Fixture::canonical(&flat);
        let visible = layout.project_with_documents(&flat, &HashSet::from([roots[0]]));

        assert_eq!(visible[0].line.text, "--- (1 of 2) [2]: 10,20");
    }

    #[test]
    fn collapsed_array_document_preview_stays_bounded_after_header() {
        let input = format!(
            "[{}] 0",
            (0..200)
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        let flat = json(&input);
        let roots: Vec<_> = flat
            .0
            .iter()
            .enumerate()
            .filter(|(_, row)| row.parent.is_nil() && !row.is_closing_of_container())
            .map(|(node, _)| node)
            .collect();
        let layout = Fixture::canonical(&flat);
        let visible = layout.project_with_documents(&flat, &HashSet::from([roots[0]]));

        assert!(visible[0].line.text.len() <= "--- (1 of 2) ".len() + 256);
        assert!(visible[0].line.text.ends_with('…'));
    }

    #[test]
    fn shared_header_mapping_and_table_rows_remain_visible() {
        let mut flat = yaml("- a: .inf\n  b: 2\n- a: .nan\n  b: 4\n");
        let layout = Fixture::canonical(&flat);
        let rows = children(&flat, 0);
        let fields = children(&flat, rows[1]);
        let visible = layout.layout.project_with_documents(&flat, &HashSet::new());
        let header = layout.layout.render(
            &flat,
            layout.layout.visible_line(&flat, &visible, 0).unwrap(),
            fields[0],
        );
        let key_span = header
            .spans
            .iter()
            .find(|span| {
                span.node == fields[0]
                    && matches!(span.role, TokenRole::Key | TokenRole::FieldDefinition)
            })
            .unwrap();
        assert_eq!(key_span.source, flat[fields[0]].key_range);
        assert_eq!(&header.text[key_span.range.clone()], "a");
        assert!(
            layout.lines[2]
                .spans
                .iter()
                .any(|span| span.node == fields[0] && span.role == TokenRole::Warning)
        );
        flat.collapse(rows[1]);
        let collapsed = layout.project(&flat);
        assert_eq!(collapsed[2].absolute, 2);
        assert!(!layout.nodes[rows[1]].collapsible);
        assert!(layout.nodes[0].collapsible);
        assert_eq!(collapsed[2].line.text, layout.lines[2].text);
        assert!(
            collapsed[2]
                .line
                .spans
                .iter()
                .any(|span| span.node == fields[0])
        );
        flat.expand(rows[1]);
        assert_eq!(layout.project(&flat)[2].line.text, layout.lines[2].text);
    }
    #[test]
    fn escaped_locators_and_semantic_warning_counts() {
        let flat = yaml("- \"a\\nb\": .inf\n- \"a\\nb\": .nan\n");
        let layout = Fixture::canonical(&flat);
        assert!(layout.lines[1].text.ends_with("at field \"a\\nb\""));
        assert_eq!(layout.nodes[0].descendant_warnings, 2);
        assert_eq!(layout.warnings.len(), 2);
        let mut flat = json(r#"{"a":{"x":1,"x":2},"a":0}"#);
        let layout = Fixture::canonical(&flat);
        flat.collapse(1);
        let visible = layout.project(&flat);
        assert!(
            visible[0]
                .line
                .text
                .ends_with("# WARN Duplicate key; Contains 2 hidden warnings")
        );
        assert_eq!(layout.nodes[0].descendant_warnings, 4);
    }
    #[test]
    fn root_anchors_and_bounded_preview() {
        let mut flat = json("{\"a\":{\"b\":1},\"c\":2}");
        let layout = Fixture::canonical(&flat);
        assert_eq!(layout.nodes[0].line, 0);
        assert!(!layout.nodes[0].collapsible);
        flat.collapse(1);
        let visible = layout.project(&flat);
        assert_eq!(
            visible.iter().map(|line| line.absolute).collect::<Vec<_>>(),
            vec![0, 2]
        );
        // Exercise the parsed-model boundary without involving the tokenizer's
        // unrelated large non-ASCII string behavior.
        let mut flat = json(r#"["x"]"#);
        flat.1 = format!("[\"{}\"]", "界".repeat(100_000));
        let length = flat.1.len();
        flat[0].range = 0..length;
        flat[1].range = 1..length - 1;
        flat[2].range = length - 1..length;
        let layout = Fixture::for_view(&flat, 120, &HashSet::new());
        flat.collapse(0);
        let visible = layout.project(&flat);
        assert!(visible[0].line.text.len() < 280);
        assert!(visible[0].line.text.ends_with('…'));
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
    fn mappings_and_collapse_restore() {
        let mut flat = json(r#"{"tags":[1,2],"rows":[{"a":3},{"a":4}],"obj":{"x":{"y":5}}}"#);
        let layout = Fixture::canonical(&flat);
        assert_eq!(layout.nodes[2].line, layout.nodes[3].line);
        flat.collapse(1);
        let visible = layout.project(&flat);
        assert_eq!(visible[0].line.text, "tags[2]: 1,2");
        assert!(
            visible[0]
                .line
                .spans
                .iter()
                .any(|s| s.role == TokenRole::Number)
        );
        assert!(
            !visible[0]
                .line
                .spans
                .iter()
                .any(|s| s.role == TokenRole::Preview)
        );
        flat.expand(1);
        assert_eq!(layout.project(&flat)[0].line.text, layout.lines[0].text);
        let mut dup = json(r#"{"a":{"b":{"x":1,"x":2}}}"#);
        let l = Fixture::canonical(&dup);
        dup.collapse(2);
        dup.collapse(1);
        assert!(
            l.project(&dup)[0]
                .line
                .text
                .contains("Contains 2 hidden warnings")
        );
        dup.expand(1);
        assert!(dup[2].is_collapsed());
        assert_eq!(l.project(&dup).len(), 2);
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
    fn list_first_field_inline_warnings_keep_element_locators() {
        let flat = yaml("- vals: [.inf, .nan]\n  nested: {}\n");
        assert_eq!(
            text(&flat),
            "[1]:\n  - vals[2]: .inf,.nan  # WARN Non-finite number at [0]; Non-finite number at [1]\n    nested:"
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
    fn collapsed_table_header_retains_field_style_source_and_identity() {
        let mut flat = json(r#"[{"a":1},{"a":2}]"#);
        let layout = Fixture::canonical(&flat);
        let rows = children(&flat, 0);
        let first = children(&flat, rows[0])[0];
        let selected = children(&flat, rows[1])[0];
        flat.collapse(0);
        let projected = layout.layout.project_with_documents(&flat, &HashSet::new());
        let header = layout.layout.render(
            &flat,
            layout.layout.visible_line(&flat, &projected, 0).unwrap(),
            selected,
        );
        let key = header
            .spans
            .iter()
            .find(|span| span.node == selected && span.role == TokenRole::FieldDefinition)
            .unwrap();
        assert_eq!(key.source, flat[selected].key_range);
        assert_eq!(&header.text[key.range.clone()], "a");
        let (node, source) = crate::lineprinter::hit_test(&header, key.range.start);
        assert_eq!(
            node, first,
            "mouse targeting is independent of the selected alias"
        );
        assert_eq!(
            source,
            flat[first].key_range.as_ref().map(|range| range.start)
        );
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

    #[test]
    fn unsupported_control_escapes_are_warned_but_literal_escape_text_is_not() {
        let flat = json(r#"{"control":"\u0001","literal":"\\u0001","line":"\n"}"#);
        let layout = Fixture::canonical(&flat);
        assert_eq!(
            layout.lines[0].text,
            r#"control: "\u0001"  # WARN Non-standard string escape"#
        );
        assert_eq!(
            layout
                .warnings
                .iter()
                .filter(|warning| warning.kind == WarningKind::NonStandardStringEscape)
                .count(),
            1
        );
        assert!(!text(&flat).chars().any(|ch| ch == '\u{1}'));
        let key = json(r#"{"\u0001":1}"#);
        assert!(text(&key).contains("# WARN Non-standard string escape"));
        let key = yaml("? [\"\\u0001\"]\n: value\n");
        assert!(text(&key).contains("Non-string key; Non-standard string escape"));
    }
    #[test]
    fn mixed_warning_kinds_follow_node_order_and_hidden_summary_is_last() {
        let flat = yaml(r#"["\u0001", .inf, 1e1000000]"#);
        let layout = Fixture::canonical(&flat);
        assert_eq!(
            layout.lines[0].text,
            r#"[3]: "\u0001",.inf,1e1000000  # WARN Non-standard string escape at [0]; Non-finite number at [1]; Non-canonical number at [2]"#
        );
        assert_eq!(
            layout
                .warnings
                .iter()
                .map(|warning| warning.kind)
                .collect::<Vec<_>>(),
            vec![
                WarningKind::NonStandardStringEscape,
                WarningKind::NonFiniteNumber,
                WarningKind::NonCanonicalNumber
            ]
        );
        let mut flat = yaml(".inf: {a: .inf}");
        let layout = Fixture::canonical(&flat);
        flat.collapse(1);
        assert!(
            layout.project(&flat)[0]
                .line
                .text
                .ends_with("# WARN Non-finite number; Non-string key; Contains 1 hidden warnings")
        );
    }
    #[test]
    fn selected_nested_root_renders_at_baseline_without_excluded_lines() {
        let flat = json(r#"{"outer":{"kept":1},"tail":{"excluded":2}}"#);
        let selected = children(&flat, 0)[0];
        let layout = Fixture::for_view_with_roots(&flat, 120, &HashSet::new(), &[selected]);
        let text = layout
            .lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(text, vec!["kept: 1"]);
        assert_eq!(layout.nodes[selected].line, 0);
        assert!(
            layout
                .lines
                .iter()
                .flat_map(|line| line.spans.iter())
                .all(|span| { layout.active_root_for(span.node) == Some(selected) })
        );
    }

    #[test]
    fn selected_root_omits_key_warnings_even_when_collapsed() {
        let mut flat = json(r#"{"k\u0001":[1,2,3,4,5,6,7]}"#);
        let root = children(&flat, 0)[0];
        let layout = Fixture::for_view_with_roots(&flat, 120, &HashSet::new(), &[root]);
        assert!(layout.warnings.is_empty());
        flat.collapse(root);
        assert!(!layout.project(&flat)[0].line.text.contains("# WARN"));

        let flat = json(r#"{"k\u0001":"v\u0001"}"#);
        let root = children(&flat, 0)[0];
        let layout = Fixture::for_view_with_roots(&flat, 120, &HashSet::new(), &[root]);
        assert_eq!(layout.warnings.len(), 1);
        assert_eq!(
            layout.warnings[0].kind,
            WarningKind::NonStandardStringEscape
        );
        assert!(layout.lines[0].text.contains("# WARN"));
    }

    #[test]
    fn selected_array_element_warning_has_no_excluded_parent_locator() {
        let flat = yaml("- .inf\n- 1\n");
        let root = children(&flat, 0)[0];
        let layout = Fixture::for_view_with_roots(&flat, 120, &HashSet::new(), &[root]);
        assert_eq!(layout.warnings[0].kind, WarningKind::NonFiniteNumber);
        assert!(!layout.lines[0].text.contains("at [0]"));
    }

    #[test]
    fn selected_root_warnings_and_sequence_numbers_are_filtered() {
        let flat = yaml("- a: .inf\n- b: .nan\n");
        let rows = children(&flat, 0);
        let layout = Fixture::for_view_with_roots(&flat, 120, &HashSet::new(), &[rows[0]]);
        assert_eq!(layout.lines.len(), 1);
        assert!(layout.lines[0].text.contains("Non-finite number"));
        assert!(!layout.lines[0].text.contains("b:"));
        let stream = json(r#"{"a":1} {"b":2} {"c":3}"#);
        let roots = crate::path_filter::document_roots(&stream);
        let filtered =
            Fixture::for_view_with_roots(&stream, 120, &HashSet::new(), &[roots[2], roots[0]]);
        assert_eq!(filtered.lines[0].text, "--- (1 of 2)");
        assert_eq!(filtered.lines[2].text, "--- (2 of 2)");
    }

    #[test]
    fn selected_first_field_is_not_merged_with_its_excluded_parent() {
        for (input, header) in [
            (r#"{"items":[{"a":1},{"a":2}]}"#, "[2]{a}:"),
            (r#"{"value":1}"#, "1"),
        ] {
            let flat = json(input);
            let root = children(&flat, 0)[0];
            let layout = layout::Layout::new(&flat, &[root], 120, true, &HashSet::new());
            let projection = layout.project_with_documents(&flat, &HashSet::new());
            let row = layout.visible_line(&flat, &projection, 0).unwrap();
            assert_eq!(row.owner, root);
            assert_eq!(layout.render(&flat, row, root).text, header);
        }
    }

    #[test]
    fn filtered_out_of_order_subtrees_keep_exact_addresses_across_reflow_and_collapse() {
        let input = format!(
            r#"{{"skip":[{}],"first":{{"x":[1,2],"after":7}},"last":[{{"id":1}},{{"id":2}}]}}"#,
            vec!["0"; 512].join(",")
        );
        let mut flat = json(&input);
        let fields = children(&flat, 0);
        let roots = [fields[2], fields[1]];
        let x = children(&flat, fields[1])[0];
        let after = children(&flat, fields[1])[1];
        let mut layout = layout::Layout::new(&flat, &roots, 120, true, &HashSet::new());
        let projection = layout.project_with_documents(&flat, &HashSet::new());
        let lines: Vec<_> = (0..projection.len())
            .map(|i| {
                let row = layout.visible_line(&flat, &projection, i).unwrap();
                layout.render(&flat, row, row.owner).text
            })
            .collect();
        assert_eq!(
            lines,
            [
                "--- (1 of 2)",
                "[2]{id}:",
                "  1",
                "  2",
                "--- (2 of 2)",
                "x[2]: 1,2",
                "after: 7",
            ]
        );
        assert_eq!(layout.node(&flat, after).line, 6);
        layout.reflow(&flat, 9, true, &HashSet::new());
        assert_eq!(layout.line_count(), 9);
        assert_eq!(layout.node(&flat, after).line, 8);
        flat.collapse(0); // An excluded ancestor must not collapse selected roots.
        flat.collapse(x);
        let projected = layout.project_with_documents(&flat, &HashSet::from([roots[0]]));
        let addresses: Vec<_> = (0..projected.len())
            .map(|i| layout.visible_line(&flat, &projected, i).unwrap().absolute)
            .collect();
        assert_eq!(addresses, [0, 4, 5, 8]);
        assert_eq!(layout.row(&flat, 8).unwrap().owner, after);
        layout.reflow(&flat, 120, true, &HashSet::new());
        assert_eq!(layout.node(&flat, after).line, 6);
    }

    #[test]
    fn repeated_key_shapes_recheck_changed_keys_and_scalar_warnings() {
        let mut input = format!("[{},", vec![r#"{"a":1,"b":2}"#; 128].join(","));
        input.push_str(r#"{"a":1},{"a":2,"b":3,"a":4},"#);
        input.push_str(r#"{"\u0061":1e1000000,"b":3},"#);
        input.push_str(r#"{"a":4,"\u0061":5},"#);
        input.push_str(r#"{"\u0001":6,"b":7},"#);
        input.push_str(r#"{"\u0001":8,"b":9}]"#);
        let flat = json(&input);
        let layout = Fixture::for_view(&flat, 120, &HashSet::new());
        assert_eq!(layout.lines[0].text, "[134]:");
        assert_eq!(layout.nodes[0].descendant_warnings, 7);
        let kinds: Vec<_> = layout.warnings.iter().map(|warning| warning.kind).collect();
        assert_eq!(
            kinds,
            [
                WarningKind::DuplicateKey,
                WarningKind::DuplicateKey,
                WarningKind::NonCanonicalNumber,
                WarningKind::DuplicateKey,
                WarningKind::DuplicateKey,
                WarningKind::NonStandardStringEscape,
                WarningKind::NonStandardStringEscape,
            ]
        );
    }
}
