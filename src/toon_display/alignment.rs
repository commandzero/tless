//! Enabled-table measurements, independent of formatted viewport rows.
use super::geometry::{key_text, value_text};
use super::index::{Analysis, children, string};
use super::{FlatJson, Value, quote_json_into, value_needs_quotes};
use unicode_width::UnicodeWidthStr;

pub struct TableMetrics {
    pub widths: Vec<usize>,
    pub leaf_offsets: Vec<usize>,
    /// Absolute cell start imposed by the widest keyed entry prefix.
    pub entry_prefix: usize,
    // The final token and annotation belong to the same row.
    max_row_tail: usize,
    header_tail: usize,
}

impl TableMetrics {
    pub fn measure(flat: &FlatJson, analysis: &Analysis, table: usize) -> Self {
        let mut measurement = Measurement {
            flat,
            analysis,
            quoted: String::new(),
        };
        let (widths, leaf_offsets) = header_columns(flat, table);
        let header_tail =
            widths.last().copied().unwrap() + 2 + measurement.annotation_width(table, table + 1);
        let mut result = Self {
            widths,
            leaf_offsets,
            header_tail,
            entry_prefix: 0,
            max_row_tail: 0,
        };
        let entry_depth = measurement.keyed_entry_depth(table);
        let mut numeric_annotation: Option<usize> = None;
        for row in children(flat, table) {
            if let Some(depth) = entry_depth {
                let key_width = UnicodeWidthStr::width(key_text(flat, row).as_ref());
                result.entry_prefix = result.entry_prefix.max(depth * 2 + key_width + 2);
            }
            let (last_width, numeric) =
                measurement.measure_cells(row, &mut result.widths, &result.leaf_offsets);
            let annotation = measurement.annotation_width(row, *flat.subtree_range(row).end());
            if numeric {
                numeric_annotation = Some(numeric_annotation.unwrap_or(0).max(annotation));
            } else {
                result.max_row_tail = result.max_row_tail.max(last_width + annotation);
            }
        }
        if let Some(annotation) = numeric_annotation {
            result.max_row_tail = result
                .max_row_tail
                .max(result.widths.last().copied().unwrap() + annotation);
        }
        result
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
            .max(self.entry_prefix)
            .saturating_add(preceding)
            .saturating_add(self.header_tail.max(self.max_row_tail))
    }
}

struct Measurement<'a> {
    flat: &'a FlatJson,
    analysis: &'a Analysis,
    quoted: String,
}

impl Measurement<'_> {
    fn keyed_entry_depth(&self, table: usize) -> Option<usize> {
        if self.flat[table].is_array() {
            return None;
        }
        let root = self.analysis.root_for(table).unwrap();
        let implicit = !self.flat[root].is_array() && !self.analysis.node(self.flat, root).table;
        Some(self.flat[table].depth - self.flat[root].depth - usize::from(implicit) + 1)
    }

    fn measure_cells(
        &mut self,
        row: usize,
        widths: &mut [usize],
        offsets: &[usize],
    ) -> (usize, bool) {
        let mut last = (0, false);
        for (column, cell) in self.analysis.table_cells(self.flat, row).enumerate() {
            let width = self.cell_width(cell) + offsets[column];
            widths[column] = widths[column].max(width);
            last = (width, matches!(self.flat[cell].value, Value::Number));
        }
        last
    }

    fn cell_width(&mut self, cell: usize) -> usize {
        if !matches!(self.flat[cell].value, Value::String) {
            return UnicodeWidthStr::width(value_text(self.flat, cell).as_ref());
        }
        let value = string(self.flat, cell);
        if value_needs_quotes(&value) {
            quote_json_into(&value, &mut self.quoted);
            UnicodeWidthStr::width(self.quoted.as_str())
        } else {
            UnicodeWidthStr::width(value.as_ref())
        }
    }

    // Count warning fragments without materializing a row or annotation.
    fn annotation_width(&mut self, node: usize, end: usize) -> usize {
        let mut width = 0;
        for owner in node..end {
            for warning in self.analysis.warnings(owner) {
                let position = super::format::WarningPosition {
                    node: owner,
                    first: width == 0,
                    field: owner != node,
                };
                let mut text = super::format::WarningText {
                    flat: self.flat,
                    quoted: &mut self.quoted,
                };
                text.parts(warning, position, |part, _| {
                    width += UnicodeWidthStr::width(part)
                });
            }
        }
        width
    }
}

fn header_columns(flat: &FlatJson, table: usize) -> (Vec<usize>, Vec<usize>) {
    let first = children(flat, table)
        .next()
        .expect("eligible table has rows");
    let mut schema = Vec::new();
    let mut pending = 0;
    for field in children(flat, first) {
        header_widths(flat, field, &mut pending, &mut schema);
    }
    schema.into_iter().unzip()
}

/// Attribute each group prefix to its first leaf and each closing brace to
/// its last leaf, so a width includes all header glyphs in its column.
fn header_widths(
    flat: &FlatJson,
    field: usize,
    pending: &mut usize,
    out: &mut Vec<(usize, usize)>,
) {
    for node in flat.subtree_range(field) {
        if flat[node].is_closing_of_container() {
            out.last_mut().unwrap().0 += 1;
        } else {
            let width = UnicodeWidthStr::width(key_text(flat, node).as_ref());
            if super::scalar(flat, node) {
                out.push((*pending + width, *pending));
                *pending = 0;
            } else {
                *pending += width + 1;
            }
        }
    }
}
