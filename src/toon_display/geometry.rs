//! Width-dependent logical addresses, separate from semantic analysis and text.
use super::index::{Analysis, child_count, children, key, numeric, string};
use super::line_index::LineIndex;
use super::{quote_json, quote_key, scalar, value_needs_quotes};
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
    lines: LineIndex,
    root_lines: Vec<usize>,
    pub width: usize,
    inline_limit: usize,
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
        Value::String => {
            let value = string(flat, node);
            if value_needs_quotes(&value) {
                Cow::Owned(quote_json(&value))
            } else {
                value
            }
        }
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
            lines: LineIndex::new(0),
            root_lines: Vec::with_capacity(analysis.roots.len() + 1),
            width: 0,
            inline_limit,
        };
        loop {
            result.width = terminal_width.saturating_sub(gutter + 2);
            result.lines.reset(&analysis.lines);
            let mut widest_inline = 0;
            for &node in &analysis.arrays {
                if expanded.contains(&node) {
                    continue;
                }
                let root = analysis.root_for(node).unwrap();
                let depth = Self::depth(flat, root, node);
                let list = node != root
                    && flat[node]
                        .parent
                        .as_option()
                        .is_some_and(|parent| flat[parent].is_array());
                if let Some(width) =
                    result.inline_width(flat, analysis, node, depth, list, node == root)
                {
                    widest_inline = widest_inline.max(width);
                    result
                        .lines
                        .clear(node + 1..*flat.subtree_range(node).end());
                }
            }
            result.lines.finish();
            result.root_lines.clear();
            let mut total = 0;
            for &root in &analysis.roots {
                result.root_lines.push(total);
                total += result.lines.rank(*flat.subtree_range(root).end() + 1)
                    - result.lines.rank(root)
                    + usize::from(analysis.roots.len() > 1);
            }
            result.root_lines.push(total);
            let required = if numbers { digits(total).max(2) + 1 } else { 0 };
            let required_width = terminal_width.saturating_sub(required + 2);
            if required <= gutter || widest_inline <= required_width {
                result.width = required_width;
                break;
            }
            gutter = required;
        }
        result
    }

    pub fn line_count(&self) -> usize {
        *self.root_lines.last().unwrap()
    }

    fn depth(flat: &FlatJson, root: usize, node: usize) -> usize {
        let implicit_root = flat[root].is_opening_of_container() && !flat[root].is_array();
        flat[node]
            .depth
            .saturating_sub(flat[root].depth + usize::from(implicit_root))
    }

    pub fn inline(&self, flat: &FlatJson, node: usize) -> bool {
        flat[node].is_array()
            && child_count(flat, node) != 0
            && self.lines.rank(*flat.subtree_range(node).end() + 1) - self.lines.rank(node) == 1
    }

    /// Materialize only the requested descriptor. Rank/select handles wide
    /// sibling lists without keeping positions or walking preceding values.
    pub fn row(&self, flat: &FlatJson, analysis: &Analysis, line: usize) -> Option<Row> {
        if line >= self.line_count() {
            return None;
        }
        let ordinal = self.root_lines.partition_point(|&start| start <= line) - 1;
        let root = analysis.roots[ordinal];
        let sequence = usize::from(analysis.roots.len() > 1);
        if sequence != 0 && line == self.root_lines[ordinal] {
            return Some(Row {
                owner: root,
                node: root,
                depth: 0,
                kind: Kind::Document,
            });
        }
        let node = self
            .lines
            .select(self.lines.rank(root) + line - self.root_lines[ordinal] - sequence)?;
        let table_row = analysis.table_row(flat, node);
        let mut owner = node;
        if node != root && !table_row && flat[node].index_in_parent == 0 {
            if let Some(parent) = flat[node].parent.as_option() {
                if parent != root && !flat[parent].is_array() && flat[parent].key_range.is_none() {
                    owner = parent;
                }
            }
        }
        Some(Row {
            owner,
            node,
            depth: Self::depth(flat, root, node),
            kind: if table_row {
                Kind::TableRow
            } else {
                Kind::Value {
                    list: node != root
                        && flat[node]
                            .parent
                            .as_option()
                            .is_some_and(|parent| flat[parent].is_array()),
                    root: node == root,
                }
            },
        })
    }

    pub fn position(&self, flat: &FlatJson, analysis: &Analysis, node: usize) -> Position {
        let Some(ordinal) = analysis.root_index_for(node) else {
            return Position::default();
        };
        let root = analysis.roots[ordinal];
        let start = self.root_lines[ordinal];
        let body = start + usize::from(analysis.roots.len() > 1);
        if node == root {
            return Position {
                line: start,
                body_line: body,
                end: self.root_lines[ordinal + 1],
                inline: self.inline(flat, node),
            };
        }
        let shared = match flat[node].parent.as_option() {
            Some(parent) if analysis.table_cell(flat, node) || self.inline(flat, parent) => {
                Some(parent)
            }
            _ => None,
        };
        let anchor = shared.unwrap_or(node);
        let line = body + self.lines.rank(anchor) - self.lines.rank(root);
        Position {
            line,
            body_line: line,
            end: if shared.is_some() {
                line + 1
            } else {
                body + self.lines.rank(*flat.subtree_range(node).end() + 1) - self.lines.rank(root)
            },
            inline: self.inline(flat, node),
        }
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
        let count = child_count(flat, node);
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
        let info = analysis.node(flat, node);
        if info.hidden_warnings == 0 && info.warning_count() == 0 {
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
        if analysis.table_cell(flat, node)
            && source.is_some_and(|offset| {
                flat[node]
                    .key_range
                    .as_ref()
                    .is_some_and(|range| range.contains(&offset))
            })
        {
            let row = flat[node].parent.unwrap();
            return self
                .position(flat, analysis, flat[row].parent.unwrap())
                .body_line;
        }
        if source.is_some() && scalar(flat, node) {
            self.position(flat, analysis, node).body_line
        } else {
            self.position(flat, analysis, node).line
        }
    }
}
