use std::fs::File;
use std::io;
use std::io::Write;
use std::ops::ControlFlow;

use arboard::Clipboard;
use rustyline::error::ReadlineError;
use termion::event::Key;
use termion::event::MouseButton::{Left, WheelDown, WheelUp};
use termion::event::MouseEvent::Press;
use termion::screen::{ToAlternateScreen, ToMainScreen};

use crate::command::{Command, WriteFormat};
#[cfg(feature = "colorscheme")]
use crate::config::Config;
use crate::flatjson;
use crate::input::TuiEvent;
use crate::input::TuiEvent::{KeyEvent, MouseEvent, WinChEvent};
use crate::jsonstringunescaper::safe_unescape_json_string;
use crate::lineprinter::JS_IDENTIFIER;
use crate::options::{Opt, OutputFormat};
use crate::screenwriter::{MessageSeverity, ScreenWriter};
use crate::search::{JumpDirection, SearchDirection, SearchState};
use crate::theme::Theme;
use crate::types::TTYDimensions;
use crate::viewer::{Action, JsonViewer};

pub struct App {
    #[cfg(feature = "colorscheme")]
    config: Config,
    viewer: JsonViewer,
    screen_writer: ScreenWriter,
    input_state: InputState,
    input_buffer: Vec<u8>,
    input_filename: String,
    search_state: SearchState,
    message: Option<(String, MessageSeverity)>,
    clipboard_context: Result<Clipboard, arboard::Error>,
}

// State to determine how to process the next event input.
//
// The default state accepts most commands, and also buffers
// number inputs to provide a count for movement commands.
//
// Other commands require a combination of (non-numeric) key
// presses. When one of these commands is partially inputted,
// pressing a key not part of the combination will cancel
// the combination command, and no action will be performed.
#[derive(Clone, Copy, PartialEq)]
enum InputState {
    Default,
    PendingPCommand,
    PendingYCommand,
    PendingZCommand,
    WaitingForAnyKeyPress,
}

enum EventOutcome {
    Redraw(Option<Action>),
    RevealMatch(Option<Action>),
    KeepScreen,
    Quit,
}

// Various things that can be copied/printed.
#[derive(Copy, Clone)]
enum ContentTarget {
    PrettyPrintedValue,
    ToonValue,
    OneLineValue,
    String,
    Key,
    DotPath,
    BracketPath,
    QueryPath,
}

impl ContentTarget {
    fn from_command_key(event: TuiEvent, value_key: char, dot_path_key: char) -> Option<Self> {
        let KeyEvent(Key::Char(key)) = event else {
            return None;
        };
        match key {
            key if key == value_key => Some(Self::PrettyPrintedValue),
            key if key == dot_path_key => Some(Self::DotPath),
            't' => Some(Self::ToonValue),
            'v' => Some(Self::OneLineValue),
            's' => Some(Self::String),
            'k' => Some(Self::Key),
            'b' => Some(Self::BracketPath),
            'q' => Some(Self::QueryPath),
            _ => None,
        }
    }
}

// Help contents that we pipe to less.
const HELP: &str = std::include_str!("./tless.help");
const TOON_HELP: &str = std::include_str!("./toon.help");

pub const MAX_BUFFER_SIZE: usize = 9;
const BELL: &str = "\x07";

// https://docs.rs/termion/2.0.1/src/termion/input.rs.html#176-180
//
// The termion MouseTerminal sends the following escape codes:
//
// ESC [ ? 1000 h
// ESC [ ? 1002 h
// ESC [ ? 1015 h
// ESC [ ? 1006 h
//
// https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
//
// 1000 enables better mouse support; 1002 enables button-event tracking,
// then 1015 and 1006 change the format that mouse events are sent in.
// (It seems like 1006 overrides 1015, but probably some legacy issue somewhere.)
//
// When we print stuff to the screen with 'p', we want to allow the user to
// highlight it with their mouse, so we disable the regular mouse button tracking.
const DISABLE_MOUSE_BUTTON_TRACKING: &str = "\x1b[?1002l";
const ENABLE_MOUSE_BUTTON_TRACKING: &str = "\x1b[?1002h";

