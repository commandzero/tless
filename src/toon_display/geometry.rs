//! Width-dependent logical addresses, separate from semantic analysis and text.
use super::index::{Analysis, child_count, children, key, numeric, string};
use super::line_index::LineIndex;
use super::{quote_json, scalar, value_needs_quotes};
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

#[derive(Clone, Copy)]
pub struct GeometryOptions {
    pub terminal_width: usize,
    pub numbers: bool,
    pub inline_limit: usize,
}

impl GeometryOptions {
    pub fn new(terminal_width: usize, numbers: bool) -> Self {
        Self {
            terminal_width,
            numbers,
            inline_limit: 5,
        }
    }
}

pub fn key_text(flat: &FlatJson, node: usize) -> Cow<'_, str> {
    match key(flat, node) {
        Some(key) if super::bare_key(&key) => key,
        Some(key) => Cow::Owned(quote_json(&key)),
        None => match flat.key_value(node) {
            Some(key) => Cow::Owned(format!("? {}", super::compact_key(key).0)),
            None => Cow::Borrowed(""),
        },
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
        options: GeometryOptions,
        expanded: &HashSet<usize>,
    ) -> Self {
        let mut gutter = if options.numbers { 3 } else { 0 };
        let mut geometry = Self {
            lines: LineIndex::new(0),
            root_lines: Vec::with_capacity(analysis.roots.len() + 1),
            width: 0,
            inline_limit: options.inline_limit,
        };
        loop {
            geometry.width = options.terminal_width.saturating_sub(gutter + 2);
            let widest_inline = geometry.place_inline_arrays(flat, analysis, expanded);
            geometry.index_roots(flat, analysis);

            let required = if options.numbers {
                digits(geometry.line_count()).max(2) + 1
            } else {
                0
            };
            let required_width = options.terminal_width.saturating_sub(required + 2);
            if required <= gutter || widest_inline <= required_width {
                geometry.width = required_width;
                return geometry;
            }
            gutter = required;
        }
    }

    fn place_inline_arrays(
        &mut self,
        flat: &FlatJson,
        analysis: &Analysis,
        expanded: &HashSet<usize>,
    ) -> usize {
        self.lines.reset(&analysis.lines);
        let mut widest = 0;
        for &node in &analysis.arrays {
            if expanded.contains(&node) {
                continue;
            }
            let root = analysis.root_for(node).unwrap();
            let row = Self::value_row(flat, analysis, root, node);
            if let Some(width) = self.inline_width(flat, analysis, row) {
                widest = widest.max(width);
                self.lines.clear(node + 1..*flat.subtree_range(node).end());
            }
        }
        self.lines.finish();
        widest
    }

    fn index_roots(&mut self, flat: &FlatJson, analysis: &Analysis) {
        self.root_lines.clear();
        let sequence = usize::from(analysis.roots.len() > 1);
        let mut total = 0;
        for &root in &analysis.roots {
            self.root_lines.push(total);
            total += self.lines.rank(*flat.subtree_range(root).end() + 1) - self.lines.rank(root)
                + sequence;
        }
        self.root_lines.push(total);
    }

    pub fn line_count(&self) -> usize {
        *self.root_lines.last().unwrap()
    }

    fn depth(flat: &FlatJson, analysis: &Analysis, root: usize, node: usize) -> usize {
        let implicit_root = flat[root].is_opening_of_container()
            && !flat[root].is_array()
            && !analysis.node(flat, root).table;
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
        let mut row = Self::value_row(flat, analysis, root, node);
        if analysis.table_row(flat, node) {
            row.kind = Kind::TableRow;
        } else if node != root {
            row.owner = list_object_owner(flat, root, node).unwrap_or(node);
        }
        Some(row)
    }

    fn value_row(flat: &FlatJson, analysis: &Analysis, root: usize, node: usize) -> Row {
        let is_root = node == root;
        let list = !is_root
            && flat[node]
                .parent
                .as_option()
                .is_some_and(|parent| flat[parent].is_array());
        Row {
            owner: node,
            node,
            depth: Self::depth(flat, analysis, root, node),
            kind: Kind::Value {
                list,
                root: is_root,
            },
        }
    }

    /// A projected table's structural header (not an ancestor's preview).
    pub fn table_header(&self, flat: &FlatJson, analysis: &Analysis, table: usize) -> Row {
        let position = self.position(flat, analysis, table);
        let descriptor = self.row(flat, analysis, position.body_line).unwrap();
        assert_eq!(descriptor.node, table, "table header must be in projection");
        descriptor
    }

    /// Leading spaces actually painted before the table header, accounting
    /// for the dash replacing two spaces on the first field of a list object.
    pub fn table_indentation(&self, flat: &FlatJson, analysis: &Analysis, table: usize) -> usize {
        let header = self.table_header(flat, analysis, table);
        (header.depth - usize::from(header.owner != header.node)) * 2
    }

    pub fn table_field_start(&self, flat: &FlatJson, analysis: &Analysis, table: usize) -> usize {
        let descriptor = self.table_header(flat, analysis, table);
        let Kind::Value { list, root } = descriptor.kind else {
            unreachable!("table header is a value row")
        };
        descriptor.depth * 2
            + if list { 2 } else { 0 }
            + if !root && flat[table].key_range.is_some() {
                UnicodeWidthStr::width(key_text(flat, table).as_ref())
            } else {
                0
            }
            + analysis
                .node(flat, table)
                .child_count
                .checked_ilog10()
                .unwrap_or(0) as usize
            + 4 // decimal digit, '[' and ']', and '{'
            + usize::from(!flat[table].is_array()) // keyed ':' in brackets
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
        let shared = analysis
            .table_row_owner(flat, node)
            .filter(|&row| row != node)
            .or_else(|| match flat[node].parent.as_option() {
                Some(parent) if self.inline(flat, parent) => Some(parent),
                _ => None,
            });
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

    fn inline_width(&self, flat: &FlatJson, analysis: &Analysis, row: Row) -> Option<usize> {
        let Kind::Value { list, root } = row.kind else {
            return None;
        };
        let node = row.node;
        let count = child_count(flat, node);
        if count > self.inline_limit {
            return None;
        }
        let mut width = row.depth * 2 + if list { 2 } else { 0 } + digits(count) + 3;
        if !root {
            width += UnicodeWidthStr::width(key_text(flat, node).as_ref());
        }
        width = self.inline_values_width(flat, node, width)?;
        width += inline_warning_width(flat, analysis, node);
        (width <= self.width).then_some(width)
    }

    fn inline_values_width(&self, flat: &FlatJson, node: usize, mut width: usize) -> Option<usize> {
        for child in children(flat, node) {
            if !scalar(flat, child) {
                return None;
            }
            width += 1 + UnicodeWidthStr::width(value_text(flat, child).as_ref());
            if width > self.width {
                return None;
            }
        }
        Some(width)
    }
}

fn list_object_owner(flat: &FlatJson, root: usize, node: usize) -> Option<usize> {
    if flat[node].index_in_parent != 0 {
        return None;
    }
    let parent = flat[node].parent.as_option()?;
    if parent == root || flat[parent].is_array() || flat[parent].key_range.is_some() {
        return None;
    }
    Some(parent)
}

// Warning-free rows avoid scanning children again for annotation widths.
fn inline_warning_width(flat: &FlatJson, analysis: &Analysis, node: usize) -> usize {
    let info = analysis.node(flat, node);
    if info.hidden_warnings == 0 && info.warning_count() == 0 {
        return 0;
    }
    let mut width = 0;
    for owner in std::iter::once(node).chain(children(flat, node)) {
        let locator = if owner == node {
            0
        } else {
            6 + digits(flat[owner].index_in_parent)
        };
        for warning in analysis.warnings(owner) {
            width += if width == 0 { 9 } else { 2 };
            width += warning.message().len() + locator;
        }
    }
    width
}
