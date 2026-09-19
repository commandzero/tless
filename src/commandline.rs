use std::borrow::Cow;
use std::fmt::Write;
use std::sync::{Arc, Mutex};

use rustyline::completion::Completer;
use rustyline::highlight::Highlighter;
use rustyline::hint::{Hint, Hinter};
use rustyline::validate::Validator;
use rustyline::{
    Cmd, ConditionalEventHandler, Context, Event, EventContext, EventHandler, Helper, KeyCode,
    KeyEvent, Modifiers, RepeatCount,
};

use crate::command::matching_names;
use crate::terminal::{AnsiTerminal, Style, Terminal};

/// Command-name completion and styling for the shared command/search editor.
pub struct CommandLineHelper {
    prefix: String,
    completion: Arc<Mutex<CompletionState>>,
}

impl CommandLineHelper {
    pub fn new(style: Style) -> Self {
        let mut terminal = AnsiTerminal::new(String::new());
        terminal.reset_style().unwrap();
        terminal.set_style(&style).unwrap();
        Self {
            prefix: terminal.output,
            completion: Arc::default(),
        }
    }

    pub fn bindings(&self) -> EventHandler {
        EventHandler::Conditional(Box::new(CompletionBindings(Arc::clone(&self.completion))))
    }

    pub fn set_prompt(&mut self, prompt: &str) {
        *self.completion.lock().unwrap() = CompletionState {
            enabled: prompt == ":",
            ..CompletionState::default()
        };
    }

    fn command_hint(&self, line: &str, pos: usize, width: usize) -> Option<CommandHint> {
        if !self.completion.lock().unwrap().enabled || pos != line.len() {
            return None;
        }
        let (start, candidates) = candidates(line, pos)?;
        let prefix = &line[start..pos];
        let first = *candidates.first()?;
        if prefix.is_empty() || candidates.contains(&prefix) {
            return None;
        }
        let suffix = &first[prefix.len()..];
        // Command tokens and their leading spaces are ASCII. Reserve the prompt
        // and the terminal's final cell to avoid a bottom-row autowrap.
        let available = width.saturating_sub(line.len() + 2);
        if available == 0 {
            return None;
        }
        Some(CommandHint {
            display: suffix[..suffix.len().min(available)].to_owned(),
            suffix: suffix.to_owned(),
        })
    }

    fn styled(&self, text: &str) -> String {
        let mut output = self.prefix.clone();
        output.write_str(text).unwrap();
        // Keep the background active for Rustyline's erase-to-end-of-line.
        output
    }
}

impl Highlighter for CommandLineHelper {
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

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(format!("{}\x1b[2m{hint}{}", self.prefix, self.prefix))
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

impl Completer for CommandLineHelper {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<String>)> {
        let mut state = self.completion.lock().unwrap();
        if !state.enabled {
            return Ok((pos, Vec::new()));
        }
        let Some((start, mut names)) = candidates(line, pos) else {
            state.cycling = false;
            return Ok((pos, Vec::new()));
        };
        if state.backward {
            names.reverse();
        }
        Ok((start, names.into_iter().map(str::to_owned).collect()))
    }
}
impl Hinter for CommandLineHelper {
    type Hint = CommandHint;

