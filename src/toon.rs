use crate::flatjson::{self, FlatJson, Index, OptionIndex, Value};

lazy_static::lazy_static! {
    static ref JSON_NUMBER: regex::Regex = regex::Regex::new(r"^-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?$").unwrap();
}

#[derive(Default, Clone, Copy)]
pub struct EncodeOptions {
    _private: (),
}

pub fn encode_document(
    document: &FlatJson,
    options: EncodeOptions,
) -> Result<String, ToonDiagnostic> {
    if document.0.is_empty() || document[0].next_sibling.is_some() {
        return Err(ToonDiagnostic::new(
            ToonErrorKind::UnsupportedMultiRoot,
            "TOON output requires exactly one root",
        ));
    }
    encode_value(document, 0, options)
}

pub fn encode_value(
    document: &FlatJson,
    index: Index,
    _options: EncodeOptions,
) -> Result<String, ToonDiagnostic> {
    let index = if document[index].is_closing_of_container() {
        document[index].pair_index().unwrap()
    } else {
        index
    };
    let end = if document[index].is_opening_of_container() {
        document[index].pair_index().unwrap()
    } else {
        index
    };
    for row in &document.0[index..=end] {
        if (row.is_opening_of_container()
            || matches!(row.value, Value::EmptyArray | Value::EmptyObject))
            && row.depth - document[index].depth + 1 > 256
        {
            return Err(ToonDiagnostic::new(
                ToonErrorKind::UnsafeNesting,
                "TOON output supports at most 256 nested containers",
            ));
        }
    }
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("toon-encode".to_owned())
            .stack_size(16 * 1024 * 1024)
            .spawn_scoped(scope, || {
                let value = export_value(document, index, "$")?;
                toon_format::encode_default(&value).map_err(Into::into)
            })
            .map_err(|e| ToonDiagnostic::new(ToonErrorKind::InternalConversion, e.to_string()))?
            .join()
            .map_err(|_| ToonDiagnostic::new(ToonErrorKind::InternalConversion, "codec panicked"))?
    })
}

