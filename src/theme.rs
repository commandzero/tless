use clap::ValueEnum;

use crate::terminal::{
    Color, Style, BLUE, CYAN, GREEN, LIGHT_BLACK, LIGHT_BLUE, LIGHT_CYAN, LIGHT_YELLOW, MAGENTA,
    RED, WHITE, YELLOW,
};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum ThemeName {
    #[default]
    Classic,
    Cyan,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Theme {
    name: ThemeName,
    colors: [Option<Color>; THEME_COLOR_COUNT],
}

const THEME_COLOR_COUNT: usize = ThemeColor::CommandLineBackground as usize + 1;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ThemeColor {
    Null,
    Boolean,
    Number,
    String,
    EmptyContainer,
    ObjectKey,
    FocusedObjectKey,
    ArrayIndex,
    Punctuation,
    PrimitiveTrailingComma,
    ContainerDelimiter,
    FocusedContainerDelimiter,
    Ellipsis,
    PreviewText,
    PreviewCount,
    LineNumber,
    FocusedLineNumber,
    EmptyRowMarker,
    TruncationIndicator,
    StatusBar,
    StatusText,
    MessageInfo,
    MessageWarning,
    MessageError,
    SearchMatch,
    SearchMatchPreview,
    SearchMatchCurrent,
    StatusBarForeground,
    StatusBarBackground,
    CommandLineForeground,
    CommandLineBackground,
}

impl ThemeColor {
    pub fn from_config_key(key: &str) -> Option<Self> {
        match key {
            "null" => Some(Self::Null),
            "boolean" => Some(Self::Boolean),
            "number" => Some(Self::Number),
            "string" => Some(Self::String),
            "empty-container" => Some(Self::EmptyContainer),
            "object-key" => Some(Self::ObjectKey),
            "focused-object-key" => Some(Self::FocusedObjectKey),
            "array-index" => Some(Self::ArrayIndex),
            "punctuation" => Some(Self::Punctuation),
            "primitive-trailing-comma" => Some(Self::PrimitiveTrailingComma),
            "container-delimiter" => Some(Self::ContainerDelimiter),
            "focused-container-delimiter" => Some(Self::FocusedContainerDelimiter),
            "ellipsis" => Some(Self::Ellipsis),
            "preview-text" => Some(Self::PreviewText),
            "preview-count" => Some(Self::PreviewCount),
            "line-number" => Some(Self::LineNumber),
            "focused-line-number" => Some(Self::FocusedLineNumber),
            "empty-row-marker" => Some(Self::EmptyRowMarker),
            "truncation-indicator" => Some(Self::TruncationIndicator),
            "status-bar" => Some(Self::StatusBar),
            "status-text" => Some(Self::StatusText),
            "message-info" => Some(Self::MessageInfo),
            "message-warning" => Some(Self::MessageWarning),
            "message-error" => Some(Self::MessageError),
            "search-match" => Some(Self::SearchMatch),
            "search-match-preview" => Some(Self::SearchMatchPreview),
            "search-match-current" => Some(Self::SearchMatchCurrent),
            "status-bar-foreground" => Some(Self::StatusBarForeground),
            "status-bar-background" => Some(Self::StatusBarBackground),
            "command-line-foreground" => Some(Self::CommandLineForeground),
            "command-line-background" => Some(Self::CommandLineBackground),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum JsonValueKind {
    Null,
    Boolean,
    Number,
    String,
    EmptyObject,
    EmptyArray,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MessageSeverity {
    Info,
    Warn,
    Error,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum StyleRole {
    JsonValue(JsonValueKind),
    ObjectKey,
    ArrayIndex,
    Punctuation,
    PrimitiveTrailingComma,
    ContainerDelimiter,
    Ellipsis,
    PreviewText,
    PreviewCount,
    LineNumber,
    EmptyRowMarker,
    TruncationIndicator,
    StatusBar,
    StatusPathBase,
    StatusText,
    Message(MessageSeverity),
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum DisplayContext {
    #[default]
    Main,
    Preview,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum FocusState {
    #[default]
    None,
    Row,
    PairedContainer,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum SearchState {
    #[default]
    None,
    Match,
    CurrentMatch,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct StyleState {
    pub context: DisplayContext,
    pub focus: FocusState,
    pub search: SearchState,
}

impl StyleState {
    pub const fn main() -> Self {
        Self {
            context: DisplayContext::Main,
            focus: FocusState::None,
            search: SearchState::None,
        }
    }

    pub const fn preview() -> Self {
        Self {
            context: DisplayContext::Preview,
            ..Self::main()
        }
    }

    pub const fn focused(mut self) -> Self {
        self.focus = FocusState::Row;
        self
    }

    pub const fn paired_container(mut self) -> Self {
        self.focus = FocusState::PairedContainer;
        self
    }

    pub const fn search(mut self, search: SearchState) -> Self {
        self.search = search;
        self
    }
}

impl Theme {
    pub const fn built_in(name: ThemeName) -> Self {
        Self {
            name,
            colors: [None; THEME_COLOR_COUNT],
        }
    }

    pub fn with_color(mut self, key: ThemeColor, color: Color) -> Self {
        self.colors[key as usize] = Some(color);
        self
    }

    pub fn style(&self, role: StyleRole, state: StyleState) -> Style {
        let mut style = self.base_style(role);

        style = self.apply_focus(role, state.focus, style);

        let search = if state.context == DisplayContext::Preview
            && state.search == SearchState::CurrentMatch
        {
            SearchState::Match
        } else {
            state.search
        };

        let mut style = match search {
            SearchState::None => style,
            SearchState::Match => self.search_match_style(state.context),
            SearchState::CurrentMatch => self.current_match_style(),
        };

        if let Some(color) = self.colors[Self::color_key(role, state, search) as usize] {
            style.fg = color;
        }

        match role {
            StyleRole::StatusBar | StyleRole::StatusPathBase => self.apply_row_colors(
                style,
                ThemeColor::StatusBarForeground,
                ThemeColor::StatusBarBackground,
            ),
            StyleRole::StatusText | StyleRole::Message(_) => {
                // Severity colors take precedence over the command row's text color.
                if role == StyleRole::StatusText {
                    style = self.apply_row_colors(
                        style,
                        ThemeColor::CommandLineForeground,
                        ThemeColor::CommandLineBackground,
                    );
                } else if let Some(bg) = self.colors[ThemeColor::CommandLineBackground as usize] {
                    style.bg = bg;
                }
                style
            }
            _ => style,
        }
    }

    fn apply_row_colors(
        &self,
        mut style: Style,
        foreground: ThemeColor,
        background: ThemeColor,
    ) -> Style {
        let fg = self.colors[foreground as usize];
        let bg = self.colors[background as usize];
        if fg.is_some() || bg.is_some() {
            // Explicit colors describe what users see, independent of reverse video.
            if style.inverted {
                std::mem::swap(&mut style.fg, &mut style.bg);
                style.inverted = false;
            }
            style.fg = fg.unwrap_or(style.fg);
            style.bg = bg.unwrap_or(style.bg);
        }
        style
    }

    fn color_key(role: StyleRole, state: StyleState, search: SearchState) -> ThemeColor {
        match search {
            SearchState::Match => {
                if state.context == DisplayContext::Preview {
                    return ThemeColor::SearchMatchPreview;
                }
                return ThemeColor::SearchMatch;
            }
            SearchState::CurrentMatch => return ThemeColor::SearchMatchCurrent,
            SearchState::None => {}
        }

        match (role, state.focus) {
            (StyleRole::ObjectKey, FocusState::Row) => ThemeColor::FocusedObjectKey,
            (StyleRole::ContainerDelimiter, FocusState::Row | FocusState::PairedContainer) => {
                ThemeColor::FocusedContainerDelimiter
            }
            (StyleRole::LineNumber, FocusState::Row) => ThemeColor::FocusedLineNumber,
            (StyleRole::JsonValue(JsonValueKind::Null), _) => ThemeColor::Null,
            (StyleRole::JsonValue(JsonValueKind::Boolean), _) => ThemeColor::Boolean,
            (StyleRole::JsonValue(JsonValueKind::Number), _) => ThemeColor::Number,
            (StyleRole::JsonValue(JsonValueKind::String), _) => ThemeColor::String,
            (StyleRole::JsonValue(JsonValueKind::EmptyObject | JsonValueKind::EmptyArray), _) => {
                ThemeColor::EmptyContainer
            }
            (StyleRole::ObjectKey, _) => ThemeColor::ObjectKey,
            (StyleRole::ArrayIndex, _) => ThemeColor::ArrayIndex,
            (StyleRole::Punctuation, _) => ThemeColor::Punctuation,
            (StyleRole::PrimitiveTrailingComma, _) => ThemeColor::PrimitiveTrailingComma,
            (StyleRole::ContainerDelimiter, _) => ThemeColor::ContainerDelimiter,
            (StyleRole::Ellipsis, _) => ThemeColor::Ellipsis,
            (StyleRole::PreviewText, _) => ThemeColor::PreviewText,
            (StyleRole::PreviewCount, _) => ThemeColor::PreviewCount,
            (StyleRole::LineNumber, _) => ThemeColor::LineNumber,
            (StyleRole::EmptyRowMarker, _) => ThemeColor::EmptyRowMarker,
            (StyleRole::TruncationIndicator, _) => ThemeColor::TruncationIndicator,
            (StyleRole::StatusBar | StyleRole::StatusPathBase, _) => ThemeColor::StatusBar,
            (StyleRole::StatusText, _) => ThemeColor::StatusText,
            (StyleRole::Message(MessageSeverity::Info), _) => ThemeColor::MessageInfo,
            (StyleRole::Message(MessageSeverity::Warn), _) => ThemeColor::MessageWarning,
            (StyleRole::Message(MessageSeverity::Error), _) => ThemeColor::MessageError,
        }
    }

    fn base_style(&self, role: StyleRole) -> Style {
        let color = |fg: Color| Style {
            fg,
            ..Style::default()
        };

        match role {
            StyleRole::JsonValue(kind) => color(self.json_value_color(kind)),
            StyleRole::ObjectKey => color(match self.name {
                ThemeName::Classic => LIGHT_BLUE,
                ThemeName::Cyan => CYAN,
            }),
            StyleRole::ArrayIndex | StyleRole::LineNumber | StyleRole::Ellipsis => Style {
                dimmed: true,
                ..Style::default()
            },
            StyleRole::Punctuation | StyleRole::StatusText => Style::default(),
            StyleRole::PrimitiveTrailingComma => match self.name {
                ThemeName::Classic => Style::default(),
                ThemeName::Cyan => Style {
                    dimmed: true,
                    ..Style::default()
                },
            },
            StyleRole::ContainerDelimiter => match self.name {
                ThemeName::Classic => Style::default(),
                ThemeName::Cyan => Style {
                    dimmed: true,
                    ..Style::default()
                },
            },
            StyleRole::PreviewText => Style {
                dimmed: true,
                ..Style::default()
            },
            StyleRole::PreviewCount | StyleRole::EmptyRowMarker => color(LIGHT_BLACK),
            StyleRole::TruncationIndicator => color(LIGHT_BLACK),
            StyleRole::StatusBar => Style {
                inverted: true,
                ..Style::default()
            },
            StyleRole::StatusPathBase => Style {
                bg: LIGHT_BLACK,
                inverted: true,
                ..Style::default()
            },
            StyleRole::Message(severity) => color(match severity {
                MessageSeverity::Info => WHITE,
                MessageSeverity::Warn => YELLOW,
                MessageSeverity::Error => RED,
            }),
        }
    }

    fn json_value_color(&self, kind: JsonValueKind) -> Color {
        match (self.name, kind) {
            (ThemeName::Classic, JsonValueKind::Null) => LIGHT_BLACK,
            (ThemeName::Cyan, JsonValueKind::Null) => LIGHT_BLUE,
            (ThemeName::Classic, JsonValueKind::Boolean) => YELLOW,
            (ThemeName::Cyan, JsonValueKind::Boolean) => MAGENTA,
            (_, JsonValueKind::Number) => MAGENTA,
            (_, JsonValueKind::String) => GREEN,
            (ThemeName::Classic, JsonValueKind::EmptyObject | JsonValueKind::EmptyArray) => WHITE,
            (ThemeName::Cyan, JsonValueKind::EmptyObject | JsonValueKind::EmptyArray) => {
                LIGHT_BLACK
            }
        }
    }

    fn apply_focus(&self, role: StyleRole, focus: FocusState, style: Style) -> Style {
        match (role, focus) {
            (_, FocusState::None) => style,
            (StyleRole::ObjectKey, FocusState::Row) => match self.name {
                ThemeName::Classic => Style {
                    bg: BLUE,
                    inverted: true,
                    bold: true,
                    ..Style::default()
                },
                ThemeName::Cyan => Style {
                    fg: LIGHT_CYAN,
                    ..Style::default()
                },
            },
            (StyleRole::ArrayIndex, FocusState::Row) => Style {
                inverted: true,
                bold: true,
                ..Style::default()
            },
            (StyleRole::ContainerDelimiter, FocusState::Row | FocusState::PairedContainer) => {
                match self.name {
                    ThemeName::Classic => Style {
                        bold: true,
                        ..Style::default()
                    },
                    ThemeName::Cyan => Style {
                        fg: YELLOW,
                        ..Style::default()
                    },
                }
            }
            (StyleRole::LineNumber, FocusState::Row) => Style {
                fg: YELLOW,
                ..Style::default()
            },
            (StyleRole::TruncationIndicator, FocusState::Row) => Style {
                bold: true,
                ..Style::default()
            },
            _ => style,
        }
    }

    fn search_match_style(&self, context: DisplayContext) -> Style {
        if context == DisplayContext::Preview {
            Style {
                fg: LIGHT_BLACK,
                inverted: self.name == ThemeName::Classic,
                ..Style::default()
            }
        } else {
            Style {
                fg: YELLOW,
                ..Style::default()
            }
        }
    }

    fn current_match_style(&self) -> Style {
        match self.name {
            ThemeName::Classic => Style {
                inverted: true,
                bold: true,
                ..Style::default()
            },
            ThemeName::Cyan => Style {
                fg: LIGHT_YELLOW,
                underline: true,
                ..Style::default()
            },
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::built_in(ThemeName::Classic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fg(color: Color) -> Style {
        Style {
            fg: color,
            ..Style::default()
        }
    }

    #[test]
    fn classic_json_palette_matches_main() {
        let theme = Theme::default();
        let state = StyleState::main();

        for (kind, expected) in [
            (JsonValueKind::Null, LIGHT_BLACK),
            (JsonValueKind::Boolean, YELLOW),
            (JsonValueKind::Number, MAGENTA),
            (JsonValueKind::String, GREEN),
            (JsonValueKind::EmptyObject, WHITE),
            (JsonValueKind::EmptyArray, WHITE),
        ] {
            assert_eq!(theme.style(StyleRole::JsonValue(kind), state), fg(expected));
        }
    }

    #[test]
    fn classic_base_roles_match_main() {
        let theme = Theme::default();
        let state = StyleState::main();

        for (role, expected) in [
            (StyleRole::ObjectKey, fg(LIGHT_BLUE)),
            (
                StyleRole::ArrayIndex,
                Style {
                    dimmed: true,
                    ..Style::default()
                },
            ),
            (StyleRole::Punctuation, Style::default()),
            (StyleRole::PrimitiveTrailingComma, Style::default()),
            (StyleRole::ContainerDelimiter, Style::default()),
            (
                StyleRole::Ellipsis,
                Style {
                    dimmed: true,
                    ..Style::default()
                },
            ),
            (
                StyleRole::PreviewText,
                Style {
                    dimmed: true,
                    ..Style::default()
                },
            ),
            (StyleRole::PreviewCount, fg(LIGHT_BLACK)),
            (
                StyleRole::LineNumber,
                Style {
                    dimmed: true,
                    ..Style::default()
                },
            ),
            (StyleRole::EmptyRowMarker, fg(LIGHT_BLACK)),
            (StyleRole::TruncationIndicator, fg(LIGHT_BLACK)),
            (
                StyleRole::StatusBar,
                Style {
                    inverted: true,
                    ..Style::default()
                },
            ),
            (
                StyleRole::StatusPathBase,
                Style {
                    bg: LIGHT_BLACK,
                    inverted: true,
                    ..Style::default()
                },
            ),
            (StyleRole::StatusText, Style::default()),
            (StyleRole::Message(MessageSeverity::Info), fg(WHITE)),
            (StyleRole::Message(MessageSeverity::Warn), fg(YELLOW)),
            (StyleRole::Message(MessageSeverity::Error), fg(RED)),
        ] {
            assert_eq!(theme.style(role, state), expected, "role: {role:?}");
        }
    }

    #[test]
    fn cyan_palette_contains_only_specified_base_differences() {
        let classic = Theme::built_in(ThemeName::Classic);
        let cyan = Theme::built_in(ThemeName::Cyan);
        let state = StyleState::main();

        assert_eq!(
            cyan.style(StyleRole::JsonValue(JsonValueKind::Null), state),
            fg(LIGHT_BLUE)
        );
        assert_eq!(
            cyan.style(StyleRole::JsonValue(JsonValueKind::Boolean), state),
            fg(MAGENTA)
        );
        assert_eq!(cyan.style(StyleRole::ObjectKey, state), fg(CYAN));
        assert_eq!(
            cyan.style(StyleRole::PrimitiveTrailingComma, state),
            Style {
                dimmed: true,
                ..Style::default()
            }
        );

        for role in [
            StyleRole::JsonValue(JsonValueKind::Number),
            StyleRole::JsonValue(JsonValueKind::String),
            StyleRole::Punctuation,
            StyleRole::LineNumber,
            StyleRole::StatusBar,
            StyleRole::Message(MessageSeverity::Error),
        ] {
            assert_eq!(cyan.style(role, state), classic.style(role, state));
        }
    }

    #[test]
    fn focus_and_search_precedence_is_explicit() {
        let classic = Theme::default();
        let cyan = Theme::built_in(ThemeName::Cyan);

        assert_eq!(
            classic.style(StyleRole::ObjectKey, StyleState::main().focused()),
            Style {
                bg: BLUE,
                inverted: true,
                bold: true,
                ..Style::default()
            }
        );
        assert_eq!(
            cyan.style(StyleRole::ObjectKey, StyleState::main().focused()),
            fg(LIGHT_CYAN)
        );
        assert_eq!(
            classic.style(
                StyleRole::ObjectKey,
                StyleState::main()
                    .focused()
                    .search(SearchState::CurrentMatch),
            ),
            Style {
                inverted: true,
                bold: true,
                ..Style::default()
            }
        );
        assert_eq!(
            cyan.style(
                StyleRole::ObjectKey,
                StyleState::main()
                    .focused()
                    .search(SearchState::CurrentMatch),
            ),
            Style {
                fg: LIGHT_YELLOW,
                underline: true,
                ..Style::default()
            }
        );
    }

    #[test]
    fn preview_reduces_current_match_to_ordinary_match() {
        let classic = Theme::default();
        let cyan = Theme::built_in(ThemeName::Cyan);
        let state = StyleState::preview().search(SearchState::CurrentMatch);

        assert_eq!(
            classic.style(StyleRole::PreviewText, state),
            Style {
                fg: LIGHT_BLACK,
                inverted: true,
                ..Style::default()
            }
        );
        assert_eq!(cyan.style(StyleRole::PreviewText, state), fg(LIGHT_BLACK));
    }

    #[test]
    fn paired_container_focus_uses_theme_delimiter_style() {
        let classic = Theme::default();
        let cyan = Theme::built_in(ThemeName::Cyan);
        let state = StyleState::main().paired_container();

        assert_eq!(
            classic.style(StyleRole::ContainerDelimiter, state),
            Style {
                bold: true,
                ..Style::default()
            }
        );
        assert_eq!(cyan.style(StyleRole::ContainerDelimiter, state), fg(YELLOW));
    }

    #[test]
    fn status_roles_preserve_classic_styles() {
        let theme = Theme::default();
        let state = StyleState::main();

        assert_eq!(
            theme.style(StyleRole::StatusBar, state),
            Style {
                inverted: true,
                ..Style::default()
            }
        );
        assert_eq!(
            theme.style(StyleRole::StatusPathBase, state),
            Style {
                bg: LIGHT_BLACK,
                inverted: true,
                ..Style::default()
            }
        );
        assert_eq!(
            theme.style(StyleRole::Message(MessageSeverity::Info), state),
            fg(WHITE)
        );
    }

    #[test]
    fn default_color_constant_is_representable() {
        assert_eq!(
            Theme::default()
                .style(StyleRole::Punctuation, StyleState::main())
                .fg,
            Color::Default
        );
    }

    #[test]
    fn preview_search_override_uses_context_for_every_role() {
        let theme = Theme::default()
            .with_color(ThemeColor::SearchMatchPreview, BLUE)
            .with_color(ThemeColor::SearchMatch, RED);
        for role in [
            StyleRole::PreviewText,
            StyleRole::ObjectKey,
            StyleRole::Ellipsis,
        ] {
            assert_eq!(
                theme
                    .style(
                        role,
                        StyleState::preview().search(SearchState::CurrentMatch)
                    )
                    .fg,
                BLUE
            );
            assert_eq!(
                theme
                    .style(role, StyleState::main().search(SearchState::Match))
                    .fg,
                RED
            );
        }
    }
}
