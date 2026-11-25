//! Whole-document machine output, independent of terminal presentation.
use std::fmt::Write;
use std::ops::RangeInclusive;

use crate::flatjson::{self, ContainerType, FlatJson, KeyValue, Row, Value};
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
        OutputFormat::Json => encode_roots(document, roots, Layout::PrettyJson),
        OutputFormat::Yaml => encode_roots(document, roots, Layout::Yaml),
        OutputFormat::Toon => {
            crate::toon::encode_roots(document, roots, crate::toon::EncodeOptions::default())
                .map_err(|error| error.to_string())
        }
    }
}

/// Encode each selected root as one compact JSON record terminated by LF.
pub fn serialize_jsonl_roots(document: &FlatJson, roots: &[usize]) -> Result<String, String> {
    encode_roots(document, roots, Layout::CompactJson)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Layout {
    PrettyJson,
    CompactJson,
    Yaml,
}

fn encode_roots(document: &FlatJson, roots: &[usize], layout: Layout) -> Result<String, String> {
    let mut encoder = Encoder {
        document,
        layout,
        output: String::new(),
    };
    for &root in roots {
        encoder.write_root(root)?;
    }
    Ok(encoder.output)
}

struct Encoder<'a> {
    document: &'a FlatJson,
    layout: Layout,
    output: String,
}

impl Encoder<'_> {
    fn write_root(&mut self, root: usize) -> Result<(), String> {
        let range = self.document.subtree_range(root);
        if self.layout == Layout::Yaml {
            self.output.push_str("---\n");
        }

        // Closing delimiters are rows too, so no traversal stack is needed.
        for index in range.clone() {
            self.write_row(index, &range)?;
        }
        if !self.pretty() {
            self.output.push('\n');
        }
        Ok(())
    }

    fn write_row(&mut self, index: usize, range: &RangeInclusive<usize>) -> Result<(), String> {
        let base_depth = self.document[*range.start()].depth;
        let is_root = index == *range.start();
        let is_last = index == *range.end();
        let row = &self.document[index];
        // A closing delimiter inherits its opener's indentation and siblings.
        let node = match row.value {
            Value::CloseContainer { open_index, .. } => &self.document[open_index],
            _ => row,
        };
        if self.pretty() {
            for _ in 0..node.depth.saturating_sub(base_depth) {
                self.output.push_str("  ");
            }
        }

        if !is_root && !row.is_closing_of_container() {
            self.write_key(index)?;
        }
        self.write_value(index)?;
        if needs_comma(row, node, is_root, is_last) {
            self.output.push(',');
        }
        if self.pretty() {
            self.output.push('\n');
        }
        Ok(())
    }

    fn write_key(&mut self, index: usize) -> Result<(), String> {
        let Some(range) = &self.document[index].key_range else {
            return Ok(());
        };
        let yaml = self.layout == Layout::Yaml;
        match self.document.key_value(index) {
            Some(KeyValue::String(key)) => quote_mapping_key(&mut self.output, key, yaml),
            Some(key) if yaml => {
                self.output.push_str("? ");
                key_yaml(&mut self.output, key);
                self.output.push(' ');
            }
            Some(_) => return Err("JSON output requires string mapping keys".to_owned()),
            None => {
                let raw = &self.document.1[range.clone()];
                if yaml {
                    quote_mapping_key(&mut self.output, &decode(raw)?, true);
                } else {
                    self.output.push_str(raw);
                }
            }
        }
        self.output.push(':');
        if self.pretty() {
            self.output.push(' ');
        }
        Ok(())
    }

    fn write_value(&mut self, index: usize) -> Result<(), String> {
        let row = &self.document[index];
        let raw = || &self.document.1[row.range.clone()];
        match row.value {
            Value::OpenContainer { container_type, .. } => {
                self.output.push_str(container_type.open_str());
            }
            Value::CloseContainer { container_type, .. } => {
                self.output.push_str(container_type.close_str());
            }
            Value::EmptyObject => self.output.push_str("{}"),
            Value::EmptyArray => self.output.push_str("[]"),
            Value::String => self.write_string(index, raw())?,
            Value::Number if self.layout != Layout::Yaml => {
                self.output.push_str(&json_number(raw())?);
            }
            Value::Number | Value::Boolean | Value::Null => self.output.push_str(raw()),
        }
        Ok(())
    }

    fn write_string(&mut self, index: usize, raw: &str) -> Result<(), String> {
        let yaml = self.layout == Layout::Yaml;
        match self.document.string_value(index) {
            Some(text) => quote(&mut self.output, text, yaml),
            None if yaml => quote(&mut self.output, &decode(raw)?, true),
            None => self.output.push_str(raw),
        }
        Ok(())
    }

    fn pretty(&self) -> bool {
        self.layout != Layout::CompactJson
    }
}

