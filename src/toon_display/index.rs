//! Structural TOON analysis. No rendered lines or source-map graphs are retained.
use super::{WarningKind, compact_key, decode_string, number, scalar, unsupported_controls};
use crate::flatjson::{FlatJson, KeyValue, OptionIndex, Value};
use std::borrow::Cow;
use std::collections::HashMap;

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
    pub nodes: Vec<Node>,
    pub occurrences: HashMap<usize, (usize, usize)>,
    extra_warnings: HashMap<usize, Vec<WarningKind>>,
    pub roots: Vec<usize>,
    root_ranges: Vec<(usize, usize)>,
}

pub fn children(flat: &FlatJson, node: usize) -> impl Iterator<Item = usize> + '_ {
    let mut next = flat[node].first_child();
    std::iter::from_fn(move || match next {
        OptionIndex::Nil => None,
        OptionIndex::Index(node) => {
            next = flat[node].next_sibling;
            Some(node)
        }
    })
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
    match &flat[node].key_value {
        Some(KeyValue::String(key)) => Some(Cow::Borrowed(key)),
        Some(_) => None,
        None => flat[node]
            .key_range
            .as_ref()
            .map(|range| decoded(&flat.1[range.clone()])),
    }
}

pub fn string(flat: &FlatJson, node: usize) -> Cow<'_, str> {
    match &flat[node].string_value {
        Some(value) => Cow::Borrowed(value),
        None => decoded(&flat.1[flat[node].range.clone()]),
    }
}

pub fn numeric(raw: &str) -> (Cow<'_, str>, Option<WarningKind>) {
    let digits = raw.strip_prefix('-').unwrap_or(raw);
    if raw.len() <= 4096
        && !digits.is_empty()
        && (digits == "0" || !digits.starts_with('0'))
        && digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return (Cow::Borrowed(if raw == "-0" { "0" } else { raw }), None);
    }
    let (value, warning) = number(raw);
    (Cow::Owned(value), warning)
}

impl Node {
    fn warn(&mut self, warning: WarningKind) -> bool {
        let bit = 1 << warning as u8;
        let repeated = self.warnings & bit != 0;
        self.warnings |= bit;
        repeated
    }
    pub fn warnings(&self) -> impl Iterator<Item = WarningKind> + '_ {
        [
            WarningKind::DuplicateKey,
            WarningKind::NonFiniteNumber,
            WarningKind::NonCanonicalNumber,
            WarningKind::NonStringKey,
            WarningKind::NonStandardStringEscape,
        ]
        .into_iter()
        .filter(|kind| self.warnings & (1 << *kind as u8) != 0)
    }
    pub fn warning_count(self) -> usize {
        self.warnings.count_ones() as usize
    }
}

