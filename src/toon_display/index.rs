//! Structural TOON analysis. No rendered lines or source-map graphs are retained.
use super::line_index::LineIndex;
use super::{WarningKind, compact_key, decode_string, number, scalar, unsupported_controls};
use crate::flatjson::{FlatJson, KeyValue, Value};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Default)]
pub struct Node {
    pub child_count: usize,
    pub hidden_warnings: usize,
    pub table: bool,
    pub table_row: bool,
    pub table_cell: bool,
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
        WarningKind::NonStandardStringEscape,
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
                    if flat[open_index].is_array() {
                        if scalar(flat, flat[open_index].first_child().unwrap()) {
                            result.arrays.push(open_index);
                        } else if result.is_table(flat, open_index) {
                            result.tables.insert(open_index);
                            result.lines.clear(open_index + 1..i);
                            for row in children(flat, open_index) {
                                result.lines.insert(row);
                            }
                        }
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
                        if !known_clean && unsupported_controls(&decoded) {
                            fields.clean = false;
                            result.warn(flat, i, root, WarningKind::NonStandardStringEscape);
                        }
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
                match row.value {
                    Value::String => {
                        if unsupported_controls(&string(flat, i)) {
                            result.warn(flat, i, root, WarningKind::NonStandardStringEscape);
                        }
                    }
                    Value::Number => {
                        let raw = &flat.1[row.range.clone()];
                        if !bounded_integer(raw) && !bounded_fraction(raw) {
                            if let Some(warning) = number(raw).1 {
                                result.warn(flat, i, root, warning);
                            }
                        }
                    }
                    _ => {}
                }
                if row.is_opening_of_container() && !row.is_array() {
                    let fields = &mut keys[row.depth];
                    fields.seen = 0;
                    fields.same = fields.previous_clean;
                    fields.clean = true;
                }
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
            table_cell: self.table_cell(flat, node),
            warnings: self.warnings.get(&node).copied().unwrap_or(0),
        }
    }

    pub fn table_row(&self, flat: &FlatJson, node: usize) -> bool {
        flat[node]
            .parent
            .as_option()
            .is_some_and(|parent| self.tables.contains(&parent))
    }

    pub fn table_cell(&self, flat: &FlatJson, node: usize) -> bool {
        !flat[node].is_closing_of_container()
            && flat[node]
                .parent
                .as_option()
                .is_some_and(|parent| self.table_row(flat, parent))
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

    fn is_table(&self, flat: &FlatJson, array: usize) -> bool {
        let Some(first) = children(flat, array).next() else {
            return false;
        };
        let fields = child_count(flat, first);
        if fields == 0 || flat[first].is_array() {
            return false;
        }
        if children(flat, first).any(|field| {
            !scalar(flat, field)
                || self.warnings.get(&field).is_some_and(|info| {
                    info & ((1 << WarningKind::DuplicateKey as u8)
                        | (1 << WarningKind::NonStringKey as u8))
                        != 0
                })
        }) {
            return false;
        }
        let raw_keys = flat.3.keys.is_empty();
        // The first row was fully validated above; compare only later rows.
        children(flat, array).skip(1).all(|row| {
            !flat[row].is_array()
                && child_count(flat, row) == fields
                && children(flat, row)
                    .zip(children(flat, first))
                    .all(|(a, b)| {
                        scalar(flat, a)
                            && (raw_keys
                                && flat.1[flat[a].key_range.clone().unwrap()]
                                    == flat.1[flat[b].key_range.clone().unwrap()]
                                || key(flat, a) == key(flat, b))
                            && !self.warnings.get(&a).is_some_and(|info| {
                                info & (1 << WarningKind::NonStringKey as u8) != 0
                            })
                    })
        })
    }
}
