//! Materialization of one logical row from structural metadata.
use super::alignment::TableMetrics;
use super::geometry::{Geometry, Kind, Row, key_text, value_text};
use super::index::{Analysis, children, key};
use super::{
    DisplayLine, FlatJson, Preview, SourceMap, Span, TokenRole, Value, bounded_prefix,
    preview_append, quote_json, quote_key, scalar, string_source_map,
};

#[cfg(test)]
std::thread_local! {
    pub(super) static FORMATTED_ROWS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub fn row(
    flat: &FlatJson,
    analysis: &Analysis,
    geometry: &Geometry,
    descriptor: Row,
    collapsed: Option<usize>,
    focused: usize,
    alignment: Option<&TableMetrics>,
) -> DisplayLine {
    #[cfg(test)]
    FORMATTED_ROWS.with(|count| count.set(count.get() + 1));
    let mut line = if let Some(node) = collapsed {
        collapsed_row(flat, analysis, geometry, descriptor, node, focused)
    } else if descriptor.kind == Kind::Document {
        document_header(analysis, descriptor.node)
    } else {
        let mut line = header(flat, analysis, descriptor, focused, alignment);
        let node = descriptor.node;
        if descriptor.kind == Kind::TableRow {
            if let Some(metrics) = alignment {
                let table = flat[node].parent.unwrap();
                let start = TableMetrics::field_start(flat, analysis, geometry, table);
                let indentation = descriptor.depth * 2;
                line.text
                    .extend(std::iter::repeat_n(' ', start.saturating_sub(indentation)));
            }
            for (column, child) in children(flat, node).enumerate() {
                if column > 0 {
                    line.token(",", node, TokenRole::PrimitiveTrailingComma, None);
                }
                let start = line.text.len();
                value(flat, &mut line, child);
                if let Some(metrics) = alignment {
                    if column + 1 < metrics.widths.len() {
                        let width = unicode_width::UnicodeWidthStr::width(&line.text[start..]);
                        line.text
                            .extend(std::iter::repeat_n(' ', metrics.widths[column] - width));
                    }
                }
            }
        } else if geometry.inline(flat, node) {
            for (index, child) in children(flat, node).enumerate() {
                line.token(
                    if index == 0 { " " } else { "," },
                    node,
                    if index == 0 {
                        TokenRole::Punctuation
                    } else {
                        TokenRole::PrimitiveTrailingComma
                    },
                    None,
                );
                value(flat, &mut line, child);
            }
        } else if scalar(flat, node) {
            value(flat, &mut line, node);
        }
        if descriptor.owner != descriptor.node {
            let offset = descriptor.depth.saturating_sub(1) * 2;
            line.text.replace_range(offset..offset + 2, "- ");
            line.spans.push(Span {
                range: offset..offset + 1,
                node: descriptor.owner,
                role: TokenRole::ContainerDelimiter,
                source: None,
                source_map: Vec::new(),
            });
            line.owner = descriptor.owner;
        }
        annotate(flat, analysis, geometry, &mut line);
        line
    };
    map_sources(flat, &mut line);
    line
}

fn document_header(analysis: &Analysis, node: usize) -> DisplayLine {
    let mut line = DisplayLine::new(node, String::new());
    line.separator = true;
    line.token("---", node, TokenRole::ContainerDelimiter, None);
    let position = analysis
        .roots
        .iter()
        .position(|&root| root == node)
        .unwrap();
    line.token(
        &format!(" ({} of {})", position + 1, analysis.roots.len()),
        node,
        TokenRole::DocumentPosition,
        None,
    );
    line
}

fn header(
    flat: &FlatJson,
    analysis: &Analysis,
    descriptor: Row,
    focused: usize,
    alignment: Option<&TableMetrics>,
) -> DisplayLine {
    let node = descriptor.node;
    let mut line = DisplayLine::new(descriptor.owner, "  ".repeat(descriptor.depth));
    let Kind::Value { list, root } = descriptor.kind else {
        return line;
    };
    if list {
        line.token("-", node, TokenRole::ContainerDelimiter, None);
    }
    let object = matches!(flat[node].value, Value::EmptyObject)
        || flat[node].is_opening_of_container() && !flat[node].is_array();
    if object && (root || flat[node].key_range.is_none()) {
        return line;
    }
    if list {
        line.text.push(' ');
    }
    if !root && flat[node].key_range.is_some() {
        line.token(
            &key_text(flat, node),
            node,
            TokenRole::Key,
            flat[node].key_range.clone(),
        );
    }
    if flat[node].is_array() || matches!(flat[node].value, Value::EmptyArray) {
        line.token(
            &format!("[{}]", analysis.node(flat, node).child_count),
            node,
            TokenRole::ArrayIndex,
            None,
        );
        if analysis.node(flat, node).table {
            line.token("{", node, TokenRole::ContainerDelimiter, None);
            let first = children(flat, node).next().unwrap();
            for (column, field) in children(flat, first).enumerate() {
                if column > 0 {
                    line.token(",", node, TokenRole::PrimitiveTrailingComma, None);
                }
                // Mouse targeting always uses the first row. One additional
                // alias is enough for the explicitly focused field's identity.
                let start = line.text.len();
                line.token(
                    &key_text(flat, field),
                    field,
                    TokenRole::FieldDefinition,
                    flat[field].key_range.clone(),
                );
                if focused != field
                    && analysis.table_cell(flat, focused)
                    && flat[focused].index_in_parent == column
                    && flat[flat[focused].parent.unwrap()].parent.unwrap() == node
                {
                    let mut alias = line.spans.last().unwrap().clone();
                    alias.node = focused;
                    alias.source = flat[focused].key_range.clone();
                    line.spans.push(alias);
                }
                if let Some(metrics) = alignment {
                    if column + 1 < metrics.widths.len() {
                        let width = unicode_width::UnicodeWidthStr::width(&line.text[start..]);
                        line.text
                            .extend(std::iter::repeat_n(' ', metrics.widths[column] - width));
                    }
                }
            }
            line.token("}", node, TokenRole::ContainerDelimiter, None);
        }
        line.token(
            ":",
            node,
            if analysis.node(flat, node).child_count == 0 {
                TokenRole::EmptyContainer
            } else {
                TokenRole::Punctuation
            },
            None,
        );
    } else if object {
        line.token(":", node, TokenRole::Punctuation, None);
    } else if !root && flat[node].key_range.is_some() {
        line.token(": ", node, TokenRole::Punctuation, None);
    }
    line
}

fn value(flat: &FlatJson, line: &mut DisplayLine, node: usize) {
    let role = match flat[node].value {
        Value::String => TokenRole::String,
        Value::Number => TokenRole::Number,
        Value::Boolean => TokenRole::Boolean,
        _ => TokenRole::Null,
    };
    line.token(
        &value_text(flat, node),
        node,
        role,
        Some(flat[node].range.clone()),
    );
}

fn annotate(flat: &FlatJson, analysis: &Analysis, geometry: &Geometry, line: &mut DisplayLine) {
    let mut nodes: Vec<_> = line
        .spans
        .iter()
        .filter(|span| {
            !matches!(span.role, TokenRole::Key | TokenRole::FieldDefinition)
                || geometry.position(flat, analysis, span.node).line
                    == geometry.position(flat, analysis, line.owner).body_line
        })
        .map(|span| span.node)
        .collect();
    nodes.push(line.owner);
    nodes.sort_unstable();
    nodes.dedup();
    let mut first = true;
    for node in nodes {
        let mut warnings: Vec<_> = analysis.warnings(node).collect();
        warnings.sort();
        for warning in warnings {
            if first {
                line.token("  # WARN ", line.owner, TokenRole::Warning, None);
                first = false;
            } else {
                line.token("; ", line.owner, TokenRole::Warning, None);
            }
            let locator = if analysis.is_root(node) {
                String::new()
            } else if analysis.table_cell(flat, node) {
                format!(
                    " at field {}",
                    quote_json(&key(flat, node).unwrap_or_default())
                )
            } else if let Some(parent) = flat[node].parent.as_option() {
                if flat[parent].is_array()
                    && geometry.position(flat, analysis, node).line
                        == geometry.position(flat, analysis, line.owner).body_line
                {
                    format!(" at [{}]", flat[node].index_in_parent)
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            line.token(
                &format!("{}{locator}", warning.message()),
                node,
                TokenRole::Warning,
                None,
            );
        }
    }
}

fn preview(flat: &FlatJson, analysis: &Analysis, node: usize) -> Preview {
    let mut preview = Preview {
        source: Some(flat[node].range.clone()),
        ..Preview::default()
    };
    for child in children(flat, node) {
        if !preview.text.is_empty() {
            preview_append(
                &mut preview,
                if flat[node].is_array() { "," } else { "; " },
                None,
            );
        }
        if !flat[node].is_array() {
            if let Some(key) = key(flat, child) {
                preview_append(
                    &mut preview,
                    &quote_key(bounded_prefix(&key, 256)),
                    flat[child].key_range.clone(),
                );
            } else if flat.key_value(child).is_some() {
                preview_append(
                    &mut preview,
                    &quote_key(bounded_prefix(&key_text(flat, child), 256)),
                    flat[child].key_range.clone(),
                );
            }
            preview_append(&mut preview, ": ", None);
        }
        if scalar(flat, child) {
            preview_append(
                &mut preview,
                &value_text(flat, child),
                Some(flat[child].range.clone()),
            );
        } else if flat[child].is_array() || matches!(flat[child].value, Value::EmptyArray) {
            preview_append(
                &mut preview,
                &format!("[{}]: …", analysis.node(flat, child).child_count),
                Some(flat[child].range.clone()),
            );
        } else {
            preview_append(&mut preview, "…", Some(flat[child].range.clone()));
        }
        if preview.text.len() >= 256 {
            if !preview.text.ends_with('…') {
                preview_append(&mut preview, "…", Some(flat[child].range.clone()));
            }
            break;
        }
    }
    preview
}

fn collapsed_row(
    flat: &FlatJson,
    analysis: &Analysis,
    geometry: &Geometry,
    descriptor: Row,
    node: usize,
    focused: usize,
) -> DisplayLine {
    let document = descriptor.kind == Kind::Document;
    let mut line = if document {
        document_header(analysis, node)
    } else {
        let mut header_descriptor = descriptor;
        if descriptor.owner != descriptor.node && node == descriptor.owner {
            header_descriptor.node = node;
            header_descriptor.depth = descriptor.depth.saturating_sub(1);
            header_descriptor.kind = Kind::Value {
                list: true,
                root: false,
            };
        }
        let mut line = header(flat, analysis, header_descriptor, focused, None);
        line.owner = node;
        line
    };
    if !document && !flat[node].is_array() {
        line.token(
            &format!(" ({})", analysis.node(flat, node).child_count),
            node,
            TokenRole::Count,
            None,
        );
    }
    let mut messages: Vec<_> = analysis.warnings(node).collect();
    messages.sort();
    let mut messages: Vec<_> = messages
        .iter()
        .map(|warning| warning.message().to_owned())
        .collect();
    let hidden = analysis.node(flat, node).hidden_warnings;
    if hidden > 0 {
        messages.push(format!("Contains {hidden} hidden warnings"));
    }
    let warning = if messages.is_empty() {
        String::new()
    } else {
        format!("  # WARN {}", messages.join("; "))
    };
    let mut inline = None;
    if !document
        && flat[node].is_array()
        && analysis.node(flat, node).child_count <= 5
        && children(flat, node).all(|child| scalar(flat, child))
    {
        let mut candidate = line.clone();
        for (i, child) in children(flat, node).enumerate() {
            candidate.token(
                if i == 0 { " " } else { "," },
                node,
                if i == 0 {
                    TokenRole::Punctuation
                } else {
                    TokenRole::PrimitiveTrailingComma
                },
                None,
            );
            value(flat, &mut candidate, child);
        }
        if unicode_width::UnicodeWidthStr::width(candidate.text.as_str())
            + unicode_width::UnicodeWidthStr::width(warning.as_str())
            <= geometry.width
        {
            inline = Some(candidate);
        }
    }
    if let Some(candidate) = inline {
        line = candidate;
    } else {
        let mut preview = preview(flat, analysis, node);
        if document {
            if flat[node].is_array() {
                let content = std::mem::take(&mut preview.text);
                preview.source_map.clear();
                preview_append(
                    &mut preview,
                    &format!("[{}]:", analysis.node(flat, node).child_count),
                    None,
                );
                if !content.is_empty() {
                    preview_append(&mut preview, " ", None);
                    preview_append(&mut preview, &content, None);
                }
            }
            if preview.text.is_empty() {
                match flat[node].value {
                    Value::EmptyObject => preview.text.push_str("{}"),
                    Value::EmptyArray => preview.text.push_str("[]"),
                    Value::String | Value::Number | Value::Boolean | Value::Null => {
                        preview_append(&mut preview, &value_text(flat, node), None)
                    }
                    _ => preview.text.push('…'),
                }
            }
            preview.source = None;
            preview.source_map.clear();
        }
        if !preview.text.is_empty() {
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
    }
    if !warning.is_empty() {
        line.token(&warning, node, TokenRole::Warning, None);
    }
    line
}

fn map_sources(flat: &FlatJson, line: &mut DisplayLine) {
    for span in &mut line.spans {
        if let Some(source) = &span.source {
            let raw = &flat.1[source.clone()];
            if raw.starts_with('"')
                && matches!(
                    span.role,
                    TokenRole::Key | TokenRole::FieldDefinition | TokenRole::String
                )
            {
                let parsed = if span.role == TokenRole::String {
                    flat.string_value(span.node)
                } else {
                    match flat.key_value(span.node) {
                        Some(crate::flatjson::KeyValue::String(value)) => Some(value.as_str()),
                        _ => None,
                    }
                };
                span.source_map = string_source_map(
                    raw,
                    parsed,
                    &line.text[span.range.clone()],
                    source.start,
                    span.range.start,
                );
            }
            if span.source_map.is_empty() && raw == &line.text[span.range.clone()] {
                span.source_map.push(SourceMap {
                    source: source.clone(),
                    display: span.range.clone(),
                });
            }
        }
    }
}

/// Map matching key occurrences onto one shared header without retaining an
/// alias or source-map graph for every table row.
pub fn highlight_shared_fields(
    flat: &FlatJson,
    analysis: &Analysis,
    line: &mut DisplayLine,
    matches: &[std::ops::Range<usize>],
    current: &std::ops::Range<usize>,
) {
    line.shared_matches.clear();
    if matches.is_empty() && current.is_empty() {
        return;
    }
    let Some(first) = line
        .spans
        .iter()
        .find(|span| span.role == TokenRole::FieldDefinition)
    else {
        return;
    };
    let first_row = flat[first.node].parent.unwrap();
    let array = flat[first_row].parent.unwrap();
    let columns: Vec<_> = line
        .spans
        .iter()
        .filter(|span| {
            span.role == TokenRole::FieldDefinition && flat[span.node].parent.unwrap() == first_row
        })
        .collect();
    let mut covered = vec![0u8; line.text.len()];
    let start = matches.partition_point(|query| query.end <= flat[array].range.start);
    let end = matches.partition_point(|query| query.start < flat[array].range.end);
    for (query, flag) in matches[start..end]
        .iter()
        .map(|query| (query, 1))
        .chain(std::iter::once((current, 2)))
    {
        if query.is_empty() {
            continue;
        }
        let start = flat
            .0
            .partition_point(|row| row.range_represented_by_row().start <= query.start)
            .saturating_sub(1);
        let end = flat
            .0
            .partition_point(|row| row.range_represented_by_row().start < query.end);
        for node in start..end {
            if !analysis.table_cell(flat, node) {
                continue;
            }
            let row = flat[node].parent.unwrap();
            if flat[row].parent.unwrap() != array {
                continue;
            }
            let source = flat[node].key_range.as_ref().unwrap();
            if source.start >= query.end || query.start >= source.end {
                continue;
            }
            let column = columns[flat[node].index_in_parent];
            if covered[column.range.clone()]
                .iter()
                .all(|mask| mask & flag != 0)
            {
                continue;
            }
            let raw = &flat.1[source.clone()];
            let rendered = &line.text[column.range.clone()];
            let mut mark = |source: &std::ops::Range<usize>, display: &std::ops::Range<usize>| {
                if source.start >= query.end || query.start >= source.end {
                    return;
                }
                let range = if source.len() == display.len() {
                    display.start + query.start.max(source.start) - source.start
                        ..display.start + query.end.min(source.end) - source.start
                } else {
                    display.clone()
                };
                for mask in &mut covered[range] {
                    *mask |= flag;
                }
            };
            if raw == rendered {
                mark(source, &column.range);
            } else if raw.strip_prefix('"').and_then(|raw| raw.strip_suffix('"')) == Some(rendered)
            {
                mark(&(source.start + 1..source.end - 1), &column.range);
            } else {
                let parsed = match flat.key_value(node) {
                    Some(crate::flatjson::KeyValue::String(value)) => Some(value.as_str()),
                    _ => None,
                };
                let maps =
                    string_source_map(raw, parsed, rendered, source.start, column.range.start);
                if maps.is_empty() {
                    mark(source, &column.range);
                }
                for map in maps {
                    mark(&map.source, &map.display);
                }
            }
        }
    }
    let mut start = 0;
    while start < covered.len() {
        let mask = covered[start];
        let mut end = start + 1;
        while end < covered.len() && covered[end] == mask {
            end += 1;
        }
        if mask != 0 {
            line.shared_matches.push((start..end, mask & 2 != 0));
        }
        start = end;
    }
}