fn export_value(
    document: &FlatJson,
    index: Index,
    path: &str,
) -> Result<serde_json::Value, ToonDiagnostic> {
    let row = &document[index];
    if row.is_opening_of_container() {
        let mut object = serde_json::Map::new();
        let mut array = Vec::new();
        let mut child = row.first_child();
        while let OptionIndex::Index(index) = child {
            let child_row = &document[index];
            if row.is_array() {
                array.push(export_value(
                    document,
                    index,
                    &format!("{}[{}]", path, child_row.index_in_parent),
                )?);
            } else {
                let key_text = &document.1[child_row.key_range.clone().unwrap()];
                let key: String = serde_json::from_str(key_text).map_err(|_| {
                    ToonDiagnostic::new(
                        ToonErrorKind::UnsupportedValue,
                        format!(
                            "{} entry {}: object key is not a string",
                            path,
                            child_row.index_in_parent + 1
                        ),
                    )
                })?;
                let child_path = format!("{}[{}]", path, serde_json::to_string(&key).unwrap());
                if object.contains_key(&key) {
                    return Err(ToonDiagnostic::new(
                        ToonErrorKind::DuplicateKey,
                        format!(
                            "{} entry {}: duplicate key",
                            child_path,
                            child_row.index_in_parent + 1
                        ),
                    ));
                }
                object.insert(key, export_value(document, index, &child_path)?);
            }
            child = child_row.next_sibling;
        }
        Ok(if row.is_array() {
            serde_json::Value::Array(array)
        } else {
            serde_json::Value::Object(object)
        })
    } else if matches!(row.value, Value::Number) {
        let text = &document.1[row.range.clone()];
        if !JSON_NUMBER.is_match(text) {
            return Err(ToonDiagnostic::new(
                ToonErrorKind::UnsupportedValue,
                format!("{}: not a finite JSON number", path),
            ));
        }
        let number = toon_format::utils::number::checked_json_number(text).map_err(|_| {
            ToonDiagnostic::new(
                ToonErrorKind::UnsupportedNumber,
                format!("{}: cannot preserve {}", path, text),
            )
        })?;
        let value = serde_json::Value::Number(number);
        let codec_value = toon_format::types::JsonValue::from(&value);
        if let toon_format::types::JsonValue::Number(number) = codec_value {
            let canonical = toon_format::utils::number::format_canonical_number(&number);
            if toon_format::utils::number::normalized_decimal(text)
                != toon_format::utils::number::normalized_decimal(&canonical)
            {
                return Err(ToonDiagnostic::new(
                    ToonErrorKind::UnsupportedNumber,
                    format!("{}: encoder would change {}", path, text),
                ));
            }
        }
        Ok(value)
    } else {
        serde_json::from_str(&document.1[row.range.clone()]).map_err(|e| {
            ToonDiagnostic::new(ToonErrorKind::UnsupportedValue, format!("{}: {}", path, e))
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToonErrorKind {
    Syntax,
    Indentation,
    CountMismatch,
    RowWidthMismatch,
    InvalidEscape,
    DuplicateKey,
    DuplicateField,
    UnsupportedNumber,
    UnsupportedValue,
    UnsupportedMultiRoot,
    UnsafeNesting,
    InternalConversion,
}

#[derive(Debug)]
pub struct ToonDiagnostic {
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub kind: ToonErrorKind,
    pub message: String,
}

impl ToonDiagnostic {
    fn new(kind: ToonErrorKind, message: impl Into<String>) -> Self {
        Self {
            line: None,
            column: None,
            kind,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ToonDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TOON {:?}", self.kind)?;
        if let Some(line) = self.line {
            write!(f, " at line {}", line)?;
            if let Some(column) = self.column {
                write!(f, ", column {}", column)?;
            }
        }
        write!(f, ": {}", self.message)
    }
}

impl From<toon_format::ToonError> for ToonDiagnostic {
    fn from(error: toon_format::ToonError) -> Self {
        use toon_format::{types::ProfileErrorKind, ToonError};
        let (kind, line, column) = match &error {
            ToonError::ProfileError {
                kind, line, column, ..
            } => {
                let kind = match kind {
                    ProfileErrorKind::CountMismatch => ToonErrorKind::CountMismatch,
                    ProfileErrorKind::DuplicateKey => ToonErrorKind::DuplicateKey,
                    ProfileErrorKind::DuplicateField => ToonErrorKind::DuplicateField,
                    ProfileErrorKind::UnsupportedNumber => ToonErrorKind::UnsupportedNumber,
                    ProfileErrorKind::UnsafeNesting => ToonErrorKind::UnsafeNesting,
                };
                (kind, *line, *column)
            }
            ToonError::LengthMismatch { .. } => (ToonErrorKind::CountMismatch, None, None),
            ToonError::ParseError {
                message,
                line,
                column,
                ..
            } => {
                let kind = if message.starts_with("Invalid indentation")
                    || message.starts_with("Tabs are not allowed")
                {
                    ToonErrorKind::Indentation
                } else if message.starts_with("Invalid escape")
                    || message.starts_with("Unterminated string")
                {
                    ToonErrorKind::InvalidEscape
                } else if message.starts_with("Tabular row") && message.contains("values") {
                    ToonErrorKind::RowWidthMismatch
                } else if message.starts_with("Array length mismatch") {
                    ToonErrorKind::CountMismatch
                } else {
                    ToonErrorKind::Syntax
                };
                (kind, Some(*line), Some(*column))
            }
            ToonError::InvalidStructure(message)
                if message.starts_with("Maximum nesting depth") =>
            {
                (ToonErrorKind::UnsafeNesting, None, None)
            }
            _ => (ToonErrorKind::Syntax, None, None),
        };
        let mut result = Self::new(kind, error.to_string());
        result.line = line;
        result.column = column;
        result
    }
}

pub fn parse(input: &str) -> Result<FlatJson, ToonDiagnostic> {
    // The released recursive codec has large stack frames in debug builds.
    // A fixed stack budget supports the same bounded depth in all builds.
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("toon-decode".to_owned())
            .stack_size(16 * 1024 * 1024)
            .spawn_scoped(scope, || parse_on_codec_stack(input))
            .map_err(|e| ToonDiagnostic::new(ToonErrorKind::InternalConversion, e.to_string()))?
            .join()
            .map_err(|_| ToonDiagnostic::new(ToonErrorKind::InternalConversion, "codec panicked"))?
    })
}

fn parse_on_codec_stack(input: &str) -> Result<FlatJson, ToonDiagnostic> {
    if input.starts_with('\u{feff}') {
        let mut error = ToonDiagnostic::new(ToonErrorKind::Syntax, "initial BOM is not supported");
        error.line = Some(1);
        error.column = Some(1);
        return Err(error);
    }
    let input = input.replace("\r\n", "\n");
    let value: serde_json::Value = if input.trim_matches(&[' ', '\n'][..]).is_empty() {
        serde_json::Value::Object(serde_json::Map::new())
    } else {
        toon_format::decode_strict(&input)?
    };
    let json = value.to_string();
    flatjson::parse_top_level_json(json)
        .map_err(|e| ToonDiagnostic::new(ToonErrorKind::InternalConversion, e))
}

#[cfg(test)]
#[path = "toon_fixtures.rs"]
mod fixtures;

#[cfg(test)]
mod tests {
    #[test]
    fn empty_object_arrays_have_a_valid_list_round_trip() {
        let doc = crate::flatjson::parse_top_level_json("[{},{}]".to_owned()).unwrap();
        let encoded = super::encode_document(&doc, super::EncodeOptions::default()).unwrap();
        assert_eq!(encoded, "[2]:\n  -\n  -");
        assert_eq!(super::parse(&encoded).unwrap().1, doc.1);
    }

    #[test]
    fn diagnostic_categories_do_not_depend_on_key_contents() {
        let error = super::ToonDiagnostic::from(toon_format::ToonError::parse_error(
            1,
            1,
            "Expected Colon, got String(\" rows, but got \" )",
        ));
        assert_eq!(error.kind, super::ToonErrorKind::Syntax);
        for name in [
            "UnsupportedNumber",
            "DuplicateField",
            "depth",
            "indent",
            "quote",
        ] {
            let error = super::parse(&format!("{name}: 1\n{name}: 2")).unwrap_err();
            assert_eq!(error.kind, super::ToonErrorKind::DuplicateKey);
            assert_eq!(error.line, Some(2));
        }
    }
    #[test]
    fn input_depth_counts_the_object_around_a_named_array() {
        let nested_arrays = |count: usize| {
            let mut input = "a[1]:\n".to_owned();
            for level in 1..count {
                input.push_str(&"  ".repeat(level));
                input.push_str("- [1]:\n");
            }
            input.push_str(&"  ".repeat(count));
            input.push_str("- 0");
            input
        };
        assert!(super::parse(&nested_arrays(255)).is_ok());
        assert_eq!(
            super::parse(&nested_arrays(256)).unwrap_err().kind,
            super::ToonErrorKind::UnsafeNesting
        );
    }
    #[cfg(debug_assertions)]
    #[test]
    fn blank_line_scanner_work_grows_linearly() {
        let visits = |lines| {
            let input = format!("{}value: 1", "\n".repeat(lines));
            let mut parser = toon_format::decode::parser::Parser::new(
                &input,
                toon_format::DecodeOptions::default(),
            )
            .unwrap();
            let value = parser.parse().unwrap();
            assert_eq!(value["value"], 1);
            parser.scanner_visits()
        };
        let small = visits(100_000);
        let large = visits(200_000);
        assert!(
            large * 10 <= small * 22,
            "{} versus {} visits",
            small,
            large
        );
    }

    #[test]
    fn decoded_navigation_search_and_paths_match_json_with_escaped_unicode() {
        use crate::search::{JumpDirection, SearchDirection, SearchState};
        use crate::viewer::{Action, JsonViewer, Mode};
        let input = "\"é.key\":\n  rows[2]: \"line\\nAda\",雪";
        let json = r#"{"é.key":{"rows":["line\nAda","雪"]}}"#;
        let mut toon = JsonViewer::new(super::parse(input).unwrap(), Mode::Data);
        let mut reference = JsonViewer::new(
            crate::flatjson::parse_top_level_json(json.to_owned()).unwrap(),
            Mode::Data,
        );
        for action in [
            Action::MoveDown(1),
            Action::ToggleCollapsed,
            Action::ToggleCollapsed,
            Action::MoveDown(2),
            Action::ToggleMode,
            Action::FocusParent,
            Action::FocusMatchingPair,
        ] {
            toon.perform_action(action);
            reference.perform_action(action);
            assert_eq!(toon.focused_row, reference.focused_row);
            assert_eq!(toon.mode, reference.mode);
            assert_eq!(
                toon.flatjson.pretty_printed_value(toon.focused_row),
                reference
                    .flatjson
                    .pretty_printed_value(reference.focused_row)
            );
            assert_eq!(
                toon.flatjson
                    .build_path_to_node(crate::flatjson::PathType::Bracket, toon.focused_row),
                reference
                    .flatjson
                    .build_path_to_node(crate::flatjson::PathType::Bracket, reference.focused_row)
            );
        }
        let mut a = SearchState::initialize_search(
            "雪".to_owned(),
            &toon.flatjson.1,
            SearchDirection::Forward,
        )
        .unwrap();
        let mut b = SearchState::initialize_search(
            "雪".to_owned(),
            &reference.flatjson.1,
            SearchDirection::Forward,
        )
        .unwrap();
        assert_eq!(
            a.jump_to_match(0, &toon.flatjson, JumpDirection::Next, 1),
            b.jump_to_match(0, &reference.flatjson, JumpDirection::Next, 1)
        );
    }
    #[test]
    fn strict_diagnostics_are_located_and_huge_counts_fail_safely() {
        assert_eq!(
            super::parse("[1]{a,b}:\n  1").unwrap_err().kind,
            super::ToonErrorKind::RowWidthMismatch
        );
        let duplicate = super::parse("a: 1\na: 2").unwrap_err();
        assert_eq!(duplicate.kind, super::ToonErrorKind::DuplicateKey);
        assert_eq!(duplicate.line, Some(2));
        let duplicate = super::parse("[1]{a,a}:\n  1,2").unwrap_err();
        assert_eq!(duplicate.kind, super::ToonErrorKind::DuplicateField);
        assert_eq!(duplicate.line, Some(1));
        for input in [
            "[18446744073709551615]: x",
            "[2]{a}:\n  1\n\n  2",
            "a:\n\tb: 1",
            "[#2]: a,b",
        ] {
            assert!(super::parse(input).is_err(), "{}", input);
        }
    }
    #[test]
    fn decoding_never_reclassifies_out_of_range_numbers_as_strings() {
        for input in [
            "18446744073709551616".to_owned(),
            "9".repeat(1025),
            "value: 1e1025".to_owned(),
        ] {
            assert!(super::parse(&input).is_err(), "{}", input);
        }
        for input in ["05", "007", "18446744073709551616 words"] {
            let doc = super::parse(input).unwrap();
            assert!(doc[0].is_string());
        }
    }
    #[test]
    fn unsupported_yaml_does_not_block_a_supported_focused_value() {
        let doc =
            crate::flatjson::parse_top_level_yaml("bad: .inf\ngood: 42\n".to_owned()).unwrap();
        let error = super::encode_document(&doc, super::EncodeOptions::default()).unwrap_err();
        assert_eq!(error.kind, super::ToonErrorKind::UnsupportedValue);
        assert!(error.message.contains("[\"bad\"]"));
        assert_eq!(
            super::encode_value(&doc, 2, super::EncodeOptions::default()).unwrap(),
            "42"
        );
        let doc = crate::flatjson::parse_top_level_yaml("? [a, b]\n: 1\n".to_owned()).unwrap();
        let error = super::encode_document(&doc, super::EncodeOptions::default()).unwrap_err();
        assert_eq!(error.kind, super::ToonErrorKind::UnsupportedValue);
        assert!(error.message.contains("$ entry 1"));
    }
    #[test]
    fn export_depth_is_relative_to_the_selected_subtree() {
        let input = format!("{}0{}", "[".repeat(257), "]".repeat(257));
        let doc = crate::flatjson::parse_top_level_json(input).unwrap();
        assert_eq!(
            super::encode_document(&doc, super::EncodeOptions::default())
                .unwrap_err()
                .kind,
            super::ToonErrorKind::UnsafeNesting
        );
        assert!(super::encode_value(&doc, 1, super::EncodeOptions::default()).is_ok());
    }
    #[test]
    fn focused_export_includes_collapsed_children_and_normalizes_closing_rows() {
        let mut doc =
            crate::flatjson::parse_top_level_json(r#"{"a":[1,2]} 42"#.to_owned()).unwrap();
        assert_eq!(
            super::encode_document(&doc, super::EncodeOptions::default())
                .unwrap_err()
                .kind,
            super::ToonErrorKind::UnsupportedMultiRoot
        );
        doc.collapse(1);
        for index in [1, doc[1].pair_index().unwrap()] {
            assert_eq!(
                super::encode_value(&doc, index, super::EncodeOptions::default()).unwrap(),
                "[2]: 1,2"
            );
        }
    }
    #[test]
    fn export_preserves_exact_numbers_or_rejects_them() {
        for number in ["0.123456789012345678901", "1e1025", "9223372036854775808.1"] {
            let doc = crate::flatjson::parse_top_level_json(number.to_owned()).unwrap();
            assert_eq!(
                super::encode_document(&doc, super::EncodeOptions::default())
                    .unwrap_err()
                    .kind,
                super::ToonErrorKind::UnsupportedNumber
            );
        }
        for (number, output) in [
            ("18446744073709551615", "18446744073709551615"),
            ("-9223372036854775808", "-9223372036854775808"),
            ("0.1", "0.1"),
            ("-0", "0"),
        ] {
            let doc = crate::flatjson::parse_top_level_json(number.to_owned()).unwrap();
            assert_eq!(
                super::encode_document(&doc, super::EncodeOptions::default()).unwrap(),
                output
            );
        }
    }
    #[test]
    fn export_rejects_duplicate_decoded_keys_without_losing_an_entry() {
        let doc =
            crate::flatjson::parse_top_level_json(r#"{"a":1,"\u0061":2}"#.to_owned()).unwrap();
        let error = super::encode_document(&doc, super::EncodeOptions::default()).unwrap_err();
        assert_eq!(error.kind, super::ToonErrorKind::DuplicateKey);
        assert!(error.message.contains("[\"a\"]"));
        assert!(error.message.contains("entry 2"));
    }
    #[test]
    fn encodes_json_as_canonical_tabular_toon() {
        let doc = crate::flatjson::parse_top_level_json(
            r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Lin"}]}"#.to_owned(),
        )
        .unwrap();
        assert_eq!(
            super::encode_document(&doc, super::EncodeOptions::default()).unwrap(),
            "users[2]{id,name}:\n  1,Ada\n  2,Lin"
        );
    }
    #[test]
    fn enforces_container_depth_and_scans_long_blank_runs() {
        fn nested(count: usize) -> String {
            let mut input = String::new();
            for depth in 0..count - 1 {
                input.push_str(&"  ".repeat(depth));
                input.push_str("a:\n");
            }
            input.push_str(&"  ".repeat(count - 1));
            input.push_str("value: 1");
            input
        }
        assert!(super::parse(&nested(256)).is_ok());
        assert!(super::parse(&nested(257)).is_err());
        assert!(super::parse(&format!("{}value: 1", "\n".repeat(100_000))).is_ok());
    }
    #[test]
    fn accepts_empty_input_and_crlf_but_rejects_bom() {
        for input in ["", " \n  \n"] {
            assert_eq!(super::parse(input).unwrap().1, "{}");
        }
        assert_eq!(
            super::parse("name: Ada\r\n\r\n").unwrap().1,
            r#"{ "name": "Ada" }"#
        );
        assert!(super::parse("\u{feff}name: Ada").is_err());
    }
    #[test]
    fn rejects_duplicate_decoded_names() {
        for input in [
            "a: 1\na: 2",
            "\"a.b\": 1\na.b: 2",
            "[1]{a,a}:\n  1,2",
            "outer:\n  a: 1\n  \"a\": 2",
        ] {
            assert!(super::parse(input).is_err(), "{}", input);
        }
    }
    #[test]
    fn rejects_decimal_rounding_before_accepting_a_value() {
        for input in [
            "0.123456789012345678901",
            "a: 0.123456789012345678901",
            "[1]: 0.123456789012345678901",
        ] {
            assert!(super::parse(input).is_err(), "{}", input);
        }
    }
    #[test]
    fn reads_a_toon_object_as_existing_json_rows() {
        let actual = super::parse("name: Ada\nscores[2]: 1,2").unwrap();
        let expected =
            crate::flatjson::parse_top_level_json(r#"{"name":"Ada","scores":[1,2]}"#.to_owned())
                .unwrap();
        assert_eq!(actual.1, expected.1);
        assert_eq!(format!("{:?}", actual.0), format!("{:?}", expected.0));
    }
}
