// Borealis dark-mode CSS palette with TextMate string colors. See openspec/specs/color-themes/spec.md.
use super::{FocusState, JsonValueKind, MessageSeverity, SearchState, StyleRole, StyleState};
use crate::terminal::{Color, Style};

const BACKGROUND: Color = Color::Rgb(5, 15, 33);
const TEXT: Color = Color::Rgb(202, 211, 226);
const MUTED: Color = Color::Rgb(152, 168, 195);
const SELECTION: Color = Color::Rgb(10, 35, 66);
const KEY: Color = Color::Rgb(97, 162, 255);
const STRING: Color = Color::Rgb(2, 188, 183);
const NUMBER: Color = Color::Rgb(179, 134, 249);
const BOOLEAN: Color = Color::Rgb(238, 114, 166);
const SEARCH: Color = Color::Rgb(250, 203, 61);
const SEARCH_LIGHT: Color = Color::Rgb(252, 216, 131);
const CONSTANT: Color = Color::Rgb(179, 134, 249);
const WARNING: Color = Color::Rgb(252, 216, 131);
const ERROR: Color = Color::Rgb(238, 76, 72);
const ERROR_LIGHT: Color = Color::Rgb(246, 114, 106);

pub(super) fn style(role: StyleRole, state: StyleState, search: SearchState) -> Style {
    let fg = match role {
        StyleRole::JsonValue(JsonValueKind::Null) => CONSTANT,
        StyleRole::JsonValue(JsonValueKind::Boolean) => BOOLEAN,
        StyleRole::JsonValue(JsonValueKind::Number) => NUMBER,
        StyleRole::JsonValue(JsonValueKind::String) => STRING,
        StyleRole::ObjectKey => KEY,
        StyleRole::ArrayIndex
        | StyleRole::LineNumber
        | StyleRole::ContainerDelimiter
        | StyleRole::Ellipsis
        | StyleRole::PreviewText
        | StyleRole::EmptyRowMarker
        | StyleRole::TruncationIndicator => MUTED,
        StyleRole::Message(MessageSeverity::Warn) => WARNING,
        StyleRole::Message(MessageSeverity::Error) => {
            if state.focus == FocusState::Row {
                ERROR_LIGHT
            } else {
                ERROR
            }
        }
        _ => TEXT,
    };
    let mut style = Style {
        fg,
        bg: if matches!(role, StyleRole::StatusBar | StyleRole::StatusPathBase) {
            SELECTION
        } else {
            BACKGROUND
        },
        ..Style::default()
    };
    if (state.focus == FocusState::Row
        && !matches!(
            role,
            StyleRole::Document
                | StyleRole::StatusBar
                | StyleRole::StatusPathBase
                | StyleRole::StatusText
                | StyleRole::Message(_)
        ))
        || (state.focus == FocusState::PairedContainer && role == StyleRole::ContainerDelimiter)
    {
        style.bg = SELECTION;
    }
    // Search replaces focus and keeps the underline without bold.
    if search != SearchState::None {
        style = Style {
            fg: if search == SearchState::CurrentMatch {
                SEARCH_LIGHT
            } else {
                SEARCH
            },
            bg: BACKGROUND,
            underlined: true,
            ..Style::default()
        };
    }
    style
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{Theme, ThemeColor, ThemeName};

    #[test]
    fn structural_colors_and_accent_pairs_never_use_bold() {
        let theme = Theme::built_in(ThemeName::Borealis);
        for focus in [
            FocusState::None,
            FocusState::Row,
            FocusState::PairedContainer,
        ] {
            let state = StyleState {
                focus,
                ..StyleState::main()
            };
            for role in [
                StyleRole::Punctuation,
                StyleRole::PrimitiveTrailingComma,
                StyleRole::PreviewCount,
                StyleRole::FieldDefinition,
            ] {
                let style = theme.style(role, state);
                assert_eq!(style.fg, TEXT);
                assert!(!style.bold);
                assert!(!style.dimmed);
            }
            assert_eq!(
                theme.style(StyleRole::ContainerDelimiter, state).fg,
                theme.style(StyleRole::LineNumber, state).fg
            );
            assert_eq!(
                theme
                    .style(StyleRole::Message(MessageSeverity::Error), state)
                    .fg,
                if focus == FocusState::Row {
                    ERROR_LIGHT
                } else {
                    ERROR
                }
            );
            for search in [SearchState::Match, SearchState::CurrentMatch] {
                let style = theme.style(StyleRole::ObjectKey, StyleState { search, ..state });
                assert_eq!(
                    style.fg,
                    if search == SearchState::CurrentMatch {
                        SEARCH_LIGHT
                    } else {
                        SEARCH
                    }
                );
                assert!(style.underlined);
                assert!(!style.bold);
            }
        }
    }

    #[test]
    fn preserves_source_colors_and_search_precedence() {
        let theme = Theme::built_in(ThemeName::Borealis);
        let state = StyleState::main();
        assert_eq!(theme.document_style().bg, Color::Rgb(5, 15, 33));
        for (role, color) in [
            (StyleRole::ObjectKey, Color::Rgb(97, 162, 255)),
            (
                StyleRole::JsonValue(JsonValueKind::String),
                Color::Rgb(2, 188, 183),
            ),
            (StyleRole::JsonValue(JsonValueKind::Number), NUMBER),
            (StyleRole::JsonValue(JsonValueKind::Boolean), BOOLEAN),
        ] {
            assert_eq!(theme.style(role, state).fg, color);
            let focused = theme.style(role, state.focused());
            assert_eq!(focused.fg, color);
            assert_eq!(focused.bg, SELECTION);
            assert!(!focused.bold);
            assert_eq!(
                theme.style(role, state.focused().search(SearchState::Match)),
                theme.style(role, state.search(SearchState::Match))
            );
        }
        assert_ne!(
            theme.style(StyleRole::ObjectKey, state.search(SearchState::Match)),
            theme.style(
                StyleRole::ObjectKey,
                state.search(SearchState::CurrentMatch)
            )
        );
        let custom = theme.with_color(ThemeColor::ObjectKey, Color::C256(123));
        assert_eq!(
            custom.style(StyleRole::ObjectKey, state).fg,
            Color::C256(123)
        );
    }
}
