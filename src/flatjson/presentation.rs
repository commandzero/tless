//! Pretty-print a selected value while preserving its source spelling.
use std::fmt::{self, Write};

use super::{FlatJson, Index, Value};

impl FlatJson {
    pub fn pretty_printed_value(&self, value_index: Index) -> Result<String, fmt::Error> {
        let row = &self[value_index];
        let Some(container) = row.value.container_type() else {
            return Ok(self.1[row.range.clone()].to_string());
        };

        let range = self.subtree_range(value_index);
        let mut buf = String::new();
        writeln!(buf, "{}", container.open_str())?;
        for index in range.start() + 1..*range.end() {
            self.write_pretty_row(&mut buf, index, row.depth)?;
        }
        writeln!(buf, "{}", container.close_str())?;
        Ok(buf)
    }

    fn write_pretty_row(&self, buf: &mut String, index: Index, depth_offset: usize) -> fmt::Result {
        let row = &self[index];
        for _ in depth_offset..row.depth {
            buf.push_str("  ");
        }
        if let Some(range) = &row.key_range {
            write!(buf, "{}: ", &self.1[range.clone()])?;
        }

        match row.value {
            Value::OpenContainer { container_type, .. } => buf.push_str(container_type.open_str()),
            Value::CloseContainer { container_type, .. } => {
                buf.push_str(container_type.close_str())
            }
            _ => buf.push_str(&self.1[row.range.clone()]),
        }
        if self.pretty_row_has_comma(index) {
            buf.push(',');
        }
        buf.push('\n');
        Ok(())
    }

    fn pretty_row_has_comma(&self, index: Index) -> bool {
        let row = &self[index];
        if row.parent.is_nil() {
            return false;
        }
        match row.value {
            Value::OpenContainer { .. } => false,
            Value::CloseContainer { open_index, .. } => self[open_index].next_sibling.is_some(),
            _ => row.next_sibling.is_some(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::flatjson::parse_top_level_json;

    #[test]
    fn test_pretty_print() {
        const JSON: &str = r#"{"a":1,"b":[2,{},[],false],"c":null}
            [ "d"   , [1,{      "e" :   7 }]   ]"#;
        const PRETTY: &str = r#"{
  "a": 1,
  "b": [
    2,
    {},
    [],
    false
  ],
  "c": null
}
[
  "d",
  [
    1,
    {
      "e": 7
    }
  ]
]
"#;
        let fj = parse_top_level_json(JSON.to_owned()).unwrap();
        assert_eq!(
            PRETTY,
            crate::output::serialize_roots(
                &fj,
                crate::options::OutputFormat::Json,
                &crate::path_filter::document_roots(&fj),
            )
            .unwrap()
        );
    }

    #[test]
    fn test_pretty_printed_value() {
        const JSON: &str = r#"[[{"3":3,"4":[5, 6, {"8": false}]}]]"#;
        let fj = parse_top_level_json(JSON.to_owned()).unwrap();
        const PRETTY_INNER_OBJ: &str = r#"{
  "3": 3,
  "4": [
    5,
    6,
    {
      "8": false
    }
  ]
}
"#;
        assert_eq!(PRETTY_INNER_OBJ, fj.pretty_printed_value(2).unwrap());
        assert_eq!("3", fj.pretty_printed_value(3).unwrap());

        const PRETTY_ARRAY: &str = r#"[
  5,
  6,
  {
    "8": false
  }
]
"#;
        assert_eq!(PRETTY_ARRAY, fj.pretty_printed_value(4).unwrap());
        assert_eq!("6", fj.pretty_printed_value(6).unwrap());

        const PRETTY_NESTED_OBJ: &str = "{\n  \"8\": false\n}\n";
        assert_eq!(PRETTY_NESTED_OBJ, fj.pretty_printed_value(7).unwrap());
    }
}
