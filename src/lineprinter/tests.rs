use super::*;
use crate::flatjson::{parse_top_level_json, parse_top_level_yaml};
use crate::terminal::Terminal;
use crate::terminal::test::{TextOnlyTerminal, VisibleEscapesTerminal};
use crate::theme::Theme;
use crate::toon_display::{Span, TokenRole, fixture::Fixture};

mod annotations;
mod painting;
mod viewport;

fn text(line: &DisplayLine, width: usize, offset: usize) -> String {
    let mut terminal = TextOnlyTerminal::new();
    LinePainter::new(line, usize::MAX..usize::MAX)
        .paint(&mut terminal, LineViewport::new(line, offset, 0), width)
        .unwrap();
    terminal.output().to_string()
}
