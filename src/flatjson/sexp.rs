//! Optional S-expression output from flat rows.
use std::ops::Range;

use super::{FlatJson, Index, KeyValue, Value};
use crate::jsonstringunescaper::{UnescapeError, unsafe_unescape_json_string};

impl FlatJson {
    fn sexp_atom_needs_escaping(s: &str) -> bool {
        // https://github.com/janestreet/sexplib0/blob/master/src/sexp.ml#L58
        let bytes = s.as_bytes();
        s.is_empty()
            || bytes
                .iter()
                .any(|byte| matches!(byte, 0..=32 | 127..=255 | b'"' | b'(' | b')' | b';' | b'\\'))
            || bytes.windows(2).any(|pair| matches!(pair, b"#|" | b"|#"))
    }

    fn escape_and_write_sexp_atom(buf: &mut String, atom: &str) {
        // https://github.com/janestreet/sexplib0/blob/master/src/sexp.ml#L81-L132
        buf.push('"');
        for ch in atom.bytes() {
            match ch {
                // Double quote and backslashes are escaped
                b'"' => buf.push_str(r#"\""#),
                b'\\' => buf.push_str(r#"\\"#),
                // White space get special escape codes
                b'\n' => buf.push_str(r#"\n"#),
                b'\t' => buf.push_str(r#"\t"#),
                b'\r' => buf.push_str(r#"\r"#),
                8 /* backspace */ => buf.push_str(r#"\b"#),
                32..=126 => buf.push(ch as char),
                _ => {
                    buf.push('\\');
                    let zero = b'0';
                    buf.push((zero + (ch / 100)) as char);
                    buf.push((zero + ((ch / 10) % 10)) as char);
                    buf.push((zero + (ch % 10)) as char);
                }
            }
        }
        buf.push('"');
    }

    fn write_sexp_atom(
        &self,
        buf: &mut String,
        range: Range<usize>,
        decoded: Option<&str>,
    ) -> Result<(), UnescapeError> {
        let unescaped;
        let value = if let Some(value) = decoded {
            value
        } else {
            unescaped = unsafe_unescape_json_string(&self.1[range.start + 1..range.end - 1])?;
            &unescaped
        };
        if Self::sexp_atom_needs_escaping(value) {
            Self::escape_and_write_sexp_atom(buf, value);
        } else {
            buf.push_str(value);
        }
        Ok(())
    }

    pub fn sexp_string(&self, roots: &[Index]) -> Result<String, UnescapeError> {
        let mut buf = String::new();
        for &root in roots {
            let range = self.subtree_range(root);
            let root = *range.start();
            for index in range {
                self.write_sexp_row(&mut buf, index, root)?;
            }
            buf.push('\n');
        }
        Ok(buf)
    }

    fn write_sexp_row(
        &self,
        buf: &mut String,
        index: Index,
        root: Index,
    ) -> Result<(), UnescapeError> {
        let row = &self[index];
        if index != root {
            if row.parent.is_some() && row.prev_sibling.is_some() {
                buf.push(' ');
            }
            self.write_sexp_key(buf, index)?;
        }

        match row.value {
            Value::Null | Value::EmptyObject | Value::EmptyArray => buf.push_str("()"),
            Value::Boolean | Value::Number => buf.push_str(&self.1[row.range.clone()]),
            Value::String => {
                self.write_sexp_atom(buf, row.range.clone(), self.string_value(index))?
            }
            Value::OpenContainer { .. } => buf.push('('),
            Value::CloseContainer { .. } => buf.push(')'),
        }
        if self.ends_sexp_entry(index, root) {
            buf.push(')');
        }
        Ok(())
    }

    fn write_sexp_key(&self, buf: &mut String, index: Index) -> Result<(), UnescapeError> {
        let Some(range) = &self[index].key_range else {
            return Ok(());
        };
        let decoded = match self.key_value(index) {
            Some(KeyValue::String(value)) => Some(value.as_str()),
            _ => None,
        };
        buf.push('(');
        self.write_sexp_atom(buf, range.clone(), decoded)?;
        buf.push(' ');
        Ok(())
    }

    fn ends_sexp_entry(&self, index: Index, root: Index) -> bool {
        let owner = match self[index].value {
            Value::OpenContainer { .. } => return false,
            Value::CloseContainer { open_index, .. } => open_index,
            _ => index,
        };
        owner != root && self[owner].key_range.is_some()
    }
}

#[cfg(test)]
mod tests {
    use crate::flatjson::parse_top_level_json;

    #[test]
    fn test_sexp_string() {
        const JSON: &str = r#"{"a":1,"b":[2,{},[],false],"c":null}
            [ "d"   , [1,{      "e" :   7 }]   ]"#;
        const PRETTY: &str = r#"((a 1) (b (2 () () false)) (c ()))
(d (1 ((e 7))))
"#;
        let fj = parse_top_level_json(JSON.to_owned()).unwrap();
        assert_eq!(
            PRETTY,
            fj.sexp_string(&crate::path_filter::document_roots(&fj))
                .unwrap()
        );
    }

    #[test]
    fn test_sexp_string_escaping() {
        const JSON: &str = r#"["", "a b", "a\"b", "a\\b", "a#|b", "a|#b", "a;b", {"a(b": "a)b"}]
            ["\n\t\r\b", "\u0000\u001f\u007f"]"#;
        const PRETTY: &str = r#"("" "a b" "a\"b" "a\\b" "a#|b" "a|#b" "a;b" (("a(b" "a)b")))
("\n\t\r\b" "\000\031\127")
"#;
        let fj = parse_top_level_json(JSON.to_owned()).unwrap();
        assert_eq!(
            PRETTY,
            fj.sexp_string(&crate::path_filter::document_roots(&fj))
                .unwrap()
        );
    }
}