    fn hint(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Option<Self::Hint> {
        let (width, _) = termion::terminal_size().ok()?;
        self.command_hint(line, pos, usize::from(width))
    }
}
impl Validator for CommandLineHelper {}
impl Helper for CommandLineHelper {}

/// Completion text is separate from its clipped on-screen representation.
pub struct CommandHint {
    display: String,
    suffix: String,
}

impl Hint for CommandHint {
    fn display(&self) -> &str {
        &self.display
    }
    fn completion(&self) -> Option<&str> {
        Some(&self.suffix)
    }
}

fn candidates(line: &str, pos: usize) -> Option<(usize, Vec<&'static str>)> {
    let start = line.len() - line.trim_start_matches(' ').len();
    let end = line[start..]
        .find(' ')
        .map_or(line.len(), |end| start + end);
    if pos != end {
        return None;
    }
    let names = matching_names(&line[start..end]);
    if names.is_empty() {
        None
    } else {
        Some((start, names))
    }
}

#[derive(Default)]
struct CompletionState {
    enabled: bool,
    cycling: bool,
    backward: bool,
}

impl CompletionState {
    fn handle(&mut self, key: KeyEvent) -> Option<Cmd> {
        if !self.enabled {
            return None;
        }
        let backward = key == KeyEvent(KeyCode::BackTab, Modifiers::NONE);
        let forward = key == KeyEvent(KeyCode::Tab, Modifiers::NONE) || key == KeyEvent::ctrl('I');
        if forward || backward {
            if !self.cycling {
                self.cycling = true;
                self.backward = backward;
                // Rustyline starts only Cmd::Complete, always at candidate zero.
                // Reverse the candidates for backward initiation, then translate
                // subsequent directions so its native restore slot stays correct.
                return Some(Cmd::Complete);
            }
            return Some(if backward == self.backward {
                Cmd::Complete
            } else {
                Cmd::CompleteBackward
            });
        }
        self.cycling = false;
        None
    }
}

struct CompletionBindings(Arc<Mutex<CompletionState>>);

impl ConditionalEventHandler for CompletionBindings {
    fn handle(
        &self,
        event: &Event,
        _n: RepeatCount,
        _positive: bool,
        _context: &EventContext<'_>,
    ) -> Option<Cmd> {
        let Some(key) = event.get(0) else {
            self.0.lock().unwrap().cycling = false;
            return None;
        };
        self.0.lock().unwrap().handle(*key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::{BLUE, WHITE};

    #[test]
    fn completion_only_matches_at_the_command_token_end() {
        assert_eq!(candidates("  wri café.toon", 5).unwrap().0, 2);
        assert!(candidates("", 0).unwrap().1.contains(&"help"));
        assert!(candidates("  ", 2).unwrap().1.contains(&"write"));
        for (line, pos) in [
            ("write", 2),
            ("write file", 10),
            ("set ", 4),
            ("WR", 2),
            ("é", 1),
            ("é", 2),
        ] {
            assert!(candidates(line, pos).is_none(), "{line:?} at {pos}");
        }
    }

    #[test]
    fn hints_are_long_names_and_only_visible_in_command_mode() {
        let mut helper = CommandLineHelper::new(Style::default());
        for prompt in ["/", "?", ""] {
            helper.set_prompt(prompt);
            assert!(helper.command_hint("se", 2, 80).is_none());
        }
        helper.set_prompt(":");
        assert_eq!(helper.command_hint("se", 2, 80).unwrap().display(), "t");
        assert_eq!(helper.command_hint("h", 1, 80).unwrap().display(), "elp");
        for line in [
            "",
            " ",
            "write",
            "write!",
            "write file",
            "unknown",
            "WR",
            "é",
        ] {
            assert!(
                helper.command_hint(line, line.len(), 80).is_none(),
                "{line:?}"
            );
        }
        assert!(helper.command_hint("wri", 2, 80).is_none());
    }

    #[test]
    fn hints_fit_the_row_and_keep_their_full_completion() {
        let mut helper = CommandLineHelper::new(Style::default());
        helper.set_prompt(":");
        assert!(helper.command_hint("wri", 3, 5).is_none());
        let hint = helper.command_hint("wri", 3, 6).unwrap();
        assert_eq!(hint.display(), "t");
        assert_eq!(hint.completion(), Some("te"));
        assert_eq!(helper.command_hint("wri", 3, 7).unwrap().display(), "te");
    }

    #[test]
    fn reapplies_colors_to_prompts_and_edited_input() {
        let highlighter = CommandLineHelper::new(Style {
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
