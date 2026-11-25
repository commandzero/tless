//! Collapsed rows and their bounded previews.
use super::{DisplayLine, Formatter, Kind, Row, Span, TokenRole, Value, scalar, value_text};
use crate::toon_display::geometry::key_text;
use crate::toon_display::index::{Node, children, key};
use crate::toon_display::{Preview, SourceMap, bounded_prefix, preview_append, quote_key};
use unicode_width::UnicodeWidthStr;

const LIMIT: usize = 256;

impl Formatter<'_> {
    pub(super) fn collapsed_row(&self, row: Row, node: usize) -> DisplayLine {
        let document = row.separator();
        let mut line = self.collapsed_header(row, node);
        let info = self.analysis.node(self.flat, node);
        if self.show_collapsed_count(node, document, info) {
            line.token(
                &format!(" ({})", info.child_count),
                node,
                TokenRole::Count,
                None,
            );
        }

        let warning = self.collapsed_warning(node, info);
        if document || !self.try_inline(&mut line, node, &warning) {
            let preview = self.preview(node);
            let preview = if document {
                self.document_preview(preview, node, info)
            } else {
                preview
            };
            append_preview(&mut line, node, preview);
        }
        if !warning.is_empty() {
            line.token(&warning, node, TokenRole::Warning, None);
        }
        line
    }

    fn collapsed_header(&self, mut row: Row, node: usize) -> DisplayLine {
        if row.separator() {
            return self.document_header(node);
        }
        if row.owner != row.node && node == row.owner {
            row.node = node;
            row.depth = row.depth.saturating_sub(1);
            row.kind = Kind::Value {
                list: true,
                root: false,
            };
        }
        let mut line = self.header(row, None);
        line.owner = node;
        line
    }

    fn show_collapsed_count(&self, node: usize, document: bool, info: Node) -> bool {
        if document || info.table {
            return false;
        }
        !self.flat[node].is_array()
    }

    fn collapsed_warning(&self, node: usize, info: Node) -> String {
        let mut kinds: Vec<_> = self.analysis.warnings(node).collect();
        kinds.sort();
        let mut messages: Vec<_> = kinds
            .iter()
            .map(|warning| warning.message().to_owned())
            .collect();
        if info.hidden_warnings > 0 {
            messages.push(format!("Contains {} hidden warnings", info.hidden_warnings));
        }
        if messages.is_empty() {
            String::new()
        } else {
            format!("  # WARN {}", messages.join("; "))
        }
    }

    fn try_inline(&self, line: &mut DisplayLine, node: usize, warning: &str) -> bool {
        let info = self.analysis.node(self.flat, node);
        if !self.flat[node].is_array() || info.child_count > 5 {
            return false;
        }
        if !children(self.flat, node).all(|child| scalar(self.flat, child)) {
            return false;
        }
        let mut candidate = line.clone();
        self.inline_values(&mut candidate, node);
        let width =
            UnicodeWidthStr::width(candidate.text.as_str()) + UnicodeWidthStr::width(warning);
        if width > self.geometry.width {
            return false;
        }
        *line = candidate;
        true
    }

    fn preview(&self, node: usize) -> Preview {
        let mut preview = Preview {
            source: Some(self.flat[node].range.clone()),
            ..Preview::default()
        };
        let array = self.flat[node].is_array();
        for child in children(self.flat, node) {
            if !preview.text.is_empty() {
                preview_append(&mut preview, if array { "," } else { "; " }, None);
            }
            if !array {
                self.preview_key(&mut preview, child);
            }
            let detail = self.preview_value(child);
            let source = Some(self.flat[child].range.clone());
            preview_append(&mut preview, &detail, source.clone());
            if preview.text.len() >= LIMIT {
                finish_preview(&mut preview, source);
                break;
            }
        }
        preview
    }

    fn preview_key(&self, preview: &mut Preview, node: usize) {
        let name = match key(self.flat, node) {
            Some(name) => Some(name),
            None if self.flat.key_value(node).is_some() => Some(key_text(self.flat, node)),
            None => None,
        };
        if let Some(name) = name {
            let spelling = quote_key(bounded_prefix(&name, LIMIT));
            preview_append(preview, &spelling, self.flat[node].key_range.clone());
        }
        preview_append(preview, ": ", None);
    }

    fn preview_value(&self, node: usize) -> std::borrow::Cow<'_, str> {
        if scalar(self.flat, node) {
            return value_text(self.flat, node);
        }
        if self.flat[node].is_array() || matches!(self.flat[node].value, Value::EmptyArray) {
            let count = self.analysis.node(self.flat, node).child_count;
            return format!("[{count}]: …").into();
        }
        "…".into()
    }

    fn document_preview(&self, mut preview: Preview, node: usize, info: Node) -> Preview {
        preview.source = None;
        preview.source_map.clear();
        if self.flat[node].is_array() {
            prepend_array_count(&mut preview, info.child_count);
        }
        if preview.text.is_empty() {
            let detail = match self.flat[node].value {
                Value::EmptyObject => "{}".into(),
                Value::EmptyArray => "[]".into(),
                _ if scalar(self.flat, node) => value_text(self.flat, node),
                _ => "…".into(),
            };
            preview_append(&mut preview, &detail, None);
        }
        preview
    }
}

fn append_preview(line: &mut DisplayLine, node: usize, preview: Preview) {
    if preview.text.is_empty() {
        return;
    }
    let start = line.text.len();
    line.text.push(' ');
    line.text.push_str(&preview.text);
    let source_map = preview
        .source_map
        .into_iter()
        .map(|map| SourceMap {
            source: map.source,
            display: start + 1 + map.display.start..start + 1 + map.display.end,
        })
        .collect();
    line.spans.push(Span {
        range: start..line.text.len(),
        node,
        role: TokenRole::Preview,
        source: preview.source,
        source_map,
    });
}

fn finish_preview(preview: &mut Preview, source: Option<std::ops::Range<usize>>) {
    if !preview.text.ends_with('…') {
        preview_append(preview, "…", source);
    }
}

fn prepend_array_count(preview: &mut Preview, count: usize) {
    let content = std::mem::take(&mut preview.text);
    preview_append(preview, &format!("[{count}]:"), None);
    if !content.is_empty() {
        preview_append(preview, " ", None);
        preview_append(preview, &content, None);
    }
}
