use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::Ordering;

use nix::fcntl::{FcntlArg, OFlag, fcntl};
use nix::pty::openpty;
use nix::sys::termios::{LocalFlags, tcgetattr};

use super::{NEXT_SESSION, SessionOptions};

pub(super) struct Terminal {
    master: File,
    child: Child,
    size: libc::winsize,
    expected_exit: Option<(i32, LocalFlags)>,
    _fixture: Option<InputFile>,
}

impl Terminal {
    pub(super) fn spawn(input: &str, options: &SessionOptions<'_>) -> Self {
        let size = libc::winsize {
            ws_row: 24,
            ws_col: options.width,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let pair = openpty(Some(&size), None).unwrap();
        let master = File::from(pair.master);
        fcntl(&master, FcntlArg::F_SETFL(OFlag::O_NONBLOCK)).unwrap();

        let fixture = (!options.piped).then(|| InputFile::new(input));
        let expected_exit = options
            .expected_status
            .map(|status| (status, input_flags(&master)));
        let child = spawn_child(
            pair.slave.into(),
            fixture.as_ref().map(|file| file.0.as_path()),
            options,
        );

        let mut terminal = Self {
            master,
            child,
            size,
            expected_exit,
            _fixture: fixture,
        };
        if options.piped {
            terminal
                .child
                .stdin
                .take()
                .unwrap()
                .write_all(input.as_bytes())
                .unwrap();
        }
        terminal
    }

    pub(super) fn read_more(&mut self, output: &mut Vec<u8>) -> io::Result<bool> {
        let mut buffer = [0; 16384];
        match self.master.read(&mut buffer) {
            Ok(0) => Ok(false),
            Ok(count) => {
                output.extend_from_slice(&buffer[..count]);
                Ok(true)
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(true),
            Err(error) if error.raw_os_error() == Some(libc::EIO) => Ok(false),
            Err(error) => Err(error),
        }
    }

    pub(super) fn write_input(&mut self, bytes: &[u8]) {
        self.master.write_all(bytes).unwrap();
    }

    pub(super) fn resize(&mut self, narrow: bool) {
        if narrow {
            self.size.ws_col = 16;
        }
        self.size.ws_row = 8;
        // SAFETY: The master descriptor is live and the ioctl reads a winsize.
        let result = unsafe { libc::ioctl(self.master.as_raw_fd(), libc::TIOCSWINSZ, &self.size) };
        assert_ne!(result, -1, "{}", io::Error::last_os_error());
    }

    pub(super) fn raw_input_enabled(&self) -> bool {
        input_flags(&self.master).is_empty()
    }

    pub(super) fn check_raw_mode_at_startup(&self) {
        if matches!(self.expected_exit, Some((0, _))) {
            assert!(
                self.raw_input_enabled(),
                "viewer input must begin after raw-mode setup"
            );
        }
    }

    pub(super) fn finish(&mut self, output: &[u8]) {
        let status = self.child.wait().unwrap();
        let Some((expected_code, original_flags)) = self.expected_exit else {
            assert!(status.success(), "{}", String::from_utf8_lossy(output));
            return;
        };
        assert_eq!(status.code(), Some(expected_code), "{output:?}");
        assert_eq!(
            input_flags(&self.master),
            original_flags,
            "child exit must leave the original ECHO/ICANON flags"
        );
        if expected_code == 0 {
            assert_main_screen_restored(output);
        }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if !matches!(self.child.try_wait(), Ok(Some(_))) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn input_flags(master: &File) -> LocalFlags {
    tcgetattr(master).unwrap().local_flags & (LocalFlags::ECHO | LocalFlags::ICANON)
}

fn assert_main_screen_restored(output: &[u8]) {
    let entered = output
        .windows(8)
        .rposition(|bytes| bytes == b"\x1b[?1049h")
        .unwrap();
    let left = output
        .windows(8)
        .rposition(|bytes| bytes == b"\x1b[?1049l")
        .unwrap();
    assert!(left > entered, "exit must return to the main screen");
}

fn spawn_child(slave: File, fixture: Option<&Path>, options: &SessionOptions<'_>) -> Child {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tless"));
    if options.expected_status.is_some() {
        command
            .env(
                "LESS",
                "-PsTLESS_NATIVE_PAGER_READY -PmTLESS_NATIVE_PAGER_READY -PMTLESS_NATIVE_PAGER_READY",
            )
            .env("LESSHISTFILE", "-")
            .env_remove("LESSOPEN")
            .env_remove("LESSCLOSE");
    }
    if options.observe_copy.is_some() {
        command.env_remove("WAYLAND_DISPLAY");
    }
    if let Some(argument) = options.argument {
        command.arg(argument);
    }

    let stdin = match fixture {
        Some(path) => {
            command.arg(path);
            Stdio::from(slave.try_clone().unwrap())
        }
        None => Stdio::piped(),
    };
    command
        .env("TERM", "xterm-256color")
        .stdin(stdin)
        .stdout(slave.try_clone().unwrap())
        .stderr(slave.try_clone().unwrap());

    let slave_fd = slave.as_raw_fd();
    // SAFETY: The slave remains open through spawn. The child uses terminal
    // syscalls to attach its own controlling terminal before exec.
    unsafe {
        command.pre_exec(move || {
            if libc::setsid() == -1 || libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0) == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    command.spawn().unwrap()
}

struct InputFile(PathBuf);

impl InputFile {
    fn new(input: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "tless-pty-{}-{}.json",
            std::process::id(),
            NEXT_SESSION.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let fixture = Self(path);
        file.write_all(input.as_bytes()).unwrap();
        fixture
    }
}

impl Drop for InputFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
