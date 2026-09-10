//! TOON document layout over parsed row identities, independent of the optional codec.
use crate::flatjson::{FlatJson, KeyValue, OptionIndex, Value};
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenRole {
    Key,
    FieldDefinition,
    String,
    Number,
    Boolean,
    Null,
    Structure,
    Warning,
    Preview,
    Count,
}
#[derive(Clone, Debug)]
pub struct Span {
    pub range: Range<usize>,
    pub node: usize,
    pub role: TokenRole,
    pub source: Option<Range<usize>>,
    pub source_map: Vec<SourceMap>,
}
#[derive(Clone, Debug)]
pub struct SourceMap {
    pub source: Range<usize>,
    pub display: Range<usize>,
}

impl Span {
    pub fn matching_ranges(&self, query: &Range<usize>) -> Vec<Range<usize>> {
        let overlaps = |range: &Range<usize>| range.start < query.end && query.start < range.end;
        if !self.source.as_ref().is_some_and(overlaps) {
            return vec![];
        }
        if self.source_map.is_empty() {
            return vec![self.range.clone()];
        }
        self.source_map
            .iter()
            .filter(|map| overlaps(&map.source))
            .map(|map| {
                if map.source.len() == map.display.len() {
                    map.display.start + query.start.max(map.source.start) - map.source.start
                        ..map.display.start + query.end.min(map.source.end) - map.source.start
                } else {
                    map.display.clone()
                }
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct DisplayLine {
    pub text: String,
    pub spans: Vec<Span>,
    pub owner: usize,
    pub separator: bool,
}
#[derive(Clone, Debug, Default)]
pub struct NodeLayout {
    pub line: usize,
    pub extent: Range<usize>,
    pub header_end: usize,
    pub collapsible: bool,
    pub entry_count: usize,
    pub inline_array: bool,
    pub table_row: bool,
    pub table_cell: bool,
    pub descendant_warnings: usize,
    /// One-based occurrence ordinal, only present for repeated decoded keys.
    pub occurrence: Option<usize>,
    pub occurrence_total: Option<usize>,
    pub spans: Vec<(usize, Range<usize>)>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WarningKind {
    DuplicateKey,
    NonFiniteNumber,
    NonCanonicalNumber,
    NonStringKey,
    MultipleRoots,
    NonStandardStringEscape,
}
impl WarningKind {
    fn message(self) -> &'static str {
        match self {
            Self::DuplicateKey => "Duplicate key",
            Self::NonFiniteNumber => "Non-finite number",
            Self::NonCanonicalNumber => "Non-canonical number",
            Self::NonStringKey => "Non-string key",
            Self::MultipleRoots => "Multiple document roots",
            Self::NonStandardStringEscape => "Non-standard string escape",
        }
    }
}
#[derive(Clone, Debug)]
pub struct Warning {
    pub node: usize,
    pub kind: WarningKind,
}
#[derive(Clone, Debug)]
pub struct VisibleLine {
    pub absolute: usize,
    pub line: DisplayLine,
}
pub struct Layout {
    pub lines: Vec<DisplayLine>,
    pub nodes: Vec<NodeLayout>,
    pub warnings: Vec<Warning>,
    own_warnings: Vec<Vec<WarningKind>>,
    previews: Vec<String>,
    keys: Vec<Option<String>>,
    scalars: Vec<String>,
    line_containers: Vec<Vec<usize>>,
    inline_width: usize,
    inline_limit: usize,
    expanded_arrays: HashSet<usize>,
}

pub fn normalize_node(flat: &FlatJson, node: usize) -> usize {
    if flat[node].is_closing_of_container() {
        flat[node].pair_index().unwrap()
    } else {
        node
    }
}
fn children(flat: &FlatJson, node: usize) -> Vec<usize> {
    let mut result = Vec::new();
    let mut next = flat[node].first_child();
    while let OptionIndex::Index(i) = next {
        result.push(i);
        next = flat[i].next_sibling;
    }
    result
}
fn string_key(flat: &FlatJson, node: usize) -> Option<String> {
    match &flat[node].key_value {
        Some(KeyValue::String(text)) => Some(text.clone()),
        Some(_) => None,
        None => flat[node]
            .key_range
            .as_ref()
            .map(|range| decode_string(&flat.1[range.clone()])),
    }
}

fn scalar(flat: &FlatJson, node: usize) -> bool {
    matches!(
        flat[node].value,
        Value::String | Value::Number | Value::Boolean | Value::Null
    )
}
impl DisplayLine {
    fn new(owner: usize, prefix: String) -> Self {
        Self {
            text: prefix,
            spans: vec![],
            owner,
            separator: false,
        }
    }
    fn token(&mut self, text: &str, node: usize, role: TokenRole, source: Option<Range<usize>>) {
        let start = self.text.len();
        self.text.push_str(text);
        self.spans.push(Span {
            range: start..self.text.len(),
            node,
            role,
            source,
            source_map: vec![],
        });
    }
}
impl Layout {
    /// Unconstrained canonical layout for the retained TOON grammar fixtures.
    #[cfg(test)]
    pub fn canonical(flat: &FlatJson) -> Self {
        Self::build(flat, usize::MAX, usize::MAX, &HashSet::new())
    }

    pub fn for_view(flat: &FlatJson, width: usize, expanded_arrays: &HashSet<usize>) -> Self {
        Self::build(flat, width, 5, expanded_arrays)
    }

    fn build(
        flat: &FlatJson,
        width: usize,
        limit: usize,
        expanded_arrays: &HashSet<usize>,
    ) -> Self {
        let n = flat.0.len();
        let mut result = Self {
            lines: vec![],
            nodes: vec![NodeLayout::default(); n],
            warnings: vec![],
            own_warnings: vec![vec![]; n],
            previews: vec![String::new(); n],
            keys: vec![None; n],
            scalars: vec![String::new(); n],
            line_containers: vec![],
            inline_width: width,
            inline_limit: limit,
            expanded_arrays: expanded_arrays.clone(),
        };
        for (i, row) in flat.0.iter().enumerate() {
            if row.is_closing_of_container() {
                continue;
            }
            if let Some(range) = &row.key_range {
                let raw = &flat.1[range.clone()];
                if let Some(key) = &row.key_value {
                    match key {
                        KeyValue::String(text) => {
                            result.keys[i] = Some(text.clone());
                            if unsupported_controls(text) {
                                result.own_warnings[i].push(WarningKind::NonStandardStringEscape);
                            }
                        }
                        key => {
                            result.own_warnings[i].push(WarningKind::NonStringKey);
                            let (text, warnings) = compact_key(key);
                            result.keys[i] = Some(format!("? {text}"));
                            result.own_warnings[i].extend(warnings);
                        }
                    }
                } else {
                    let text = decode_string(raw);
                    if unsupported_controls(&text) {
                        result.own_warnings[i].push(WarningKind::NonStandardStringEscape);
                    }
                    result.keys[i] = Some(text);
                }
            }
            result.scalars[i] = match row.value {
                Value::String => {
                    let decoded = row
                        .string_value
                        .clone()
                        .unwrap_or_else(|| decode_string(&flat.1[row.range.clone()]));
                    if unsupported_controls(&decoded) {
                        result.own_warnings[i].push(WarningKind::NonStandardStringEscape);
                    }
                    quote_value(&decoded)
                }
                Value::Number => {
                    let (text, warning) = number(&flat.1[row.range.clone()]);
                    result.own_warnings[i].extend(warning);
                    text
                }
                Value::Boolean | Value::Null => flat.1[row.range.clone()].to_owned(),
                _ => String::new(),
            };
            let kids = children(flat, i);
            result.nodes[i].entry_count = kids.len();
            result.nodes[i].collapsible =
                !kids.is_empty() && !(row.parent.is_nil() && !row.is_array());
            if row.is_opening_of_container() && !row.is_array() {
                let mut counts = HashMap::new();
                // Read typed YAML keys or decode JSON keys before child caches exist.
                for &child in &kids {
                    if let Some(key) = string_key(flat, child) {
                        *counts.entry(key).or_insert(0usize) += 1;
                    }
                }
                let mut ordinal = HashMap::new();
                for child in kids {
                    if let Some(key) = string_key(flat, child) {
                        let total = counts[&key];
                        if total > 1 {
                            let occurrence = ordinal.entry(key).or_insert(0);
                            *occurrence += 1;
                            result.nodes[child].occurrence = Some(*occurrence);
                            result.nodes[child].occurrence_total = Some(total);
                            result.own_warnings[child].push(WarningKind::DuplicateKey);
                        }
                    }
                }
            }
        }
        for i in 0..n {
            result.own_warnings[i].sort();
            for &kind in &result.own_warnings[i] {
                result.warnings.push(Warning { node: i, kind });
            }
        }
        // Bottom-up aggregation counts semantic records once, including warnings in complex keys.
        for i in (0..n).rev() {
            if flat[i].is_closing_of_container() {
                continue;
            }
            if let OptionIndex::Index(parent) = flat[i].parent {
                result.nodes[parent].descendant_warnings +=
                    result.nodes[i].descendant_warnings + result.own_warnings[i].len();
            }
        }
        let roots: Vec<_> = flat
            .0
            .iter()
            .enumerate()
            .filter(|(_, r)| r.parent.is_nil() && !r.is_closing_of_container())
            .map(|(i, _)| i)
            .collect();
        for &root in &roots {
            if roots.len() > 1 {
                let mut line = DisplayLine::new(root, "---".into());
                line.separator = true;
                line.token(
                    "  # WARN Multiple document roots",
                    root,
                    TokenRole::Warning,
                    None,
                );
                result.lines.push(line);
                result.warnings.push(Warning {
                    node: root,
                    kind: WarningKind::MultipleRoots,
                });
            }
            result.render(flat, root, 0, false);
        }
        for i in 0..n {
            if flat[i].is_closing_of_container() {
                result.nodes[i] = result.nodes[normalize_node(flat, i)].clone();
            }
        }
        result.line_containers = vec![vec![]; result.lines.len()];
        for i in 0..n {
            if result.nodes[i].collapsible && !flat[i].is_closing_of_container() {
                result.line_containers[result.nodes[i].line].push(i);
            }
            if result.previews[i].is_empty() && flat[i].is_opening_of_container() {
                result.previews[i] = result.preview(flat, i);
            }
        }
        for line_idx in 0..result.lines.len() {
            result.annotate(flat, line_idx);
            let line = &mut result.lines[line_idx];
            for span in &mut line.spans {
                if matches!(
                    span.role,
                    TokenRole::Key | TokenRole::FieldDefinition | TokenRole::String
                ) {
                    if let Some(source) = &span.source {
                        let row = &flat[span.node];
                        let parsed =
                            if matches!(span.role, TokenRole::Key | TokenRole::FieldDefinition) {
                                match &row.key_value {
                                    Some(KeyValue::String(text)) => Some(text.as_str()),
                                    _ => None,
                                }
                            } else {
                                row.string_value.as_deref()
                            };
                        let raw = &flat.1[source.clone()];
                        if raw.starts_with('"') {
                            span.source_map = string_source_map(
                                raw,
                                parsed,
                                &line.text[span.range.clone()],
                                source.start,
                                span.range.start,
                            );
                        }
                    }
                }
            }
            for span in &mut line.spans {
                if let Some(source) = &span.source {
                    if span.source_map.is_empty()
                        && flat.1[source.clone()] == line.text[span.range.clone()]
                    {
                        span.source_map.push(SourceMap {
                            source: source.clone(),
                            display: span.range.clone(),
                        });
                    }
                }
            }
            for span in &result.lines[line_idx].spans {
                result.nodes[span.node]
                    .spans
                    .push((line_idx, span.range.clone()));
            }
        }
        result
            .warnings
            .sort_by_key(|warning| (warning.node, warning.kind));
        // Include warning annotations in the fit decision, after their locators
        // are known. Moving values to separate lines also moves their warnings.
        let overflowing: Vec<_> = result
            .nodes
            .iter()
            .enumerate()
            .filter(|(node, info)| {
                !flat[*node].is_closing_of_container()
                    && info.inline_array
                    && UnicodeWidthStr::width(result.lines[info.line].text.as_str()) > width
            })
            .map(|(node, _)| node)
            .collect();
        if !overflowing.is_empty() {
            result.expanded_arrays.extend(overflowing);
            return Self::build(flat, width, limit, &result.expanded_arrays);
        }
        result
    }
    fn key(&self, flat: &FlatJson, node: usize) -> String {
        match &self.keys[node] {
            Some(key)
                if flat[node]
                    .key_range
                    .as_ref()
                    .is_some_and(|r| flat.1[r.clone()].starts_with('[')) =>
            {
                key.clone()
            }
            Some(key) => quote_key(key),
            None => String::new(),
        }
    }
    fn table(&self, flat: &FlatJson, kids: &[usize]) -> bool {
        let Some(&first) = kids.first() else {
            return false;
        };
        let fields = children(flat, first);
        if fields.is_empty() || flat[first].is_array() {
            return false;
        }
        let mut unique = HashSet::new();
        if !fields.iter().all(|&i| {
            scalar(flat, i)
                && self.own_warnings[i]
                    .iter()
                    .all(|w| *w != WarningKind::NonStringKey)
                && unique.insert(self.keys[i].as_ref())
        }) {
            return false;
        }
        kids.iter().all(|&row| {
            let row_fields = children(flat, row);
            !flat[row].is_array()
                && row_fields.len() == fields.len()
                && row_fields.iter().zip(&fields).all(|(&a, &b)| {
                    scalar(flat, a)
                        && self.keys[a] == self.keys[b]
                        && !self.own_warnings[a].contains(&WarningKind::NonStringKey)
                })
        })
    }
    fn value_token(&self, flat: &FlatJson, line: &mut DisplayLine, node: usize) {
        let role = match flat[node].value {
            Value::String => TokenRole::String,
            Value::Number => TokenRole::Number,
            Value::Boolean => TokenRole::Boolean,
            _ => TokenRole::Null,
        };
        line.token(
            &self.scalars[node],
            node,
            role,
            Some(flat[node].range.clone()),
        );
    }
    fn render(&mut self, flat: &FlatJson, node: usize, depth: usize, list: bool) {
        let start = self.lines.len();
        self.nodes[node].line = start;
        let kids = children(flat, node);
        let mut line = DisplayLine::new(node, "  ".repeat(depth));
        if list {
            line.token("-", node, TokenRole::Structure, None);
        }
        let object = matches!(flat[node].value, Value::EmptyObject)
            || (flat[node].is_opening_of_container() && !flat[node].is_array());
        if object && flat[node].key_range.is_none() {
            self.nodes[node].header_end = line.text.len();
            if kids.is_empty() {
                self.lines.push(line);
            } else {
                for (ordinal, &child) in kids.iter().enumerate() {
                    self.render(flat, child, depth + usize::from(list), false);
                    if list && ordinal == 0 {
                        let first = &mut self.lines[start];
                        // The first object's field occupies the hyphen line; nested content
                        // keeps the extra indentation required by the TOON list grammar.
                        let position = depth * 2;
                        first.text.replace_range(position..position + 2, "- ");
                        first.spans.push(Span {
                            range: position..position + 1,
                            node,
                            role: TokenRole::Structure,
                            source: None,
                            source_map: vec![],
                        });
                        first.owner = node;
                    }
                }
            }
        } else {
            if list {
                line.text.push(' ');
            }
            if flat[node].key_range.is_some() {
                line.token(
                    &self.key(flat, node),
                    node,
                    TokenRole::Key,
                    flat[node].key_range.clone(),
                );
            }
            if flat[node].is_array() || matches!(flat[node].value, Value::EmptyArray) {
                let table = self.table(flat, &kids);
                line.token(
                    &format!("[{}]", kids.len()),
                    node,
                    TokenRole::Structure,
                    None,
                );
                if table {
                    for &row in &kids {
                        self.nodes[row].table_row = true;
                        self.nodes[row].collapsible = false;
                        for field in children(flat, row) {
                            self.nodes[field].table_cell = true;
                        }
                    }
                    let table_fields: Vec<_> =
                        kids.iter().map(|&row| children(flat, row)).collect();
                    line.token("{", node, TokenRole::Structure, None);
                    for (column, &field) in table_fields[0].iter().enumerate() {
                        if column != 0 {
                            line.token(",", node, TokenRole::Structure, None);
                        }
                        let begin = line.text.len();
                        line.token(
                            &self.key(flat, field),
                            field,
                            TokenRole::FieldDefinition,
                            flat[field].key_range.clone(),
                        );
                        let range = begin..line.text.len();
                        for row in &table_fields[1..] {
                            let other = row[column];
                            line.spans.push(Span {
                                range: range.clone(),
                                node: other,
                                role: TokenRole::FieldDefinition,
                                source: flat[other].key_range.clone(),
                                source_map: vec![],
                            });
                        }
                    }
                    line.token("}", node, TokenRole::Structure, None);
                }
                line.token(":", node, TokenRole::Structure, None);
                self.nodes[node].header_end = line.text.len();
                let inline_width = UnicodeWidthStr::width(line.text.as_str())
                    + kids
                        .iter()
                        .map(|&i| UnicodeWidthStr::width(self.scalars[i].as_str()))
                        .sum::<usize>()
                    + kids.len(); // One leading space and commas between values.
                let inline = kids.is_empty()
                    || (kids.iter().all(|&i| scalar(flat, i))
                        && kids.len() <= self.inline_limit
                        && inline_width <= self.inline_width
                        && !self.expanded_arrays.contains(&node));
                if inline {
                    self.nodes[node].inline_array = !kids.is_empty();
                    if !kids.is_empty() {
                        line.text.push(' ');
                    }
                    for (index, &child) in kids.iter().enumerate() {
                        if index != 0 {
                            line.token(",", node, TokenRole::Structure, None);
                        }
                        self.nodes[child].line = start;
                        self.nodes[child].extent = start..start + 1;
                        self.value_token(flat, &mut line, child);
                    }
                    self.lines.push(line);
                } else {
                    self.lines.push(line);
                    if table {
                        for &row in &kids {
                            let idx = self.lines.len();
                            self.nodes[row].line = idx;
                            self.nodes[row].extent = idx..idx + 1;
                            let mut row_line = DisplayLine::new(row, "  ".repeat(depth + 1));
                            self.nodes[row].header_end = row_line.text.len();
                            for (column, child) in children(flat, row).into_iter().enumerate() {
                                if column != 0 {
                                    row_line.token(",", row, TokenRole::Structure, None);
                                }
                                self.nodes[child].line = idx;
                                self.nodes[child].extent = idx..idx + 1;
                                self.value_token(flat, &mut row_line, child);
                            }
                            self.lines.push(row_line);
                        }
                    } else {
                        for &child in &kids {
                            self.render(flat, child, depth + 1, true);
                        }
                    }
                }
            } else if object {
                line.token(":", node, TokenRole::Structure, None);
                self.nodes[node].header_end = line.text.len();
                self.lines.push(line);
                for &child in &kids {
                    self.render(flat, child, depth + 1, false);
                }
            } else {
                if flat[node].key_range.is_some() {
                    line.token(": ", node, TokenRole::Structure, None);
                }
                self.value_token(flat, &mut line, node);
                self.nodes[node].header_end = line.text.len();
                self.lines.push(line);
            }
        }
        self.nodes[node].extent = start..self.lines.len();
        self.previews[node] = self.preview(flat, node);
    }
    fn preview(&self, flat: &FlatJson, node: usize) -> String {
        let mut preview = String::new();
        for child in children(flat, node) {
            if !preview.is_empty() {
                preview.push_str(if flat[node].is_array() { "," } else { "; " });
            }
            if !flat[node].is_array() {
                // The key is already cached; truncate before copying into the preview.
                if let Some(key) = &self.keys[child] {
                    append_preview(&mut preview, &quote_key(bounded_prefix(key, 256)));
                }
                append_preview(&mut preview, ": ");
            }
            if scalar(flat, child) {
                append_preview(&mut preview, &self.scalars[child]);
            } else if flat[child].is_array() || matches!(flat[child].value, Value::EmptyArray) {
                append_preview(
                    &mut preview,
                    &format!("[{}]: …", self.nodes[child].entry_count),
                );
            } else {
                append_preview(&mut preview, "…");
            }
            if preview.len() >= 256 {
                if !preview.ends_with('…') {
                    preview.push('…');
                }
                break;
            }
        }
        preview
    }
    fn annotate(&mut self, flat: &FlatJson, line: usize) {
        if self.lines[line].separator {
            return;
        }
        let mut ids: Vec<_> = self.lines[line]
            .spans
            .iter()
            .filter(|s| {
                !matches!(s.role, TokenRole::Key | TokenRole::FieldDefinition)
                    || self.nodes[s.node].line == line
            })
            .map(|s| s.node)
            .collect();
        ids.push(self.lines[line].owner);
        ids.sort_unstable();
        ids.dedup();
        let owner = self.lines[line].owner;
        let mut messages = vec![];
        for node in ids {
            for warning in &self.own_warnings[node] {
                let locator = if self.nodes[node].table_cell {
                    format!(
                        " at field {}",
                        quote_json(self.keys[node].as_deref().unwrap_or(""))
                    )
                } else if let OptionIndex::Index(parent) = flat[node].parent {
                    if flat[parent].is_array() && self.nodes[node].line == self.nodes[parent].line {
                        format!(" at [{}]", flat[node].index_in_parent)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                messages.push((node, format!("{}{locator}", warning.message())));
            }
        }
        if !messages.is_empty() {
            self.lines[line].token("  # WARN ", owner, TokenRole::Warning, None);
            for (ordinal, (node, message)) in messages.into_iter().enumerate() {
                if ordinal > 0 {
                    self.lines[line].token("; ", owner, TokenRole::Warning, None);
                }
                self.lines[line].token(&message, node, TokenRole::Warning, None);
            }
        }
    }
    fn collapsed_inline_array(
        &self,
        flat: &FlatJson,
        node: usize,
        header: &DisplayLine,
        warning_width: usize,
    ) -> Option<DisplayLine> {
        if !flat[node].is_array() || self.nodes[node].entry_count > self.inline_limit.min(5) {
            return None;
        }
        let kids = children(flat, node);
        if !kids.iter().all(|&child| scalar(flat, child)) {
            return None;
        }
        let mut line = header.clone();
        for (index, child) in kids.into_iter().enumerate() {
            line.token(
                if index == 0 { " " } else { "," },
                node,
                TokenRole::Structure,
                None,
            );
            let original = &self.lines[self.nodes[child].line];
            let mut span = original
                .spans
                .iter()
                .find(|span| {
                    span.node == child
                        && matches!(
                            span.role,
                            TokenRole::String
                                | TokenRole::Number
                                | TokenRole::Boolean
                                | TokenRole::Null
                        )
                })?
                .clone();
            let value = &original.text[span.range.clone()];
            if UnicodeWidthStr::width(line.text.as_str())
                + UnicodeWidthStr::width(value)
                + warning_width
                > self.inline_width
            {
                return None;
            }
            let start = line.text.len();
            line.text.push_str(value);
            for map in &mut span.source_map {
                map.display = start + map.display.start - span.range.start
                    ..start + map.display.end - span.range.start;
            }
            span.range = start..line.text.len();
            line.spans.push(span);
        }
        (UnicodeWidthStr::width(line.text.as_str()) + warning_width <= self.inline_width)
            .then_some(line)
    }

    /// Applies current collapse flags without changing the cached expanded grammar.
    pub fn project(&self, flat: &FlatJson) -> Vec<VisibleLine> {
        let mut visible = vec![];
        let mut absolute = 0;
        while absolute < self.lines.len() {
            let original = &self.lines[absolute];
            let mut line = original.clone();
            if !line.separator {
                let collapsed = self.line_containers[absolute]
                    .iter()
                    .copied()
                    .filter(|&i| flat[i].is_collapsed())
                    .min_by_key(|&i| flat[i].depth);
                if let Some(node) = collapsed {
                    let info = &self.nodes[node];
                    line.text.truncate(info.header_end);
                    line.spans.retain(|s| s.range.end <= info.header_end);
                    line.owner = node;
                    if !flat[node].is_array() {
                        let count = info.entry_count;
                        line.token(&format!(" ({count})"), node, TokenRole::Count, None);
                    }
                    let mut messages: Vec<_> = self.own_warnings[node]
                        .iter()
                        .map(|w| w.message().to_owned())
                        .collect();
                    if info.descendant_warnings > 0 {
                        messages.push(format!(
                            "Contains {} hidden warnings",
                            info.descendant_warnings
                        ));
                    }
                    let warning = if messages.is_empty() {
                        String::new()
                    } else {
                        format!("  # WARN {}", messages.join("; "))
                    };
                    if let Some(inline) = self.collapsed_inline_array(
                        flat,
                        node,
                        &line,
                        UnicodeWidthStr::width(warning.as_str()),
                    ) {
                        line = inline;
                    } else if !self.previews[node].is_empty() {
                        line.token(
                            &format!(" {}", self.previews[node]),
                            node,
                            TokenRole::Preview,
                            None,
                        );
                    }
                    if !messages.is_empty() {
                        line.token(&warning, node, TokenRole::Warning, None);
                    }
                    visible.push(VisibleLine { absolute, line });
                    absolute = info.extent.end.max(absolute + 1);
                    continue;
                }
            }
            visible.push(VisibleLine { absolute, line });
            absolute += 1;
        }
        visible
    }
}

fn quote_json(value: &str) -> String {
    use std::fmt::Write;
    let mut text = String::with_capacity(value.len() + 2);
    text.push('"');
    for ch in value.chars() {
        match ch {
            '"' => text.push_str("\\\""),
            '\\' => text.push_str("\\\\"),
            '\n' => text.push_str("\\n"),
            '\r' => text.push_str("\\r"),
            '\t' => text.push_str("\\t"),
            ch if ch.is_control() => {
                write!(text, "\\u{:04x}", ch as u32).unwrap();
            }
            _ => text.push(ch),
        }
    }
    text.push('"');
    text
}
fn decode_string(raw: &str) -> String {
    let inner = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(raw);
    // YAML's existing flattened strings can contain literal backslashes. Only
    // call the JSON unescaper when its syntactic precondition is satisfied.
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('u') => {
                    for _ in 0..4 {
                        if !chars.next().is_some_and(|c| c.is_ascii_hexdigit()) {
                            return inner.to_owned();
                        }
                    }
                }
                Some('"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't') => {}
                _ => return inner.to_owned(),
            }
        }
    }
    crate::jsonstringunescaper::unsafe_unescape_json_string(inner)
        .unwrap_or_else(|_| inner.to_owned())
}
fn quote_key(key: &str) -> String {
    let mut chars = key.chars();
    if chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    {
        key.to_owned()
    } else {
        quote_json(key)
    }
}
fn quote_value(value: &str) -> String {
    lazy_static::lazy_static! { static ref NUMERIC: regex::Regex=regex::Regex::new(r"^-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?$").unwrap(); }
    if value.is_empty()
        || value.trim() != value
        || matches!(
            value,
            "true" | "false" | "null" | ".inf" | "+.inf" | "-.inf" | ".nan"
        )
        || value.starts_with('-')
        || value.contains('#')
        || NUMERIC.is_match(value)
        || value
            .chars()
            .any(|c| c.is_control() || ":\"\\[]{},".contains(c))
    {
        quote_json(value)
    } else {
        value.to_owned()
    }
}
/// Exact decimal normalization, with the output length checked before allocating.
fn number(raw: &str) -> (String, Option<WarningKind>) {
    match raw.to_ascii_lowercase().as_str() {
        ".inf" | "+.inf" => return (".inf".into(), Some(WarningKind::NonFiniteNumber)),
        "-.inf" => return ("-.inf".into(), Some(WarningKind::NonFiniteNumber)),
        ".nan" | "+.nan" | "-.nan" => return (".nan".into(), Some(WarningKind::NonFiniteNumber)),
        _ => {}
    }
    let fallback = || (raw.to_owned(), Some(WarningKind::NonCanonicalNumber));
    lazy_static::lazy_static! {static ref DECIMAL:regex::Regex=regex::Regex::new(r"^(-?)(0|[1-9][0-9]*)(?:\.([0-9]+))?(?:[eE]([+-]?[0-9]+))?$").unwrap();}
    let Some(parts) = DECIMAL.captures(raw) else {
        return fallback();
    };
    let integer = &parts[2];
    let fraction = parts.get(3).map_or("", |m| m.as_str());
    let digits = format!("{integer}{fraction}");
    let trimmed = digits.trim_start_matches('0');
    if trimmed.is_empty() {
        return ("0".into(), None);
    }
    let leading = digits.len() - trimmed.len();
    let significant = trimmed.trim_end_matches('0');
    let exponent = match parts.get(4) {
        Some(exp) => match exp.as_str().parse::<i64>() {
            Ok(n) => n,
            Err(_) => return fallback(),
        },
        None => 0,
    };
    let point = match (integer.len() as i64)
        .checked_add(exponent)
        .and_then(|n| n.checked_sub(leading as i64))
    {
        Some(n) => n,
        None => return fallback(),
    };
    let negative = &parts[1] == "-";
    let size = if point <= 0 {
        point
            .checked_neg()
            .and_then(|n| n.checked_add(2))
            .and_then(|n| n.checked_add(significant.len() as i64))
    } else {
        Some(point.max(significant.len() as i64) + i64::from(point < (significant.len() as i64)))
    };
    let Some(size) = size.and_then(|n| n.checked_add(i64::from(negative))) else {
        return fallback();
    };
    if size > 4096 {
        return fallback();
    }
    let mut text = String::with_capacity(size as usize);
    if negative {
        text.push('-');
    }
    if point <= 0 {
        text.push_str("0.");
        text.extend(std::iter::repeat_n('0', (-point) as usize));
        text.push_str(significant);
    } else if point as usize >= significant.len() {
        text.push_str(significant);
        text.extend(std::iter::repeat_n('0', point as usize - significant.len()));
    } else {
        let point = point as usize;
        text.push_str(&significant[..point]);
        text.push('.');
        text.push_str(&significant[point..]);
    }
    (text, None)
}

/// Map decoded characters back to the matcher spelling, retaining quote removal
/// and escapes. Stream characters into coalesced runs without per-character storage.
fn string_source_map(
    raw: &str,
    parsed: Option<&str>,
    rendered: &str,
    source: usize,
    display: usize,
) -> Vec<SourceMap> {
    let quoted = rendered.starts_with('"') && rendered.ends_with('"');
    let mut mappings = Vec::<SourceMap>::new();
    let mut rendered_bytes = 0;
    let mut valid = true;
    let mut append = |text: &str, range: Range<usize>| {
        valid &= rendered.get(rendered_bytes..rendered_bytes + text.len()) == Some(text);
        let map = SourceMap {
            source: source + range.start..source + range.end,
            display: display + rendered_bytes..display + rendered_bytes + text.len(),
        };
        if let Some(previous) = mappings.last_mut().filter(|previous| {
            previous.source.len() == previous.display.len()
                && map.source.len() == map.display.len()
                && previous.source.end == map.source.start
                && previous.display.end == map.display.start
        }) {
            previous.source.end = map.source.end;
            previous.display.end = map.display.end;
        } else {
            mappings.push(map);
        }
        rendered_bytes += text.len();
    };
    if quoted {
        append("\"", 0..1);
    }
    let mut character = |ch: char, range: Range<usize>| {
        let mut buffer = [0; 4];
        if quoted {
            match ch {
                '"' => append("\\\"", range),
                '\\' => append("\\\\", range),
                '\n' => append("\\n", range),
                '\r' => append("\\r", range),
                '\t' => append("\\t", range),
                ch if ch.is_control() => append(&format!("\\u{:04x}", ch as u32), range),
                ch => append(ch.encode_utf8(&mut buffer), range),
            }
        } else {
            append(ch.encode_utf8(&mut buffer), range);
        }
    };
    if let Some(parsed) = parsed {
        let mut offset = 1;
        for ch in parsed.chars() {
            let length = if ch == '\n' { 2 } else { ch.len_utf8() };
            character(ch, offset..offset + length);
            offset += length;
        }
    } else {
        let mut offset = 1;
        let end = raw.len().saturating_sub(1);
        while offset < end {
            let start = offset;
            let ch = raw[offset..].chars().next().unwrap();
            offset += ch.len_utf8();
            if ch == '\\' && offset < end {
                let escaped = raw.as_bytes()[offset];
                offset += 1;
                if escaped == b'u' {
                    offset = (offset + 4).min(end);
                    if raw
                        .get(start + 2..offset)
                        .and_then(|s| u16::from_str_radix(s, 16).ok())
                        .is_some_and(|code| (0xd800..=0xdbff).contains(&code))
                        && raw[offset..].starts_with("\\u")
                    {
                        offset = (offset + 6).min(end);
                    }
                }
                for ch in decode_string(&format!("\"{}\"", &raw[start..offset])).chars() {
                    character(ch, start..offset);
                }
            } else {
                character(ch, start..offset);
            }
        }
    }
    if quoted {
        append("\"", raw.len() - 1..raw.len());
    }
    if valid && rendered_bytes == rendered.len() {
        mappings
    } else {
        vec![]
    }
}

fn unsupported_controls(text: &str) -> bool {
    text.chars()
        .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
}

fn compact_key(key: &KeyValue) -> (String, Vec<WarningKind>) {
    fn render(key: &KeyValue, warnings: &mut Vec<WarningKind>) -> String {
        match key {
            KeyValue::String(text) => {
                if unsupported_controls(text) {
                    warnings.push(WarningKind::NonStandardStringEscape);
                }
                quote_json(text)
            }
            KeyValue::Number(token) => {
                let (text, warning) = number(token);
                warnings.extend(warning);
                text
            }
            KeyValue::Boolean(value) => value.to_string(),
            KeyValue::Null => "null".into(),
            KeyValue::Array(values) => format!(
                "[{}]",
                values
                    .iter()
                    .map(|value| render(value, warnings))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            KeyValue::Object(entries) => format!(
                "{{{}}}",
                entries
                    .iter()
                    .map(|(key, value)| format!(
                        "{}:{}",
                        render(key, warnings),
                        render(value, warnings)
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
    let mut warnings = vec![];
    let text = render(key, &mut warnings);
    (text, warnings)
}

fn bounded_prefix(text: &str, limit: usize) -> &str {
    let mut end = text.len().min(limit);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}
fn append_preview(preview: &mut String, text: &str) {
    let remaining = 256usize.saturating_sub(preview.len());
    if remaining == 0 {
        return;
    }
    preview.push_str(bounded_prefix(text, remaining));
    if text.len() > remaining {
        preview.push('…');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn json(input: &str) -> FlatJson {
        let (rows, text, depth) = crate::jsonparser::parse(input.into()).unwrap();
        FlatJson(rows, text, depth)
    }
    fn yaml(input: &str) -> FlatJson {
        let (rows, text, depth) = crate::yamlparser::parse(input.into()).unwrap();
        FlatJson(rows, text, depth)
    }
    fn text(flat: &FlatJson) -> String {
        Layout::canonical(flat)
            .lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
    #[test]
    fn retained_standard_fixtures() {
        let fixtures = [
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/primitives.json"),
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/objects.json"),
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/arrays-primitive.json"),
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/arrays-nested.json"),
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/arrays-objects.json"),
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/arrays-tabular.json"),
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/whitespace.json"),
        ];
        let mut checked = 0;
        for fixture in fixtures {
            let fixture = json(fixture);
            let field = |object, name: &str| {
                children(&fixture, object).into_iter().find(|&i| {
                    fixture[i]
                        .key_range
                        .as_ref()
                        .is_some_and(|r| decode_string(&fixture.1[r.clone()]) == name)
                })
            };
            for test in children(&fixture, field(0, "tests").unwrap()) {
                if field(test, "options").is_some() {
                    continue;
                }
                let name = field(test, "name").unwrap();
                let name = decode_string(&fixture.1[fixture[name].range.clone()]);
                // The viewer's explicit order-preservation contract selects lists here.
                if name == "uses field order from first object for tabular headers" {
                    continue;
                }
                let input = field(test, "input").unwrap();
                let expected = field(test, "expected").unwrap();
                assert_eq!(
                    text(&json(&fixture.1[fixture[input].range.clone()])),
                    decode_string(&fixture.1[fixture[expected].range.clone()]),
                    "{name}"
                );
                checked += 1;
            }
        }
        assert!(checked > 80);
    }
    #[test]
    fn standard_shapes_and_order() {
        assert_eq!(
            text(&json(
                r#"{"tags":["rust","cli"],"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Lin"}]}"#
            )),
            "tags[2]: rust,cli\nusers[2]{id,name}:\n  1,Ada\n  2,Lin"
        );
        for (input, expected) in [
            ("null", "null"),
            ("{}", ""),
            ("[]", "[0]:"),
            ("[{},{}]", "[2]:\n  -\n  -"),
            ("[[1,2],[]]", "[2]:\n  - [2]: 1,2\n  - [0]:"),
            (r#"{"e":{},"a":[]}"#, "e:\na[0]:"),
            (
                r#"[{"a":1,"b":2},{"b":3,"a":4}]"#,
                "[2]:\n  - a: 1\n    b: 2\n  - b: 3\n    a: 4",
            ),
        ] {
            assert_eq!(text(&json(input)), expected, "{input}");
        }
    }
    #[test]
    fn nested_first_field_grammar() {
        assert_eq!(
            text(&json(r#"[{"rows":[{"id":1},{"id":2}],"x":true}]"#)),
            "[1]:\n  - rows[2]{id}:\n      1\n      2\n    x: true"
        );
    }
    #[test]
    fn duplicate_decoded_keys_keep_identity() {
        let flat = json(r#"{"box":{"a":1,"\u0061":2}}"#);
        let layout = Layout::canonical(&flat);
        assert_eq!(
            text(&flat),
            "box:\n  a: 1  # WARN Duplicate key\n  a: 2  # WARN Duplicate key"
        );
        assert_eq!(layout.nodes[1].entry_count, 2);
        assert_eq!(layout.nodes[1].descendant_warnings, 2);
        assert_eq!(layout.nodes[2].occurrence, Some(1));
        assert_eq!(layout.nodes[3].occurrence, Some(2));
        assert_eq!(layout.lines[1].owner, 2);
        assert_eq!(layout.lines[2].owner, 3);
    }
    #[test]
    fn exact_bounded_decimals() {
        for (input, expected) in [
            ("0.123456789012345678901", "0.123456789012345678901"),
            ("-0.000e99", "0"),
            ("1.2300e2", "123"),
            ("1000e-5", "0.01"),
            ("1e-3", "0.001"),
            ("12.345e1", "123.45"),
        ] {
            assert_eq!(number(input), (expected.into(), None));
        }
        for input in [
            "1e1000000",
            "1e99999999999999999999999999",
            "1e-99999999999999999999",
            "1_000",
            "+1",
            "01",
            "1e4096",
        ] {
            assert_eq!(
                number(input),
                (input.into(), Some(WarningKind::NonCanonicalNumber))
            );
        }
        assert_eq!(number("1e4095").0.len(), 4096);
        assert_eq!(number("1e-4094").0.len(), 4096);
    }
    #[test]
    fn warnings_on_shared_lines_and_source_text() {
        assert_eq!(
            text(&yaml("[1, .inf, .nan]")),
            "[3]: 1,.inf,.nan  # WARN Non-finite number at [1]; Non-finite number at [2]"
        );
        assert_eq!(
            text(&json(
                r##"["# WARN Duplicate key",".inf","true","05","a,b","-x"]"##
            )),
            r##"[6]: "# WARN Duplicate key",".inf","true","05","a,b","-x""##
        );
        let flat = yaml("- a: .inf\n- a: .nan\n");
        assert_eq!(text(&flat),"[2]{a}:\n  .inf  # WARN Non-finite number at field \"a\"\n  .nan  # WARN Non-finite number at field \"a\"");
    }
    #[test]
    fn typed_recursive_keys_and_multiple_roots() {
        assert_eq!(
            text(&yaml("1: number\n\"1\": string\n")),
            "? 1: number  # WARN Non-string key\n\"1\": string"
        );
        assert_eq!(
            text(&yaml("? [.inf, {a: true}]\n: value\n")),
            "? [.inf,{\"a\":true}]: value  # WARN Non-finite number; Non-string key"
        );
        assert_eq!(
            text(&yaml("---\n{}\n---\n7\n")),
            "---  # WARN Multiple document roots\n\n---  # WARN Multiple document roots\n7"
        );
    }
    #[test]
    fn shared_header_mapping_and_table_rows_remain_visible() {
        let mut flat = yaml("- a: .inf\n  b: 2\n- a: .nan\n  b: 4\n");
        let layout = Layout::canonical(&flat);
        let rows = children(&flat, 0);
        let fields = children(&flat, rows[1]);
        let key_span = layout.lines[0]
            .spans
            .iter()
            .find(|span| {
                span.node == fields[0]
                    && matches!(span.role, TokenRole::Key | TokenRole::FieldDefinition)
            })
            .unwrap();
        assert_eq!(key_span.source, flat[fields[0]].key_range);
        assert_eq!(&layout.lines[0].text[key_span.range.clone()], "a");
        assert!(layout.lines[2]
            .spans
            .iter()
            .any(|span| span.node == fields[0] && span.role == TokenRole::Warning));
        flat.collapse(rows[1]);
        let collapsed = layout.project(&flat);
        assert_eq!(collapsed[2].absolute, 2);
        assert!(!layout.nodes[rows[1]].collapsible);
        assert!(layout.nodes[0].collapsible);
        assert_eq!(collapsed[2].line.text, layout.lines[2].text);
        assert!(collapsed[2]
            .line
            .spans
            .iter()
            .any(|span| span.node == fields[0]));
        flat.expand(rows[1]);
        assert_eq!(layout.project(&flat)[2].line.text, layout.lines[2].text);
    }
    #[test]
    fn escaped_locators_and_semantic_warning_counts() {
        let flat = yaml("- \"a\\nb\": .inf\n- \"a\\nb\": .nan\n");
        let layout = Layout::canonical(&flat);
        assert!(layout.lines[1].text.ends_with("at field \"a\\nb\""));
        assert_eq!(layout.nodes[0].descendant_warnings, 2);
        assert_eq!(layout.warnings.len(), 2);
        let mut flat = json(r#"{"a":{"x":1,"x":2},"a":0}"#);
        let layout = Layout::canonical(&flat);
        flat.collapse(1);
        let visible = layout.project(&flat);
        assert!(visible[0]
            .line
            .text
            .ends_with("# WARN Duplicate key; Contains 2 hidden warnings"));
        assert_eq!(layout.nodes[0].descendant_warnings, 4);
    }
    #[test]
    fn root_anchors_and_bounded_preview() {
        let mut flat = json("{\"a\":{\"b\":1},\"c\":2}");
        let layout = Layout::canonical(&flat);
        assert_eq!(layout.nodes[0].line, 0);
        assert!(!layout.nodes[0].collapsible);
        flat.collapse(1);
        let visible = layout.project(&flat);
        assert_eq!(
            visible.iter().map(|line| line.absolute).collect::<Vec<_>>(),
            vec![0, 2]
        );
        // Exercise the parsed-model boundary without involving the tokenizer's
        // unrelated large non-ASCII string behavior.
        let mut flat = json(r#"["x"]"#);
        flat.1 = format!("[\"{}\"]", "界".repeat(100_000));
        let length = flat.1.len();
        flat[0].range = 0..length;
        flat[1].range = 1..length - 1;
        flat[2].range = length - 1..length;
        let layout = Layout::for_view(&flat, 120, &HashSet::new());
        flat.collapse(0);
        let visible = layout.project(&flat);
        assert!(visible[0].line.text.len() < 280);
        assert!(visible[0].line.text.ends_with('…'));
    }
    #[test]
    fn mappings_and_collapse_restore() {
        let mut flat = json(r#"{"tags":[1,2],"rows":[{"a":3},{"a":4}],"obj":{"x":{"y":5}}}"#);
        let layout = Layout::canonical(&flat);
        assert_eq!(layout.nodes[2].line, layout.nodes[3].line);
        assert_ne!(layout.nodes[2].spans, layout.nodes[3].spans);
        flat.collapse(1);
        let visible = layout.project(&flat);
        assert_eq!(visible[0].line.text, "tags[2]: 1,2");
        assert!(visible[0]
            .line
            .spans
            .iter()
            .any(|s| s.role == TokenRole::Number));
        assert!(!visible[0]
            .line
            .spans
            .iter()
            .any(|s| s.role == TokenRole::Preview));
        flat.expand(1);
        assert_eq!(layout.project(&flat)[0].line.text, layout.lines[0].text);
        let mut dup = json(r#"{"a":{"b":{"x":1,"x":2}}}"#);
        let l = Layout::canonical(&dup);
        dup.collapse(2);
        dup.collapse(1);
        assert!(l.project(&dup)[0]
            .line
            .text
            .contains("Contains 2 hidden warnings"));
        dup.expand(1);
        assert!(dup[2].is_collapsed());
        assert_eq!(l.project(&dup).len(), 2);
    }
    #[test]
    fn typed_complex_keys_preserve_quotes_backslashes_and_nested_values() {
        let flat = yaml("? {'quote\"x': true}\n: value\n");
        assert!(
            flat.1.contains(r#""quote"x": true"#),
            "historical copy/search spelling remains unchanged"
        );
        assert_eq!(
            text(&flat),
            r#"? {"quote\"x":true}: value  # WARN Non-string key"#
        );
        let flat = yaml("? ['quote\"x', '\\literal', {'a:b': [true, null, 1]}]\n: value\n");
        assert_eq!(
            text(&flat),
            r#"? ["quote\"x","\\literal",{"a:b":[true,null,1]}]: value  # WARN Non-string key"#
        );
        assert!(!Layout::canonical(&flat)
            .warnings
            .iter()
            .any(|w| w.kind == WarningKind::NonCanonicalNumber));
    }

    #[test]
    fn list_first_field_inline_warnings_keep_element_locators() {
        let flat = yaml("- vals: [.inf, .nan]\n  nested: {}\n");
        assert_eq!(text(&flat), "[1]:\n  - vals[2]: .inf,.nan  # WARN Non-finite number at [0]; Non-finite number at [1]\n    nested:");
    }

    #[test]
    fn collapsed_short_arrays_preserve_inline_value_roles() {
        let mut flat = json(r#"["text",2,true,null,5]"#);
        let layout = Layout::for_view(&flat, 120, &HashSet::from([0]));
        flat.collapse(0);
        let projected = layout.project(&flat);
        let line = &projected[0].line;
        assert_eq!(line.text, "[5]: text,2,true,null,5");
        assert!(!line
            .spans
            .iter()
            .any(|span| span.role == TokenRole::Preview));
        for role in [
            TokenRole::String,
            TokenRole::Number,
            TokenRole::Boolean,
            TokenRole::Null,
        ] {
            assert!(line
                .spans
                .iter()
                .any(|span| span.role == role && span.source.is_some()));
        }
        let narrow = Layout::for_view(&flat, 10, &HashSet::from([0]));
        assert!(narrow.project(&flat)[0]
            .line
            .spans
            .iter()
            .any(|span| span.role == TokenRole::Preview));
        let mut long = json("[1,2,3,4,5,6]");
        let layout = Layout::for_view(&long, 120, &HashSet::new());
        long.collapse(0);
        assert!(layout.project(&long)[0]
            .line
            .spans
            .iter()
            .any(|span| span.role == TokenRole::Preview));
    }

    #[test]
    fn collapsed_table_header_retains_field_style_source_and_identity() {
        let mut flat = json(r#"[{"a":1},{"a":2}]"#);
        let layout = Layout::canonical(&flat);
        flat.collapse(0);
        let projected = layout.project(&flat);
        let keys: Vec<_> = projected[0]
            .line
            .spans
            .iter()
            .filter(|span| matches!(span.role, TokenRole::Key | TokenRole::FieldDefinition))
            .collect();
        assert_eq!(keys.len(), 2);
        assert_ne!(keys[0].node, keys[1].node);
        assert!(keys.iter().all(|span| span.source.is_some()));
        let column = keys[0].range.start;
        let (node, source) = crate::lineprinter::hit_test(&projected[0].line, column);
        assert_eq!(node, keys[0].node);
        assert_eq!(source, keys[0].source.as_ref().map(|range| range.start));
    }

    #[test]
    fn string_matches_map_only_the_rendered_substring_after_quotes_and_escapes() {
        for input in [
            r#"{"value":"aaaaaaaaNEEDLE"}"#,
            r#"{"value":"\u754c\n\"\\\ud83d\ude00NEEDLE"}"#,
        ] {
            let flat = json(input);
            let layout = Layout::canonical(&flat);
            let source = flat.1.find("NEEDLE").unwrap();
            let span = layout.lines[0]
                .spans
                .iter()
                .find(|span| span.role == TokenRole::String)
                .unwrap();
            let ranges = span.matching_ranges(&(source..source + 6));
            assert_eq!(
                ranges
                    .iter()
                    .map(|range| &layout.lines[0].text[range.clone()])
                    .collect::<String>(),
                "NEEDLE"
            );
        }
        let flat = json(r#""\u0061b\u0063""#);
        let layout = Layout::canonical(&flat);
        let span = &layout.lines[0].spans[0];
        assert_eq!(span.matching_ranges(&(1..7)), vec![0..1]);
        assert_eq!(layout.lines[0].text, "abc");
    }

    #[test]
    fn large_plain_string_mapping_coalesces_without_character_storage() {
        let input = format!("\"{}NEEDLE\"", "a".repeat(1_000_000));
        // Exercise display allocation directly, independently of parser throughput.
        let mut flat = json(r#""""#);
        flat.1 = input;
        flat.0[0].range = 0..flat.1.len();
        let layout = Layout::canonical(&flat);
        let span = &layout.lines[0].spans[0];
        assert_eq!(span.source_map.len(), 1);
        assert_eq!(
            span.matching_ranges(&(1_000_001..1_000_007)),
            vec![1_000_000..1_000_006]
        );
    }

    #[test]
    fn unsupported_control_escapes_are_warned_but_literal_escape_text_is_not() {
        let flat = json(r#"{"control":"\u0001","literal":"\\u0001","line":"\n"}"#);
        let layout = Layout::canonical(&flat);
        assert_eq!(
            layout.lines[0].text,
            r#"control: "\u0001"  # WARN Non-standard string escape"#
        );
        assert_eq!(
            layout
                .warnings
                .iter()
                .filter(|warning| warning.kind == WarningKind::NonStandardStringEscape)
                .count(),
            1
        );
        assert!(!text(&flat).chars().any(|ch| ch == '\u{1}'));
        let key = json(r#"{"\u0001":1}"#);
        assert!(text(&key).contains("# WARN Non-standard string escape"));
        let key = yaml("? [\"\\u0001\"]\n: value\n");
        assert!(text(&key).contains("Non-string key; Non-standard string escape"));
    }
    #[test]
    fn mixed_warning_kinds_follow_node_order_and_hidden_summary_is_last() {
        let flat = yaml(r#"["\u0001", .inf, 1e1000000]"#);
        let layout = Layout::canonical(&flat);
        assert_eq!(
            layout.lines[0].text,
            r#"[3]: "\u0001",.inf,1e1000000  # WARN Non-standard string escape at [0]; Non-finite number at [1]; Non-canonical number at [2]"#
        );
        assert_eq!(
            layout
                .warnings
                .iter()
                .map(|warning| warning.kind)
                .collect::<Vec<_>>(),
            vec![
                WarningKind::NonStandardStringEscape,
                WarningKind::NonFiniteNumber,
                WarningKind::NonCanonicalNumber
            ]
        );
        let mut flat = yaml(".inf: {a: .inf}");
        let layout = Layout::canonical(&flat);
        flat.collapse(1);
        assert!(layout.project(&flat)[0]
            .line
            .text
            .ends_with("# WARN Non-finite number; Non-string key; Contains 1 hidden warnings"));
    }
}
