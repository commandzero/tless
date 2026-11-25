//! Structural TOON analysis. No rendered lines or source-map graphs are retained.
use super::line_index::LineIndex;
use super::{WarningKind, decode_string, number, scalar};
use crate::flatjson::{ContainerType, FlatJson, KeyValue, Row, Value};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

mod scan;
use scan::Scan;

#[derive(Clone, Copy, Debug, Default)]
pub struct Node {
    pub child_count: usize,
    pub hidden_warnings: usize,
    pub table: bool,
    pub table_row: bool,
    warnings: u8,
}

/// Exceptional metadata is sparse; ordinary nodes do not own heap collections.
pub struct Analysis {
    pub lines: LineIndex,
    pub arrays: Vec<usize>,
    pub occurrences: HashMap<usize, (usize, usize)>,
    warnings: HashMap<usize, u8>,
    hidden_warnings: HashMap<usize, usize>,
    tables: HashSet<usize>,
    extra_warnings: HashMap<usize, Vec<WarningKind>>,
    pub roots: Vec<usize>,
    root_ranges: Vec<(usize, usize, usize)>,
}

pub fn children(flat: &FlatJson, node: usize) -> impl Iterator<Item = usize> + '_ {
    let mut next = flat[node].first_child();
    std::iter::from_fn(move || match next.as_option() {
        None => None,
        Some(node) => {
            next = flat[node].next_sibling;
            Some(node)
        }
    })
}

pub fn child_count(flat: &FlatJson, node: usize) -> usize {
    match flat[node].value {
        Value::OpenContainer { close_index, .. } => {
            flat[flat[close_index].last_child().unwrap()].index_in_parent + 1
        }
        _ => 0,
    }
}

pub fn decoded(raw: &str) -> Cow<'_, str> {
    let inner = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(raw);
    if inner.contains('\\') {
        Cow::Owned(decode_string(raw))
    } else {
        Cow::Borrowed(inner)
    }
}

pub fn key(flat: &FlatJson, node: usize) -> Option<Cow<'_, str>> {
    match flat.key_value(node) {
        Some(KeyValue::String(key)) => Some(Cow::Borrowed(key)),
        Some(_) => None,
        None => flat[node]
            .key_range
            .as_ref()
            .map(|range| decoded(&flat.1[range.clone()])),
    }
}

fn keys_equal(flat: &FlatJson, a: usize, b: usize) -> bool {
    if a == b {
        return true;
    }
    if flat.key_value(a).is_none() && flat.key_value(b).is_none() {
        if let (Some(left), Some(right)) = (&flat[a].key_range, &flat[b].key_range) {
            if flat.1[left.clone()] == flat.1[right.clone()] {
                return true;
            }
        }
    }
    key(flat, a) == key(flat, b)
}

pub fn string(flat: &FlatJson, node: usize) -> Cow<'_, str> {
    match flat.string_value(node) {
        Some(value) => Cow::Borrowed(value),
        None => decoded(&flat.1[flat[node].range.clone()]),
    }
}

