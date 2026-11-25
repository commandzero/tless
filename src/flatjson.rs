use crate::chunked_vec::ChunkedVec;
use std::collections::{HashMap, HashSet};
use std::ops::Range;

use crate::jsonparser;
use crate::yamlparser;

mod path;
mod presentation;
#[cfg(feature = "sexp")]
mod sexp;

pub type Index = usize;

/// A single-word optional node link. `usize::MAX` cannot index a Rust allocation.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OptionIndex(Index);

impl OptionIndex {
    pub const NIL: Self = Self(NIL);

    pub fn as_option(&self) -> Option<Index> {
        (!self.is_nil()).then_some(self.0)
    }

    pub fn is_nil(&self) -> bool {
        self.0 == NIL
    }

    pub fn is_some(&self) -> bool {
        !self.is_nil()
    }

    pub fn unwrap(&self) -> Index {
        assert!(!self.is_nil(), "Called .unwrap() on Nil OptionIndex");
        self.0
    }
}

pub const NIL: usize = usize::MAX;

impl From<usize> for OptionIndex {
    fn from(i: usize) -> Self {
        Self(i)
    }
}

#[derive(PartialEq, Copy, Clone)]
pub enum PathType {
    Dot,
    Bracket,
    Query,
    // Just used for the status bar.
    DotWithTopLevelIndex,
}

#[derive(Debug)]
pub struct FlatJson(
    pub ChunkedVec<Row>,
    // Single-line pretty printed version of the JSON.
    // Rows will contain references into this.
    pub String,
    // Max nesting depth.
    pub usize,
    pub DocumentMetadata,
);

/// Parsed values that cannot be recovered from the JSON spelling. Ordinary JSON
/// nodes pay no per-node cost for YAML's typed keys and decoded strings.
#[derive(Debug, Default)]
pub struct DocumentMetadata {
    pub keys: HashMap<Index, KeyValue>,
    pub strings: HashMap<Index, String>,
    collapsed: HashSet<Index>,
}

impl FlatJson {
    pub fn key_value(&self, index: Index) -> Option<&KeyValue> {
        self.3.keys.get(&index)
    }

    pub fn string_value(&self, index: Index) -> Option<&str> {
        self.3.strings.get(&index).map(String::as_str)
    }

    pub fn has_collapsed(&self) -> bool {
        !self.3.collapsed.is_empty()
    }

    pub fn collapsed_nodes(&self) -> impl Iterator<Item = Index> + '_ {
        self.3.collapsed.iter().copied()
    }
    #[cfg(test)]
    pub fn prev_visible_row(&self, index: Index) -> OptionIndex {
        if index == 0 {
            return OptionIndex::NIL;
        }

        let row = &self.0[index - 1];

        if row.is_closing_of_container() && row.is_collapsed() {
            row.pair_index()
        } else {
            OptionIndex::from(index - 1)
        }
    }

    #[cfg(test)]
    pub fn next_visible_row(&self, mut index: Index) -> OptionIndex {
        // If row is collapsed container, jump to closing char and move past there.
        if self.0[index].is_opening_of_container() && self.0[index].is_collapsed() {
            index = self.0[index].pair_index().unwrap();
        }

        // We can always go to the next row, unless we're at the end of the file.
        if index == self.0.len() - 1 {
            return OptionIndex::NIL;
        }

        OptionIndex::from(index + 1)
    }

    #[cfg(test)]
    pub fn prev_item(&self, mut index: Index) -> OptionIndex {
        while let Some(i) = self.prev_visible_row(index).as_option() {
            if !self.0[i].is_closing_of_container() {
                return OptionIndex::from(i);
            }

            index = i;
        }

        OptionIndex::NIL
    }

    #[cfg(test)]
    pub fn next_item(&self, mut index: Index) -> OptionIndex {
        while let Some(i) = self.next_visible_row(index).as_option() {
            if !self.0[i].is_closing_of_container() {
                return OptionIndex::from(i);
            }

            index = i;
        }

        OptionIndex::NIL
    }

    pub fn expand(&mut self, index: Index) {
        let open = match self[index].value {
            Value::CloseContainer { open_index, .. } => open_index,
            _ => index,
        };
        self.3.collapsed.remove(&open);
        if let Some(pair) = self.0[index].pair_index().as_option() {
            self.0[pair].expand();
        }
        self.0[index].expand();
    }

    pub fn collapse(&mut self, index: Index) {
        if self[index].is_container() {
            let open = match self[index].value {
                Value::CloseContainer { open_index, .. } => open_index,
                _ => index,
            };
            self.3.collapsed.insert(open);
        }
        if let Some(pair) = self.0[index].pair_index().as_option() {
            self.0[pair].collapse();
        }
        self.0[index].collapse();
    }

    /// Inclusive flat-row span of a value, accepting its opening or closing row.
    pub fn subtree_range(&self, index: Index) -> std::ops::RangeInclusive<Index> {
        let start = if self[index].is_closing_of_container() {
            self[index].pair_index().unwrap()
        } else {
            index
        };
        let end = self[start].pair_index().as_option().unwrap_or(start);
        start..=end
    }
}

