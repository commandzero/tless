//! Whole-document machine output, independent of terminal presentation.
use std::fmt::Write;

use crate::flatjson::{self, FlatJson, KeyValue, Value};
use crate::jsonstringunescaper::unsafe_unescape_json_string;
use crate::options::{DataFormat, OutputFormat};

pub fn parse_input(input: String, input_format: DataFormat) -> Result<FlatJson, String> {
    let parsed = match input_format {
        DataFormat::Json => flatjson::parse_top_level_json(input),
        DataFormat::Yaml => flatjson::parse_top_level_yaml(input),
        DataFormat::Toon => crate::toon::parse(&input).map_err(|error| error.to_string()),
    };
    parsed.map_err(|error| format!("Unable to parse input: {error}"))
}

/// Serialize the requested nodes as standalone document roots.
///
/// The rows and source text remain owned by `document`: selected roots are only
/// traversal entry points, so duplicate keys, ordering, and number spellings
/// are retained without copying a second document model.
pub fn serialize_roots(
    document: &FlatJson,
    format: OutputFormat,
    roots: &[usize],
) -> Result<String, String> {
    match format {
        OutputFormat::Json => encode_roots(document, roots, false),
        OutputFormat::Yaml => encode_roots(document, roots, true),
        OutputFormat::Toon => {
            crate::toon::encode_roots(document, roots, crate::toon::EncodeOptions::default())
                .map_err(|error| error.to_string())
        }
    }
}

fn encode_roots(document: &FlatJson, roots: &[usize], yaml: bool) -> Result<String, String> {
    let mut output = String::new();
    for &root in roots {
        let range = document.subtree_range(root);
        let (start, end) = (*range.start(), *range.end());
        if yaml {
            output.push_str("---\n");
        }
        encode_root(document, start, end, yaml, &mut output)?;
    }
    Ok(output)
}

fn encode_root(
    document: &FlatJson,
    start: usize,
    end: usize,
    yaml: bool,
    output: &mut String,
) -> Result<(), String> {
    let base_depth = document[start].depth;
    // Flat rows include closing delimiters, so this remains iterative and does
    // not allocate a recursive traversal stack.
    for index in start..=end {
        let row = &document[index];
        let closing = row.is_closing_of_container();
        let is_root = index == start;
        // A closing delimiter belongs at its opening value's indentation.
        let node = if closing {
            &document[row.pair_index().unwrap()]
        } else {
            row
        };
        let relative_depth = node.depth.saturating_sub(base_depth);
        for _ in 0..relative_depth {
            output.push_str("  ");
        }

        if !closing {
            // The selected node is a standalone root: omit only its owning key.
            if !is_root {
                if let Some(range) = &row.key_range {
                    match &row.key_value {
                        Some(KeyValue::String(key)) => quote_mapping_key(output, key, yaml),
                        Some(key) if yaml => {
                            output.push_str("? ");
                            key_yaml(output, key);
                            output.push(' ');
                        }
                        Some(_) => {
                            return Err("JSON output requires string mapping keys".to_owned());
                        }
                        None => {
                            if yaml {
                                quote_mapping_key(
                                    output,
                                    &decode(&document.1[range.clone()])?,
                                    yaml,
                                );
                            } else {
                                output.push_str(&document.1[range.clone()]);
                            }
                        }
                    }
                    output.push_str(": ");
                }
            }
        }

        match row.value {
            Value::OpenContainer { .. } => {
                output.push_str(if row.is_array() { "[\n" } else { "{\n" });
                continue;
            }
            Value::CloseContainer { .. } => {
                output.push(if row.is_array() { ']' } else { '}' });
            }
            Value::EmptyObject => output.push_str("{}"),
            Value::EmptyArray => output.push_str("[]"),
            Value::String => {
                if !yaml && row.string_value.is_none() {
                    output.push_str(&document.1[row.range.clone()]);
                } else {
                    let text = match &row.string_value {
                        Some(text) => text.clone(),
                        None => decode(&document.1[row.range.clone()])?,
                    };
                    quote(output, &text, yaml);
                }
            }
            Value::Number => {
                let text = &document.1[row.range.clone()];
                if yaml {
                    output.push_str(text);
                } else {
                    output.push_str(&json_number(text)?);
                }
            }
            Value::Boolean | Value::Null => output.push_str(&document.1[row.range.clone()]),
        }

        if index != end && !is_root && node.next_sibling.is_some() {
            output.push(',');
        }
        output.push('\n');
    }
    Ok(())
}

fn decode(raw: &str) -> Result<String, String> {
    unsafe_unescape_json_string(&raw[1..raw.len() - 1]).map_err(|e| e.to_string())
}

