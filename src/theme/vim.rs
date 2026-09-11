use super::{FocusState, JsonValueKind, MessageSeverity, StyleRole, StyleState, ThemeName};
use crate::terminal::{Color, Style};

#[path = "vim_palettes.rs"]
mod palettes;

pub(super) struct VimPalette {
    normal: Style,
    constant: Style,
    boolean: Style,
    number: Style,
    string: Style,
    identifier: Style,
    delimiter: Style,
    comment: Style,
    linenr: Style,
    cursorlinenr: Style,
    nontext: Style,
    statusline: Style,
    modemsg: Style,
    warningmsg: Style,
    errormsg: Style,
    search: Style,
    incsearch: Style,
    matchparen: Style,
    visual: Style,
}

const fn s(fg: u8, bg: u8, flags: u8) -> Style {
    Style {
        fg: Color::C256(fg),
        bg: Color::C256(bg),
        bold: flags & 1 != 0,
        underlined: flags & 2 != 0,
        inverted: flags & 4 != 0,
        ..Style::default()
    }
}

impl VimPalette {
    pub(super) fn base_style(&self, role: StyleRole) -> Style {
        match role {
            StyleRole::Document => self.normal,
            StyleRole::JsonValue(kind) => match kind {
                JsonValueKind::Null => self.constant,
                JsonValueKind::Boolean => self.boolean,
                JsonValueKind::Number => self.number,
                JsonValueKind::String => self.string,
                JsonValueKind::EmptyObject | JsonValueKind::EmptyArray => self.delimiter,
            },
            StyleRole::ObjectKey | StyleRole::FieldDefinition => self.identifier,
            StyleRole::ArrayIndex | StyleRole::LineNumber => self.linenr,
            StyleRole::Punctuation | StyleRole::StatusText => self.normal,
            StyleRole::PrimitiveTrailingComma | StyleRole::ContainerDelimiter => self.delimiter,
            StyleRole::Ellipsis | StyleRole::PreviewText | StyleRole::PreviewCount => self.comment,
            StyleRole::EmptyRowMarker | StyleRole::TruncationIndicator => self.nontext,
            StyleRole::StatusBar | StyleRole::StatusPathBase => self.statusline,
            StyleRole::Message(severity) => match severity {
                MessageSeverity::Info => self.modemsg,
                MessageSeverity::Warn => self.warningmsg,
                MessageSeverity::Error => self.errormsg,
            },
        }
    }

    pub(super) fn focused_style(&self, role: StyleRole, state: StyleState) -> Style {
        let base = self.base_style(role);
        let mut style = match (role, state.focus) {
            (
                StyleRole::ObjectKey | StyleRole::FieldDefinition | StyleRole::ArrayIndex,
                FocusState::Row,
            ) => self.visual,
            (StyleRole::ContainerDelimiter, FocusState::Row | FocusState::PairedContainer) => {
                self.matchparen
            }
            (StyleRole::LineNumber, FocusState::Row) => self.cursorlinenr,
            (StyleRole::TruncationIndicator, FocusState::Row) => self.nontext,
            _ => return base,
        };
        // Focus must remain visible even when Vim links a selection group to Normal.
        style.bold = true;
        style.underlined = true;
        style
    }

    pub(super) fn search_style(&self, current: bool) -> Style {
        if current {
            Style {
                bold: true,
                underlined: true,
                ..self.incsearch
            }
        } else {
            self.search
        }
    }
}