impl std::ops::Index<usize> for FlatJson {
    type Output = Row;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for FlatJson {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

/// Typed YAML keys are retained beside the historical flattened copy/search spelling.
/// Flattened complex keys are not a parseable interchange representation.
#[derive(Clone, Debug)]
pub enum KeyValue {
    String(String),
    Number(String),
    Boolean(bool),
    Null,
    Array(Vec<KeyValue>),
    Object(Vec<(KeyValue, KeyValue)>),
}

#[derive(Debug)]
pub struct Row {
    pub parent: OptionIndex,
    // Should these also be set on the CloseContainers?
    pub prev_sibling: OptionIndex,
    pub next_sibling: OptionIndex,

    pub depth: usize,
    pub index_in_parent: usize,
    pub range: Range<usize>,
    pub key_range: Option<Range<usize>>,
    pub value: Value,
}

impl Row {
    pub fn is_primitive(&self) -> bool {
        self.value.is_primitive()
    }
    pub fn is_container(&self) -> bool {
        self.value.is_container()
    }
    pub fn is_string(&self) -> bool {
        self.value.is_string()
    }
    pub fn is_opening_of_container(&self) -> bool {
        self.value.is_opening_of_container()
    }
    pub fn is_closing_of_container(&self) -> bool {
        self.value.is_closing_of_container()
    }
    pub fn is_collapsed(&self) -> bool {
        self.value.is_collapsed()
    }
    pub fn is_expanded(&self) -> bool {
        self.value.is_expanded()
    }
    pub fn is_array(&self) -> bool {
        self.value.is_array()
    }

    fn expand(&mut self) {
        self.value.expand()
    }
    fn collapse(&mut self) {
        self.value.collapse()
    }

    pub fn first_child(&self) -> OptionIndex {
        self.value.first_child()
    }
    pub fn last_child(&self) -> OptionIndex {
        self.value.last_child()
    }
    pub fn pair_index(&self) -> OptionIndex {
        self.value.pair_index()
    }

    // The range of what the row represents on the screen. If the row is
    // a container, and it is collapsed, this includes the entire range
    // of the container, but if it is expanded, it just represents the
    // single opening character.
    //
    // This also includes the key range for objects.
    pub fn range_represented_by_row(&self) -> Range<usize> {
        let start = match &self.key_range {
            Some(key_range) => key_range.start,
            None => self.range.start,
        };

        let end = if self.is_container() && self.is_expanded() {
            self.range.start + 1
        } else {
            self.range.end
        };

        start..end
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ContainerType {
    Object,
    Array,
}

impl ContainerType {
    pub fn open_str(&self) -> &'static str {
        match self {
            ContainerType::Object => "{",
            ContainerType::Array => "[",
        }
    }

    pub fn close_str(&self) -> &'static str {
        match self {
            ContainerType::Object => "}",
            ContainerType::Array => "]",
        }
    }
}

#[derive(Debug)]
pub enum Value {
    Null,
    Boolean,
    Number,
    String,
    EmptyObject,
    EmptyArray,
    OpenContainer {
        container_type: ContainerType,
        collapsed: bool,
        first_child: Index,
        close_index: Index,
    },
    CloseContainer {
        container_type: ContainerType,
        collapsed: bool,
        last_child: Index,
        open_index: Index,
    },
}

impl Value {
    pub fn is_primitive(&self) -> bool {
        !self.is_container()
    }

    pub fn is_container(&self) -> bool {
        matches!(
            self,
            Value::OpenContainer { .. } | Value::CloseContainer { .. }
        )
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Value::String)
    }

    pub fn container_type(&self) -> Option<ContainerType> {
        match self {
            Value::OpenContainer { container_type, .. } => Some(*container_type),
            Value::CloseContainer { container_type, .. } => Some(*container_type),
            _ => None,
        }
    }

    pub fn is_opening_of_container(&self) -> bool {
        matches!(self, Value::OpenContainer { .. })
    }

    pub fn is_closing_of_container(&self) -> bool {
        matches!(self, Value::CloseContainer { .. })
    }

    pub fn is_collapsed(&self) -> bool {
        match self {
            Value::OpenContainer { collapsed, .. } => *collapsed,
            Value::CloseContainer { collapsed, .. } => *collapsed,
            _ => false,
        }
    }

    pub fn is_expanded(&self) -> bool {
        !self.is_collapsed()
    }

    pub fn is_array(&self) -> bool {
        matches!(
            self,
            Value::OpenContainer {
                container_type: ContainerType::Array,
                ..
            } | Value::CloseContainer {
                container_type: ContainerType::Array,
                ..
            }
        )
    }

    fn expand(&mut self) {
        self.set_collapsed(false)
    }

    fn collapse(&mut self) {
        self.set_collapsed(true)
    }

    fn set_collapsed(&mut self, val: bool) {
        match self {
            Value::OpenContainer { collapsed, .. } => *collapsed = val,
            Value::CloseContainer { collapsed, .. } => *collapsed = val,
            _ => {}
        }
    }

    fn first_child(&self) -> OptionIndex {
        match self {
            Value::OpenContainer { first_child, .. } => OptionIndex::from(*first_child),
            _ => OptionIndex::NIL,
        }
    }

    fn last_child(&self) -> OptionIndex {
        match self {
            Value::CloseContainer { last_child, .. } => OptionIndex::from(*last_child),
            _ => OptionIndex::NIL,
        }
    }

    fn pair_index(&self) -> OptionIndex {
        match self {
            Value::OpenContainer { close_index, .. } => OptionIndex::from(*close_index),
            Value::CloseContainer { open_index, .. } => OptionIndex::from(*open_index),
            _ => OptionIndex::NIL,
        }
    }
}

pub fn parse_top_level_json(json: String) -> Result<FlatJson, String> {
    let (rows, pretty, depth) = jsonparser::parse(json)?;
    Ok(FlatJson(rows, pretty, depth, DocumentMetadata::default()))
}

pub fn parse_top_level_yaml(yaml: String) -> Result<FlatJson, String> {
    let (rows, pretty, depth, values) = yamlparser::parse(yaml)?;
    Ok(FlatJson(rows, pretty, depth, values))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;

    const OBJECT: &str = r#"{
        "1": 1,
        "2": [
            3,
            "4"
        ],
        "6": {
            "7": null,
            "8": true,
            "9": 9
        },
        "11": 11
    }"#;

