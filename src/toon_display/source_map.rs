//! Map a rendered string back to source bytes without retaining each character.
use super::SourceMap;
use super::scalar_text::{decode_string, escaped_literal};
use std::ops::Range;

pub(super) fn string_source_map(
    raw: &str,
    parsed: Option<&str>,
    rendered: &str,
    offsets: (usize, usize),
) -> Vec<SourceMap> {
    let mut mapping = Mapping::new(rendered, offsets);
    if mapping.quoted {
        mapping.append("\"", 0..1);
    }
    match parsed {
        Some(parsed) => mapping.yaml_string(parsed),
        None => mapping.json_string(raw),
    }
    if mapping.quoted {
        mapping.append("\"", raw.len() - 1..raw.len());
    }
    if mapping.valid && mapping.bytes == rendered.len() {
        mapping.runs
    } else {
        Vec::new()
    }
}

struct Mapping<'a> {
    rendered: &'a str,
    quoted: bool,
    valid: bool,
    bytes: usize,
    source_offset: usize,
    display_offset: usize,
    runs: Vec<SourceMap>,
}

impl<'a> Mapping<'a> {
    fn new(rendered: &'a str, (source_offset, display_offset): (usize, usize)) -> Self {
        Self {
            rendered,
            quoted: rendered.starts_with('"') && rendered.ends_with('"'),
            valid: true,
            bytes: 0,
            source_offset,
            display_offset,
            runs: Vec::new(),
        }
    }

    fn append(&mut self, text: &str, range: Range<usize>) {
        let end = self.bytes + text.len();
        self.valid &= self.rendered.get(self.bytes..end) == Some(text);
        let run = SourceMap {
            source: self.source_offset + range.start..self.source_offset + range.end,
            display: self.display_offset + self.bytes..self.display_offset + end,
        };
        if let Some(previous) = self
            .runs
            .last_mut()
            .filter(|previous| can_join(previous, &run))
        {
            previous.source.end = run.source.end;
            previous.display.end = run.display.end;
        } else {
            self.runs.push(run);
        }
        self.bytes = end;
    }

    fn append_char(&mut self, ch: char, range: Range<usize>) {
        if self.quoted {
            if let Some(literal) = escaped_literal(ch) {
                self.append(literal, range);
                return;
            }
            if ch.is_control() {
                self.append(&format!("\\u{:04x}", ch as u32), range);
                return;
            }
        }
        self.append(ch.encode_utf8(&mut [0; 4]), range);
    }

    fn yaml_string(&mut self, parsed: &str) {
        let mut offset = 1;
        for ch in parsed.chars() {
            // YAML's flattened representation escapes newlines.
            let end = offset + if ch == '\n' { 2 } else { ch.len_utf8() };
            self.append_char(ch, offset..end);
            offset = end;
        }
    }

    fn json_string(&mut self, raw: &str) {
        let mut offset = 1;
        let end = raw.len().saturating_sub(1);
        while offset < end {
            let start = offset;
            let ch = raw[offset..].chars().next().unwrap();
            offset += ch.len_utf8();
            if ch == '\\' && offset < end {
                offset = escape_end(raw, offset, end);
                let decoded = decode_string(&format!("\"{}\"", &raw[start..offset]));
                for ch in decoded.chars() {
                    self.append_char(ch, start..offset);
                }
            } else {
                self.append_char(ch, start..offset);
            }
        }
    }
}

fn escape_end(raw: &str, escaped: usize, end: usize) -> usize {
    if raw.as_bytes()[escaped] != b'u' {
        return escaped + 1;
    }

    let code_end = (escaped + 5).min(end);
    let high_surrogate = raw
        .get(escaped + 1..code_end)
        .and_then(|digits| u16::from_str_radix(digits, 16).ok())
        .is_some_and(|code| (0xd800..=0xdbff).contains(&code));
    if high_surrogate && raw[code_end..].starts_with("\\u") {
        return (code_end + 6).min(end);
    }
    code_end
}

fn can_join(left: &SourceMap, right: &SourceMap) -> bool {
    left.source.end == right.source.start
        && left.display.end == right.display.start
        && left.source.len() == left.display.len()
        && right.source.len() == right.display.len()
}
