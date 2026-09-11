#![allow(dead_code)]

use clap::ValueEnum;

#[cfg(feature = "colorscheme")]
mod borealis;
#[cfg(feature = "colorscheme")]
mod vim;

#[cfg(feature = "colorscheme")]
use crate::terminal::LIGHT_BLUE;
use crate::terminal::{
    Color, Style, BLUE, CYAN, GREEN, LIGHT_BLACK, LIGHT_CYAN, LIGHT_YELLOW, MAGENTA, RED, WHITE,
    YELLOW,
};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum ThemeName {
    #[default]
    #[value(name = "default", alias("classic"))]
    Default,
    #[cfg(feature = "colorscheme")]
    Cyan,
    #[cfg(feature = "colorscheme")]
    Borealis,
    #[cfg(feature = "colorscheme")]
    #[value(name = "blue")]
    VimBlue,
    #[cfg(feature = "colorscheme")]
    #[value(name = "catppuccin")]
    VimCatppuccin,
    #[cfg(feature = "colorscheme")]
    #[value(name = "darkblue")]
    VimDarkblue,
    #[cfg(feature = "colorscheme")]
    #[value(name = "vim")]
    VimDefault,
    #[cfg(feature = "colorscheme")]
    #[value(name = "delek")]
    VimDelek,
    #[cfg(feature = "colorscheme")]
    #[value(name = "desert")]
    VimDesert,
    #[cfg(feature = "colorscheme")]
    #[value(name = "elflord")]
    VimElflord,
    #[cfg(feature = "colorscheme")]
    #[value(name = "evening")]
    VimEvening,
    #[cfg(feature = "colorscheme")]
    #[value(name = "habamax")]
    VimHabamax,
    #[cfg(feature = "colorscheme")]
    #[value(name = "industry")]
    VimIndustry,
    #[cfg(feature = "colorscheme")]
    #[value(name = "koehler")]
    VimKoehler,
    #[cfg(feature = "colorscheme")]
    #[value(name = "lunaperche")]
    VimLunaperche,
    #[cfg(feature = "colorscheme")]
    #[value(name = "morning")]
    VimMorning,
    #[cfg(feature = "colorscheme")]
    #[value(name = "murphy")]
    VimMurphy,
    #[cfg(feature = "colorscheme")]
    #[value(name = "novum")]
    VimNovum,
    #[cfg(feature = "colorscheme")]
    #[value(name = "pablo")]
    VimPablo,
    #[cfg(feature = "colorscheme")]
    #[value(name = "peachpuff")]
    VimPeachpuff,
    #[cfg(feature = "colorscheme")]
    #[value(name = "quiet")]
    VimQuiet,
    #[cfg(feature = "colorscheme")]
    #[value(name = "retrobox")]
    VimRetrobox,
    #[cfg(feature = "colorscheme")]
    #[value(name = "ron")]
    VimRon,
    #[cfg(feature = "colorscheme")]
    #[value(name = "shine")]
    VimShine,
    #[cfg(feature = "colorscheme")]
    #[value(name = "slate")]
    VimSlate,
    #[cfg(feature = "colorscheme")]
    #[value(name = "sorbet")]
    VimSorbet,
    #[cfg(feature = "colorscheme")]
    #[value(name = "torte")]
    VimTorte,
    #[cfg(feature = "colorscheme")]
    #[value(name = "unokai")]
    VimUnokai,
    #[cfg(feature = "colorscheme")]
    #[value(name = "wildcharm")]
    VimWildcharm,
    #[cfg(feature = "colorscheme")]
    #[value(name = "zaibatsu")]
    VimZaibatsu,
    #[cfg(feature = "colorscheme")]
    #[value(name = "zellner")]
    VimZellner,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Theme {
    name: ThemeName,
    #[cfg(feature = "colorscheme")]
    colors: [Option<Color>; THEME_COLOR_COUNT],
}

#[cfg(feature = "colorscheme")]
const THEME_COLOR_COUNT: usize = ThemeColor::CommandLineBackground as usize + 1;

#[cfg(feature = "colorscheme")]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ThemeColor {
    DocumentForeground,
    DocumentBackground,
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

#[cfg(feature = "colorscheme")]
impl ThemeColor {
    pub fn from_config_key(key: &str) -> Option<Self> {
        match key {
            "document-foreground" => Some(Self::DocumentForeground),
            "document-background" => Some(Self::DocumentBackground),
            "null" => Some(Self::Null),
            "boolean" => Some(Self::Boolean),
            "number" => Some(Self::Number),
            "string" => Some(Self::String),
            "empty-container" => Some(Self::EmptyContainer),
            "object-key" => Some(Self::ObjectKey),
            "focused-object-key" => Some(Self::FocusedObjectKey),
            "object-key-focused" => Some(Self::FocusedObjectKey),
            "array-index" => Some(Self::ArrayIndex),
            "punctuation" => Some(Self::Punctuation),
            "primitive-trailing-comma" => Some(Self::PrimitiveTrailingComma),
            "punctuation-comma-trailing" => Some(Self::PrimitiveTrailingComma),
            "container-delimiter" => Some(Self::ContainerDelimiter),
            "focused-container-delimiter" => Some(Self::FocusedContainerDelimiter),
            "container-delimiter-focused" => Some(Self::FocusedContainerDelimiter),
            "ellipsis" => Some(Self::Ellipsis),
            "preview-text" => Some(Self::PreviewText),
            "preview-count" => Some(Self::PreviewCount),
            "line-number" => Some(Self::LineNumber),
            "focused-line-number" => Some(Self::FocusedLineNumber),
            "line-number-focused" => Some(Self::FocusedLineNumber),
            "empty-row-marker" => Some(Self::EmptyRowMarker),
            "row-marker-empty" => Some(Self::EmptyRowMarker),
            "truncation-indicator" => Some(Self::TruncationIndicator),
            "indicator-truncation" => Some(Self::TruncationIndicator),
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
    Document,
    JsonValue(JsonValueKind),
    ObjectKey,
    FieldDefinition,
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
            #[cfg(feature = "colorscheme")]
            colors: [None; THEME_COLOR_COUNT],
        }
    }

    #[cfg(feature = "colorscheme")]
    pub fn with_color(mut self, key: ThemeColor, color: Color) -> Self {
        self.colors[key as usize] = Some(color);
        self
    }

    pub fn style(&self, role: StyleRole, state: StyleState) -> Style {
        let search = {
            #[cfg(feature = "colorscheme")]
            if state.context == DisplayContext::Preview
                && state.search == SearchState::CurrentMatch
                && self.name != ThemeName::Default
            {
                SearchState::Match
            } else {
                state.search
            }
            #[cfg(not(feature = "colorscheme"))]
            {
                state.search
            }
        };

        #[cfg(feature = "colorscheme")]
        let style = if self.name == ThemeName::Borealis {
            borealis::style(role, state, search)
        } else if let Some(palette) = self.name.vim_palette() {
            match search {
                SearchState::None => palette.focused_style(role, state),
                SearchState::Match => palette.search_style(false),
                SearchState::CurrentMatch => palette.search_style(true),
            }
        } else {
            let style = self.apply_focus(role, state.focus, self.base_style(role));
            match search {
                SearchState::None => style,
                SearchState::Match => self.search_match_style(state.context),
                SearchState::CurrentMatch => self.current_match_style(),
            }
        };

        #[cfg(not(feature = "colorscheme"))]
        let style = {
            let style = self.apply_focus(role, state.focus, self.base_style(role));
            match search {
                SearchState::None => style,
                SearchState::Match => self.search_match_style(state.context),
                SearchState::CurrentMatch => self.current_match_style(),
            }
        };

        #[cfg(feature = "colorscheme")]
        match role {
            StyleRole::StatusBar | StyleRole::StatusPathBase => {
                let style =
                    self.apply_foreground_override(style, Self::color_key(role, state, search));
                self.apply_row_colors(
                    style,
                    ThemeColor::StatusBarForeground,
                    ThemeColor::StatusBarBackground,
                )
            }
            StyleRole::StatusText => {
                let style =
                    self.apply_foreground_override(style, Self::color_key(role, state, search));
                self.apply_row_colors(
                    style,
                    ThemeColor::CommandLineForeground,
                    ThemeColor::CommandLineBackground,
                )
            }
            StyleRole::Message(severity) => self.apply_row_colors(
                style,
                match severity {
                    MessageSeverity::Info => ThemeColor::MessageInfo,
                    MessageSeverity::Warn => ThemeColor::MessageWarning,
                    MessageSeverity::Error => ThemeColor::MessageError,
                },
                ThemeColor::CommandLineBackground,
            ),
            _ => {
                let mut style =
                    self.apply_foreground_override(style, Self::color_key(role, state, search));
                if style.fg == Color::Default
                    && self.colors[Self::color_key(role, state, search) as usize].is_none()
                {
                    style.fg = self.colors[ThemeColor::DocumentForeground as usize]
                        .unwrap_or(Color::Default);
                }
                if let Some(background) = self.colors[ThemeColor::DocumentBackground as usize] {
                    style.bg = background;
                }
                style
            }
        }

        #[cfg(not(feature = "colorscheme"))]
        {
            style
        }
    }

    #[cfg(feature = "colorscheme")]
    fn apply_foreground_override(&self, mut style: Style, key: ThemeColor) -> Style {
        if let Some(color) = self.colors[key as usize] {
            // Configured colors describe what users see, independent of reverse video.
            if style.inverted {
                std::mem::swap(&mut style.fg, &mut style.bg);
                style.inverted = false;
            }
            style.fg = color;
        }
        style
    }

    #[cfg(feature = "colorscheme")]
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

    #[cfg(feature = "colorscheme")]
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
            (StyleRole::Document, _) => ThemeColor::DocumentForeground,
            (StyleRole::ObjectKey | StyleRole::FieldDefinition, FocusState::Row) => {
                ThemeColor::FocusedObjectKey
            }
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
            (StyleRole::ObjectKey | StyleRole::FieldDefinition, _) => ThemeColor::ObjectKey,
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

    fn legacy_name(&self) -> LegacyTheme {
        #[cfg(feature = "colorscheme")]
        {
            if self.name == ThemeName::Cyan {
                LegacyTheme::Cyan
            } else {
                LegacyTheme::Classic
            }
        }
        #[cfg(not(feature = "colorscheme"))]
        {
            LegacyTheme::Classic
        }
    }

    /// Background shared by the gutter, document cells, and unused row width.
    pub fn row_style(&self, focused: bool) -> Style {
        let mut row = self.style(StyleRole::Document, StyleState::main());
        if focused {
            let focus = self.style(StyleRole::ObjectKey, StyleState::main().focused());
            row.bg = if focus.inverted { focus.fg } else { focus.bg };
        }
        row
    }

    pub fn style_on_row(&self, role: StyleRole, state: StyleState, focused: bool) -> Style {
        let mut style = self.style(role, state);
        if focused && state.search == SearchState::None {
            let row = self.row_style(true);
            if row.bg != self.row_style(false).bg {
                // Preserve the visible text color of reverse-video Vim styles.
                if style.inverted {
                    std::mem::swap(&mut style.fg, &mut style.bg);
                    style.inverted = false;
                }
                style.bg = row.bg;
            }
        }
        style
    }

    pub fn document_style(&self) -> Style {
        #[cfg(feature = "colorscheme")]
        {
            if self.name == ThemeName::Borealis {
                return borealis::style(StyleRole::Document, StyleState::main(), SearchState::None);
            }
            self.name.vim_palette().map_or(Style::default(), |palette| {
                palette.base_style(StyleRole::Document)
            })
        }
        #[cfg(not(feature = "colorscheme"))]
        {
            Style::default()
        }
    }

    fn base_style(&self, role: StyleRole) -> Style {
        let color = |fg: Color| Style {
            fg,
            ..Style::default()
        };

        match role {
            StyleRole::Document => self.document_style(),
            StyleRole::JsonValue(kind) => color(self.json_value_color(kind)),
            StyleRole::ObjectKey | StyleRole::FieldDefinition => color(match self.legacy_name() {
                LegacyTheme::Classic => CYAN,
                #[cfg(feature = "colorscheme")]
                LegacyTheme::Cyan => CYAN,
            }),
            StyleRole::ArrayIndex | StyleRole::LineNumber | StyleRole::Ellipsis => {
                color(LIGHT_BLACK)
            }
            StyleRole::Punctuation | StyleRole::StatusText => Style::default(),
            StyleRole::PrimitiveTrailingComma => match self.legacy_name() {
                LegacyTheme::Classic => Style::default(),
                #[cfg(feature = "colorscheme")]
                LegacyTheme::Cyan => Style {
                    dimmed: true,
                    ..Style::default()
                },
            },
            StyleRole::ContainerDelimiter => match self.legacy_name() {
                LegacyTheme::Classic => color(LIGHT_BLACK),
                #[cfg(feature = "colorscheme")]
                LegacyTheme::Cyan => Style {
                    dimmed: true,
                    ..Style::default()
                },
            },
            StyleRole::PreviewText => color(LIGHT_BLACK),
            StyleRole::PreviewCount | StyleRole::EmptyRowMarker => color(LIGHT_BLACK),
            StyleRole::TruncationIndicator => color(LIGHT_BLACK),
            StyleRole::StatusBar => Style {
                fg: crate::terminal::BLACK,
                bg: LIGHT_BLACK,
                ..Style::default()
            },
            StyleRole::StatusPathBase => Style {
                fg: WHITE,
                bg: LIGHT_BLACK,
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
        match (self.legacy_name(), kind) {
            (LegacyTheme::Classic, JsonValueKind::Null) => WHITE,
            #[cfg(feature = "colorscheme")]
            (LegacyTheme::Cyan, JsonValueKind::Null) => LIGHT_BLUE,
            (LegacyTheme::Classic, JsonValueKind::Boolean) => BLUE,
            #[cfg(feature = "colorscheme")]
            (LegacyTheme::Cyan, JsonValueKind::Boolean) => MAGENTA,
            (_, JsonValueKind::Number) => MAGENTA,
            (_, JsonValueKind::String) => GREEN,
            (LegacyTheme::Classic, JsonValueKind::EmptyObject | JsonValueKind::EmptyArray) => WHITE,
            #[cfg(feature = "colorscheme")]
            (LegacyTheme::Cyan, JsonValueKind::EmptyObject | JsonValueKind::EmptyArray) => {
                LIGHT_BLACK
            }
        }
    }

    fn apply_focus(&self, role: StyleRole, focus: FocusState, style: Style) -> Style {
        match (role, focus) {
            (_, FocusState::None) => style,
            (StyleRole::JsonValue(_), FocusState::Row) => match self.legacy_name() {
                LegacyTheme::Classic => Style {
                    fg: style.fg.bright(),
                    ..style
                },
                #[cfg(feature = "colorscheme")]
                LegacyTheme::Cyan => style,
            },
            (StyleRole::ObjectKey | StyleRole::FieldDefinition, FocusState::Row) => {
                match self.legacy_name() {
                    LegacyTheme::Classic => Style {
                        fg: LIGHT_CYAN,
                        ..style
                    },
                    #[cfg(feature = "colorscheme")]
                    LegacyTheme::Cyan => Style {
                        fg: LIGHT_CYAN,
                        ..Style::default()
                    },
                }
            }
            (StyleRole::ArrayIndex, FocusState::Row) => match self.legacy_name() {
                LegacyTheme::Classic => Style { fg: WHITE, ..style },
                #[cfg(feature = "colorscheme")]
                LegacyTheme::Cyan => Style {
                    inverted: true,
                    bold: true,
                    ..Style::default()
                },
            },
            (StyleRole::Punctuation | StyleRole::PrimitiveTrailingComma, FocusState::Row) => {
                match self.legacy_name() {
                    LegacyTheme::Classic => Style {
                        fg: crate::terminal::LIGHT_WHITE,
                        ..style
                    },
                    #[cfg(feature = "colorscheme")]
                    LegacyTheme::Cyan => style,
                }
            }
            (StyleRole::ContainerDelimiter, FocusState::Row | FocusState::PairedContainer) => {
                match self.legacy_name() {
                    LegacyTheme::Classic => Style { fg: WHITE, ..style },
                    #[cfg(feature = "colorscheme")]
                    LegacyTheme::Cyan => Style {
                        fg: YELLOW,
                        ..Style::default()
                    },
                }
            }
            (StyleRole::LineNumber, FocusState::Row) => match self.legacy_name() {
                LegacyTheme::Classic => Style { fg: WHITE, ..style },
                #[cfg(feature = "colorscheme")]
                LegacyTheme::Cyan => Style {
                    fg: YELLOW,
                    ..Style::default()
                },
            },
            (StyleRole::TruncationIndicator, FocusState::Row) => match self.legacy_name() {
                LegacyTheme::Classic => Style { fg: WHITE, ..style },
                #[cfg(feature = "colorscheme")]
                LegacyTheme::Cyan => Style {
                    bold: true,
                    ..Style::default()
                },
            },
            _ => style,
        }
    }

    fn search_match_style(&self, context: DisplayContext) -> Style {
        let _ = context;
        #[cfg(feature = "colorscheme")]
        if context == DisplayContext::Preview && self.name == ThemeName::Cyan {
            return Style {
                fg: LIGHT_BLACK,
                ..Style::default()
            };
        }
        Style {
            fg: YELLOW,
            underlined: true,
            ..Style::default()
        }
    }

    fn current_match_style(&self) -> Style {
        match self.legacy_name() {
            LegacyTheme::Classic => Style {
                fg: LIGHT_YELLOW,
                underlined: true,
                ..Style::default()
            },
            #[cfg(feature = "colorscheme")]
            LegacyTheme::Cyan => Style {
                fg: LIGHT_YELLOW,
                underlined: true,
                ..Style::default()
            },
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
enum LegacyTheme {
    Classic,
    #[cfg(feature = "colorscheme")]
    Cyan,
}

impl Default for Theme {
    fn default() -> Self {
        Self::built_in(ThemeName::Default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "colorscheme")]
    use crate::terminal::LIGHT_WHITE;

    fn fg(color: Color) -> Style {
        Style {
            fg: color,
            ..Style::default()
        }
    }

    #[test]
    #[cfg(feature = "colorscheme")]
    fn vim_palettes_preserve_source_colors_and_light_backgrounds() {
        let peachpuff = Theme::built_in(ThemeName::VimPeachpuff);
        let normal = peachpuff.document_style();
        assert_eq!(normal.fg, Color::C256(16));
        assert_eq!(normal.bg, Color::C256(223));
        let string = peachpuff.style(
            StyleRole::JsonValue(JsonValueKind::String),
            StyleState::main(),
        );
        assert_eq!(string.fg, Color::C256(161));
        assert_eq!(string.bg, normal.bg);
        let darkblue = Theme::built_in(ThemeName::VimDarkblue);
        assert_eq!(darkblue.document_style().bg, Color::C256(17));
        assert_eq!(
            darkblue.style(StyleRole::ObjectKey, StyleState::main()).fg,
            Color::C256(123)
        );
    }

    #[test]
    #[cfg(feature = "colorscheme")]
    fn every_vim_companion_preserves_focus_search_and_config_precedence() {
        let mut count = 0;
        for &name in ThemeName::value_variants() {
            if name.vim_palette().is_none() {
                continue;
            }
            count += 1;
            let theme = Theme::built_in(name);
            let state = StyleState::main();
            let role = StyleRole::ObjectKey;
            assert_ne!(
                theme.style(role, state),
                theme.style(role, state.focused()),
                "{name:?}"
            );
            assert_ne!(
                theme.style(role, state.search(SearchState::Match)),
                theme.style(role, state.search(SearchState::CurrentMatch)),
                "{name:?}",
            );
            assert_eq!(
                theme.style(
                    role,
                    StyleState::preview().search(SearchState::CurrentMatch)
                ),
                theme.style(role, StyleState::preview().search(SearchState::Match)),
            );
            let normal = theme.style(role, state);
            let configured = theme.with_color(ThemeColor::ObjectKey, GREEN);
            assert_eq!(
                configured.style(role, state),
                Style {
                    fg: GREEN,
                    ..normal
                }
            );
            let configured = theme
                .with_color(ThemeColor::CommandLineBackground, BLUE)
                .with_color(ThemeColor::MessageError, RED);
            let message = configured.style(StyleRole::Message(MessageSeverity::Error), state);
            assert_eq!(message.fg, RED);
            assert_eq!(message.bg, BLUE);
        }
        assert_eq!(count, 28);
    }

    #[test]
    fn default_json_palette_matches_main() {
        let theme = Theme::default();
        let state = StyleState::main();

        for (kind, expected) in [
            (JsonValueKind::Null, WHITE),
            (JsonValueKind::Boolean, BLUE),
            (JsonValueKind::Number, MAGENTA),
            (JsonValueKind::String, GREEN),
            (JsonValueKind::EmptyObject, WHITE),
            (JsonValueKind::EmptyArray, WHITE),
        ] {
            assert_eq!(theme.style(StyleRole::JsonValue(kind), state), fg(expected));
        }
    }

    #[cfg(feature = "colorscheme")]
    #[test]
    fn document_colors_fill_defaults_without_overriding_syntax_or_status() {
        let theme = Theme::default()
            .with_color(ThemeColor::DocumentForeground, Color::C256(252))
            .with_color(ThemeColor::DocumentBackground, Color::C256(17));
        let state = StyleState::main();
        for role in [StyleRole::Document, StyleRole::Punctuation] {
            let style = theme.style(role, state);
            assert_eq!(style.fg, Color::C256(252));
            assert_eq!(style.bg, Color::C256(17));
        }
        assert_eq!(
            theme
                .style(StyleRole::JsonValue(JsonValueKind::String), state)
                .fg,
            GREEN
        );
        for role in [
            StyleRole::ObjectKey,
            StyleRole::JsonValue(JsonValueKind::String),
            StyleRole::ContainerDelimiter,
            StyleRole::LineNumber,
            StyleRole::PreviewText,
            StyleRole::EmptyRowMarker,
        ] {
            assert_eq!(theme.style(role, state).bg, Color::C256(17));
        }
        for role in [
            StyleRole::StatusBar,
            StyleRole::StatusPathBase,
            StyleRole::StatusText,
            StyleRole::Message(MessageSeverity::Error),
        ] {
            assert_eq!(
                theme.style(role, state),
                Theme::default().style(role, state)
            );
        }
    }

    #[cfg(feature = "colorscheme")]
    #[test]
    fn selection_overrides_keep_focus_attributes() {
        let theme = Theme::default()
            .with_color(ThemeColor::DocumentBackground, Color::C256(231))
            .with_color(ThemeColor::FocusedObjectKey, Color::C256(164));
        let style = theme.style(StyleRole::ObjectKey, StyleState::main().focused());
        assert_eq!(style.fg, Color::C256(164));
        assert_eq!(style.bg, Color::C256(231));
        assert!(!style.inverted);
        assert!(!style.bold);
    }

    #[test]
    #[cfg(feature = "colorscheme")]
    fn configured_colors_normalize_reverse_video() {
        let search = Theme::built_in(ThemeName::VimBlue)
            .with_color(ThemeColor::SearchMatch, RED)
            .style(
                StyleRole::ObjectKey,
                StyleState::main().search(SearchState::Match),
            );
        assert_eq!(search.fg, RED);
        assert!(!search.inverted);

        let delimiter = Theme::built_in(ThemeName::VimBlue)
            .with_color(ThemeColor::FocusedContainerDelimiter, GREEN)
            .style(StyleRole::ContainerDelimiter, StyleState::main().focused());
        assert_eq!(delimiter.fg, GREEN);
        assert!(!delimiter.inverted);

        let message = Theme::built_in(ThemeName::VimDarkblue)
            .with_color(ThemeColor::MessageError, RED)
            .with_color(ThemeColor::CommandLineBackground, BLUE)
            .style(
                StyleRole::Message(MessageSeverity::Error),
                StyleState::main(),
            );
        assert_eq!(message.fg, RED);
        assert_eq!(message.bg, BLUE);
        assert!(!message.inverted);
    }

    #[test]
    fn default_base_roles_match_main() {
        let theme = Theme::default();
        let state = StyleState::main();

        for (role, expected) in [
            (StyleRole::ObjectKey, fg(CYAN)),
            (StyleRole::ArrayIndex, fg(LIGHT_BLACK)),
            (StyleRole::Punctuation, Style::default()),
            (StyleRole::PrimitiveTrailingComma, Style::default()),
            (StyleRole::ContainerDelimiter, fg(LIGHT_BLACK)),
            (StyleRole::Ellipsis, fg(LIGHT_BLACK)),
            (StyleRole::PreviewText, fg(LIGHT_BLACK)),
            (StyleRole::PreviewCount, fg(LIGHT_BLACK)),
            (StyleRole::LineNumber, fg(LIGHT_BLACK)),
            (StyleRole::EmptyRowMarker, fg(LIGHT_BLACK)),
            (StyleRole::TruncationIndicator, fg(LIGHT_BLACK)),
            (
                StyleRole::StatusBar,
                Style {
                    fg: crate::terminal::BLACK,
                    bg: LIGHT_BLACK,
                    ..Style::default()
                },
            ),
            (
                StyleRole::StatusPathBase,
                Style {
                    fg: WHITE,
                    bg: LIGHT_BLACK,
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
    #[cfg(feature = "colorscheme")]
    fn cyan_palette_contains_only_specified_base_differences() {
        let default_theme = Theme::built_in(ThemeName::Default);
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
            assert_eq!(cyan.style(role, state), default_theme.style(role, state));
        }
    }

    #[test]
    #[cfg(feature = "colorscheme")]
    fn focus_and_search_precedence_is_explicit() {
        let default_theme = Theme::default();
        let cyan = Theme::built_in(ThemeName::Cyan);

        assert_eq!(
            default_theme.style(StyleRole::ObjectKey, StyleState::main().focused()),
            fg(LIGHT_CYAN)
        );
        for (kind, expected) in [
            (JsonValueKind::String, Color::C16(10)),
            (JsonValueKind::Number, Color::C16(13)),
            (JsonValueKind::Boolean, LIGHT_BLUE),
            (JsonValueKind::Null, LIGHT_WHITE),
        ] {
            assert_eq!(
                default_theme.style(StyleRole::JsonValue(kind), StyleState::main().focused()),
                fg(expected)
            );
        }
        assert_eq!(
            default_theme.style(StyleRole::Punctuation, StyleState::main().focused()),
            fg(LIGHT_WHITE)
        );
        assert_eq!(
            default_theme.style(
                StyleRole::PrimitiveTrailingComma,
                StyleState::main().focused()
            ),
            fg(LIGHT_WHITE)
        );
        assert_eq!(
            default_theme.style(StyleRole::ContainerDelimiter, StyleState::main().focused()),
            fg(WHITE)
        );
        assert_eq!(
            default_theme.style(StyleRole::LineNumber, StyleState::main().focused()),
            fg(WHITE)
        );
        assert_eq!(
            default_theme.style(
                StyleRole::JsonValue(JsonValueKind::String),
                StyleState::main().search(SearchState::Match)
            ),
            Style {
                fg: YELLOW,
                underlined: true,
                ..Style::default()
            }
        );
        assert_eq!(
            cyan.style(StyleRole::ObjectKey, StyleState::main().focused()),
            fg(LIGHT_CYAN)
        );
        assert_eq!(
            default_theme.style(
                StyleRole::ObjectKey,
                StyleState::main()
                    .focused()
                    .search(SearchState::CurrentMatch),
            ),
            Style {
                fg: LIGHT_YELLOW,
                underlined: true,
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
                underlined: true,
                ..Style::default()
            }
        );
    }

    #[test]
    #[cfg(feature = "colorscheme")]
    fn preview_vim_match_reduces_current_match_to_ordinary_match() {
        let default_theme = Theme::default();
        let cyan = Theme::built_in(ThemeName::Cyan);
        let state = StyleState::preview().search(SearchState::CurrentMatch);

        assert_eq!(
            default_theme.style(StyleRole::PreviewText, state),
            Style {
                fg: LIGHT_YELLOW,
                underlined: true,
                ..Style::default()
            }
        );
        assert_eq!(cyan.style(StyleRole::PreviewText, state), fg(LIGHT_BLACK));
    }

    #[test]
    #[cfg(feature = "colorscheme")]
    fn paired_container_focus_uses_theme_delimiter_style() {
        let default_theme = Theme::default();
        let cyan = Theme::built_in(ThemeName::Cyan);
        let state = StyleState::main().paired_container();

        assert_eq!(
            default_theme.style(StyleRole::ContainerDelimiter, state),
            fg(WHITE)
        );
        assert_eq!(cyan.style(StyleRole::ContainerDelimiter, state), fg(YELLOW));
    }

    #[test]
    fn status_roles_preserve_default_styles() {
        let theme = Theme::default();
        let state = StyleState::main();

        assert_eq!(
            theme.style(StyleRole::StatusBar, state),
            Style {
                fg: crate::terminal::BLACK,
                bg: LIGHT_BLACK,
                ..Style::default()
            }
        );
        assert_eq!(
            theme.style(StyleRole::StatusPathBase, state),
            Style {
                fg: WHITE,
                bg: LIGHT_BLACK,
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
    #[cfg(feature = "colorscheme")]
    fn preview_search_override_uses_context_for_every_role() {
        let theme = Theme::default()
            .with_color(ThemeColor::SearchMatchPreview, BLUE)
            .with_color(ThemeColor::SearchMatch, RED)
            .with_color(ThemeColor::SearchMatchCurrent, GREEN);
        for role in [
            StyleRole::PreviewText,
            StyleRole::ObjectKey,
            StyleRole::Ellipsis,
        ] {
            let state = StyleState::preview().search(SearchState::Match);
            assert_eq!(theme.style(role, state).fg, BLUE);
            assert_eq!(
                theme
                    .style(
                        role,
                        StyleState::preview().search(SearchState::CurrentMatch)
                    )
                    .fg,
                GREEN
            );
        }
    }
}
