use logos::{Lexer, Logos};

use crate::chunked_vec::ChunkedVec;
use crate::flatjson::{ContainerType, Index, OptionIndex, Row, Value};
use crate::jsontokenizer::JsonToken;

struct JsonParser<'a> {
    tokenizer: Lexer<'a, JsonToken>,
    parents: Vec<Index>,
    rows: ChunkedVec<Row>,
    pretty_printed: String,
    max_depth: usize,

    peeked_token: Option<Option<JsonToken>>,
}

pub fn parse(json: String) -> Result<(ChunkedVec<Row>, String, usize), String> {
    let mut parser = JsonParser {
        tokenizer: JsonToken::lexer(&json),
        parents: vec![],
        rows: ChunkedVec::new(),
        pretty_printed: String::new(),
        max_depth: 0,
        peeked_token: None,
    };

    parser.parse_top_level_json()?;

    Ok((parser.rows, parser.pretty_printed, parser.max_depth))
}

impl<'a> JsonParser<'a> {
    fn next_token(&mut self) -> Option<JsonToken> {
        if self.peeked_token.is_some() {
            self.peeked_token.take().unwrap()
        } else {
            self.tokenizer
                .next()
                .map(|token| token.unwrap_or(JsonToken::Error))
        }
    }

    fn advance(&mut self) {
        self.next_token();
    }

    fn advance_and_consume_whitespace(&mut self) {
        self.advance();
        self.consume_whitespace();
    }

    fn peek_token_or_eof(&mut self) -> Option<JsonToken> {
        if self.peeked_token.is_none() {
            self.peeked_token = Some(
                self.tokenizer
                    .next()
                    .map(|token| token.unwrap_or(JsonToken::Error)),
            );
        }

        self.peeked_token.unwrap()
    }

    fn peek_token(&mut self) -> Result<JsonToken, String> {
        self.peek_token_or_eof()
            .ok_or_else(|| "Unexpected EOF".to_string())
    }

    fn unexpected_token<T>(&mut self) -> Result<T, String> {
        Err(format!("Unexpected token: {:?}", self.peek_token()))
    }

    fn consume_whitespace(&mut self) {
        while let Some(JsonToken::Whitespace | JsonToken::Newline) = self.peek_token_or_eof() {
            self.advance();
        }
    }

    fn parse_top_level_json(&mut self) -> Result<(), String> {
        self.consume_whitespace();
        let mut prev_top_level = self.parse_elem()?;
        let mut num_child = 0;

        loop {
            self.consume_whitespace();

            if self.peek_token_or_eof().is_none() {
                break;
            }

            self.pretty_printed.push('\n');
            let next_top_level = self.parse_elem()?;
            num_child += 1;

            self.link_siblings(next_top_level, OptionIndex::from(prev_top_level), num_child);

            prev_top_level = next_top_level;
        }

        Ok(())
    }

    fn parse_elem(&mut self) -> Result<usize, String> {
        self.consume_whitespace();

        self.max_depth = self.max_depth.max(self.parents.len());

        match self.peek_token()? {
            JsonToken::OpenCurly => {
                self.parse_container(ContainerType::Object, JsonToken::CloseCurly)
            }
            JsonToken::OpenSquare => {
                self.parse_container(ContainerType::Array, JsonToken::CloseSquare)
            }
            JsonToken::Null => self.parse_null(),
            JsonToken::True => self.parse_bool(true),
            JsonToken::False => self.parse_bool(false),
            JsonToken::Number => self.parse_number(),
            JsonToken::String => self.parse_string(),
            JsonToken::Whitespace | JsonToken::Newline => {
                panic!("Should have just consumed whitespace");
            }
            JsonToken::Error => Err("Parse error".to_string()),
            JsonToken::CloseCurly
            | JsonToken::CloseSquare
            | JsonToken::Colon
            | JsonToken::Comma => Err(format!("Unexpected character: {:?}", self.tokenizer.span())),
        }
    }

