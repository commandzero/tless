mod process;
mod script;

use std::io;
use std::sync::atomic::AtomicUsize;
use std::time::{Duration, Instant};

use self::process::Terminal;
use self::script::{InputStep, Script};

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

pub(super) static NEXT_SESSION: AtomicUsize = AtomicUsize::new(0);

pub(super) fn cursor_requests(output: &[u8], scanned: &mut usize) -> usize {
    let requests = output[*scanned..]
        .windows(4)
        .filter(|bytes| *bytes == b"\x1b[6n")
        .count();
    *scanned = output.len().saturating_sub(3);
    requests
}

#[derive(Clone, Copy, Debug)]
pub(super) enum CopyEvent {
    Before,
    Complete,
}

type CopyObserver<'a> = &'a mut dyn FnMut(CopyEvent) -> io::Result<()>;

pub(super) struct SessionOptions<'a> {
    pub(super) argument: Option<&'a str>,
    pub(super) width: u16,
    pub(super) piped: bool,
    pub(super) observe_copy: Option<CopyObserver<'a>>,
    pub(super) expected_status: Option<i32>,
}

impl Default for SessionOptions<'_> {
    fn default() -> Self {
        Self {
            argument: None,
            width: 120,
            piped: false,
            observe_copy: None,
            expected_status: None,
        }
    }
}

pub(super) fn session(input: &str, commands: &str) -> String {
    session_with_options(input, commands, SessionOptions::default())
}

pub(super) fn session_with_width(
    input: &str,
    commands: &str,
    argument: Option<&str>,
    width: u16,
) -> String {
    session_with_options(
        input,
        commands,
        SessionOptions {
            argument,
            width,
            ..SessionOptions::default()
        },
    )
}

pub(super) fn session_with_options(
    input: &str,
    commands: &str,
    options: SessionOptions<'_>,
) -> String {
    let terminal = Terminal::spawn(input, &options);
    Session {
        terminal,
        observe_copy: options.observe_copy,
        output: Vec::new(),
        scanned: 0,
        started: false,
        // Newly built binaries can take longer to start on macOS.
        deadline: Instant::now() + Duration::from_secs(90),
    }
    .run(commands)
}

struct Session<'a> {
    terminal: Terminal,
    observe_copy: Option<CopyObserver<'a>>,
    output: Vec<u8>,
    scanned: usize,
    started: bool,
    deadline: Instant,
}

impl Session<'_> {
    fn run(mut self, commands: &str) -> String {
        let mut script = Script::new(commands, self.observe_copy.is_some());
        while self.terminal.read_more(&mut self.output).unwrap() {
            self.answer_cursor_requests();
            self.check_startup();
            if script.observe_output(&self.output, &self.terminal) {
                // Read the X11 selection while the tless clipboard owner is alive.
                self.notify_copy(CopyEvent::Complete);
                self.deadline = Instant::now() + RESPONSE_TIMEOUT;
            }
            if self.started {
                self.advance(&mut script);
            }
            assert!(
                Instant::now() <= self.deadline,
                "terminal timed out: {}",
                String::from_utf8_lossy(&self.output)
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        self.terminal.finish(&self.output);
        String::from_utf8(self.output).unwrap()
    }

    fn answer_cursor_requests(&mut self) {
        for _ in 0..cursor_requests(&self.output, &mut self.scanned) {
            self.terminal.write_input(b"\x1b[1;1R");
        }
    }

    fn check_startup(&mut self) {
        // The initial cursor position is visible even when the filename is clipped.
        if !self.started && self.output.windows(6).any(|bytes| bytes == b"\x1b[1;1H") {
            self.terminal.check_raw_mode_at_startup();
            self.started = true;
            self.deadline = Instant::now() + RESPONSE_TIMEOUT;
        }
    }

    fn advance(&mut self, script: &mut Script<'_>) {
        match script.next_step(self.output.len()) {
            Some(InputStep::Resize { narrow }) => {
                self.terminal.resize(narrow);
                script.wait_after_resize();
            }
            Some(InputStep::Send {
                bytes,
                seed_clipboard,
            }) => {
                if seed_clipboard {
                    self.notify_copy(CopyEvent::Before);
                }
                self.terminal.write_input(&bytes);
                // Bound response time separately from the time it takes to type the script.
                let sent_at = Instant::now();
                self.deadline = sent_at + RESPONSE_TIMEOUT;
                script.wait_after_input(&bytes, sent_at);
            }
            None => {}
        }
    }

    fn notify_copy(&mut self, event: CopyEvent) {
        self.observe_copy.as_mut().unwrap()(event).unwrap_or_else(|error| {
            panic!(
                "clipboard {event:?} failed: {error}; {}",
                String::from_utf8_lossy(&self.output)
            );
        });
    }
}
