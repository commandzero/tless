//! Project matches from equivalent table fields onto their shared header.
use super::{Analysis, DisplayLine, FlatJson, Span, TokenRole};
use crate::toon_display::index::same_field_path;
use crate::toon_display::source_map::{mapped_range, parsed_string, string_source_map};
use std::ops::Range;

pub(in crate::toon_display) fn highlight_shared_fields(
    flat: &FlatJson,
    analysis: &Analysis,
    line: &mut DisplayLine,
    (matches, current): (&[Range<usize>], &Range<usize>),
) {
    line.shared_matches.clear();
    if matches.is_empty() && current.is_empty() {
        return;
    }
    let fields: Vec<_> = line
        .spans
        .iter()
        .filter(|span| span.role == TokenRole::FieldDefinition)
        .collect();
    let Some(first) = fields.first() else {
        return;
    };
    let table = analysis.table_owner(flat, first.node).unwrap();
    let mut header = HeaderMatches {
        flat,
        analysis,
        table,
        fields,
        text: &line.text,
        covered: vec![0; line.text.len()],
    };
    header.collect(matches, current);
    line.shared_matches = header.ranges();
}

struct HeaderMatches<'a> {
    flat: &'a FlatJson,
    analysis: &'a Analysis,
    table: usize,
    fields: Vec<&'a Span>,
    text: &'a str,
    covered: Vec<u8>,
}

impl<'a> HeaderMatches<'a> {
    fn collect(&mut self, matches: &[Range<usize>], current: &Range<usize>) {
        let range = &self.flat[self.table].range;
        let start = matches.partition_point(|query| query.end <= range.start);
        let end = matches.partition_point(|query| query.start < range.end);
        for query in &matches[start..end] {
            self.mark_query(query, 1);
        }
        self.mark_query(current, 2);
    }

    fn mark_query(&mut self, query: &Range<usize>, flag: u8) {
        if query.is_empty() {
            return;
        }
        let rows = &self.flat.0;
        let start = rows
            .partition_point(|row| row.range_represented_by_row().start <= query.start)
            .saturating_sub(1);
        let end = rows.partition_point(|row| row.range_represented_by_row().start < query.end);
        for node in start..end {
            self.mark_field(node, query, flag);
        }
    }

    fn mark_field(&mut self, node: usize, query: &Range<usize>, flag: u8) {
        let Some((column, source)) = self.matching_field(node, query) else {
            return;
        };
        if self.covered[column.range.clone()]
            .iter()
            .all(|mask| mask & flag != 0)
        {
            return;
        }
        let raw = &self.flat.1[source.clone()];
        let rendered = &self.text[column.range.clone()];
        let mut coverage = Coverage {
            bytes: &mut self.covered,
            query,
            flag,
        };

        if raw == rendered {
            coverage.mark(source, &column.range);
        } else if stripped_quotes(raw) == Some(rendered) {
            coverage.mark(&(source.start + 1..source.end - 1), &column.range);
        } else {
            let parsed = parsed_string(self.flat, node, TokenRole::FieldDefinition);
            let maps = string_source_map(raw, parsed, rendered, (source.start, column.range.start));
            if maps.is_empty() {
                coverage.mark(source, &column.range);
            }
            for map in maps {
                coverage.mark(&map.source, &map.display);
            }
        }
    }

    fn matching_field(
        &self,
        node: usize,
        query: &Range<usize>,
    ) -> Option<(&'a Span, &'a Range<usize>)> {
        let row = self.analysis.table_row_owner(self.flat, node)?;
        if row == node || self.flat[row].parent.as_option() != Some(self.table) {
            return None;
        }
        let source = self.flat[node].key_range.as_ref()?;
        if source.start >= query.end || query.start >= source.end {
            return None;
        }
        let column = self
            .fields
            .iter()
            .copied()
            .find(|span| same_field_path(self.flat, span.node, node))?;
        Some((column, source))
    }

    fn ranges(self) -> Vec<(Range<usize>, bool)> {
        let mut result = Vec::new();
        let mut start = 0;
        for run in self.covered.chunk_by(|a, b| a == b) {
            let end = start + run.len();
            if run[0] != 0 {
                result.push((start..end, run[0] & 2 != 0));
            }
            start = end;
        }
        result
    }
}

struct Coverage<'a> {
    bytes: &'a mut [u8],
    query: &'a Range<usize>,
    flag: u8,
}

impl Coverage<'_> {
    fn mark(&mut self, source: &Range<usize>, display: &Range<usize>) {
        if source.start < self.query.end && self.query.start < source.end {
            for mask in &mut self.bytes[mapped_range(source, display, self.query)] {
                *mask |= self.flag;
            }
        }
    }
}

fn stripped_quotes(raw: &str) -> Option<&str> {
    raw.strip_prefix('"').and_then(|raw| raw.strip_suffix('"'))
}
