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
        let first = children(flat, table)
            .next()
            .expect("eligible table has rows");
        let mut schema = Vec::new();
        let mut pending = 0;
        for field in children(flat, first) {
            header_widths(flat, field, &mut pending, &mut schema);
        }
        let (mut widths, leaf_offsets): (Vec<_>, Vec<_>) = schema.into_iter().unzip();
        let mut quoted = String::new();
        let header_tail = widths.last().copied().unwrap()
            + 2
            + annotation_width(flat, analysis, table, false, &mut quoted);
        let root = analysis.root_for(table).unwrap();
        let implicit = !flat[root].is_array() && !analysis.node(flat, root).table;
        let row_depth = flat[table].depth - flat[root].depth - usize::from(implicit) + 1;
        let mut entry_prefix = 0;
        let mut max_row_tail = 0;
        let mut numeric_annotation: Option<usize> = None;
        for row in children(flat, table) {
            if !flat[table].is_array() {
                entry_prefix = entry_prefix
                    .max(row_depth * 2 + UnicodeWidthStr::width(key_text(flat, row).as_ref()) + 2);
            }
            let mut last_width = 0;
            let mut last_is_number = false;
            for (column, cell) in analysis.table_cells(flat, row).enumerate() {
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
                widths[column] = widths[column].max(width + leaf_offsets[column]);
                last_width = width + leaf_offsets[column];
                last_is_number = matches!(flat[cell].value, Value::Number);
            }
            let annotation = annotation_width(flat, analysis, row, true, &mut quoted);
            if last_is_number {
                numeric_annotation = Some(numeric_annotation.unwrap_or(0).max(annotation));
            } else {
                max_row_tail = max_row_tail.max(last_width + annotation);
            }
        }
        if let Some(annotation) = numeric_annotation {
            max_row_tail = max_row_tail.max(widths.last().copied().unwrap() + annotation);
        }
        Self {
            widths,
            leaf_offsets,
            entry_prefix,
            max_row_tail,
            header_tail,
        }
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

/// Count the on-screen warning fragments without materializing an off-screen
/// row, warning list, or formatted annotation.
fn annotation_width(
    flat: &FlatJson,
    analysis: &Analysis,
    node: usize,
    row: bool,
    quoted: &mut String,
) -> usize {
    let mut width = 0;
    let mut first = true;
    let end = if row {
        *flat.subtree_range(node).end()
    } else {
        node + 1
    };
    for owner in node..end {
        for warning in analysis.warnings(owner) {
            super::format::warning_parts(
                flat,
                owner,
                warning,
                first,
                row && owner != node,
                quoted,
                |part, _| width += UnicodeWidthStr::width(part),
            );
            first = false;
        }
    }
    width
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
