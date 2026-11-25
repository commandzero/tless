//! Scan source rows once for visible positions, keys, and warnings.
use super::{Analysis, bounded_fraction, bounded_integer, key, nonempty_object};
use crate::flatjson::{ContainerType, FlatJson, Row, Value};
use crate::toon_display::{WarningKind, compact_key, number, scalar};
use std::borrow::Cow;

#[derive(Default)]
struct Keys<'a> {
    seen: usize,
    previous_clean: bool,
    clean: bool,
    same: bool,
    values: Vec<(Cow<'a, str>, usize)>,
}

/// Temporary per-depth key storage. Reusing a warning-free prefix avoids
/// decoding and sorting the structure of identical records again.
pub(super) struct Scan<'a, 'b> {
    flat: &'a FlatJson,
    analysis: &'b mut Analysis,
    root: usize,
    keys: Vec<Keys<'a>>,
    order: Vec<usize>,
    raw_keys: bool,
}

impl<'a, 'b> Scan<'a, 'b> {
    pub(super) fn new(flat: &'a FlatJson, analysis: &'b mut Analysis) -> Self {
        Self {
            flat,
            analysis,
            root: 0,
            keys: (0..=flat.2).map(|_| Keys::default()).collect(),
            order: Vec::new(),
            raw_keys: flat.3.keys.is_empty(),
        }
    }

    pub(super) fn visit_roots(&mut self, roots: &[usize]) {
        for &root in roots {
            self.root = root;
            let end = *self.flat.subtree_range(root).end() + 1;
            for (offset, row) in self.flat.0.iter_range(root..end).enumerate() {
                match row.value {
                    Value::CloseContainer { open_index, .. } => self.close(open_index),
                    _ => self.visit(root + offset, row),
                }
            }
        }
    }

    fn visit(&mut self, node: usize, row: &Row) {
        let keyed = node != self.root && row.key_range.is_some();
        if !nonempty_object(row) || keyed {
            self.analysis.lines.insert(node);
        }
        if keyed {
            self.visit_key(node);
        }

        match row.value {
            Value::Number => {
                let raw = &self.flat.1[row.range.clone()];
                if let Some(warning) = numeric_warning(raw) {
                    self.warn(node, warning);
                }
            }
            Value::OpenContainer {
                container_type: ContainerType::Object,
                ..
            } => {
                self.keys[row.depth].begin();
            }
            _ => {}
        }
    }

    fn visit_key(&mut self, node: usize) {
        let row = &self.flat[node];
        let fields = &mut self.keys[row.depth - 1];
        let raw = &self.flat.1[row.key_range.clone().unwrap()];
        if self.raw_keys && fields.reuse_raw(raw, node) {
            return;
        }
        if let Some(decoded) = key(self.flat, node) {
            fields.record(decoded, node);
        } else if let Some(key) = self.flat.key_value(node) {
            fields.same = false;
            fields.clean = false;
            self.warn(node, WarningKind::NonStringKey);
            for warning in compact_key(key).1 {
                self.warn(node, warning);
            }
        }
    }

    fn close(&mut self, node: usize) {
        let row = &self.flat[node];
        match row.value {
            Value::OpenContainer {
                container_type: ContainerType::Array,
                first_child,
                ..
            } => {
                if scalar(self.flat, first_child) {
                    self.analysis.arrays.push(node);
                }
            }
            _ => self.record_duplicates(node, row.depth),
        }
        self.propagate_warnings(node, row);
    }

    fn propagate_warnings(&mut self, node: usize, row: &Row) {
        if node == self.root {
            return;
        }
        if let Some(&hidden) = self.analysis.hidden_warnings.get(&node) {
            *self
                .analysis
                .hidden_warnings
                .entry(row.parent.unwrap())
                .or_default() += hidden;
        }
    }

    fn record_duplicates(&mut self, object: usize, depth: usize) {
        let fields = &mut self.keys[depth];
        fields.values.truncate(fields.seen);
        if !fields.same {
            fields.order_by_key(&mut self.order);
            for group in self
                .order
                .chunk_by(|&a, &b| fields.values[a].0 == fields.values[b].0)
            {
                if group.len() <= 1 {
                    continue;
                }
                fields.clean = false;
                record_occurrences(self.analysis, fields, group, object);
            }
        }
        fields.previous_clean = fields.clean;
    }

    fn warn(&mut self, node: usize, warning: WarningKind) {
        let parent = (node != self.root).then(|| self.flat[node].parent.unwrap());
        self.analysis.warn(node, parent, warning);
    }
}

fn record_occurrences(analysis: &mut Analysis, fields: &Keys<'_>, group: &[usize], object: usize) {
    for (ordinal, &index) in group.iter().enumerate() {
        let child = fields.values[index].1;
        analysis
            .occurrences
            .insert(child, (ordinal + 1, group.len()));
        analysis.warn(child, Some(object), WarningKind::DuplicateKey);
    }
}

impl<'a> Keys<'a> {
    fn begin(&mut self) {
        self.seen = 0;
        self.same = self.previous_clean;
        self.clean = true;
    }

    fn reuse_raw(&mut self, raw: &str, node: usize) -> bool {
        if !self.previous_clean {
            return false;
        }
        let Some((Cow::Borrowed(previous), owner)) = self.values.get_mut(self.seen) else {
            return false;
        };
        // A borrowed key has no escapes. Comparing to its decoded spelling
        // proves the current key has none either.
        if &raw[1..raw.len() - 1] != *previous {
            return false;
        }
        *owner = node;
        self.seen += 1;
        true
    }

    fn record(&mut self, decoded: Cow<'a, str>, node: usize) {
        if let Some((key, owner)) = self.values.get_mut(self.seen) {
            let known_clean = self.previous_clean && *key == decoded;
            self.same &= known_clean;
            if !known_clean {
                *key = decoded;
            }
            *owner = node;
        } else {
            self.same = false;
            self.values.push((decoded, node));
        }
        self.seen += 1;
    }

    fn order_by_key(&self, order: &mut Vec<usize>) {
        order.clear();
        order.extend(0..self.values.len());
        order.sort_unstable_by(|&a, &b| self.values[a].0.cmp(&self.values[b].0).then(a.cmp(&b)));
    }
}

fn numeric_warning(raw: &str) -> Option<WarningKind> {
    // Valid exponent spellings need no rendering to determine their warnings.
    lazy_static::lazy_static! {
        static ref EXPONENT: regex::Regex = regex::Regex::new(
            r"^-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?[eE][+-]?[0-9]+$"
        ).unwrap();
    }
    if bounded_integer(raw) || bounded_fraction(raw) {
        return None;
    }
    if EXPONENT.is_match(raw) {
        return None;
    }
    number(raw).1
}
