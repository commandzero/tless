//! Row prefixes and shared table field definitions.
use super::{DisplayLine, Formatter, Kind, Row, TableMetrics, TokenRole};
use crate::flatjson::{ContainerType, Value};
use crate::toon_display::geometry::key_text;
use crate::toon_display::index::child_count;

impl Formatter<'_> {
    pub(super) fn document_header(&self, node: usize) -> DisplayLine {
        let mut line = DisplayLine::new(node, String::new());
        line.separator = true;
        line.token("---", node, TokenRole::ContainerDelimiter, None);
        let position = self
            .analysis
            .roots
            .iter()
            .position(|&root| root == node)
            .unwrap();
        line.token(
            &format!(" ({} of {})", position + 1, self.analysis.roots.len()),
            node,
            TokenRole::DocumentPosition,
            None,
        );
        line
    }

    pub(super) fn header(&self, descriptor: Row, alignment: Option<&TableMetrics>) -> DisplayLine {
        let node = descriptor.node;
        let mut line = DisplayLine::new(descriptor.owner, "  ".repeat(descriptor.depth));
        let Kind::Value { list, root } = descriptor.kind else {
            return line;
        };
        if list {
            line.token("-", node, TokenRole::ContainerDelimiter, None);
        }

        let object = is_object(&self.flat[node].value);
        let keyed = !root && self.flat[node].key_range.is_some();
        let info = self.analysis.node(self.flat, node);
        if implicit_object_header(object, keyed, info.table) {
            return line;
        }
        self.header_key(&mut line, list, keyed.then_some(node));

        if info.table {
            count_token(&mut line, node, info.child_count, object);
            self.table_fields(&mut line, node, alignment);
            line.token(":", node, TokenRole::Punctuation, None);
        } else {
            self.value_header(&mut line, descriptor);
        }
        line
    }

    fn header_key(&self, line: &mut DisplayLine, list: bool, key_node: Option<usize>) {
        if list {
            line.text.push(' ');
        }
        if let Some(node) = key_node {
            self.key_token(line, node, TokenRole::Key);
        }
    }

    fn value_header(&self, line: &mut DisplayLine, descriptor: Row) {
        let Kind::Value { list, root } = descriptor.kind else {
            unreachable!("only value rows have value headers");
        };
        let node = descriptor.node;
        let row = &self.flat[node];
        match row.value {
            Value::EmptyArray if !list => {
                if !root {
                    line.token(": ", node, TokenRole::Punctuation, None);
                }
                line.token("[]", node, TokenRole::EmptyContainer, None);
            }
            Value::EmptyArray => {
                count_token(line, node, 0, false);
                line.token(":", node, TokenRole::EmptyContainer, None);
            }
            Value::OpenContainer {
                container_type: ContainerType::Array,
                ..
            } => {
                count_token(line, node, child_count(self.flat, node), false);
                line.token(":", node, TokenRole::Punctuation, None);
            }
            _ if is_object(&row.value) => line.token(":", node, TokenRole::Punctuation, None),
            _ if !root && row.key_range.is_some() => {
                line.token(": ", node, TokenRole::Punctuation, None);
            }
            _ => {}
        }
    }

    pub(super) fn key_token(&self, line: &mut DisplayLine, node: usize, role: TokenRole) {
        line.token(
            &key_text(self.flat, node),
            node,
            role,
            self.flat[node].key_range.clone(),
        );
    }
}

fn count_token(line: &mut DisplayLine, node: usize, count: usize, keyed: bool) {
    let suffix = if keyed { ":" } else { "" };
    line.token(
        &format!("[{count}{suffix}]"),
        node,
        TokenRole::ArrayIndex,
        None,
    );
}

fn is_object(value: &Value) -> bool {
    matches!(
        value,
        Value::EmptyObject
            | Value::OpenContainer {
                container_type: ContainerType::Object,
                ..
            }
    )
}

fn implicit_object_header(object: bool, keyed: bool, table: bool) -> bool {
    matches!((object, keyed, table), (true, false, false))
}
