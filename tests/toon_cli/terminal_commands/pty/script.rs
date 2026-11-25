use std::iter::Peekable;
use std::str::Bytes;
use std::time::{Duration, Instant};

use super::process::Terminal;

pub(super) struct Script<'a> {
    keys: Peekable<Bytes<'a>>,
    next_key_at: Instant,
    waiting: Vec<ExpectedResponse>,
    prefix: Option<Prefix>,
    entering_command: bool,
    help_active: bool,
    check_copy: bool,
}

enum Prefix {
    Print,
    Copy,
}

pub(super) enum InputStep {
    Resize {
        narrow: bool,
    },
    Send {
        bytes: Vec<u8>,
        seed_clipboard: bool,
    },
}

impl<'a> Script<'a> {
    pub(super) fn new(commands: &'a str, check_copy: bool) -> Self {
        Self {
            keys: commands.bytes().peekable(),
            next_key_at: Instant::now(),
            waiting: Vec::new(),
            prefix: None,
            entering_command: false,
            help_active: false,
            check_copy,
        }
    }

    pub(super) fn observe_output(&mut self, output: &[u8], terminal: &Terminal) -> bool {
        let mut copied = false;
        self.waiting.retain(|response| {
            if !response.is_complete(output, terminal) {
                return true;
            }
            match response.kind {
                Response::Help => self.help_active = true,
                Response::Copy => copied = true,
                _ => {}
            }
            false
        });
        copied
    }

    pub(super) fn next_step(&mut self, from: usize) -> Option<InputStep> {
        if !self.waiting.is_empty() || Instant::now() < self.next_key_at {
            return None;
        }
        let key = self.keys.next()?;
        // ^R shrinks both dimensions. ^Q changes only the height.
        if matches!(key, 0x12 | 0x11) {
            return Some(InputStep::Resize {
                narrow: key == 0x12,
            });
        }

        let seed_clipboard = self.record_key(key, from);
        let bytes = self.input_bytes(key);
        if bytes == b"\x1b[11~" {
            self.expect(Response::Help, from);
        } else if self.help_active && key == b'q' {
            self.help_active = false;
            self.expect(Response::HelpReturn, from);
        }
        Some(InputStep::Send {
            bytes,
            seed_clipboard,
        })
    }

    pub(super) fn wait_after_resize(&mut self) {
        self.next_key_at = Instant::now() + Duration::from_millis(100);
    }

    pub(super) fn wait_after_input(&mut self, input: &[u8], sent_at: Instant) {
        let delay = match input {
            [0x1b] => 600,
            [b':' | b'\n'] => 50,
            _ => 10,
        };
        self.next_key_at = sent_at + Duration::from_millis(delay);
    }

    fn record_key(&mut self, key: u8, from: usize) -> bool {
        let seed_clipboard = !self.entering_command && self.record_prefix(key, from);
        match (self.entering_command, key) {
            (false, b':' | b'/' | b'?') => {
                self.entering_command = true;
                self.expect(Response::Prompt, from);
            }
            (_, b'\n') | (true, 0x03) => {
                self.entering_command = false;
                self.expect(Response::Redraw, from);
            }
            _ => {}
        }
        seed_clipboard
    }

    fn record_prefix(&mut self, key: u8, from: usize) -> bool {
        match self.prefix.take() {
            Some(Prefix::Copy) => self.expect(Response::Copy, from),
            Some(Prefix::Print)
                if matches!(key, b'p' | b't' | b'v' | b's' | b'k' | b'P' | b'b' | b'q') =>
            {
                self.expect(Response::Print, from);
            }
            Some(Prefix::Print) => {}
            None if key == b'p' => self.prefix = Some(Prefix::Print),
            None if self.check_copy && key == b'y' => {
                self.prefix = Some(Prefix::Copy);
                return true;
            }
            None => {}
        }
        false
    }

    fn input_bytes(&mut self, key: u8) -> Vec<u8> {
        // Keep escape sequences such as xterm mouse input in one write.
        if key != 0x1b || self.keys.next_if_eq(&b'[').is_none() {
            return vec![key];
        }
        let mut input = vec![key, b'['];
        for next in self.keys.by_ref() {
            input.push(next);
            if next == b'~' || next.is_ascii_alphabetic() {
                break;
            }
        }
        input
    }

    fn expect(&mut self, kind: Response, from: usize) {
        self.waiting.push(ExpectedResponse { kind, from });
    }
}

enum Response {
    Prompt,
    Redraw,
    Print,
    Help,
    HelpReturn,
    Copy,
}

struct ExpectedResponse {
    kind: Response,
    from: usize,
}

impl ExpectedResponse {
    fn is_complete(&self, output: &[u8], terminal: &Terminal) -> bool {
        let recent = String::from_utf8_lossy(&output[self.from..]);
        match self.kind {
            Response::Prompt => recent.contains("\x1b[?2004h"),
            Response::Redraw => {
                recent.contains("\x1b[?2004l")
                    && (recent.contains("tless-pty-") || recent.contains("\x1b[1;1H"))
            }
            Response::Print => {
                recent.contains("Press any key to continue.") && terminal.raw_input_enabled()
            }
            Response::Help => {
                recent.contains("\x1b[?1049h") && recent.contains("TLESS_NATIVE_PAGER_READY")
            }
            Response::HelpReturn => returned_from_help(&recent) && terminal.raw_input_enabled(),
            Response::Copy => recent.contains("Copied ") && recent.contains(" to clipboard"),
        }
    }
}

fn returned_from_help(recent: &str) -> bool {
    let Some((_, restored)) = recent.rsplit_once("\x1b[?1049h") else {
        return false;
    };
    recent.contains("\x1b[?1049l") && restored.contains("tless-pty-")
}