    const NESTED_OBJECT: &str = r#"[
        {
            "2": [
                3
            ],
            "5": {
                "6": 6
            }
        }
    ]"#;

    #[test]
    fn test_flatten_json() {
        let fj = parse_top_level_json(OBJECT.to_owned()).unwrap();

        assert_flat_json_fields(
            "parent",
            &fj,
            vec![NIL, 0, 0, 2, 2, 0, 0, 6, 6, 6, 0, 0, NIL],
            |elem| elem.parent,
        );

        assert_flat_json_fields(
            "prev_sibling",
            &fj,
            vec![NIL, NIL, 1, NIL, 3, NIL, 2, NIL, 7, 8, NIL, 6, NIL],
            |elem| elem.prev_sibling,
        );

        assert_flat_json_fields(
            "next_sibling",
            &fj,
            vec![NIL, 2, 6, 4, NIL, NIL, 11, 8, 9, NIL, NIL, NIL, NIL],
            |elem| elem.next_sibling,
        );

        assert_flat_json_fields(
            "first_child",
            &fj,
            vec![1, NIL, 3, NIL, NIL, NIL, 7, NIL, NIL, NIL, NIL, NIL, NIL],
            |elem| elem.first_child(),
        );

        assert_flat_json_fields(
            "last_child",
            &fj,
            vec![NIL, NIL, NIL, NIL, NIL, 4, NIL, NIL, NIL, NIL, 9, NIL, 11],
            |elem| elem.last_child(),
        );

        assert_flat_json_fields(
            "{open,close}_index",
            &fj,
            vec![12, NIL, 5, NIL, NIL, 2, 10, NIL, NIL, NIL, 6, NIL, 0],
            |elem| elem.pair_index(),
        );

        assert_flat_json_fields(
            "depth",
            &fj,
            vec![0, 1, 1, 2, 2, 1, 1, 2, 2, 2, 1, 1, 0],
            |elem| OptionIndex::from(elem.depth),
        );
    }

    fn assert_flat_json_fields<T: Into<OptionIndex> + Debug + Copy>(
        field: &'static str,
        fj: &FlatJson,
        field_values: Vec<T>,
        accessor_fn: fn(&Row) -> OptionIndex,
    ) {
        assert_eq!(
            fj.0.len(),
            field_values.len(),
            "length of flat json and field_values don't match",
        );

        for (i, (elem, expected_value)) in fj.0.iter().zip(field_values.iter()).enumerate() {
            assert_eq!(
                accessor_fn(elem),
                Into::<OptionIndex>::into(*expected_value),
                "incorrect {field} at index {i}",
            );
        }
    }

    #[test]
    fn test_move_by_visible_rows_simple() {
        let fj = parse_top_level_json(OBJECT.to_owned()).unwrap();

        assert_visited_rows(&fj, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, NIL]);
    }

    #[test]
    fn test_move_by_visible_rows_collapsed() {
        let mut fj = parse_top_level_json(NESTED_OBJECT.to_owned()).unwrap();

        for (index, expected) in [
            (2, &[1, 2, 5, 6, 7, 8, 9, NIL][..]),
            (5, &[1, 2, 5, 8, 9, NIL]),
            (1, &[1, 9, NIL]),
            (0, &[NIL]),
        ] {
            fj.collapse(index);
            assert_visited_rows(&fj, expected.to_vec());
        }
    }

    #[test]
    fn test_move_by_items_simple() {
        let fj = parse_top_level_json(OBJECT.to_owned()).unwrap();
        assert_visited_items(&fj, vec![1, 2, 3, 4, 6, 7, 8, 9, 11, NIL]);

        let fj = parse_top_level_json(NESTED_OBJECT.to_owned()).unwrap();
        assert_visited_items(&fj, vec![1, 2, 3, 5, 6, NIL]);
    }

    #[test]
    fn test_move_by_items_collapsed() {
        let mut fj = parse_top_level_json(NESTED_OBJECT.to_owned()).unwrap();

        fj.collapse(2);
        assert_visited_items(&fj, vec![1, 2, 5, 6, NIL]);

        fj.collapse(5);
        assert_visited_items(&fj, vec![1, 2, 5, NIL]);

        fj.expand(2);
        assert_visited_items(&fj, vec![1, 2, 3, 5, NIL]);

        fj.collapse(1);
        assert_visited_items(&fj, vec![1, NIL]);

        fj.collapse(0);
        assert_visited_items(&fj, vec![NIL]);
    }

    fn assert_row_iter(
        fj: &FlatJson,
        start: Index,
        expected: &[Index],
        movement: fn(&FlatJson, Index) -> OptionIndex,
    ) {
        let mut current = start;
        for &expected in expected {
            let next = movement(fj, current).as_usize();
            assert_eq!(next, expected, "moving from row {current}");
            current = next;
        }
    }

    fn assert_visited_rows(fj: &FlatJson, expected: Vec<usize>) {
        assert_visited(
            fj,
            expected,
            FlatJson::next_visible_row,
            FlatJson::prev_visible_row,
        );
    }

    fn assert_visited_items(fj: &FlatJson, expected: Vec<usize>) {
        assert_visited(fj, expected, FlatJson::next_item, FlatJson::prev_item);
    }

    fn assert_visited(
        flat: &FlatJson,
        mut expected: Vec<Index>,
        next: fn(&FlatJson, Index) -> OptionIndex,
        prev: fn(&FlatJson, Index) -> OptionIndex,
    ) {
        assert_row_iter(flat, 0, &expected, next);
        expected.pop();
        let start = expected.pop().unwrap_or(0);
        if start != 0 {
            expected.reverse();
            expected.push(0);
        }
        expected.push(NIL);
        assert_row_iter(flat, start, &expected, prev);
    }
}
