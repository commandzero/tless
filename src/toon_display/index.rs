//! Structural TOON analysis. No rendered lines or source-map graphs are retained.
use super::line_index::LineIndex;
use super::{WarningKind, compact_key, decode_string, number, scalar};
use crate::flatjson::{FlatJson, KeyValue, Value};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

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

#[derive(Default)]
struct Keys<'a> {
    seen: usize,
    previous_clean: bool,
    clean: bool,
    same: bool,
    values: Vec<(Cow<'a, str>, usize)>,
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
        // Construction-only key scratch, reused at each nesting depth. A matching
        // prefix of a warning-free sequence is also warning-free; every changed
        // key sequence receives full duplicate and warning analysis.
        let mut keys: Vec<Keys<'_>> = (0..=flat.2).map(|_| Keys::default()).collect();
        let mut order = Vec::new();
        let raw_keys = flat.3.keys.is_empty();
        for &root in roots {
            let end = *flat.subtree_range(root).end() + 1;
            for (offset, row) in flat.0.iter_range(root..end).enumerate() {
                let i = root + offset;
                if let Value::CloseContainer { open_index, .. } = row.value {
                    if !flat[open_index].is_array() {
                        let fields = &mut keys[flat[open_index].depth];
                        fields.values.truncate(fields.seen);
                        if !fields.same {
                            order.clear();
                            order.extend(0..fields.values.len());
                            order.sort_unstable_by(|&a, &b| {
                                fields.values[a].0.cmp(&fields.values[b].0).then(a.cmp(&b))
                            });
                            let mut start = 0;
                            while start < order.len() {
                                let mut end = start + 1;
                                while end < order.len()
                                    && fields.values[order[end]].0 == fields.values[order[start]].0
                                {
                                    end += 1;
                                }
                                let count = end - start;
                                if count > 1 {
                                    fields.clean = false;
                                    for (ordinal, &index) in order[start..end].iter().enumerate() {
                                        let child = fields.values[index].1;
                                        result.occurrences.insert(child, (ordinal + 1, count));
                                        result.warn(flat, child, root, WarningKind::DuplicateKey);
                                    }
                                }
                                start = end;
                            }
                        }
                        fields.previous_clean = fields.clean;
                    }
                    if open_index != root {
                        if let Some(&hidden) = result.hidden_warnings.get(&open_index) {
                            *result
                                .hidden_warnings
                                .entry(flat[open_index].parent.unwrap())
                                .or_default() += hidden;
                        }
                    }
                    if flat[open_index].is_array()
                        && scalar(flat, flat[open_index].first_child().unwrap())
                    {
                        result.arrays.push(open_index);
                    }
                    continue;
                }
                if !row.is_opening_of_container()
                    || row.is_array()
                    || (i != root && row.key_range.is_some())
                {
                    result.lines.insert(i);
                }
                if i != root && row.key_range.is_some() {
                    let fields = &mut keys[row.depth - 1];
                    let raw_match = raw_keys
                        && fields.previous_clean
                        && fields.values.get(fields.seen).is_some_and(|entry| {
                            // Borrowed JSON keys have no escapes. Equality to
                            // that decoded spelling also proves this key has none.
                            let Cow::Borrowed(previous) = &entry.0 else {
                                return false;
                            };
                            let range = row.key_range.as_ref().unwrap();
                            &flat.1[range.start + 1..range.end - 1] == *previous
                        });
                    if raw_match {
                        fields.values[fields.seen].1 = i;
                        fields.seen += 1;
                    } else if let Some(decoded) = key(flat, i) {
                        let known_clean = fields.previous_clean
                            && fields
                                .values
                                .get(fields.seen)
                                .is_some_and(|entry| entry.0 == decoded);
                        fields.same &= known_clean;
                        if let Some(entry) = fields.values.get_mut(fields.seen) {
                            if !known_clean {
                                entry.0 = decoded;
                            }
                            entry.1 = i;
                        } else {
                            fields.values.push((decoded, i));
                        }
                        fields.seen += 1;
                    } else if let Some(key) = flat.key_value(i) {
                        fields.same = false;
                        fields.clean = false;
                        result.warn(flat, i, root, WarningKind::NonStringKey);
                        for warning in compact_key(key).1 {
                            result.warn(flat, i, root, warning);
                        }
                    }
                }
                if let Value::Number = row.value {
                    let raw = &flat.1[row.range.clone()];
                    if !bounded_integer(raw) && !bounded_fraction(raw) {
                        // Valid exponent spellings do not need rendering just
                        // to decide whether this off-screen value warns.
                        lazy_static::lazy_static! {
                            static ref EXPONENT: regex::Regex = regex::Regex::new(
                                r"^-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?[eE][+-]?[0-9]+$"
                            ).unwrap();
                        }
                        if !EXPONENT.is_match(raw) {
                            if let Some(warning) = number(raw).1 {
                                result.warn(flat, i, root, warning);
                            }
                        }
                    }
                }
                if row.is_opening_of_container() && !row.is_array() {
                    let fields = &mut keys[row.depth];
                    fields.seen = 0;
                    fields.same = fields.previous_clean;
                    fields.clean = true;
                }
            }
        }
        // Classify only after visiting the whole tree. An outer table owns
        // nested groups even if one of those objects is independently eligible.
        for &root in roots {
            let end = *flat.subtree_range(root).end();
            let mut i = root;
            while i <= end {
                let eligible_position = (i == root || flat[i].key_range.is_some())
                    && (flat[i].is_array() || child_count(flat, i) >= 2);
                if !flat[i].is_opening_of_container()
                    || !eligible_position
                    || !result.is_table(flat, i)
                {
                    i += 1;
                    continue;
                }
                result.tables.insert(i);
                if i == root && !flat[i].is_array() {
                    result.lines.insert(i);
                }
                let table_end = *flat.subtree_range(i).end();
                result.lines.clear(i + 1..table_end);
                for row in children(flat, i) {
                    result.lines.insert(row);
                }
                // A table's descendants are field groups and primitive leaves,
                // never independent tables. Do not reclassify their schemas.
                i = table_end + 1;
            }
        }
        result.lines.finish();
        result
    }

    fn warn(&mut self, flat: &FlatJson, node: usize, root: usize, warning: WarningKind) {
        let bits = self.warnings.entry(node).or_default();
        let bit = 1 << warning as u8;
        if *bits & bit != 0 {
            self.extra_warnings.entry(node).or_default().push(warning);
        }
        *bits |= bit;
        // Warning-free nodes need no metadata. Closing delimiters propagate
        // nonzero container totals, keeping deeply nested warnings linear.
        if node != root {
            *self
                .hidden_warnings
                .entry(flat[node].parent.unwrap())
                .or_default() += 1;
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
        if !flat[container].is_array() {
            if child_count(flat, container) < 2 {
                return false;
            }
            if children(flat, container).any(|entry| {
                !flat[entry].is_opening_of_container()
                    || flat[entry].is_array()
                    || self.warnings.get(&entry).is_some_and(|&bits| {
                        bits & ((1 << WarningKind::DuplicateKey as u8)
                            | (1 << WarningKind::NonStringKey as u8))
                            != 0
                    })
            }) {
                return false;
            }
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

    fn same_schema(&self, flat: &FlatJson, first: usize, row: usize) -> bool {
        if flat[first].is_array()
            || flat[row].is_array()
            || !flat[first].is_opening_of_container()
            || !flat[row].is_opening_of_container()
            || child_count(flat, first) == 0
            || child_count(flat, first) != child_count(flat, row)
        {
            return false;
        }
        let mut left = (first + 1..flat[first].pair_index().unwrap())
            .filter(|&node| !flat[node].is_closing_of_container());
        let mut right = (row + 1..flat[row].pair_index().unwrap())
            .filter(|&node| !flat[node].is_closing_of_container());
        loop {
            match (left.next(), right.next()) {
                (None, None) => return true,
                (Some(a), Some(b)) => {
                    let ordinary_object = |node| {
                        flat[node].is_opening_of_container()
                            && !flat[node].is_array()
                            && child_count(flat, node) > 0
                    };
                    if flat[a].depth - flat[first].depth != flat[b].depth - flat[row].depth
                        || !keys_equal(flat, a, b)
                        || self.warnings.get(&a).is_some_and(|&bits| {
                            bits & ((1 << WarningKind::DuplicateKey as u8)
                                | (1 << WarningKind::NonStringKey as u8))
                                != 0
                        })
                        || self.warnings.get(&b).is_some_and(|&bits| {
                            bits & ((1 << WarningKind::DuplicateKey as u8)
                                | (1 << WarningKind::NonStringKey as u8))
                                != 0
                        })
                        || !(scalar(flat, a) && scalar(flat, b)
                            || ordinary_object(a) && ordinary_object(b))
                    {
                        return false;
                    }
                }
                _ => return false,
            }
        }
    }
}
