use super::LinePainter;
use crate::terminal::{self, Color, Style};
use crate::theme::{DisplayContext, FocusState, JsonValueKind, SearchState, StyleRole, StyleState};
use crate::toon_display::{Span, TokenRole};
use std::ops::Range;

impl LinePainter<'_> {
    pub(super) fn style_at(&self, byte: usize, length: usize) -> Style {
        let focused_row = !self.focused.is_empty();
        let Some(span) = self.span_at(byte) else {
            return self
                .theme
                .map_or_else(Style::default, |theme| theme.row_style(focused_row));
        };
        let token = span.role;
        let focus = self.focused.contains(&span.node) && !is_annotation(token);
        let grapheme = byte..byte.saturating_add(length);
        let punctuation = is_quoted_key_edge(&self.line.text, span, &grapheme);
        let search = self.search_style(span, &grapheme);
        let mut style = if let Some(theme) = self.theme {
            let role = if punctuation {
                StyleRole::Punctuation
            } else {
                style_role(token)
            };
            theme.style_on_row(role, theme_state(token, focus, search), focused_row)
        } else {
            default_token_style(token, punctuation, focus, search)
        };
        if token == TokenRole::Warning {
            style.dimmed = true;
        }
        style
    }

    fn span_at(&self, byte: usize) -> Option<&Span> {
        let covers = |span: &&Span| span.range.contains(&byte);
        self.line
            .spans
            .iter()
            .filter(|span| span.node == self.focused.start)
            .find(covers)
            .or_else(|| self.line.spans.iter().find(covers))
    }

    fn search_style(&self, span: &Span, grapheme: &Range<usize>) -> SearchState {
        let overlaps_query = |query: &Range<usize>| {
            span.matching_ranges(query)
                .iter()
                .any(|range| overlaps(range, grapheme))
        };
        let mut shared_hit = false;
        for (_, current) in self
            .line
            .shared_matches
            .iter()
            .filter(|(range, _)| overlaps(range, grapheme))
        {
            if *current {
                return SearchState::CurrentMatch;
            }
            shared_hit = true;
        }
        if overlaps_query(self.current) {
            SearchState::CurrentMatch
        } else if shared_hit || self.matches.iter().any(overlaps_query) {
            SearchState::Match
        } else {
            SearchState::None
        }
    }
}

fn overlaps(left: &Range<usize>, right: &Range<usize>) -> bool {
    left.start < right.end && right.start < left.end
}

fn is_annotation(token: TokenRole) -> bool {
    matches!(
        token,
        TokenRole::Preview | TokenRole::Count | TokenRole::DocumentPosition | TokenRole::Warning
    )
}

fn is_quoted_key_edge(text: &str, span: &Span, grapheme: &Range<usize>) -> bool {
    if !matches!(span.role, TokenRole::Key | TokenRole::FieldDefinition) {
        return false;
    }
    let edge = grapheme.start == span.range.start || grapheme.end == span.range.end;
    let token = &text[span.range.clone()];
    edge && token.starts_with('"') && token.ends_with('"')
}

fn theme_state(token: TokenRole, focused: bool, search: SearchState) -> StyleState {
    let focus = if focused {
        FocusState::Row
    } else {
        FocusState::None
    };
    let context = if token == TokenRole::Preview {
        DisplayContext::Preview
    } else {
        DisplayContext::Main
    };
    StyleState {
        context,
        focus,
        search,
    }
}

fn style_role(token: TokenRole) -> StyleRole {
    match token {
        TokenRole::Key => StyleRole::ObjectKey,
        TokenRole::FieldDefinition => StyleRole::FieldDefinition,
        TokenRole::String => StyleRole::JsonValue(JsonValueKind::String),
        TokenRole::Number => StyleRole::JsonValue(JsonValueKind::Number),
        TokenRole::Boolean => StyleRole::JsonValue(JsonValueKind::Boolean),
        TokenRole::Null => StyleRole::JsonValue(JsonValueKind::Null),
        TokenRole::ArrayIndex => StyleRole::ArrayIndex,
        TokenRole::PrimitiveTrailingComma => StyleRole::PrimitiveTrailingComma,
        TokenRole::ContainerDelimiter => StyleRole::ContainerDelimiter,
        TokenRole::EmptyContainer => StyleRole::JsonValue(JsonValueKind::EmptyObject),
        TokenRole::Punctuation => StyleRole::Punctuation,
        TokenRole::Warning => StyleRole::Message(crate::theme::MessageSeverity::Warn),
        TokenRole::Preview => StyleRole::PreviewText,
        TokenRole::Count => StyleRole::PreviewCount,
        TokenRole::DocumentPosition => StyleRole::DocumentPosition,
    }
}

fn default_token_style(
    token: TokenRole,
    punctuation: bool,
    focused: bool,
    search: SearchState,
) -> Style {
    let plain_fg = if punctuation {
        terminal::DEFAULT
    } else {
        fallback_color(token)
    };
    let fg = match search {
        SearchState::CurrentMatch => terminal::LIGHT_YELLOW,
        SearchState::Match => terminal::YELLOW,
        SearchState::None if focused => plain_fg.bright(),
        SearchState::None => plain_fg,
    };
    Style {
        fg,
        underlined: search != SearchState::None,
        ..Style::default()
    }
}

fn fallback_color(token: TokenRole) -> Color {
    match token {
        TokenRole::Key | TokenRole::FieldDefinition => terminal::CYAN,
        TokenRole::String => terminal::GREEN,
        TokenRole::Number => terminal::MAGENTA,
        TokenRole::Boolean => terminal::BLUE,
        TokenRole::Null | TokenRole::EmptyContainer => terminal::WHITE,
        TokenRole::Warning => terminal::YELLOW,
        TokenRole::Count | TokenRole::Preview | TokenRole::DocumentPosition => {
            terminal::LIGHT_BLACK
        }
        TokenRole::ArrayIndex
        | TokenRole::ContainerDelimiter
        | TokenRole::PrimitiveTrailingComma
        | TokenRole::Punctuation => terminal::DEFAULT,
    }
}