fn bounded_integer(raw: &str) -> bool {
    let digits = raw.strip_prefix('-').unwrap_or(raw);
    raw.len() <= 4096
        && !digits.is_empty()
        && (digits == "0" || !digits.starts_with('0'))
        && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn bounded_fraction(raw: &str) -> bool {
    raw.len() <= 4096
        && raw.split_once('.').is_some_and(|(integer, fraction)| {
            bounded_integer(integer)
                && !fraction.is_empty()
                && fraction.bytes().all(|byte| byte.is_ascii_digit())
        })
}

pub fn numeric(raw: &str) -> (Cow<'_, str>, Option<WarningKind>) {
    if bounded_integer(raw) {
        return (Cow::Borrowed(if raw == "-0" { "0" } else { raw }), None);
    }
    let (value, warning) = number(raw);
    (Cow::Owned(value), warning)
}

impl Node {
    pub fn warning_count(self) -> usize {
        self.warnings.count_ones() as usize
    }
}

fn warning_kinds(bits: u8) -> impl Iterator<Item = WarningKind> {
    [
        WarningKind::DuplicateKey,
        WarningKind::NonFiniteNumber,
        WarningKind::NonCanonicalNumber,
        WarningKind::NonStringKey,
    ]
    .into_iter()
    .filter(move |kind| bits & (1 << *kind as u8) != 0)
}

impl Analysis {
    pub fn new(flat: &FlatJson, roots: &[usize]) -> Self {
        let mut root_ranges: Vec<_> = roots
            .iter()
            .enumerate()
            .map(|(ordinal, &root)| (root, *flat.subtree_range(root).end(), ordinal))
            .collect();
        root_ranges.sort_unstable();
        let mut result = Self {
            lines: LineIndex::new(flat.0.len()),
            arrays: Vec::new(),
            warnings: HashMap::new(),
            hidden_warnings: HashMap::new(),
            tables: HashSet::new(),
            occurrences: HashMap::new(),
            extra_warnings: HashMap::new(),
            roots: roots.to_vec(),
            root_ranges,
        };
        Scan::new(flat, &mut result).visit_roots(roots);
        for &root in roots {
            result.classify_tables(flat, root);
        }
        result.lines.finish();
        result
    }

    fn classify_tables(&mut self, flat: &FlatJson, root: usize) {
        let end = *flat.subtree_range(root).end();
        let mut node = root;
        while node <= end {
            if !self.table_candidate(flat, root, node) || !self.is_table(flat, node) {
                node += 1;
                continue;
            }
            // A table owns all its field groups and leaves. Skip the entire
            // subtree so nested objects do not receive independent layouts.
            node = self.register_table(flat, root, node);
        }
    }

    fn table_candidate(&self, flat: &FlatJson, root: usize, node: usize) -> bool {
        let row = &flat[node];
        match row.value {
            Value::OpenContainer {
                container_type: ContainerType::Array,
                ..
            } => node == root || row.key_range.is_some(),
            Value::OpenContainer {
                container_type: ContainerType::Object,
                ..
            } => (node == root || row.key_range.is_some()) && child_count(flat, node) >= 2,
            _ => false,
        }
    }

    fn register_table(&mut self, flat: &FlatJson, root: usize, node: usize) -> usize {
        self.tables.insert(node);
        if node == root && !flat[node].is_array() {
            self.lines.insert(node);
        }
        let end = *flat.subtree_range(node).end();
        self.lines.clear(node + 1..end);
        for row in children(flat, node) {
            self.lines.insert(row);
        }
        end + 1
    }

    fn warn(&mut self, node: usize, parent: Option<usize>, warning: WarningKind) {
        let bits = self.warnings.entry(node).or_default();
        let bit = 1 << warning as u8;
        if *bits & bit != 0 {
            self.extra_warnings.entry(node).or_default().push(warning);
        }
        *bits |= bit;
        // Warning-free nodes need no metadata. Closing delimiters propagate
        // nonzero container totals, keeping deeply nested warnings linear.
        if let Some(parent) = parent {
            *self.hidden_warnings.entry(parent).or_default() += 1;
        }
    }

    pub fn node(&self, flat: &FlatJson, node: usize) -> Node {
        Node {
            child_count: child_count(flat, node),
            hidden_warnings: self.hidden_warnings.get(&node).copied().unwrap_or(0),
            table: self.tables.contains(&node),
            table_row: self.table_row(flat, node),
            warnings: self.warnings.get(&node).copied().unwrap_or(0),
        }
    }

    pub fn table_owner(&self, flat: &FlatJson, node: usize) -> Option<usize> {
        let mut current = node;
        let root = self.root_for(node)?;
        loop {
            if self.tables.contains(&current) {
                return Some(current);
            }
            current = flat[current].parent.as_option()?;
            if self.root_for(current) != Some(root) {
                return None;
            }
        }
    }

    pub fn table_row_owner(&self, flat: &FlatJson, node: usize) -> Option<usize> {
        let table = self.table_owner(flat, node)?;
        if node == table {
            return None;
        }
        let mut row = node;
        while flat[row].parent.as_option() != Some(table) {
            row = flat[row].parent.as_option()?;
        }
        Some(row)
    }

    pub fn table_row(&self, flat: &FlatJson, node: usize) -> bool {
        self.table_row_owner(flat, node) == Some(node)
    }

    pub fn table_cell(&self, flat: &FlatJson, node: usize) -> bool {
        scalar(flat, node) && self.table_row_owner(flat, node).is_some()
    }

    /// Primitive leaves in encounter order; eligible rows contain no arrays.
    pub fn table_cells<'a>(
        &self,
        flat: &'a FlatJson,
        row: usize,
    ) -> impl Iterator<Item = usize> + 'a {
        let end = *flat.subtree_range(row).end();
        (row + 1..end).filter(move |&node| scalar(flat, node))
    }

    pub fn warnings(&self, node: usize) -> impl Iterator<Item = WarningKind> + '_ {
        self.warnings
            .get(&node)
            .into_iter()
            .flat_map(|&bits| warning_kinds(bits))
            .chain(
                self.extra_warnings
                    .get(&node)
                    .into_iter()
                    .flatten()
                    .copied(),
            )
    }

    pub fn is_root(&self, node: usize) -> bool {
        self.root_ranges
            .binary_search_by_key(&node, |&(root, _, _)| root)
            .is_ok()
    }

    pub fn root_for(&self, node: usize) -> Option<usize> {
        self.root_index_for(node).map(|index| self.roots[index])
    }

    pub fn root_index_for(&self, node: usize) -> Option<usize> {
        let pos = self
            .root_ranges
            .partition_point(|&(root, _, _)| root <= node);
        let &(_, end, ordinal) = self.root_ranges.get(pos.checked_sub(1)?)?;
        (node <= end).then_some(ordinal)
    }

    fn is_table(&self, flat: &FlatJson, container: usize) -> bool {
        let Some(first) = children(flat, container).next() else {
            return false;
        };
        if !flat[container].is_array() && !self.valid_keyed_entries(flat, container) {
            return false;
        }
        if child_count(flat, container) == 1 {
            self.same_schema(flat, first, first)
        } else {
            // Pairwise comparison validates both schemas. Do not walk the
            // first subtree before an immediate mismatch can reject the table.
            children(flat, container)
                .skip(1)
                .all(|row| self.same_schema(flat, first, row))
        }
    }

    fn valid_keyed_entries(&self, flat: &FlatJson, container: usize) -> bool {
        child_count(flat, container) >= 2
            && children(flat, container)
                .all(|entry| nonempty_object(&flat[entry]) && !self.invalid_table_key(entry))
    }

    fn invalid_table_key(&self, node: usize) -> bool {
        const INVALID: u8 =
            (1 << WarningKind::DuplicateKey as u8) | (1 << WarningKind::NonStringKey as u8);
        self.warnings
            .get(&node)
            .is_some_and(|&bits| bits & INVALID != 0)
    }

    fn same_schema(&self, flat: &FlatJson, first: usize, row: usize) -> bool {
        if !nonempty_object(&flat[first]) || !nonempty_object(&flat[row]) {
            return false;
        }
        if child_count(flat, first) != child_count(flat, row) {
            return false;
        }

        let members = |object| {
            (object + 1..flat[object].pair_index().unwrap())
                .filter(|&node| !flat[node].is_closing_of_container())
                .map(move |node| (node, flat[node].depth - flat[object].depth))
        };
        let mut left = members(first);
        let mut right = members(row);
        loop {
            match (left.next(), right.next()) {
                (None, None) => return true,
                (Some(a), Some(b)) if self.same_field(flat, a, b) => {}
                _ => return false,
            }
        }
    }

    fn same_field(&self, flat: &FlatJson, a: (usize, usize), b: (usize, usize)) -> bool {
        let ((left, left_depth), (right, right_depth)) = (a, b);
        if left_depth != right_depth
            || self.invalid_table_key(left)
            || self.invalid_table_key(right)
        {
            return false;
        }
        keys_equal(flat, left, right) && matching_field_types(flat, left, right)
    }
}

fn nonempty_object(row: &Row) -> bool {
    matches!(
        row.value,
        Value::OpenContainer {
            container_type: ContainerType::Object,
            ..
        }
    )
}

fn matching_field_types(flat: &FlatJson, left: usize, right: usize) -> bool {
    if scalar(flat, left) {
        scalar(flat, right)
    } else {
        nonempty_object(&flat[left]) && nonempty_object(&flat[right])
    }
}
