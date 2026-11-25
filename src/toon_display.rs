//! TOON document layout over parsed row identities, independent of the optional codec.
use crate::flatjson::{FlatJson, Value};
use std::ops::Range;

pub(crate) mod alignment;
#[cfg(test)]
pub mod fixture;
mod format;
mod geometry;
mod index;
pub mod layout;
mod line_index;
mod number;
mod preview;
mod scalar_text;
mod source_map;

use number::number;
use preview::{Preview, bounded_prefix, preview_append};
use scalar_text::{
    bare_key, compact_key, decode_string, quote_json, quote_json_into, quote_key,
    value_needs_quotes,
};
use source_map::string_source_map;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenRole {
    Key,
    FieldDefinition,
    String,
    Number,
    Boolean,
    Null,
    ArrayIndex,
    PrimitiveTrailingComma,
    ContainerDelimiter,
    EmptyContainer,
    Punctuation,
    Warning,
    Preview,
    Count,
    DocumentPosition,
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
    /// Coalesced shared-header display highlights; bool marks the current match.
    pub shared_matches: Vec<(Range<usize>, bool)>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WarningKind {
    DuplicateKey,
    NonFiniteNumber,
    NonCanonicalNumber,
    NonStringKey,
}
impl WarningKind {
    fn message(self) -> &'static str {
        match self {
            Self::DuplicateKey => "Duplicate key",
            Self::NonFiniteNumber => "Non-finite number",
            Self::NonCanonicalNumber => "Non-canonical number",
            Self::NonStringKey => "Non-string key",
        }
    }
}

pub fn normalize_node(flat: &FlatJson, node: usize) -> usize {
    if flat[node].is_closing_of_container() {
        flat[node].pair_index().unwrap()
    } else {
        node
    }
}

#[cfg(test)]
fn children(flat: &FlatJson, node: usize) -> Vec<usize> {
    let mut result = Vec::new();
    let mut next = flat[node].first_child();
    while let Some(i) = next.as_option() {
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
            shared_matches: Vec::new(),
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

#[cfg(test)]
mod tests;
