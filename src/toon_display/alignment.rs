//! Enabled-table measurements, independent of formatted viewport rows.
use super::geometry::{Geometry, Kind, key_text, value_text};
use super::index::{Analysis, children, key, string};
use super::{FlatJson, Value, quote_json, quote_json_into, value_needs_quotes};
use unicode_width::UnicodeWidthStr;

pub struct TableMetrics {
    pub widths: Vec<usize>,
    // The final token and its annotation belong to the same row; taking
    // independent maxima here would invent an unreachable horizontal extent.
    max_row_tail: usize,
    header_tail: usize,
}

impl TableMetrics {
    pub fn measure(flat: &FlatJson, analysis: &Analysis, table: usize) -> Self {
        let first = children(flat, table)
            .next()
            .expect("eligible table has rows");
        let mut widths: Vec<_> = children(flat, first)
            .map(|field| UnicodeWidthStr::width(key_text(flat, field).as_ref()))
            .collect();
        let header_tail =
            widths.last().copied().unwrap() + 2 + annotation_width(flat, analysis, table, false);
        let mut max_row_tail = 0;
        let mut quoted = String::new();
        for row in children(flat, table) {
            let mut last_width = 0;
            for (column, cell) in children(flat, row).enumerate() {
                let width = if matches!(flat[cell].value, Value::String) {
                    let value = string(flat, cell);
                    if value_needs_quotes(&value) {
                        quote_json_into(&value, &mut quoted);
                        UnicodeWidthStr::width(quoted.as_str())
                    } else {
                        UnicodeWidthStr::width(value.as_ref())
                    }
                } else {
                    UnicodeWidthStr::width(value_text(flat, cell).as_ref())
                };
                widths[column] = widths[column].max(width);
                last_width = width;
            }
            max_row_tail =
                max_row_tail.max(last_width + annotation_width(flat, analysis, row, true));
        }
        Self {
            widths,
            max_row_tail,
            header_tail,
        }
    }

    pub fn indentation(
        flat: &FlatJson,
        analysis: &Analysis,
        geometry: &Geometry,
        table: usize,
    ) -> usize {
        let position = geometry.position(flat, analysis, table);
        geometry
            .row(flat, analysis, position.body_line)
            .unwrap()
            .depth
            * 2
    }

    /// The first field's terminal column, counting the structural header prefix.
    pub fn field_start(
        flat: &FlatJson,
        analysis: &Analysis,
        geometry: &Geometry,
        table: usize,
    ) -> usize {
        let position = geometry.position(flat, analysis, table);
        let descriptor = geometry.row(flat, analysis, position.body_line).unwrap();
        let Kind::Value { list, root } = descriptor.kind else {
            unreachable!("table header is a value row")
        };
        descriptor.depth * 2
            + if list { 2 } else { 0 }
            + if !root && flat[table].key_range.is_some() {
                UnicodeWidthStr::width(key_text(flat, table).as_ref())
            } else {
                0
            }
            + analysis
                .node(flat, table)
                .child_count
                .checked_ilog10()
                .unwrap_or(0) as usize
            + 4 // decimal digit, '[' and ']', and '{'
    }

    pub fn extent(&self, field_start: usize) -> usize {
        let preceding = self
            .widths
            .iter()
            .take(self.widths.len() - 1)
            .fold(0usize, |width, column| {
                width.saturating_add(column.saturating_add(1))
            });
        field_start
            .saturating_add(preceding)
            .saturating_add(self.header_tail.max(self.max_row_tail))
    }
}

/// Mirrors the warning spelling appended by `format::annotate`, but does not
/// construct a formatted row or retain annotation strings for every cell.
fn annotation_width(flat: &FlatJson, analysis: &Analysis, node: usize, row: bool) -> usize {
    let mut width = 0;
    for owner in
        std::iter::once(node).chain(children(flat, node).take(if row { usize::MAX } else { 0 }))
    {
        for warning in analysis.warnings(owner) {
            width += if width == 0 { 9 } else { 2 }; // "  # WARN " / "; "
            width += warning.message().len();
            if row && owner != node {
                width += " at field ".len()
                    + UnicodeWidthStr::width(
                        quote_json(&key(flat, owner).unwrap_or_default()).as_str(),
                    );
            }
        }
    }
    width
}