impl App {
    pub fn new(
        opt: &Opt,
        theme: Theme,
        #[cfg(feature = "colorscheme")] config: Config,
        flatjson: flatjson::FlatJson,
        roots: Vec<usize>,
        input_filename: String,
        stdout: crate::screenwriter::TerminalOutput,
    ) -> App {
        let dimensions = TTYDimensions::from_size(termion::terminal_size().unwrap());
        let mut viewer = JsonViewer::with_roots(
            flatjson,
            roots,
            dimensions.without_status_bar(),
            opt.show_line_numbers || opt.show_relative_line_numbers,
            opt.path.is_some(),
        );
        viewer.scrolloff_setting = opt.scrolloff;

        let screen_writer = ScreenWriter::init(opt, theme, stdout, dimensions);

        App {
            #[cfg(feature = "colorscheme")]
            config,
            viewer,
            screen_writer,
            input_state: InputState::Default,
            input_buffer: vec![],
            input_filename,
            search_state: SearchState::empty(),
            message: None,
            clipboard_context: Clipboard::new(),
        }
    }

    pub fn run(&mut self, input: Box<dyn Iterator<Item = io::Result<TuiEvent>>>) {
        self.refresh_terminal_dimensions();
        self.draw_screen();

        for event in input {
            let event = match event {
                Ok(event) => event,
                Err(io_error) => {
                    self.set_error_message(format!("Error: {io_error}"));
                    self.draw_status_bar();
                    continue;
                }
            };
            if self.handle_event(event).is_break() {
                break;
            }
        }
    }

    fn handle_event(&mut self, event: TuiEvent) -> ControlFlow<()> {
        match (self.input_state, &event) {
            // Consume the first key after printing, regardless of its usual meaning.
            (InputState::WaitingForAnyKeyPress, KeyEvent(_)) => {
                let _ = write!(self.screen_writer.stdout, "{ToAlternateScreen}");
                let _ = write!(self.screen_writer.stdout, "{ENABLE_MOUSE_BUTTON_TRACKING}");
                self.input_state = InputState::Default;
                self.draw_screen();
                self.message = None;
                return ControlFlow::Continue(());
            }
            (InputState::WaitingForAnyKeyPress, _) => return ControlFlow::Continue(()),
            (_, KeyEvent(Key::Ctrl('z'))) => {
                self.suspend();
                return ControlFlow::Continue(());
            }
            _ => {}
        }

        let previous_focus = self.viewer.focused_node;
        let was_collapsed = self.viewer.effective_collapsed(previous_focus);
        let (action, reveal_match) = match self.event_outcome(event) {
            EventOutcome::Redraw(action) => (action, false),
            EventOutcome::RevealMatch(action) => (action, true),
            EventOutcome::KeepScreen => return ControlFlow::Continue(()),
            EventOutcome::Quit => return ControlFlow::Break(()),
        };

        if let Some(action) = action {
            self.apply_viewer_action(action, previous_focus);
        }
        self.update_search_after_event(reveal_match, previous_focus, was_collapsed);
        self.draw_screen();
        self.message = None;
        ControlFlow::Continue(())
    }

