//! Materialization of one logical row from structural metadata.
use super::alignment::TableMetrics;
use super::geometry::{Geometry, Kind, Row, value_text};
use super::index::{Analysis, children};
use super::layout::Layout;
use super::{DisplayLine, FlatJson, Span, TokenRole, Value, scalar};
use unicode_width::UnicodeWidthStr;

mod collapsed;
mod fields;
mod header;
mod matching;
mod warnings;

pub(super) use matching::highlight_shared_fields;
pub(super) use warnings::{WarningPosition, WarningText};

#[cfg(test)]
std::thread_local! {
    pub(super) static FORMATTED_ROWS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub struct Formatter<'a> {
    flat: &'a FlatJson,
    analysis: &'a Analysis,
    geometry: &'a Geometry,
    focused: usize,
    alignment: Option<&'a TableMetrics>,
}

impl<'a> Formatter<'a> {
    pub fn new(
        flat: &'a FlatJson,
        layout: &'a Layout,
        focused: usize,
        alignment: Option<&'a TableMetrics>,
    ) -> Self {
        Self {
            flat,
            analysis: &layout.analysis,
            geometry: &layout.geometry,
            focused,
            alignment,
        }
    }

    pub fn row(&self, descriptor: Row, collapsed: Option<usize>) -> DisplayLine {
        #[cfg(test)]
        FORMATTED_ROWS.with(|count| count.set(count.get() + 1));
        let mut line = match collapsed {
            Some(node) => self.collapsed_row(descriptor, node),
            None if descriptor.separator() => self.document_header(descriptor.node),
            None => self.expanded_row(descriptor),
        };
        super::source_map::map_sources(self.flat, &mut line);
        line
    }

    fn expanded_row(&self, row: Row) -> DisplayLine {
        let mut line = self.header(row, self.alignment);
        let node = row.node;
        if row.kind == Kind::TableRow {
            self.table_row(&mut line, node);
        } else if self.geometry.inline(self.flat, node) {
            self.inline_values(&mut line, node);
        } else if scalar(self.flat, node) {
            value(self.flat, &mut line, node, None);
        }

        if row.owner != node {
            self.place_list_delimiter(&mut line, row);
        }
        self.annotate(&mut line);
        line
    }

    fn table_row(&self, line: &mut DisplayLine, node: usize) {
        let table = self.flat[node].parent.unwrap();
        if !self.flat[table].is_array() {
            self.key_token(line, node, TokenRole::Key);
            line.token(": ", node, TokenRole::Punctuation, None);
        }
        if let Some(metrics) = self.alignment {
            let start = self
                .geometry
                .table_field_start(self.flat, self.analysis, table);
            pad_to(line, start.max(metrics.entry_prefix));
        }
        for (column, child) in self.analysis.table_cells(self.flat, node).enumerate() {
            if column > 0 {
                cell_separator(line, node, self.alignment.is_some());
            }
            self.table_cell(line, column, child);
        }
    }

    fn table_cell(&self, line: &mut DisplayLine, column: usize, node: usize) {
        let Some(metrics) = self.alignment else {
            value(self.flat, line, node, None);
            return;
        };

        let start = line.text.len();
        add_spaces(line, metrics.leaf_offsets[column]);
        if matches!(self.flat[node].value, Value::Number) {
            let width = metrics.widths[column] - metrics.leaf_offsets[column];
            value(self.flat, line, node, Some(width));
        } else {
            value(self.flat, line, node, None);
            pad_column(line, metrics, column, start);
        }
    }

    fn inline_values(&self, line: &mut DisplayLine, node: usize) {
        for (index, child) in children(self.flat, node).enumerate() {
            if index == 0 {
                line.token(" ", node, TokenRole::Punctuation, None);
            } else {
                cell_separator(line, node, false);
            }
            value(self.flat, line, child, None);
        }
    }

    fn place_list_delimiter(&self, line: &mut DisplayLine, row: Row) {
        let offset = row.depth.saturating_sub(1) * 2;
        line.text.replace_range(offset..offset + 2, "- ");
        line.spans.push(Span {
            range: offset..offset + 1,
            node: row.owner,
            role: TokenRole::ContainerDelimiter,
            source: None,
            source_map: Vec::new(),
        });
        line.owner = if self.alignment.is_some() && self.analysis.node(self.flat, row.node).table {
            row.node
        } else {
            row.owner
        };
    }
}

fn cell_separator(line: &mut DisplayLine, node: usize, aligned: bool) {
    if aligned {
        line.text.push(' ');
    } else {
        line.token(",", node, TokenRole::PrimitiveTrailingComma, None);
    }
}

fn add_spaces(line: &mut DisplayLine, width: usize) {
    line.text.extend(std::iter::repeat_n(' ', width));
}

fn pad_to(line: &mut DisplayLine, start: usize) {
    let width = UnicodeWidthStr::width(line.text.as_str());
    add_spaces(line, start.saturating_sub(width));
}

/// Padding is display-only. The final column has no trailing padding.
fn pad_column(line: &mut DisplayLine, metrics: &TableMetrics, column: usize, start: usize) {
    if column + 1 < metrics.widths.len() {
        let width = UnicodeWidthStr::width(&line.text[start..]);
        add_spaces(line, metrics.widths[column].saturating_sub(width));
    }
}

fn value(flat: &FlatJson, line: &mut DisplayLine, node: usize, number_width: Option<usize>) {
    let role = match flat[node].value {
        Value::String => TokenRole::String,
        Value::Number => TokenRole::Number,
        Value::Boolean => TokenRole::Boolean,
        _ => TokenRole::Null,
    };
    let text = value_text(flat, node);
    let text = protect_initial_bom(line, role, text);
    if let Some(width) = number_width {
        add_spaces(line, width - UnicodeWidthStr::width(text.as_ref()));
    }
    line.token(&text, node, role, Some(flat[node].range.clone()));
}

fn protect_initial_bom<'a>(
    line: &DisplayLine,
    role: TokenRole,
    text: std::borrow::Cow<'a, str>,
) -> std::borrow::Cow<'a, str> {
    // A leading BOM at root position would be removed by the TOON decoder.
    if role != TokenRole::String || !line.text.is_empty() {
        return text;
    }
    if text.starts_with('\u{feff}') {
        return std::borrow::Cow::Owned(super::quote_json(&text));
    }
    text
}
