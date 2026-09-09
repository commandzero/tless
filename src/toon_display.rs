//! TOON document layout over parsed row identities, independent of the optional codec.
use crate::flatjson::{FlatJson, OptionIndex, Value};
use std::collections::{HashMap, HashSet};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenRole {
    Key,
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
}
impl WarningKind {
    fn message(self) -> &'static str {
        match self {
            Self::DuplicateKey => "Duplicate key",
            Self::NonFiniteNumber => "Non-finite number",
            Self::NonCanonicalNumber => "Non-canonical number",
            Self::NonStringKey => "Non-string key",
            Self::MultipleRoots => "Multiple document roots",
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
        });
    }
}
impl Layout {
    pub fn new(flat: &FlatJson) -> Self {
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
        };
        for (i, row) in flat.0.iter().enumerate() {
            if row.is_closing_of_container() {
                continue;
            }
            if let Some(range) = &row.key_range {
                let raw = &flat.1[range.clone()];
                if raw.starts_with('"') {
                    result.keys[i] = Some(decode_string(raw));
                } else {
                    result.own_warnings[i].push(WarningKind::NonStringKey);
                    let inner = raw
                        .strip_prefix('[')
                        .and_then(|s| s.strip_suffix(']'))
                        .unwrap_or(raw);
                    let (text, warnings) = compact_key(inner);
                    result.keys[i] = Some(format!("? {text}"));
                    result.own_warnings[i].extend(warnings);
                }
            }
            result.scalars[i] = match row.value {
                Value::String => quote_value(&decode_string(&flat.1[row.range.clone()])),
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
                // Decode directly because children's cached keys have not been built yet.
                for &child in &kids {
                    if let Some(range) = &flat[child].key_range {
                        let raw = &flat.1[range.clone()];
                        if raw.starts_with('"') {
                            *counts.entry(decode_string(raw)).or_insert(0usize) += 1;
                        }
                    }
                }
                let mut ordinal = HashMap::new();
                for child in kids {
                    if let Some(range) = &flat[child].key_range {
                        let raw = &flat.1[range.clone()];
                        if raw.starts_with('"') {
                            let key = decode_string(raw);
                            if counts[&key] > 1 {
                                let occurrence = ordinal.entry(key).or_insert(0);
                                *occurrence += 1;
                                result.nodes[child].occurrence = Some(*occurrence);
                                result.nodes[child].occurrence_total =
                                    Some(counts[&decode_string(raw)]);
                                result.own_warnings[child].push(WarningKind::DuplicateKey);
                            }
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
            for span in &result.lines[line_idx].spans {
                result.nodes[span.node]
                    .spans
                    .push((line_idx, span.range.clone()));
            }
        }
        result
            .warnings
            .sort_by_key(|warning| (warning.node, warning.kind));
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
                            TokenRole::Key,
                            flat[field].key_range.clone(),
                        );
                        let range = begin..line.text.len();
                        for row in &table_fields[1..] {
                            let other = row[column];
                            line.spans.push(Span {
                                range: range.clone(),
                                node: other,
                                role: TokenRole::Key,
                                source: flat[other].key_range.clone(),
                            });
                        }
                    }
                    line.token("}", node, TokenRole::Structure, None);
                }
                line.token(":", node, TokenRole::Structure, None);
                self.nodes[node].header_end = line.text.len();
                if kids.iter().all(|&i| scalar(flat, i)) {
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
                preview.push(',');
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
            .filter(|s| s.role != TokenRole::Key || self.nodes[s.node].line == line)
            .map(|s| s.node)
            .collect();
        ids.push(self.lines[line].owner);
        ids.sort_unstable();
        ids.dedup();
        let owner = self.lines[line].owner;
        let mut messages = vec![];
        for node in ids {
            for warning in &self.own_warnings[node] {
                let locator = if node != owner && flat[node].parent == OptionIndex::Index(owner) {
                    if flat[owner].is_array() {
                        format!(" at [{}]", flat[node].index_in_parent)
                    } else if let Some(key) = &self.keys[node] {
                        format!(" at field {}", quote_json(key))
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
                    line.spans.retain(|s| {
                        s.range.end <= info.header_end
                            && (s.node == node || s.role == TokenRole::Structure)
                    });
                    line.owner = node;
                    if !flat[node].is_array() {
                        let count = info.entry_count;
                        line.token(
                            &format!(" {count} {}", if count == 1 { "entry" } else { "entries" }),
                            node,
                            TokenRole::Count,
                            None,
                        );
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
                    if !self.previews[node].is_empty() {
                        line.token(
                            &format!(" {}", self.previews[node]),
                            node,
                            TokenRole::Preview,
                            None,
                        );
                    }
                    if !messages.is_empty() {
                        line.token(
                            &format!("  # WARN {}", messages.join("; ")),
                            node,
                            TokenRole::Warning,
                            None,
                        );
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

/// The parser wraps non-string object keys in an extra pair of brackets.
/// Read that notation directly, retaining ordered object pairs and scalar types.
fn compact_key(raw: &str) -> (String, Vec<WarningKind>) {
    struct Reader<'a> {
        text: &'a str,
        offset: usize,
        warnings: Vec<WarningKind>,
    }
    impl Reader<'_> {
        fn whitespace(&mut self) {
            while self
                .text
                .as_bytes()
                .get(self.offset)
                .is_some_and(|b| b.is_ascii_whitespace())
            {
                self.offset += 1;
            }
        }
        fn value(&mut self, key: bool) -> String {
            self.whitespace();
            if key && self.text.as_bytes().get(self.offset) == Some(&b'[') {
                self.offset += 1;
                let value = self.value(false);
                self.whitespace();
                if self.text.as_bytes().get(self.offset) == Some(&b']') {
                    self.offset += 1;
                }
                return value;
            }
            match self.text.as_bytes().get(self.offset) {
                Some(b'"') => {
                    let start = self.offset;
                    self.offset += 1;
                    while let Some(&b) = self.text.as_bytes().get(self.offset) {
                        self.offset += 1;
                        if b == b'\\' {
                            self.offset = (self.offset + 1).min(self.text.len());
                        } else if b == b'"' {
                            break;
                        }
                    }
                    quote_json(&decode_string(&self.text[start..self.offset]))
                }
                Some(b'[') | Some(b'{') => {
                    let object = self.text.as_bytes()[self.offset] == b'{';
                    self.offset += 1;
                    let close = if object { b'}' } else { b']' };
                    let mut items = vec![];
                    loop {
                        self.whitespace();
                        if self.text.as_bytes().get(self.offset) == Some(&close) {
                            self.offset += 1;
                            break;
                        }
                        if self.offset >= self.text.len() {
                            break;
                        }
                        let begin = self.offset;
                        let mut item = self.value(object);
                        if object {
                            self.whitespace();
                            if self.text.as_bytes().get(self.offset) == Some(&b':') {
                                self.offset += 1;
                            }
                            item.push(':');
                            item.push_str(&self.value(false));
                        }
                        items.push(item);
                        self.whitespace();
                        if self.text.as_bytes().get(self.offset) == Some(&b',') {
                            self.offset += 1;
                        } else if self.text.as_bytes().get(self.offset) != Some(&close)
                            || self.offset == begin
                        {
                            break;
                        }
                    }
                    format!(
                        "{}{}{}",
                        if object { '{' } else { '[' },
                        items.join(","),
                        close as char
                    )
                }
                _ => {
                    let start = self.offset;
                    while self
                        .text
                        .as_bytes()
                        .get(self.offset)
                        .is_some_and(|b| !b.is_ascii_whitespace() && !b",]}:".contains(b))
                    {
                        self.offset += 1;
                    }
                    let token = &self.text[start..self.offset];
                    if matches!(token, "true" | "false" | "null") {
                        token.to_owned()
                    } else {
                        let (text, warning) = number(token);
                        self.warnings.extend(warning);
                        text
                    }
                }
            }
        }
    }
    let mut reader = Reader {
        text: raw,
        offset: 0,
        warnings: vec![],
    };
    let text = reader.value(false);
    (text, reader.warnings)
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
        Layout::new(flat)
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
        let layout = Layout::new(&flat);
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
    fn shared_header_mapping_and_table_row_collapse() {
        let mut flat = yaml("- a: .inf\n  b: 2\n- a: .nan\n  b: 4\n");
        let layout = Layout::new(&flat);
        let rows = children(&flat, 0);
        let fields = children(&flat, rows[1]);
        let key_span = layout.lines[0]
            .spans
            .iter()
            .find(|span| span.node == fields[0] && span.role == TokenRole::Key)
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
        assert!(collapsed[2]
            .line
            .text
            .starts_with("   2 entries a: .nan,b: 4"));
        assert!(collapsed[2]
            .line
            .text
            .ends_with("# WARN Contains 1 hidden warnings"));
        assert!(!collapsed[2]
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
        let layout = Layout::new(&flat);
        assert!(layout.lines[1].text.ends_with("at field \"a\\nb\""));
        assert_eq!(layout.nodes[0].descendant_warnings, 2);
        assert_eq!(layout.warnings.len(), 2);
        let mut flat = json(r#"{"a":{"x":1,"x":2},"a":0}"#);
        let layout = Layout::new(&flat);
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
        let layout = Layout::new(&flat);
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
        let layout = Layout::new(&flat);
        flat.collapse(0);
        let visible = layout.project(&flat);
        assert!(visible[0].line.text.len() < 280);
        assert!(visible[0].line.text.ends_with('…'));
    }
    #[test]
    fn mappings_and_collapse_restore() {
        let mut flat = json(r#"{"tags":[1,2],"rows":[{"a":3},{"a":4}],"obj":{"x":{"y":5}}}"#);
        let layout = Layout::new(&flat);
        assert_eq!(layout.nodes[2].line, layout.nodes[3].line);
        assert_ne!(layout.nodes[2].spans, layout.nodes[3].spans);
        flat.collapse(1);
        let visible = layout.project(&flat);
        assert_eq!(visible[0].line.text, "tags[2]: 1,2");
        assert!(visible[0]
            .line
            .spans
            .iter()
            .any(|s| s.role == TokenRole::Preview));
        flat.expand(1);
        assert_eq!(layout.project(&flat)[0].line.text, layout.lines[0].text);
        let mut dup = json(r#"{"a":{"b":{"x":1,"x":2}}}"#);
        let l = Layout::new(&dup);
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
}
