//! Width-dependent logical addresses, separate from semantic analysis and text.
use super::index::{Analysis, children, key, numeric, string};
use super::node_data::NodeData;
use super::{quote_key, quote_value, scalar};
use crate::chunked_vec::ChunkedVec;
use crate::flatjson::{FlatJson, Value};
use std::borrow::Cow;
use std::collections::HashSet;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, Default)]
pub struct Position {
    pub line: usize,
    pub body_line: usize,
    pub end: usize,
    pub inline: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Document,
    TableRow,
    Value { list: bool, root: bool },
}
#[derive(Clone, Copy, Debug)]
pub struct Row {
    pub owner: usize,
    pub node: usize,
    pub depth: usize,
    pub kind: Kind,
}
impl Row {
    pub fn separator(self) -> bool {
        self.kind == Kind::Document
    }
}
pub struct Geometry {
    pub positions: NodeData<Position>,
    pub rows: ChunkedVec<Row>,
    pub width: usize,
    inline_limit: usize,
    widest_inline: usize,
}

pub fn key_text(flat: &FlatJson, node: usize) -> Cow<'_, str> {
    if let Some(key) = key(flat, node) {
        let mut chars = key.chars();
        if chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        {
            key
        } else {
            Cow::Owned(quote_key(&key))
        }
    } else if let Some(key) = flat.key_value(node) {
        Cow::Owned(format!("? {}", super::compact_key(key).0))
    } else {
        Cow::Borrowed("")
    }
}
pub fn value_text(flat: &FlatJson, node: usize) -> Cow<'_, str> {
    match flat[node].value {
        Value::String => Cow::Owned(quote_value(&string(flat, node))),
        Value::Number => numeric(&flat.1[flat[node].range.clone()]).0,
        Value::Boolean | Value::Null => Cow::Borrowed(&flat.1[flat[node].range.clone()]),
        _ => Cow::Borrowed(""),
    }
}
fn digits(n: usize) -> usize {
    n.checked_ilog10().unwrap_or(0) as usize + 1
}

impl Geometry {
    pub fn new(
        flat: &FlatJson,
        analysis: &Analysis,
        terminal_width: usize,
        numbers: bool,
        expanded: &HashSet<usize>,
    ) -> Self {
        Self::with_limit(flat, analysis, terminal_width, numbers, expanded, 5)
    }

