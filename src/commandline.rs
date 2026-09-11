use std::borrow::Cow;
use std::fmt::Write;

use rustyline::Helper;
use rustyline::completion::Completer;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;

use crate::terminal::{AnsiTerminal, Style, Terminal};

/// Reapply the command row's style whenever Rustyline redraws its prompt or input.
pub struct CommandLineHighlighter {
    prefix: String,
}

impl CommandLineHighlighter {
    pub fn new(style: Style) -> Self {
        let mut terminal = AnsiTerminal::new(String::new());
        terminal.reset_style().unwrap();
        terminal.set_style(&style).unwrap();
        Self {
            prefix: terminal.output,
        }
    }

    fn styled(&self, text: &str) -> String {
        let mut output = self.prefix.clone();
        output.write_str(text).unwrap();
        // Keep the background active for Rustyline's erase-to-end-of-line.
        output
    }
}

impl Highlighter for CommandLineHighlighter {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        Cow::Owned(self.styled(line))
    }

    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        Cow::Owned(self.styled(prompt))
    }

    fn highlight_char(
        &self,
        _line: &str,
        _pos: usize,
        _kind: rustyline::highlight::CmdKind,
    ) -> bool {
        true
    }
}

impl Completer for CommandLineHighlighter {
    type Candidate = String;
}
impl Hinter for CommandLineHighlighter {
    type Hint = String;
}
impl Validator for CommandLineHighlighter {}
impl Helper for CommandLineHighlighter {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::{BLUE, WHITE};

    #[test]
    fn reapplies_colors_to_prompts_and_edited_input() {
        let highlighter = CommandLineHighlighter::new(Style {
            fg: WHITE,
            bg: BLUE,
            ..Style::default()
        });
        let prefix = "\x1b[0m\x1b[38;5;7m\x1b[48;5;4m";
        for prompt in [":", "/", "?"] {
            assert_eq!(
                highlighter.highlight_prompt(prompt, true),
                format!("{prefix}{prompt}")
            );
        }
        for line in ["", "hello", "hé", "h"] {
            assert_eq!(
                highlighter.highlight(line, line.len()),
                format!("{prefix}{line}")
            );
            assert!(highlighter.highlight_char(
                line,
                line.len(),
                rustyline::highlight::CmdKind::Other
            ));
        }
    }
}