impl Analysis {
    pub fn new(flat: &FlatJson, roots: &[usize]) -> Self {
        let mut root_ranges: Vec<_> = roots
            .iter()
            .map(|&root| (root, *flat.subtree_range(root).end()))
            .collect();
        root_ranges.sort_unstable();
        let mut result = Self {
            nodes: vec![Node::default(); flat.0.len()],
            occurrences: HashMap::new(),
            extra_warnings: HashMap::new(),
            roots: roots.to_vec(),
            root_ranges,
        };
        // Reuse sorted key scratch across objects. This avoids hashing ordinary
        // short keys and still bounds wide-object duplicate analysis to O(k log k).
        let mut keys: Vec<(Cow<'_, str>, usize)> = Vec::new();
        for &root in roots {
            for i in flat.subtree_range(root) {
                let row = &flat[i];
                if let Value::CloseContainer { open_index, .. } = row.value {
                    result.finish_node(flat, open_index, root);
                    continue;
                }
                let info = &mut result.nodes[i];
                match row.value {
                    Value::String => {
                        if unsupported_controls(&string(flat, i))
                            && info.warn(WarningKind::NonStandardStringEscape)
                        {
                            result
                                .extra_warnings
                                .entry(i)
                                .or_default()
                                .push(WarningKind::NonStandardStringEscape);
                        }
                    }
                    Value::Number => {
                        if let Some(warning) = numeric(&flat.1[row.range.clone()]).1 {
                            if info.warn(warning) {
                                result.extra_warnings.entry(i).or_default().push(warning);
                            }
                        }
                    }
                    _ => {}
                }
                info.child_count = match row.value {
                    Value::OpenContainer { close_index, .. } => {
                        flat[flat[close_index].last_child().unwrap()].index_in_parent + 1
                    }
                    _ => 0,
                };
                if row.is_opening_of_container() && !row.is_array() {
                    keys.clear();
                    for child in children(flat, i) {
                        let info = &mut result.nodes[child];
                        if let Some(decoded) = key(flat, child) {
                            if unsupported_controls(&decoded) {
                                info.warn(WarningKind::NonStandardStringEscape);
                            }
                            keys.push((decoded, child));
                        } else if let Some(key) = &flat[child].key_value {
                            info.warn(WarningKind::NonStringKey);
                            let (_, warnings) = compact_key(key);
                            for warning in warnings {
                                if info.warn(warning) {
                                    result
                                        .extra_warnings
                                        .entry(child)
                                        .or_default()
                                        .push(warning);
                                }
                            }
                        }
                    }
                    keys.sort_unstable();
                    let mut start = 0;
                    while start < keys.len() {
                        let mut end = start + 1;
                        while end < keys.len() && keys[end].0 == keys[start].0 {
                            end += 1;
                        }
                        let count = end - start;
                        if count > 1 {
                            for (ordinal, &(_, child)) in keys[start..end].iter().enumerate() {
                                result.occurrences.insert(child, (ordinal + 1, count));
                                result.nodes[child].warn(WarningKind::DuplicateKey);
                            }
                        }
                        start = end;
                    }
                }
                if !row.is_opening_of_container() {
                    result.finish_node(flat, i, root);
                }
            }
        }
        result
    }

    /// Closing delimiters provide postorder completion during the forward scan.
    /// Only nonzero warning totals need writes to an ancestor's summary.
    fn finish_node(&mut self, flat: &FlatJson, node: usize, root: usize) {
        let info = self.nodes[node];
        let warnings = info.hidden_warnings
            + info.warning_count()
            + self.extra_warnings.get(&node).map_or(0, Vec::len);
        if warnings != 0 && node != root {
            if let OptionIndex::Index(parent) = flat[node].parent {
                self.nodes[parent].hidden_warnings += warnings;
            }
        }
        if info.child_count != 0 && flat[node].is_array() && self.is_table(flat, node) {
            self.nodes[node].table = true;
            for row in children(flat, node) {
                self.nodes[row].table_row = true;
                for cell in children(flat, row) {
                    self.nodes[cell].table_cell = true;
                }
            }
        }
    }

    pub fn warnings(&self, node: usize) -> impl Iterator<Item = WarningKind> + '_ {
        self.nodes[node].warnings().chain(
            self.extra_warnings
                .get(&node)
                .into_iter()
                .flatten()
                .copied(),
        )
    }

    pub fn is_root(&self, node: usize) -> bool {
        self.root_ranges
            .binary_search_by_key(&node, |&(root, _)| root)
            .is_ok()
    }

    pub fn root_for(&self, node: usize) -> Option<usize> {
        let pos = self.root_ranges.partition_point(|&(root, _)| root <= node);
        let &(root, end) = self.root_ranges.get(pos.checked_sub(1)?)?;
        (node <= end).then_some(root)
    }

    fn is_table(&self, flat: &FlatJson, array: usize) -> bool {
        let Some(first) = children(flat, array).next() else {
            return false;
        };
        let fields = self.nodes[first].child_count;
        if fields == 0 || flat[first].is_array() {
            return false;
        }
        if children(flat, first).any(|field| {
            !scalar(flat, field)
                || self.nodes[field].warnings
                    & ((1 << WarningKind::DuplicateKey as u8)
                        | (1 << WarningKind::NonStringKey as u8))
                    != 0
        }) {
            return false;
        }
        children(flat, array).all(|row| {
            !flat[row].is_array()
                && self.nodes[row].child_count == fields
                && children(flat, row)
                    .zip(children(flat, first))
                    .all(|(a, b)| {
                        scalar(flat, a)
                            && key(flat, a) == key(flat, b)
                            && self.nodes[a].warnings & (1 << WarningKind::NonStringKey as u8) == 0
                    })
        })
    }
}