    fn suspend(&mut self) {
        let _ = self.screen_writer.stdout.suspend_raw_mode();
        let _ = write!(self.screen_writer.stdout, "{DISABLE_MOUSE_BUTTON_TRACKING}");
        let _ = write!(self.screen_writer.stdout, "{ToMainScreen}");
        let _ = write!(self.screen_writer.stdout, "{}", termion::cursor::Show);
        let _ = self.screen_writer.stdout.flush();
        let _ = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(0),
            nix::sys::signal::Signal::SIGSTOP,
        );
        let _ = write!(self.screen_writer.stdout, "{}", termion::cursor::Hide);
        let _ = write!(self.screen_writer.stdout, "{ToAlternateScreen}");
        let _ = write!(self.screen_writer.stdout, "{ENABLE_MOUSE_BUTTON_TRACKING}");
        let _ = self.screen_writer.stdout.activate_raw_mode();
        self.draw_screen();
    }

    fn refresh_terminal_dimensions(&mut self) {
        self.screen_writer.dimensions = TTYDimensions::from_size(termion::terminal_size().unwrap());
    }

    fn redraw_after_layout_change(&self, action: Option<Action>) -> EventOutcome {
        if self.search_state.active_search_state().is_some() {
            EventOutcome::RevealMatch(action)
        } else {
            EventOutcome::Redraw(action)
        }
    }

    fn event_outcome(&mut self, event: TuiEvent) -> EventOutcome {
        match (self.input_state, event) {
            // A resize preserves a pending command.
            (_, WinChEvent) => {
                self.refresh_terminal_dimensions();
                self.redraw_after_layout_change(Some(Action::ResizeViewerDimensions(
                    self.screen_writer.dimensions.without_status_bar(),
                )))
            }
            (InputState::PendingPCommand, event) => {
                if self.finish_print_command(event) {
                    EventOutcome::KeepScreen
                } else {
                    EventOutcome::Redraw(None)
                }
            }
            (InputState::PendingYCommand, event) => {
                self.finish_copy_command(event);
                EventOutcome::Redraw(None)
            }
            (InputState::PendingZCommand, event) => {
                EventOutcome::Redraw(self.finish_position_command(event))
            }
            (_, KeyEvent(key)) => self.key_outcome(key),
            (_, MouseEvent(event)) => self.mouse_outcome(event),
            (_, TuiEvent::Unknown(bytes)) => {
                self.set_error_message(format!("Unknown byte sequence: {bytes:?}"));
                EventOutcome::Redraw(None)
            }
        }
    }

    fn mouse_outcome(&mut self, event: termion::event::MouseEvent) -> EventOutcome {
        self.input_buffer.clear();
        let action = match event {
            Press(Left, column, row)
                if row <= self.screen_writer.dimensions.without_status_bar().height =>
            {
                self.screen_writer.mouse_action(&self.viewer, row, column)
            }
            Press(WheelUp, _, _) => Action::ScrollUp(3),
            Press(WheelDown, _, _) => Action::ScrollDown(3),
            _ => return EventOutcome::KeepScreen,
        };
        EventOutcome::Redraw(Some(action))
    }

    fn key_outcome(&mut self, key: Key) -> EventOutcome {
        match key {
            Key::Ctrl('c') | Key::Char('q') => return EventOutcome::Quit,
            Key::F(1) => self.show_help(),
            Key::Esc => {
                self.input_buffer.clear();
                self.search_state.set_no_longer_actively_searching();
            }
            Key::Char('0') if self.input_buffer.is_empty() => {
                return EventOutcome::Redraw(Some(Action::FocusFirstSibling));
            }
            Key::Char(digit @ '0'..='9') => self.buffer_input(digit as u8),
            Key::Char('p') => self.start_pending_command(InputState::PendingPCommand, b'p'),
            Key::Char('y') => match &self.clipboard_context {
                Ok(_) => self.start_pending_command(InputState::PendingYCommand, b'y'),
                Err(err) => self.set_error_message(format!("Unable to access clipboard: {err}")),
            },
            Key::Char('z') => self.start_pending_command(InputState::PendingZCommand, b'z'),
            Key::Char(':') => {
                if self.run_command_prompt().is_break() {
                    return EventOutcome::Quit;
                }
                self.input_buffer.clear();
            }
            _ => {
                let outcome = self.action_key_outcome(key);
                self.input_buffer.clear();
                return outcome;
            }
        }
        EventOutcome::Redraw(None)
    }

    fn start_pending_command(&mut self, state: InputState, prefix: u8) {
        self.input_state = state;
        self.input_buffer.clear();
        self.buffer_input(prefix);
    }

    fn action_key_outcome(&mut self, key: Key) -> EventOutcome {
        if let Some(action) = self.navigation_action(key) {
            return EventOutcome::Redraw(Some(action));
        }
        match key {
            Key::Char('n') => return self.repeat_search(JumpDirection::Next),
            Key::Char('N') => return self.repeat_search(JumpDirection::Prev),
            Key::Char(key @ ('/' | '?' | '*' | '#')) => return self.start_search_from_key(key),
            Key::Char('\t') => {
                if let Some(table) = self.viewer.toggle_table_alignment() {
                    self.screen_writer
                        .reset_table_alignment(&self.viewer, table);
                }
            }
            Key::Ctrl('l') => {
                self.toggle_wrapping();
                return self.redraw_after_layout_change(None);
            }
            Key::Char('.') => {
                let count = self.parse_input_buffer_as_number().saturating_mul(10);
                self.screen_writer
                    .scroll_focused_line_right(&self.viewer, count);
            }
            Key::Char(',') => {
                let count = self.parse_input_buffer_as_number().saturating_mul(10);
                self.screen_writer
                    .scroll_focused_line_left(&self.viewer, count);
            }
            Key::Char('<') => self
                .screen_writer
                .decrease_indentation_level(self.viewer.flatjson.2 as u16),
            Key::Char('>') => self.screen_writer.increase_indentation_level(),
            Key::Char(';') => self
                .screen_writer
                .scroll_focused_line_to_an_end(&self.viewer),
            _ => eprint!("{BELL}\r"),
        }
        EventOutcome::Redraw(None)
    }

    fn navigation_action(&mut self, key: Key) -> Option<Action> {
        let action = match key {
            Key::Up | Key::Char('k') | Key::Ctrl('p') | Key::Backspace => {
                Action::MoveUp(self.parse_input_buffer_as_number())
            }
            Key::Down | Key::Char('j') | Key::Ctrl('n') | Key::Char('\n') => {
                Action::MoveDown(self.parse_input_buffer_as_number())
            }
            Key::Ctrl('e') => Action::ScrollDown(self.parse_input_buffer_as_number()),
            Key::Ctrl('y') => Action::ScrollUp(self.parse_input_buffer_as_number()),
            Key::Ctrl('d') => Action::JumpDown(self.maybe_parse_input_buffer_as_number()),
            Key::Ctrl('u') => Action::JumpUp(self.maybe_parse_input_buffer_as_number()),
            Key::Ctrl('b') | Key::PageUp => Action::PageUp(self.parse_input_buffer_as_number()),
            Key::Ctrl('f') | Key::PageDown => Action::PageDown(self.parse_input_buffer_as_number()),
            Key::Char('K') => Action::FocusPrevSibling(self.parse_input_buffer_as_number()),
            Key::Char('J') => Action::FocusNextSibling(self.parse_input_buffer_as_number()),
            Key::Char('g') => self.jump_to_line(Action::FocusTop, false),
            Key::Char('G') => self.jump_to_line(Action::FocusBottom, true),
            Key::Char('w') => Action::MoveDownUntilDepthChange,
            Key::Char('b') => Action::MoveUpUntilDepthChange,
            Key::Left | Key::Char('h') => Action::MoveLeft,
            Key::Right | Key::Char('l') => Action::MoveRight,
            Key::Char('H') => Action::FocusParent,
            Key::Char('[') => Action::FocusParentOrPreviousSibling,
            Key::Char(']') => Action::FocusNextAtParentLevel,
            Key::Char('c') => Action::CollapseNodeAndSiblings,
            Key::Char('C') => Action::DeepCollapseNodeAndSiblings,
            Key::Char('e') => Action::ExpandNodeAndSiblings,
            Key::Char('E') => Action::DeepExpandNodeAndSiblings,
            Key::Char(' ') => Action::ToggleCollapsed,
            Key::Char('^') => Action::FocusFirstSibling,
            Key::Char('$') => Action::FocusLastSibling,
            Key::Home => Action::FocusTop,
            Key::End => Action::FocusBottom,
            _ => return None,
        };
        Some(action)
    }

    fn jump_to_line(&mut self, fallback: Action, make_visible: bool) -> Action {
        match self.maybe_parse_input_buffer_as_number() {
            Some(line) => Action::JumpTo {
                line: line - 1,
                make_visible,
            },
            None => fallback,
        }
    }

    fn toggle_wrapping(&mut self) {
        self.viewer.toggle_wrapping();
        self.screen_writer.reset_horizontal_offsets(&self.viewer);
        let state = if self.viewer.wrapping_enabled {
            "on"
        } else {
            "off"
        };
        let width = usize::from(self.screen_writer.dimensions.width);
        let mut message = match width {
            17.. => format!("Line wrapping {state}"),
            12..=16 => format!("Wrapping {state}"),
            _ => state.to_string(),
        };
        message.truncate(width);
        self.set_info_message(message);
    }

    fn start_search_from_key(&mut self, key: char) -> EventOutcome {
        let count = self.parse_input_buffer_as_number();
        let direction = if matches!(key, '/' | '*') {
            SearchDirection::Forward
        } else {
            SearchDirection::Reverse
        };
        let action = if matches!(key, '/' | '?') {
            self.get_search_input_and_start_search(direction, count)
        } else {
            self.start_object_key_search(direction, count)
        };
        match action {
            Some(action) => EventOutcome::RevealMatch(Some(action)),
            None => EventOutcome::Redraw(None),
        }
    }

    fn repeat_search(&mut self, direction: JumpDirection) -> EventOutcome {
        let count = self.parse_input_buffer_as_number();
        EventOutcome::RevealMatch(self.jump_to_search_match(direction, count))
    }

    fn finish_print_command(&mut self, event: TuiEvent) -> bool {
        self.input_buffer.clear();
        let waiting_for_key = ContentTarget::from_command_key(event, 'p', 'P')
            .is_some_and(|target| self.print_content(target));
        self.input_state = if waiting_for_key {
            InputState::WaitingForAnyKeyPress
        } else {
            InputState::Default
        };
        waiting_for_key
    }

    fn finish_copy_command(&mut self, event: TuiEvent) {
        if let Some(target) = ContentTarget::from_command_key(event, 'y', 'p') {
            self.copy_content(target);
        }
        self.input_state = InputState::Default;
        self.input_buffer.clear();
    }

    fn finish_position_command(&mut self, event: TuiEvent) -> Option<Action> {
        self.input_state = InputState::Default;
        self.input_buffer.clear();
        match event {
            KeyEvent(Key::Char('t')) => Some(Action::MoveFocusedLineToTop),
            KeyEvent(Key::Char('z')) => Some(Action::MoveFocusedLineToCenter),
            KeyEvent(Key::Char('b')) => Some(Action::MoveFocusedLineToBottom),
            _ => None,
        }
    }

    fn apply_viewer_action(&mut self, action: Action, previous_focus: flatjson::Index) {
        self.viewer.perform_action(action);
        if self.should_reveal_table_cell(action, previous_focus) {
            self.screen_writer.invalidate_focus();
        }
        if self.viewer.wrapping_enabled
            && matches!(
                action,
                Action::ScrollUp(_)
                    | Action::ScrollDown(_)
                    | Action::PageUp(_)
                    | Action::PageDown(_)
                    | Action::JumpUp(_)
                    | Action::JumpDown(_)
                    | Action::MoveFocusedLineToTop
                    | Action::MoveFocusedLineToCenter
                    | Action::MoveFocusedLineToBottom
                    | Action::FocusNodeAt { .. }
                    | Action::NoOp
            )
        {
            self.screen_writer.accept_viewport_focus(&self.viewer);
        }
    }

    fn should_reveal_table_cell(&self, action: Action, previous_focus: flatjson::Index) -> bool {
        let focused_node = self.viewer.focused_node;
        // Only explicit horizontal cell selection requests a reveal.
        if focused_node == previous_focus
            || !matches!(
                action,
                Action::MoveLeft
                    | Action::MoveRight
                    | Action::FocusFirstSibling
                    | Action::FocusLastSibling
                    | Action::FocusPrevSibling(_)
                    | Action::FocusNextSibling(_)
                    | Action::FocusParentOrPreviousSibling
                    | Action::FocusNextAtParentLevel
            )
        {
            return false;
        }

        self.viewer
            .layout
            .analysis
            .table_row_owner(&self.viewer.flatjson, focused_node)
            .is_some_and(|row| row != focused_node)
    }

    fn update_search_after_event(
        &mut self,
        jumped_to_match: bool,
        previous_focus: flatjson::Index,
        was_collapsed: bool,
    ) {
        if jumped_to_match {
            self.reveal_current_match();
        } else if self.viewer.focused_node != previous_focus {
            self.search_state.set_no_longer_actively_searching();
        } else if self.viewer.effective_collapsed(previous_focus) != was_collapsed {
            // A collapse or expansion keeps matches visible and clears the current match.
            self.search_state
                .set_matches_visible_if_actively_searching();
        }
    }

    fn reveal_current_match(&mut self) {
        let range = self.search_state.current_match_range();
        // Reflow can move a table key back to its shared header. Restore the
        // source anchor before resolving the match to physical rows.
        self.viewer.perform_action(Action::FocusNode {
            node: self.viewer.focused_node,
            source: Some(range.start),
        });
        self.screen_writer
            .scroll_line_to_search_match(&mut self.viewer, range);
    }

    fn draw_screen(&mut self) {
        let generation = self.viewer.layout_generation;
        let previous_dimensions = self.viewer.dimensions;
        self.viewer.set_viewport(
            self.screen_writer.dimensions.without_status_bar(),
            self.screen_writer.show_line_numbers || self.screen_writer.show_relative_line_numbers,
        );
        let wrap_changed = self.screen_writer.sync_wrap_geometry(&mut self.viewer);
        let reflow = wrap_changed
            || generation != self.viewer.layout_generation
            || previous_dimensions.height != self.viewer.dimensions.height;
        if reflow {
            // Projection and height-only changes cannot create a horizontal
            // reveal need. Width changes are handled by the table viewport's
            // right-edge reveal; filtering invalidates focus in apply_path.
            if self.viewer.focused_table().is_none() {
                self.screen_writer.invalidate_focus();
            }
            if self.search_state.active_search_state().is_some() {
                self.reveal_current_match();
            }
        }
        self.screen_writer.print(
            &mut self.viewer,
            &self.input_buffer,
            &self.input_filename,
            &self.search_state,
            &self.message,
        );
    }

    fn draw_status_bar(&mut self) {
        self.screen_writer.print_status_bar(
            &self.viewer,
            &self.input_buffer,
            &self.input_filename,
            &self.search_state,
            &self.message,
        );
    }

    fn set_info_message(&mut self, s: String) {
        self.message = Some((s, MessageSeverity::Info));
    }

    fn set_warning_message(&mut self, s: String) {
        self.message = Some((s, MessageSeverity::Warn));
    }

    fn set_error_message(&mut self, s: String) {
        self.message = Some((s, MessageSeverity::Error));
    }

    fn run_command_prompt(&mut self) -> ControlFlow<()> {
        let Some(command) = self.readline(":", "command") else {
            return ControlFlow::Continue(());
        };

        match Command::parse(&command) {
            Command::Quit => return ControlFlow::Break(()),
            Command::Help => self.show_help(),
            Command::Path(path) => self.apply_path(&path),
            #[cfg(feature = "colorscheme")]
            Command::Colorscheme(name) => match self.config.resolve(&name) {
                Ok(theme) => self.screen_writer.set_theme(theme),
                Err(error) => self.set_error_message(error),
            },
            Command::SetShowLineNumber(value) => {
                self.screen_writer.show_line_numbers =
                    value.unwrap_or(!self.screen_writer.show_line_numbers);
            }
            Command::SetShowRelativeLineNumber(value) => {
                self.screen_writer.show_relative_line_numbers =
                    value.unwrap_or(!self.screen_writer.show_relative_line_numbers);
            }
            Command::WriteFile {
                filename,
                overwrite_existing,
                write_format,
            } => self.write_contents_to_file(filename, overwrite_existing, write_format),
            Command::Unknown => self.set_warning_message(format!("Unknown command: {command}")),
        }

        ControlFlow::Continue(())
    }

    fn apply_path(&mut self, input: &str) {
        let roots = crate::path_filter::PathFilter::parse(input)
            .and_then(|path| path.resolve(&self.viewer.flatjson));
        match roots {
            Ok(roots) => {
                self.viewer.set_roots(roots);
                self.search_state = SearchState::empty();
                self.screen_writer.invalidate_focus();
            }
            Err(error) => self.set_error_message(error),
        }
    }

    // Get user input via a readline prompt. May fail to return input if
    // the user deliberately cancels the prompt via Ctrl-C or Ctrl-D, or
    // if an actual error occurs, in which case an error message is set.
    fn readline(&mut self, prompt: &str, purpose: &str) -> Option<String> {
        match self.screen_writer.get_command(prompt) {
            Ok(s) => Some(s),
            // User hit Ctrl-C or Ctrl-D to cancel prompt
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => None,
            Err(err) => {
                self.set_error_message(format!("Error getting {purpose}: {err}"));
                None
            }
        }
    }

    fn buffer_input(&mut self, ch: u8) {
        // Don't buffer leading 0s.
        if self.input_buffer.is_empty() && ch == b'0' {
            return;
        }

        if self.input_buffer.len() >= MAX_BUFFER_SIZE {
            self.input_buffer.rotate_left(1);
            self.input_buffer.pop();
        }
        self.input_buffer.push(ch);
    }

    fn maybe_parse_input_buffer_as_number(&mut self) -> Option<usize> {
        let n = str::parse::<usize>(std::str::from_utf8(&self.input_buffer).unwrap());
        self.input_buffer.clear();
        n.ok()
    }

    fn parse_input_buffer_as_number(&mut self) -> usize {
        self.maybe_parse_input_buffer_as_number().unwrap_or(1)
    }

    fn get_search_input_and_start_search(
        &mut self,
        direction: SearchDirection,
        jumps: usize,
    ) -> Option<Action> {
        let prompt_str = match direction {
            SearchDirection::Forward => "/",
            SearchDirection::Reverse => "?",
        };

        let search_term = self.readline(prompt_str, "search input")?;

        // In vim, /<CR> or ?<CR> is a longcut for repeating the previous search.
        if search_term.is_empty() {
            // This will actually set the direction of a search going forward.
            self.search_state.direction = direction;
            self.jump_to_search_match(JumpDirection::Next, jumps)
        } else {
            if self.initialize_search(direction, search_term) {
                if !self.search_state.any_matches() {
                    self.set_warning_message(self.search_state.no_matches_message());
                    None
                } else {
                    self.jump_to_search_match(JumpDirection::Next, jumps)
                }
            } else {
                None
            }
        }
    }

    fn initialize_search(&mut self, direction: SearchDirection, search_term: String) -> bool {
        let ranges = self
            .viewer
            .active_roots()
            .iter()
            .map(|&root| self.viewer.flatjson[root].range.clone());
        match SearchState::initialize_search_in_ranges(
            search_term,
            &self.viewer.flatjson.1,
            direction,
            ranges,
        ) {
            Ok(ss) => {
                self.search_state = ss;
                true
            }
            Err(err_message) => {
                self.set_error_message(err_message);
                false
            }
        }
    }

    fn start_object_key_search(
        &mut self,
        direction: SearchDirection,
        jumps: usize,
    ) -> Option<Action> {
        if self.initialize_object_key_search(direction) {
            self.jump_to_search_match(JumpDirection::Next, jumps)
        } else {
            let message = match direction {
                SearchDirection::Forward => "Must be focused on Object key to use '*'",
                SearchDirection::Reverse => "Must be focused on Object key to use '#'",
            };
            self.set_warning_message(message.to_string());
            None
        }
    }

    fn initialize_object_key_search(&mut self, direction: SearchDirection) -> bool {
        if let Some(key_range) = &self.viewer.flatjson[self.viewer.focused_node].key_range {
            // Note key_range already includes quotes around key.
            let object_key = format!("{}: ", &self.viewer.flatjson.1[key_range.clone()]);
            self.initialize_search(direction, object_key)
        } else {
            false
        }
    }

    fn jump_to_search_match(
        &mut self,
        jump_direction: JumpDirection,
        jumps: usize,
    ) -> Option<Action> {
        if !self.search_state.ever_searched {
            self.set_info_message("Type / to search".to_string());
            return None;
        } else if !self.search_state.any_matches() {
            self.set_warning_message(self.search_state.no_matches_message());
            return None;
        }

        let destination = self.search_state.jump_to_match(
            self.viewer.focused_node,
            &self.viewer.flatjson,
            jump_direction,
            jumps,
        );
        Some(Action::FocusNode {
            node: destination,
            source: Some(self.search_state.current_match_range().start),
        })
    }

    fn show_help(&mut self) {
        let _ = write!(self.screen_writer.stdout, "{ToMainScreen}");
        let child = std::process::Command::new("less")
            .arg("-r")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::inherit())
            .spawn();

        match child {
            Ok(mut child) => {
                if let Some(ref mut stdin) = child.stdin {
                    let _ = stdin.write_all(HELP.as_bytes());
                    let _ = stdin.write_all(TOON_HELP.as_bytes());
                    #[cfg(feature = "colorscheme")]
                    let _ = stdin.write_all(
                        b"\n:colorscheme <name>  Switch to a built-in or configured theme for this session.\n",
                    );
                    let _ = stdin.flush();
                }
                let _ = child.wait();
            }
            Err(err) => {
                self.set_error_message(format!("Error piping help documentation to less: {err}"));
            }
        }

        let _ = write!(self.screen_writer.stdout, "{ToAlternateScreen}");
    }

    fn get_content_target_data(&self, content_target: ContentTarget) -> Result<String, String> {
        let json = &self.viewer.flatjson.1;
        let focused_node_index = self.viewer.focused_node;
        let focused_node = &self.viewer.flatjson[focused_node_index];

        let data = match content_target {
            ContentTarget::ToonValue => crate::toon::encode_value(
                &self.viewer.flatjson,
                focused_node_index,
                crate::toon::EncodeOptions::default(),
            )
            .map_err(|e| e.to_string())?,
            ContentTarget::PrettyPrintedValue if focused_node.is_container() => self
                .viewer
                .flatjson
                .pretty_printed_value(focused_node_index)
                .unwrap(),
            ContentTarget::PrettyPrintedValue | ContentTarget::OneLineValue => {
                let range = focused_node.range.clone();
                json[range].to_string()
            }
            ContentTarget::String => {
                if !focused_node.is_string() {
                    return Err("Current value is not a string".to_string());
                }

                let range = focused_node.range.clone();
                let quoteless_range = (range.start + 1)..(range.end - 1);
                let string_value = &json[quoteless_range];

                match safe_unescape_json_string(string_value) {
                    Ok(unescaped) => unescaped,
                    Err(err) => {
                        return Err(format!("{err}"));
                    }
                }
            }
            ContentTarget::Key => {
                let Some(key_range) = &focused_node.key_range else {
                    return Err("No object key to copy".to_string());
                };

                let quoteless_range = (key_range.start + 1)..(key_range.end - 1);

                // Preserve the existing key-copy contract independently of display syntax.
                if JS_IDENTIFIER.is_match(&json[quoteless_range.clone()]) {
                    json[quoteless_range].to_string()
                } else {
                    json[key_range.clone()].to_string()
                }
            }
            ct @ (ContentTarget::DotPath
            | ContentTarget::BracketPath
            | ContentTarget::QueryPath) => {
                let path_type = match ct {
                    ContentTarget::DotPath => flatjson::PathType::Dot,
                    ContentTarget::BracketPath => flatjson::PathType::Bracket,
                    ContentTarget::QueryPath => flatjson::PathType::Query,
                    _ => unreachable!(),
                };

                self.viewer
                    .flatjson
                    .build_path_to_node(path_type, focused_node_index)?
            }
        };

        Ok(data)
    }

    fn copy_content(&mut self, content_target: ContentTarget) {
        match self.get_content_target_data(content_target) {
            Ok(content) => {
                // Checked when the user first hits 'y'.
                let clipboard = self.clipboard_context.as_mut().unwrap();

                let focused_node = &self.viewer.flatjson[self.viewer.focused_node];

                let content_type = match content_target {
                    ContentTarget::ToonValue => "TOON value",
                    ContentTarget::PrettyPrintedValue if focused_node.is_container() => {
                        "pretty-printed value"
                    }
                    ContentTarget::PrettyPrintedValue | ContentTarget::OneLineValue => "value",
                    ContentTarget::String => "string contents",
                    ContentTarget::Key => "key",
                    ContentTarget::DotPath => "path",
                    ContentTarget::BracketPath => "bracketed path",
                    ContentTarget::QueryPath => "query path",
                };

                if let Err(err) = clipboard.set_text(content) {
                    self.set_error_message(format!(
                        "Unable to copy {content_type} to clipboard: {err}"
                    ));
                } else {
                    self.set_info_message(format!("Copied {content_type} to clipboard"));
                }
            }
            Err(err) => self.set_warning_message(err),
        }
    }

    fn print_content(&mut self, content_target: ContentTarget) -> bool {
        match self.get_content_target_data(content_target) {
            Ok(content) => {
                // Exit raw mode so that the terminal interprets newlines as usual.
                let _ = self.screen_writer.stdout.suspend_raw_mode();
                // Go to the main screen so that the text will persist after exiting.
                let _ = write!(self.screen_writer.stdout, "{ToMainScreen}");
                // Disable mouse button tracking so that the user can use their mouse
                // to highlight the text.
                let _ = write!(self.screen_writer.stdout, "{DISABLE_MOUSE_BUTTON_TRACKING}");
                let _ = write!(
                    self.screen_writer.stdout,
                    "{}{}{}\n\nPress any key to continue.",
                    termion::clear::All,
                    termion::cursor::Goto(1, 1),
                    content
                );
                let _ = self.screen_writer.stdout.flush();
                // Go back to raw mode so we can immediately get key presses.
                let _ = self.screen_writer.stdout.activate_raw_mode();
                true
            }
            Err(err) => {
                self.set_warning_message(err);
                false
            }
        }
    }

    fn write_contents_to_file(
        &mut self,
        filename: String,
        overwrite_existing: bool,
        write_format: WriteFormat,
    ) {
        let file_contents: Result<String, String> = match write_format {
            WriteFormat::Json => crate::output::serialize_roots(
                &self.viewer.flatjson,
                OutputFormat::Json,
                self.viewer.active_roots(),
            ),
            WriteFormat::Yaml => crate::output::serialize_roots(
                &self.viewer.flatjson,
                OutputFormat::Yaml,
                self.viewer.active_roots(),
            ),
            WriteFormat::Ndjson => crate::output::serialize_jsonl_roots(
                &self.viewer.flatjson,
                self.viewer.active_roots(),
            ),
            #[cfg(feature = "sexp")]
            WriteFormat::Sexp => self
                .viewer
                .flatjson
                .sexp_string(self.viewer.active_roots())
                .map_err(|error| error.to_string()),
            WriteFormat::Toon => crate::output::serialize_roots(
                &self.viewer.flatjson,
                OutputFormat::Toon,
                self.viewer.active_roots(),
            ),
        };
        let file_contents = match file_contents {
            Ok(contents) => contents,
            Err(error) => {
                self.set_error_message(format!("Error formatting file contents: {error}"));
                return;
            }
        };
        let mut file_open_options = File::options();
        file_open_options
            .write(true)
            .create(overwrite_existing)
            .truncate(overwrite_existing)
            .create_new(!overwrite_existing);

        match file_open_options.open(&filename) {
            Err(err) => match err.kind() {
                io::ErrorKind::AlreadyExists => self
                    .set_error_message(format!("{filename} already exists (add ! to overwrite)")),
                _ => self.set_error_message(format!("Error opening file for writing: {err}")),
            },
            Ok(mut file) => {
                match file
                    .write_all(file_contents.as_bytes())
                    .and_then(|()| file.flush())
                {
                    Ok(()) => self.set_info_message(format!("{filename} written")),
                    Err(err) => self.set_error_message(format!("Error writing file: {err}")),
                }
            }
        }
    }
}
