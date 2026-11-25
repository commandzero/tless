//! Path formatting for clipboard, query, and status display.
use std::borrow::Cow;
use std::fmt::Write;

use super::{FlatJson, Index, KeyValue, PathType, Row};
use crate::jsonstringunescaper::unsafe_unescape_json_string;
use crate::lineprinter::JS_IDENTIFIER;

impl FlatJson {
    pub fn build_path_to_node(&self, path_type: PathType, index: Index) -> Result<String, String> {
        let mut buf = String::new();

        // A copied top-level value is the complete-document filter path.
        if self[index].parent.is_nil() {
            match path_type {
                PathType::Dot | PathType::Query => return Ok(".".to_owned()),
                PathType::Bracket => {
                    return Err("Cannot build path to top-level element".to_string());
                }
                PathType::DotWithTopLevelIndex => {}
            }
        }

        self.build_path_to_node_impl(path_type, index, &mut buf)?;
        Ok(buf)
    }

    fn build_path_to_node_impl(
        &self,
        path_type: PathType,
        index: Index,
        buf: &mut String,
    ) -> Result<(), String> {
        let row = &self[index];
        if row.is_closing_of_container() {
            return self.build_path_to_node_impl(path_type, row.pair_index().unwrap(), buf);
        }

        if let Some(parent) = row.parent.as_option() {
            self.build_path_to_node_impl(path_type, parent, buf)?;
        }

        if row.key_range.is_some() {
            return self.write_path_key(index, path_type, buf);
        }
        write_path_index(buf, row, index, path_type).map_err(|error| error.to_string())
    }

    fn write_path_key(
        &self,
        index: Index,
        path_type: PathType,
        buf: &mut String,
    ) -> Result<(), String> {
        let range = self[index]
            .key_range
            .clone()
            .expect("object member has a key");
        let raw = &self.1[range];
        if raw.starts_with('[') {
            // YAML's typed keys keep the original bracketed spelling.
            if path_type == PathType::Query {
                return Err("Path to node contains non-string keys not supported in JSON".into());
            }
            buf.push_str(raw);
            return Ok(());
        }
        if path_type == PathType::Dot {
            return self.write_copied_key(index, buf);
        }

        let key = &raw[1..raw.len() - 1];
        let part = match path_type {
            PathType::Bracket => write!(buf, "[{raw}]"),
            _ if JS_IDENTIFIER.is_match(key) => write!(buf, ".{key}"),
            PathType::Query if self[index].depth == 1 => write!(buf, ".[{raw}]"),
            _ => write!(buf, "[{raw}]"),
        };
        part.map_err(|error| error.to_string())
    }

    fn write_copied_key(&self, index: Index, buf: &mut String) -> Result<(), String> {
        // A copied path must round-trip the decoded key through the path filter.
        let key = self
            .decoded_string_key(index)?
            .ok_or_else(|| "Unable to decode string object key".to_owned())?;
        if JS_IDENTIFIER.is_match(&key) {
            write!(buf, ".{key}").map_err(|error| error.to_string())
        } else {
            let quoted = serde_json::to_string(&key)
                .map_err(|error| format!("Unable to escape object key: {error}"))?;
            write!(buf, "[{quoted}]").map_err(|error| error.to_string())
        }
    }

    pub fn decoded_string_key(&self, index: Index) -> Result<Option<Cow<'_, str>>, String> {
        let row = &self[index];
        match self.key_value(index) {
            Some(KeyValue::String(text)) => return Ok(Some(Cow::Borrowed(text))),
            Some(_) => return Ok(None),
            None => {}
        }
        let Some(range) = &row.key_range else {
            return Ok(None);
        };
        let raw = &self.1[range.start + 1..range.end - 1];
        if raw.as_bytes().contains(&b'\\') {
            unsafe_unescape_json_string(raw)
                .map(Cow::Owned)
                .map(Some)
                .map_err(|error| format!("Unable to decode object key: {error}"))
        } else {
            Ok(Some(Cow::Borrowed(raw)))
        }
    }
}

