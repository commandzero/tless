//! TOON scalar and key spelling used for both measurement and rendering.
use super::{WarningKind, number};
use crate::flatjson::KeyValue;

pub(super) fn quote_json(value: &str) -> String {
    let mut text = String::with_capacity(value.len() + 2);
    quote_json_into(value, &mut text);
    text
}
pub(super) fn quote_json_into(value: &str, text: &mut String) {
    use std::fmt::Write;
    text.clear();
    text.push('"');
    for ch in value.chars() {
        if let Some(literal) = escaped_literal(ch) {
            text.push_str(literal);
        } else if ch.is_control() {
            write!(text, "\\u{:04x}", ch as u32).unwrap();
        } else {
            text.push(ch);
        }
    }
    text.push('"');
}
pub(super) fn escaped_literal(ch: char) -> Option<&'static str> {
    match ch {
        '"' => Some("\\\""),
        '\\' => Some("\\\\"),
        '\n' => Some("\\n"),
        '\r' => Some("\\r"),
        '\t' => Some("\\t"),
        _ => None,
    }
}

pub(super) fn decode_string(raw: &str) -> String {
    let inner = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(raw);
    // YAML's existing flattened strings can contain literal backslashes. Only
    // call the JSON unescaper when its syntactic precondition is satisfied.
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' && !valid_escape(&mut chars) {
            return inner.to_owned();
        }
    }
    crate::jsonstringunescaper::unsafe_unescape_json_string(inner)
        .unwrap_or_else(|_| inner.to_owned())
}

fn valid_escape(chars: &mut std::str::Chars<'_>) -> bool {
    match chars.next() {
        Some('u') => (0..4).all(|_| chars.next().is_some_and(|c| c.is_ascii_hexdigit())),
        Some('"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't') => true,
        _ => false,
    }
}

pub(super) fn quote_key(key: &str) -> String {
    if bare_key(key) {
        key.to_owned()
    } else {
        quote_json(key)
    }
}

pub(super) fn bare_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

pub(super) fn value_needs_quotes(value: &str) -> bool {
    lazy_static::lazy_static! { static ref NUMERIC: regex::Regex=regex::Regex::new(r"^[+-]?[0-9]+(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?$").unwrap(); }
    if reserved_value(value) || NUMERIC.is_match(value) {
        return true;
    }
    value
        .chars()
        .any(|c| c.is_control() || ":\"\\[]{},".contains(c))
}

fn reserved_value(value: &str) -> bool {
    if value.starts_with([' ', '\t', '-', '#']) || value.ends_with([' ', '\t']) {
        return true;
    }
    matches!(
        value,
        "" | "true" | "false" | "null" | ".inf" | "+.inf" | "-.inf" | ".nan"
    )
}

pub(super) fn compact_key(key: &KeyValue) -> (String, Vec<WarningKind>) {
    fn render(key: &KeyValue, warnings: &mut Vec<WarningKind>) -> String {
        match key {
            KeyValue::String(text) => quote_json(text),
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