    fn parse_container(
        &mut self,
        container_type: ContainerType,
        closing_token: JsonToken,
    ) -> Result<usize, String> {
        let open = self.create_row(
            Value::OpenContainer {
                container_type,
                collapsed: false,
                first_child: 0,
                close_index: 0,
            },
            1,
        );
        self.parents.push(open);
        self.pretty_printed.push_str(container_type.open_str());
        self.advance_and_consume_whitespace();

        let mut previous = OptionIndex::NIL;
        let mut position = 0;
        while self.peek_token()? != closing_token {
            let child = self.parse_child(container_type, position)?;
            self.link_siblings(child, previous, position);
            previous = OptionIndex::from(child);
            position += 1;

            match self.peek_token()? {
                JsonToken::Comma => self.advance_and_consume_whitespace(),
                token if token == closing_token => break,
                _ => return self.unexpected_token(),
            }
        }

        self.advance();
        self.parents.pop();
        self.finish_container(container_type, open, previous);
        self.pretty_printed.push_str(container_type.close_str());
        Ok(open)
    }

    fn parse_child(
        &mut self,
        container_type: ContainerType,
        position: usize,
    ) -> Result<usize, String> {
        if position != 0 {
            self.pretty_printed.push_str(", ");
        }

        let key_range = match container_type {
            ContainerType::Object => {
                if position == 0 {
                    self.pretty_printed.push(' ');
                }
                Some(self.parse_key()?)
            }
            ContainerType::Array => None,
        };
        let child = self.parse_elem()?;
        self.rows[child].key_range = key_range;
        self.consume_whitespace();
        Ok(child)
    }

    fn parse_key(&mut self) -> Result<std::ops::Range<usize>, String> {
        if self.peek_token()? != JsonToken::String {
            return self.unexpected_token();
        }

        let start = self.pretty_printed.len();
        self.pretty_printed.push_str(self.tokenizer.slice());
        let range = start..self.pretty_printed.len();
        self.advance_and_consume_whitespace();

        if self.peek_token()? != JsonToken::Colon {
            return self.unexpected_token();
        }
        self.advance_and_consume_whitespace();
        self.pretty_printed.push_str(": ");
        Ok(range)
    }

    fn link_siblings(&mut self, child: usize, previous: OptionIndex, position: usize) {
        self.rows[child].prev_sibling = previous;
        self.rows[child].index_in_parent = position;
        if let Some(previous) = previous.as_option() {
            self.rows[previous].next_sibling = OptionIndex::from(child);
        }
    }

    fn finish_container(&mut self, container_type: ContainerType, open: usize, last: OptionIndex) {
        self.rows[open].value = match last.as_option() {
            Some(last_child) => self.close_container(container_type, open, last_child),
            None => match container_type {
                ContainerType::Array => Value::EmptyArray,
                ContainerType::Object => Value::EmptyObject,
            },
        };
        self.rows[open].range.end = self.pretty_printed.len() + 1;
    }

    fn close_container(
        &mut self,
        container_type: ContainerType,
        open: usize,
        last_child: usize,
    ) -> Value {
        if matches!(container_type, ContainerType::Object) {
            self.pretty_printed.push(' ');
        }

        let close_index = self.create_row(
            Value::CloseContainer {
                container_type,
                collapsed: false,
                last_child,
                open_index: open,
            },
            1,
        );
        Value::OpenContainer {
            container_type,
            collapsed: false,
            // Children are appended in document order immediately after the opening row.
            first_child: open + 1,
            close_index,
        }
    }

    fn parse_null(&mut self) -> Result<usize, String> {
        self.advance();
        let row_index = self.create_row(Value::Null, 4);
        self.pretty_printed.push_str("null");
        Ok(row_index)
    }

    fn parse_bool(&mut self, b: bool) -> Result<usize, String> {
        self.advance();

        let (bool_str, len) = if b { ("true", 4) } else { ("false", 5) };

        let row_index = self.create_row(Value::Boolean, len);
        self.pretty_printed.push_str(bool_str);

        Ok(row_index)
    }