fn write_path_index(
    buf: &mut String,
    row: &Row,
    index: Index,
    path_type: PathType,
) -> std::fmt::Result {
    if row.parent.is_nil() {
        // In a stream only the status path includes the root's ordinal.
        return match path_type {
            PathType::DotWithTopLevelIndex if index != 0 || row.next_sibling.is_some() => {
                write!(buf, "[{}]", row.index_in_parent)
            }
            _ => Ok(()),
        };
    }

    match path_type {
        PathType::Query if row.depth == 1 => buf.write_str(".[]"),
        PathType::Query => buf.write_str("[]"),
        _ => write!(buf, "[{}]", row.index_in_parent),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flatjson::{parse_top_level_json, parse_top_level_yaml};

    #[test]
    fn test_root_object_build_path_to_node() {
        const ROOT_OBJECT: &str = r#"{
            "non js key": 1,
            "plain_key": [
                {},
                {
                    "nested": 5,
                },
            ],
        }"#;

        let fj = parse_top_level_json(ROOT_OBJECT.to_owned()).unwrap();

        assert_root_paths(&fj, 0, "");

        let path = r#"["non js key"]"#;
        let paths = (path, path, r#".["non js key"]"#, path);
        assert_paths_to_node(&fj, 1, paths);

        let nested_paths = (
            ".plain_key[1].nested",
            r#"["plain_key"][1]["nested"]"#,
            ".plain_key[].nested",
            ".plain_key[1].nested",
        );
        assert_paths_to_node(&fj, 5, nested_paths);
    }

    #[test]
    fn test_root_array_build_path_to_node() {
        const ROOT_ARRAY: &str = r#"[
            1,
            {
                "nested": {
                    "more nested": 4,
                },
            },
        ]"#;

        let fj = parse_top_level_json(ROOT_ARRAY.to_owned()).unwrap();

        assert_root_paths(&fj, 0, "");

        let paths = ("[0]", "[0]", ".[]", "[0]");
        assert_paths_to_node(&fj, 1, paths);

        let nested_paths = (
            r#"[1].nested["more nested"]"#,
            r#"[1]["nested"]["more nested"]"#,
            r#".[].nested["more nested"]"#,
            r#"[1].nested["more nested"]"#,
        );
        assert_paths_to_node(&fj, 4, nested_paths);
    }

    #[test]
    fn test_multi_top_level_build_path_to_node() {
        const MULTI_TOP_LEVEL: &str = r#"[
            {
                "nested": [
                    3,
                ],
            },
        ]
        {
            "plain_key": [
                {
                    "nested": 10,
                },
            ],
        }"#;

        let fj = parse_top_level_json(MULTI_TOP_LEVEL.to_owned()).unwrap();

        assert_root_paths(&fj, 0, "[0]");
        assert_root_paths(&fj, 7, "[1]");

        let paths = (
            "[0].nested[0]",
            r#"[0]["nested"][0]"#,
            ".[].nested[]",
            "[0][0].nested[0]",
        );
        assert_paths_to_node(&fj, 3, paths);

        let paths = (
            ".plain_key[0].nested",
            r#"["plain_key"][0]["nested"]"#,
            ".plain_key[].nested",
            "[1].plain_key[0].nested",
        );
        assert_paths_to_node(&fj, 10, paths);
    }

    #[test]
    fn test_build_path_to_node_yaml_non_string_key() {
        use PathType::*;

        const YAML: &str = r#"{
            [1, 1]: 1,
        }"#;
        let fj = parse_top_level_yaml(YAML.to_owned()).unwrap();
        assert_eq!("[[1, 1]]", fj.build_path_to_node(Dot, 1).unwrap());
        assert_eq!("[[1, 1]]", fj.build_path_to_node(Bracket, 1).unwrap());
        assert!(fj.build_path_to_node(Query, 1).is_err());
    }

    #[test]
    fn test_yp_escapes_yaml_string_keys() {
        use PathType::Dot;

        let flat = parse_top_level_yaml("---\n\"a\\\"b\\\\c\": value\n".to_owned()).unwrap();
        let formatted = flat.build_path_to_node(Dot, 1).unwrap();
        let path = crate::path_filter::PathFilter::parse(&formatted).unwrap();
        assert_eq!(vec![1], path.resolve(&flat).unwrap());
    }

    #[track_caller]
    fn assert_paths_to_node(fj: &FlatJson, index: Index, paths: (&str, &str, &str, &str)) {
        use PathType::*;

        let dot = fj.build_path_to_node(Dot, index).unwrap();
        let bracket = fj.build_path_to_node(Bracket, index).unwrap();
        let query = fj.build_path_to_node(Query, index).unwrap();
        let dot_top_level = fj.build_path_to_node(DotWithTopLevelIndex, index).unwrap();

        assert_eq!(
            paths,
            (
                dot.as_str(),
                bracket.as_str(),
                query.as_str(),
                dot_top_level.as_str()
            )
        );
    }

    #[track_caller]
    fn assert_root_paths(flat: &FlatJson, index: Index, status: &str) {
        use PathType::*;
        for (path, expected) in [(Dot, "."), (Query, "."), (DotWithTopLevelIndex, status)] {
            assert_eq!(flat.build_path_to_node(path, index).unwrap(), expected);
        }
        assert!(flat.build_path_to_node(Bracket, index).is_err());
    }
}
