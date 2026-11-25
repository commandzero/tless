//! Bounded previews keep only visible text and its source runs.
use super::SourceMap;
use std::ops::Range;

#[derive(Clone, Debug, Default)]
pub(super) struct Preview {
    pub(super) text: String,
    pub(super) source: Option<Range<usize>>,
    pub(super) source_map: Vec<SourceMap>,
}

pub(super) fn bounded_prefix(text: &str, limit: usize) -> &str {
    let mut end = text.len().min(limit);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn truncate_preview(preview: &mut Preview, limit: usize) {
    let end = bounded_prefix(&preview.text, limit).len();
    preview.text.truncate(end);
    for map in &mut preview.source_map {
        map.display.end = map.display.end.min(end);
    }
    preview
        .source_map
        .retain(|map| map.display.start < map.display.end);
}

const LIMIT: usize = 256;
const PREFIX_LIMIT: usize = LIMIT - '…'.len_utf8();

fn append_ellipsis(preview: &mut Preview) -> usize {
    truncate_preview(preview, PREFIX_LIMIT);
    let start = preview.text.len();
    preview.text.push('…');
    start
}

pub(super) fn preview_append(preview: &mut Preview, text: &str, source: Option<Range<usize>>) {
    let start = preview.text.len();
    let remaining = LIMIT.saturating_sub(start);
    if remaining == 0 {
        if !text.is_empty() {
            let start = append_ellipsis(preview);
            preview.map_source(source, start);
        }
        return;
    }

    let limit = if text.len() > remaining {
        PREFIX_LIMIT.saturating_sub(start)
    } else {
        remaining
    };
    let part = bounded_prefix(text, limit);
    preview.text.push_str(part);
    if part.len() < text.len() {
        append_ellipsis(preview);
        preview.map_source(source, start.min(PREFIX_LIMIT));
    } else {
        preview.map_source(source, start);
    }
}

impl Preview {
    fn map_source(&mut self, source: Option<Range<usize>>, start: usize) {
        if let Some(source) = source {
            self.source_map.push(SourceMap {
                source,
                display: start..self.text.len(),
            });
        }
    }
}
