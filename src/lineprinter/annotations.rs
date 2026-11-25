use crate::toon_display::{DisplayLine, Span, TokenRole};
use std::borrow::Cow;
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Reserve visible space for counts and the final warning before allocating preview cells.
/// Byte spans are adjusted together with text so mouse ownership remains unchanged.
pub fn fit_annotations(line: &DisplayLine, width: usize) -> Cow<'_, DisplayLine> {
    let Some(preview) = line
        .spans
        .iter()
        .find(|span| span.role == TokenRole::Preview)
    else {
        return Cow::Borrowed(line);
    };
    let total = UnicodeWidthStr::width(line.text.as_str());
    if total <= width {
        return Cow::Borrowed(line);
    }
    let range = &preview.range;
    let sample = &line.text[range.clone()];
    let allowed = width.saturating_sub(total - UnicodeWidthStr::width(sample));
    let replacement = truncate_preview(sample, allowed);
    let mut clipped = line.clone();
    clipped.text.replace_range(range.clone(), &replacement);
    for span in &mut clipped.spans {
        adjust_span(span, range, replacement.len());
    }
    Cow::Owned(clipped)
}

fn truncate_preview(text: &str, allowed: usize) -> String {
    let mut replacement = String::new();
    if allowed == 0 {
        return replacement;
    }
    let mut used = 0;
    for grapheme in text.graphemes(true) {
        let cells = UnicodeWidthStr::width(grapheme);
        if used + cells >= allowed {
            break;
        }
        replacement.push_str(grapheme);
        used += cells;
    }
    replacement.push('…');
    replacement
}

fn adjust_span(span: &mut Span, preview: &Range<usize>, replacement_len: usize) {
    if span.range == *preview {
        let remap =
            |byte: usize| preview.start + byte.saturating_sub(preview.start).min(replacement_len);
        for map in &mut span.source_map {
            map.display = remap(map.display.start)..remap(map.display.end);
        }
        span.source_map
            .retain(|map| map.display.start < map.display.end);
        span.range.end = span.range.start + replacement_len;
    } else if span.range.start >= preview.end {
        let new_end = preview.start + replacement_len;
        span.range.start = span.range.start - preview.end + new_end;
        span.range.end = span.range.end - preview.end + new_end;
    }
}
