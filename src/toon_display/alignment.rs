//! Enabled-table measurements, independent of formatted viewport rows.
use super::geometry::{key_text, value_text};
use super::index::{Analysis, children, string};
use super::{FlatJson, Value, quote_json_into, value_needs_quotes};
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
        let mut quoted = String::new();
        let header_tail = widths.last().copied().unwrap()
            + 2
            + annotation_width(flat, analysis, table, false, &mut quoted);
        let mut max_row_tail = 0;
        let mut numeric_annotation: Option<usize> = None;
        for row in children(flat, table) {
            let mut last_width = 0;
            let mut last_is_number = false;
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
    for owner in
        std::iter::once(node).chain(children(flat, node).take(if row { usize::MAX } else { 0 }))
    {
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