    pub fn with_limit(
        flat: &FlatJson,
        analysis: &Analysis,
        terminal_width: usize,
        numbers: bool,
        expanded: &HashSet<usize>,
        inline_limit: usize,
    ) -> Self {
        let mut gutter = if numbers { 3 } else { 0 };
        let mut result = Self {
            positions: analysis.nodes.filled_like(),
            rows: ChunkedVec::new(),
            width: 0,
            inline_limit,
            widest_inline: 0,
        };
        loop {
            result.width = terminal_width.saturating_sub(gutter + 2);
            result.rows.clear();
            result.widest_inline = 0;
            for &root in &analysis.roots {
                let header = result.rows.len();
                if analysis.roots.len() > 1 {
                    result.rows.push(Row {
                        owner: root,
                        node: root,
                        depth: 0,
                        kind: Kind::Document,
                    });
                }
                result.outline(flat, analysis, root, 0, false, true, expanded);
                if analysis.roots.len() > 1 {
                    result.positions[root].line = header;
                }
            }
            let required = if numbers {
                digits(result.rows.len()).max(2) + 1
            } else {
                0
            };
            let required_width = terminal_width.saturating_sub(required + 2);
            if required <= gutter || result.widest_inline <= required_width {
                result.width = required_width;
                break;
            }
            gutter = required;
        }
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn outline(
        &mut self,
        flat: &FlatJson,
        analysis: &Analysis,
        node: usize,
        depth: usize,
        list: bool,
        root: bool,
        expanded: &HashSet<usize>,
    ) {
        let start = self.rows.len();
        self.positions[node] = Position {
            line: start,
            body_line: start,
            end: start,
            inline: false,
        };
        let object = matches!(flat[node].value, Value::EmptyObject)
            || flat[node].is_opening_of_container() && !flat[node].is_array();
        if object && (root || flat[node].key_range.is_none()) {
            if analysis.nodes[node].child_count == 0 {
                self.rows.push(Row {
                    owner: node,
                    node,
                    depth,
                    kind: Kind::Value { list, root },
                });
            } else {
                for child in children(flat, node) {
                    self.outline(
                        flat,
                        analysis,
                        child,
                        depth + usize::from(list),
                        false,
                        false,
                        expanded,
                    );
                }
                if list {
                    self.rows[start].owner = node;
                }
            }
        } else {
            self.rows.push(Row {
                owner: node,
                node,
                depth,
                kind: Kind::Value { list, root },
            });
            if flat[node].is_array() || matches!(flat[node].value, Value::EmptyArray) {
                let measured = if expanded.contains(&node) {
                    None
                } else {
                    self.inline_width(flat, analysis, node, depth, list, root)
                };
                if analysis.nodes[node].child_count == 0 || measured.is_some() {
                    self.positions[node].inline = analysis.nodes[node].child_count != 0;
                    if self.positions[node].inline {
                        self.widest_inline = self.widest_inline.max(measured.unwrap());
                    }
                    for child in children(flat, node) {
                        self.positions[child] = Position {
                            line: start,
                            body_line: start,
                            end: start + 1,
                            inline: false,
                        };
                    }
                } else if analysis.nodes[node].table {
                    for row in children(flat, node) {
                        let line = self.rows.len();
                        self.positions[row] = Position {
                            line,
                            body_line: line,
                            end: line + 1,
                            inline: false,
                        };
                        for cell in children(flat, row) {
                            self.positions[cell] = self.positions[row];
                        }
                        self.rows.push(Row {
                            owner: row,
                            node: row,
                            depth: depth + 1,
                            kind: Kind::TableRow,
                        });
                    }
                } else {
                    for child in children(flat, node) {
                        self.outline(flat, analysis, child, depth + 1, true, false, expanded);
                    }
                }
            } else if object {
                for child in children(flat, node) {
                    self.outline(flat, analysis, child, depth + 1, false, false, expanded);
                }
            }
        }
        self.positions[node].end = self.rows.len();
    }

    fn inline_width(
        &self,
        flat: &FlatJson,
        analysis: &Analysis,
        node: usize,
        depth: usize,
        list: bool,
        root: bool,
    ) -> Option<usize> {
        let count = analysis.nodes[node].child_count;
        if count > self.inline_limit {
            return None;
        }
        let mut width = depth * 2 + if list { 2 } else { 0 } + digits(count) + 3;
        if !root {
            width += UnicodeWidthStr::width(key_text(flat, node).as_ref());
        }
        for child in children(flat, node) {
            if !scalar(flat, child) {
                return None;
            }
            width += 1 + UnicodeWidthStr::width(value_text(flat, child).as_ref());
            if width > self.width {
                return None;
            }
        }
        if analysis.nodes[node].hidden_warnings == 0 && analysis.nodes[node].warning_count() == 0 {
            return (width <= self.width).then_some(width);
        }
        // Annotation spelling is shared with the row formatter. Unusual warnings
        // are sparse; ordinary values never allocate a warning string here.
        let mut first = true;
        for owner in std::iter::once(node).chain(children(flat, node)) {
            for warning in analysis.warnings(owner) {
                width += if first { 9 } else { 2 };
                first = false;
                width += warning.message().len();
                if owner != node {
                    width += 6 + digits(flat[owner].index_in_parent);
                }
            }
        }
        (width <= self.width).then_some(width)
    }

    pub fn source_line(
        &self,
        flat: &FlatJson,
        analysis: &Analysis,
        node: usize,
        source: Option<usize>,
    ) -> usize {
        if analysis.nodes[node].table_cell
            && source.is_some_and(|offset| {
                flat[node]
                    .key_range
                    .as_ref()
                    .is_some_and(|range| range.contains(&offset))
            })
        {
            let row = flat[node].parent.unwrap();
            return self.positions[flat[row].parent.unwrap()].body_line;
        }
        if source.is_some() && scalar(flat, node) {
            self.positions[node].body_line
        } else {
            self.positions[node].line
        }
    }
}