fn quote_mapping_key(output: &mut String, text: &str, yaml: bool) {
    let start = output.len();
    quote(output, text, yaml);
    // YAML implicit keys are limited to 1024 source characters, including
    // quotes and escape sequences. Explicit keys have no such length limit.
    if yaml && output[start..].chars().count() > 1024 {
        output.insert_str(start, "? ");
        output.push(' ');
    }
}

fn quote(output: &mut String, text: &str, yaml: bool) {
    output.push('"');
    for ch in text.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch < ' '
                || (yaml && (ch.is_control() || matches!(ch, '\u{2028}' | '\u{2029}'))) =>
            {
                write!(output, "\\u{:04x}", ch as u32).unwrap();
            }
            ch => output.push(ch),
        }
    }
    output.push('"');
}

pub(crate) fn json_number(text: &str) -> Result<String, String> {
    lazy_static::lazy_static! {
        static ref NUMBER: regex::Regex = regex::Regex::new(r"^-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?$").unwrap();
        static ref YAML_DECIMAL: regex::Regex = regex::Regex::new(r"^([+-]?)([0-9]*)(?:\.([0-9]*))?([eE][+-]?[0-9]+)?$").unwrap();
    }
    if NUMBER.is_match(text) {
        return Ok(text.to_owned());
    }
    // Normalize YAML decimal syntax without rounding through floating point.
    let parts = YAML_DECIMAL
        .captures(text)
        .ok_or_else(|| "JSON output requires finite JSON numbers".to_owned())?;
    let integer = &parts[2];
    let fraction = parts.get(3).map(|part| part.as_str());
    if integer.is_empty() && fraction.unwrap_or("").is_empty() {
        return Err("JSON output requires finite JSON numbers".to_owned());
    }
    let mut result = String::new();
    if &parts[1] == "-" {
        result.push('-');
    }
    let integer = integer.trim_start_matches('0');
    result.push_str(if integer.is_empty() { "0" } else { integer });
    if let Some(fraction) = fraction {
        result.push('.');
        result.push_str(if fraction.is_empty() { "0" } else { fraction });
    }
    if let Some(exponent) = parts.get(4) {
        result.push_str(exponent.as_str());
    }
    Ok(result)
}

fn key_yaml(output: &mut String, key: &KeyValue) {
    match key {
        KeyValue::String(text) => quote(output, text, true),
        KeyValue::Number(text) => output.push_str(text),
        KeyValue::Boolean(value) => output.push_str(if *value { "true" } else { "false" }),
        KeyValue::Null => output.push_str("null"),
        KeyValue::Array(items) => {
            output.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                key_yaml(output, item);
            }
            output.push(']');
        }
        KeyValue::Object(entries) => {
            output.push('{');
            for (i, (key, value)) in entries.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                output.push_str("? ");
                key_yaml(output, key);
                output.push_str(" : ");
                key_yaml(output, value);
            }
            output.push('}');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_stream_roots_keep_json_tokens_and_yaml_framing() {
        let document = parse_input(
            r#"{"a":0.123456789012345678901,"a":1e1000000} {"a":2}"#.to_owned(),
            DataFormat::Json,
        )
        .unwrap();
        assert_eq!(
            serialize_roots(&document, OutputFormat::Json, &[1, 2, 5]).unwrap(),
            "0.123456789012345678901\n1e1000000\n2\n"
        );
        assert_eq!(
            serialize_roots(&document, OutputFormat::Yaml, &[1, 2, 5]).unwrap(),
            "---\n0.123456789012345678901\n---\n1e1000000\n---\n2\n"
        );
    }

    #[test]
    fn selected_yaml_root_does_not_validate_excluded_typed_keys() {
        let document =
            parse_input("good: 42\n? [bad]\n: .inf\n".to_owned(), DataFormat::Yaml).unwrap();
        let good = document
            .0
            .iter()
            .position(|row| matches!(row.key_value.as_ref(), Some(KeyValue::String(key)) if key == "good"))
            .unwrap();
        assert_eq!(
            serialize_roots(&document, OutputFormat::Json, &[good]).unwrap(),
            "42\n"
        );
    }

    #[test]
    fn scalar_and_empty_roots_are_standalone_values() {
        let document = parse_input("null {}".to_owned(), DataFormat::Json).unwrap();
        assert_eq!(
            serialize_roots(&document, OutputFormat::Json, &[0]).unwrap(),
            "null\n"
        );
        assert_eq!(
            serialize_roots(&document, OutputFormat::Json, &[1]).unwrap(),
            "{}\n"
        );
        assert_eq!(
            serialize_roots(&document, OutputFormat::Yaml, &[1]).unwrap(),
            "---\n{}\n"
        );
    }
}
