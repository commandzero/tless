//! Table header fields, focused aliases, and their display-only padding.
use super::{DisplayLine, Formatter, TableMetrics, TokenRole, Value};
use crate::toon_display::index::same_field_path;
use crate::toon_display::scalar;
use unicode_width::UnicodeWidthStr;

impl Formatter<'_> {
    pub(super) fn table_fields(
        &self,
        line: &mut DisplayLine,
        node: usize,
        alignment: Option<&TableMetrics>,
    ) {
        line.token("{", node, TokenRole::ContainerDelimiter, None);
        let first = self.flat[node].first_child().unwrap();
        if let Some(metrics) = alignment {
            super::pad_to(line, metrics.entry_prefix);
        }
        let mut starts = alignment.map(|_| Vec::new());
        self.field_definitions(line, first, starts.as_mut());
        if let Some((metrics, starts)) = alignment.zip(starts) {
            pad_header_columns(line, metrics, &starts);
        }
        line.token("}", node, TokenRole::ContainerDelimiter, None);
    }

    fn field_definitions(
        &self,
        line: &mut DisplayLine,
        row: usize,
        mut columns: Option<&mut Vec<usize>>,
    ) {
        let mut start = line.text.len();
        let end = *self.flat.subtree_range(row).end();
        for node in row + 1..end {
            if let Value::CloseContainer { open_index, .. } = self.flat[node].value {
                line.token("}", open_index, TokenRole::ContainerDelimiter, None);
                continue;
            }
            if self.flat[node].index_in_parent > 0 {
                super::cell_separator(line, node, columns.is_some());
                start = line.text.len();
            }
            self.key_token(line, node, TokenRole::FieldDefinition);
            self.alias_focused_field(line, node);
            if !scalar(self.flat, node) {
                line.token("{", node, TokenRole::ContainerDelimiter, None);
                continue;
            }
            if let Some(columns) = columns.as_deref_mut() {
                columns.push(start);
            }
        }
    }

    fn alias_focused_field(&self, line: &mut DisplayLine, node: usize) {
        if self.focused == node || self.flat[self.focused].depth != self.flat[node].depth {
            return;
        }
        if !self.focused_field_is_peer(node) || !same_field_path(self.flat, node, self.focused) {
            return;
        }
        let mut alias = line.spans.last().unwrap().clone();
        alias.node = self.focused;
        alias.source = self.flat[self.focused].key_range.clone();
        line.spans.push(alias);
    }

    fn focused_field_is_peer(&self, node: usize) -> bool {
        self.analysis
            .table_row_owner(self.flat, self.focused)
            .filter(|&row| row != self.focused)
            .and_then(|row| self.flat[row].parent.as_option())
            .is_some_and(|table| self.analysis.table_owner(self.flat, node) == Some(table))
    }
}

/// Insert from right to left so source-bearing span offsets remain stable.
fn pad_header_columns(line: &mut DisplayLine, metrics: &TableMetrics, starts: &[usize]) {
    for column in (0..starts.len().saturating_sub(1)).rev() {
        let end = starts[column + 1] - 1;
        let width = UnicodeWidthStr::width(&line.text[starts[column]..end]);
        let padding = metrics.widths[column].saturating_sub(width);
        if padding == 0 {
            continue;
        }
        insert_padding(line, end, padding);
    }
}

fn insert_padding(line: &mut DisplayLine, offset: usize, padding: usize) {
    line.text.insert_str(offset, &" ".repeat(padding));
    for span in &mut line.spans {
        if span.range.start >= offset {
            span.range.start += padding;
        }
        if span.range.end > offset {
            span.range.end += padding;
        }
    }
}