fn needs_comma(row: &Row, node: &Row, is_root: bool, is_last: bool) -> bool {
    !row.is_opening_of_container() && !is_root && !is_last && node.next_sibling.is_some()
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
            ch if needs_unicode_escape(ch, yaml) => {
                write!(output, "\\u{:04x}", ch as u32).unwrap();
            }
            ch => output.push(ch),
        }
    }
    output.push('"');
}

fn needs_unicode_escape(ch: char, yaml: bool) -> bool {
    if ch < ' ' {
        return true;
    }
    yaml && (ch.is_control() || matches!(ch, '\u{2028}' | '\u{2029}'))
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
    let sign = if &parts[1] == "-" { "-" } else { "" };
    let integer = digits_or_zero(integer.trim_start_matches('0'));
    let exponent = parts.get(4).map_or("", |part| part.as_str());
    Ok(match fraction {
        Some(fraction) => format!("{sign}{integer}.{}{exponent}", digits_or_zero(fraction)),
        None => format!("{sign}{integer}{exponent}"),
    })
}

fn digits_or_zero(digits: &str) -> &str {
    if digits.is_empty() { "0" } else { digits }
}

fn key_yaml(output: &mut String, key: &KeyValue) {
    match key {
        KeyValue::String(text) => quote(output, text, true),
        KeyValue::Number(text) => output.push_str(text),
        KeyValue::Boolean(value) => output.push_str(if *value { "true" } else { "false" }),
        KeyValue::Null => output.push_str("null"),
        KeyValue::Array(items) => {
            write_key_collection(output, ContainerType::Array, items, key_yaml)
        }
        KeyValue::Object(entries) => {
            write_key_collection(output, ContainerType::Object, entries, write_key_entry);
        }
    }
}

fn write_key_collection<T>(
    output: &mut String,
    container_type: ContainerType,
    items: &[T],
    write_item: fn(&mut String, &T),
) {
    output.push_str(container_type.open_str());
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        write_item(output, item);
    }
    output.push_str(container_type.close_str());
}

fn write_key_entry(output: &mut String, (key, value): &(KeyValue, KeyValue)) {
    output.push_str("? ");
    key_yaml(output, key);
    output.push_str(" : ");
    key_yaml(output, value);
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
        let good = (0..document.0.len())
            .find(|&index| matches!(document.key_value(index), Some(KeyValue::String(key)) if key == "good"))
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

    #[test]
    fn compact_records_preserve_nested_values_duplicate_keys_and_number_spelling() {
        let document = parse_input(
            r#"{"selected":{"a":0.123456789012345678901,"a":[{"s":"a\nb\u0000\t\r\u001f"},2]},"ignored":9} [1,{"ok":false}] null [] {}"#
                .to_owned(),
            DataFormat::Json,
        )
        .unwrap();
        let roots = crate::path_filter::document_roots(&document);
        assert_eq!(
            serialize_jsonl_roots(&document, &[1, roots[1], roots[2], roots[3], roots[4]]).unwrap(),
            concat!(
                r#"{"a":0.123456789012345678901,"a":[{"s":"a\nb\u0000\t\r\u001f"},2]}"#,
                "\n",
                r#"[1,{"ok":false}]"#,
                "\nnull\n[]\n{}\n"
            )
        );
        assert_eq!(serialize_jsonl_roots(&document, &[]).unwrap(), "");
    }

    #[test]
    fn compact_yaml_values_escape_decoded_controls_and_normalize_numbers() {
        let document = parse_input(
            r#"selected: ["a\nb\t\r\u0000\u001f", 1.]"#.to_owned(),
            DataFormat::Yaml,
        )
        .unwrap();
        assert_eq!(
            serialize_jsonl_roots(&document, &[1]).unwrap(),
            "[\"a\\nb\\t\\r\\u0000\\u001f\",1.0]\n"
        );
    }

    #[test]
    fn compact_conversion_rejects_only_invalid_selected_yaml_values() {
        let document = parse_input(
            "good: [1, 2]\nnonfinite: .inf\n? [bad]\n: 3\n".to_owned(),
            DataFormat::Yaml,
        )
        .unwrap();
        let find = |name| {
            (0..document.0.len())
                .find(|&index| matches!(document.key_value(index), Some(KeyValue::String(key)) if key == name))
                .unwrap()
        };
        assert_eq!(
            serialize_jsonl_roots(&document, &[find("good")]).unwrap(),
            "[1,2]\n"
        );
        assert!(serialize_jsonl_roots(&document, &[find("good"), find("nonfinite")]).is_err());
        let typed_key_document =
            parse_input("good: [1, 2]\n? [bad]\n: 3\n".to_owned(), DataFormat::Yaml).unwrap();
        assert!(serialize_jsonl_roots(&typed_key_document, &[0]).is_err());
    }
}
