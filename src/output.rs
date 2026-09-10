//! Whole-document machine output, independent of terminal presentation.
use std::fmt::Write;

use crate::flatjson::{self, FlatJson, KeyValue, Value};
use crate::jsonstringunescaper::unsafe_unescape_json_string;
use crate::options::{DataFormat, OutputFormat};

pub fn serialize(
    input: String,
    input_format: DataFormat,
    format: OutputFormat,
) -> Result<String, String> {
    #[cfg(not(feature = "toon"))]
    if format == OutputFormat::Toon {
        return Err("TOON output is unavailable in this build; rebuild with --features toon, or use -o json or -o yaml.".to_owned());
    }
    let document = match input_format {
        DataFormat::Json => flatjson::parse_top_level_json(input),
        DataFormat::Yaml => flatjson::parse_top_level_yaml(input),
        #[cfg(feature = "toon")]
        DataFormat::Toon => crate::toon::parse(&input).map_err(|e| e.to_string()),
    }
    .map_err(|e| format!("Unable to parse input: {e}"))?;
    if format == OutputFormat::Toon {
        #[cfg(feature = "toon")]
        {
            // YAML's backing text is for display and does not JSON-escape all
            // decoded strings. Supply safe JSON text to the existing adapter.
            let document = if input_format == DataFormat::Yaml && !document.0.is_empty() {
                let json = encode(&document, OutputFormat::Json)
                    .map_err(|e| format!("Unable to serialize TOON output: {e}"))?;
                flatjson::parse_top_level_json(json)
                    .map_err(|e| format!("Unable to serialize TOON output: {e}"))?
            } else {
                document
            };
            return crate::toon::encode_document(&document, crate::toon::EncodeOptions::default())
                .map_err(|e| format!("Unable to serialize output: {e}"));
        }
        #[cfg(not(feature = "toon"))]
        unreachable!();
    }
    if format == OutputFormat::Json && input_format == DataFormat::Json {
        return Ok(document.pretty_printed());
    }
    encode(&document, format).map_err(|e| format!("Unable to serialize output: {e}"))
}

fn encode(document: &FlatJson, format: OutputFormat) -> Result<String, String> {
    let yaml = format == OutputFormat::Yaml;
    let mut output = String::new();
    // Flat rows include closing delimiters, so traversal needs no recursive stack.
    for row in &document.0 {
        let closing = row.is_closing_of_container();
        if !closing {
            if row.depth == 0 && yaml {
                output.push_str("---\n");
            }
            output.push_str(&"  ".repeat(row.depth));
            if let Some(range) = &row.key_range {
                match &row.key_value {
                    Some(KeyValue::String(key)) => quote_mapping_key(&mut output, key, yaml),
                    Some(key) if yaml => {
                        output.push_str("? ");
                        key_yaml(&mut output, key);
                        output.push(' ');
                    }
                    Some(_) => return Err("JSON output requires string mapping keys".to_owned()),
                    None => {
                        quote_mapping_key(&mut output, &decode(&document.1[range.clone()])?, yaml)
                    }
                }
                output.push_str(": ");
            }
        }
        match row.value {
            Value::OpenContainer { .. } => {
                output.push_str(if row.is_array() { "[\n" } else { "{\n" });
                continue;
            }
            Value::CloseContainer { .. } => {
                output.push_str(&"  ".repeat(row.depth));
                output.push(if row.is_array() { ']' } else { '}' });
            }
            Value::EmptyObject => output.push_str("{}"),
            Value::EmptyArray => output.push_str("[]"),
            Value::String => {
                let text = match &row.string_value {
                    Some(text) => text.clone(),
                    None => decode(&document.1[row.range.clone()])?,
                };
                quote(&mut output, &text, yaml);
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
        let node = if closing {
            &document[row.pair_index().unwrap()]
        } else {
            row
        };
        if row.depth > 0 && node.next_sibling.is_some() {
            output.push(',');
        }
        output.push('\n');
    }
    Ok(output)
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

fn json_number(text: &str) -> Result<String, String> {
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