    fn parse_number(&mut self) -> Result<usize, String> {
        let row_index = self.create_row(Value::Number, self.tokenizer.slice().len());
        self.pretty_printed.push_str(self.tokenizer.slice());
        self.advance();
        Ok(row_index)
    }

    fn parse_string(&mut self) -> Result<usize, String> {
        let row_index = self.create_row(Value::String, self.tokenizer.slice().len());

        // The token includes the quotation marks.
        self.pretty_printed.push_str(self.tokenizer.slice());

        self.advance();
        Ok(row_index)
    }

    // Add a new row to the FlatJson representation.
    //
    // self.pretty_printed should NOT include the added row yet;
    // we use the current length of self.pretty_printed as the
    // starting index of the row's range.
    #[inline(always)]
    fn create_row(&mut self, value: Value, length: usize) -> usize {
        let index = self.rows.len();

        let parent = match self.parents.last() {
            None => OptionIndex::NIL,
            Some(row_index) => OptionIndex::from(*row_index),
        };

        let range_start = self.pretty_printed.len();

        self.rows.push(Row {
            // Set correctly by us
            parent,
            depth: self.parents.len(),
            value,

            // Scalars are complete immediately; containers extend this range
            // when their closing delimiter is parsed.
            range: range_start..range_start + length,

            // To be filled in by caller
            prev_sibling: OptionIndex::NIL,
            next_sibling: OptionIndex::NIL,
            index_in_parent: 0,
            key_range: None,
        });

        index
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_row_ranges() {
        //            0 2    7  10   15    21   26    32     39 42
        let json = r#"{ "a": 1, "b": true, "c": null, "ddd": [] }"#.to_owned();
        let (rows, _, _) = parse(json).unwrap();

        assert_eq!(rows[0].range, 0..43); // Object
        assert_eq!(rows[1].key_range, Some(2..5)); // "a": 1
        assert_eq!(rows[1].range, 7..8); // "a": 1
        assert_eq!(rows[2].key_range, Some(10..13)); // "b": true
        assert_eq!(rows[2].range, 15..19); // "b": true
        assert_eq!(rows[3].key_range, Some(21..24)); // "c": null
        assert_eq!(rows[3].range, 26..30); // "c": null
        assert_eq!(rows[4].range, 39..41); // "ddd": []
        assert_eq!(rows[5].range, 42..43); // }

        //            01   5        14     21 23
        let json = r#"[14, "apple", false, {}]"#.to_owned();
        let (rows, _, _) = parse(json).unwrap();

        assert_eq!(rows[0].range, 0..24); // Array
        assert_eq!(rows[1].range, 1..3); // 14
        assert_eq!(rows[2].range, 5..12); // "apple"
        assert_eq!(rows[3].range, 14..19); // false
        assert_eq!(rows[4].range, 21..23); // {}
        assert_eq!(rows[5].range, 23..24); // ]

        //            01 3      10     17    23  27   32   37 40    46   51
        let json = r#"[{ "abc": "str", "de": 14, "f": null }, true, false]"#.to_owned();
        let (rows, _, _) = parse(json).unwrap();

        assert_eq!(rows[0].range, 0..52); // Array
        assert_eq!(rows[1].range, 1..38); // Object
        assert_eq!(rows[2].key_range, Some(3..8)); // "abc": "str"
        assert_eq!(rows[2].range, 10..15); // "abc": "str"
        assert_eq!(rows[3].key_range, Some(17..21)); // "de": 14
        assert_eq!(rows[3].range, 23..25); // "de": 14
        assert_eq!(rows[4].key_range, Some(27..30)); // "f": null
        assert_eq!(rows[4].range, 32..36); // "f": null
        assert_eq!(rows[5].range, 37..38); // }
        assert_eq!(rows[6].range, 40..44); // true
        assert_eq!(rows[7].range, 46..51); // false
        assert_eq!(rows[8].range, 51..52); // ]
    }
}
