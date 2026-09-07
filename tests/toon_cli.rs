use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run(args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tless"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

#[cfg(feature = "toon")]
mod terminal_commands {
    use super::*;
    use std::fs::File;
    use std::io::{self, Read};
    use std::os::unix::io::{AsRawFd, FromRawFd};
    use std::os::unix::process::CommandExt;
    use std::time::{Duration, Instant};
    static NEXT_SESSION: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    pub fn cursor_requests(output: &[u8], scanned: &mut usize) -> usize {
        let requests = output[*scanned..]
            .windows(4)
            .filter(|bytes| *bytes == b"\x1b[6n")
            .count();
        *scanned = output.len().saturating_sub(3);
        requests
    }

    #[test]
    fn terminal_peer_answers_cursor_requests_across_read_boundaries() {
        let request = b"prefix\x1b[6nsuffix";
        for split in 0..=request.len() {
            let mut scanned = 0;
            let first = cursor_requests(&request[..split], &mut scanned);
            let second = cursor_requests(request, &mut scanned);
            assert_eq!(first + second, 1, "split at {}", split);
        }
    }

    fn session(input: &str, commands: &str) -> String {
        session_with_format(input, commands, None)
    }

    fn session_with_format(input: &str, commands: &str, format: Option<&str>) -> String {
        let path = std::env::temp_dir().join(format!(
            "tless-pty-{}-{}.json",
            std::process::id(),
            NEXT_SESSION.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let mut input_file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        input_file.write_all(input.as_bytes()).unwrap();
        let mut master = -1;
        let mut slave = -1;
        let mut size = libc::winsize {
            ws_row: 24,
            ws_col: 120,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        // Each child gets its own controlling terminal, never the user's terminal.
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &mut size,
                )
            },
            0
        );
        let mut master = unsafe { File::from_raw_fd(master) };
        let slave = unsafe { File::from_raw_fd(slave) };
        let slave_fd = slave.as_raw_fd();
        let mut command = Command::new(env!("CARGO_BIN_EXE_tless"));
        if let Some(format) = format {
            command.arg(format);
        }
        command
            .arg(&path)
            .env("TERM", "xterm-256color")
            .stdin(slave.try_clone().unwrap())
            .stdout(slave.try_clone().unwrap())
            .stderr(slave.try_clone().unwrap());
        unsafe {
            command.pre_exec(move || {
                if libc::setsid() == -1 || libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0) == -1 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command.spawn().unwrap();
        drop(command);
        drop(slave);
        assert_ne!(
            unsafe { libc::fcntl(master.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) },
            -1
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut output = Vec::new();
        let mut sent = false;
        let mut keys = commands.bytes();
        let mut next_key_at = Instant::now();
        let mut waiting_for_prompt = None;
        let mut waiting_for_redraw = None;
        let mut scanned_cursor_requests = 0;
        loop {
            let mut buffer = [0; 16384];
            match master.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    output.extend_from_slice(&buffer[..n]);
                    for _ in 0..cursor_requests(&output, &mut scanned_cursor_requests) {
                        master.write_all(b"\x1b[1;1R").unwrap();
                    }
                    if !sent && String::from_utf8_lossy(&output).contains("tless-pty-") {
                        sent = true;
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) if error.raw_os_error() == Some(libc::EIO) => break,
                Err(error) => panic!("{}", error),
            }
            if let Some(start) = waiting_for_prompt {
                if output[start..]
                    .windows(8)
                    .any(|bytes| bytes == b"\x1b[?2004h")
                {
                    waiting_for_prompt = None;
                }
            }
            if let Some(start) = waiting_for_redraw {
                let recent = String::from_utf8_lossy(&output[start..]);
                if recent.contains("\x1b[?2004l") && recent.contains("tless-pty-") {
                    waiting_for_redraw = None;
                }
            }
            if sent
                && waiting_for_prompt.is_none()
                && waiting_for_redraw.is_none()
                && Instant::now() >= next_key_at
            {
                if let Some(key) = keys.next() {
                    if key == b':' {
                        waiting_for_prompt = Some(output.len());
                    }
                    if key == b'\n' {
                        waiting_for_redraw = Some(output.len());
                    }
                    master.write_all(&[key]).unwrap();
                    next_key_at = Instant::now()
                        + Duration::from_millis(if key == b':' || key == b'\n' { 50 } else { 10 });
                }
            }
            // Read until terminal EOF/EIO, including bytes queued before exit.
            if Instant::now() > deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("terminal timed out: {}", String::from_utf8_lossy(&output));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let status = child.wait().unwrap();
        std::fs::remove_file(path).unwrap();
        assert!(status.success(), "{}", String::from_utf8_lossy(&output));
        String::from_utf8(output).unwrap()
    }

    #[test]
    fn write_open_failure_reports_an_error_and_keeps_the_viewer_usable() {
        let target = std::env::temp_dir()
            .join(format!("tless-no-directory-{}", std::process::id()))
            .join("output.toon");
        let output = session("42", &format!(":wt {}\npt q", target.display()));
        assert!(output.contains("Error opening file for writing"));
        assert!(!output.contains(" written"));
        assert!(output.contains("42\r\n\r\nPress any key to continue."));
        assert!(!target.exists());
    }

    #[test]
    fn writes_canonical_toon_through_the_viewer_command() {
        let target = std::env::temp_dir().join(format!("tless-write-{}.toon", std::process::id()));
        let output = session(r#"{"a":1}"#, &format!(":wt {}\nq", target.display()));
        assert!(output.contains("written"), "{}", output);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "a: 1");
        std::fs::remove_file(target).unwrap();
    }

    #[test]
    fn writes_toon_from_yaml_and_toon_inputs_without_changing_json_commands() {
        let target = std::env::temp_dir().join(format!("tless-formats-{}.out", std::process::id()));
        for format in ["--yaml", "--toon"] {
            let output = session_with_format(
                "a: 1",
                &format!(":writetoon {}\nq", target.display()),
                Some(format),
            );
            assert!(output.contains("written"));
            assert_eq!(std::fs::read_to_string(&target).unwrap(), "a: 1");
            std::fs::remove_file(&target).unwrap();
            session_with_format(
                "a: 1",
                &format!(":write {}\nq", target.display()),
                Some(format),
            );
            assert_eq!(
                std::fs::read_to_string(&target).unwrap(),
                "{\n  \"a\": 1\n}\n"
            );
            std::fs::remove_file(&target).unwrap();
        }
    }

    #[test]
    fn prints_focused_canonical_toon_on_the_persistent_screen() {
        let output = session(r#"{"items":[1,2]}"#, "jpt q");
        assert!(
            output.contains("[2]: 1,2\r\n\r\nPress any key to continue."),
            "{}",
            output
        );
    }

    #[test]
    fn bang_writes_replace_the_entire_file_after_successful_encoding() {
        let target =
            std::env::temp_dir().join(format!("tless-replace-{}.toon", std::process::id()));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .unwrap();
        file.write_all(b"original contents with a long suffix")
            .unwrap();
        let output = session("1 2", &format!(":wt! {}\nq", target.display()));
        assert!(output.contains("requires exactly one root"));
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "original contents with a long suffix"
        );
        let output = session("42", &format!(":wt {}\nq", target.display()));
        assert!(output.contains("already exists"));
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "original contents with a long suffix"
        );
        let output = session("42", &format!(":writetoon! {}\nq", target.display()));
        assert!(output.contains("written"));
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "42");
        session("{}", &format!(":wt! {}\nq", target.display()));
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "");
        std::fs::remove_file(&target).unwrap();
        session("42", &format!(":wt! {}\nq", target.display()));
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "42");
        std::fs::remove_file(&target).unwrap();
        let output = session("1 2", &format!(":wt! {}\nq", target.display()));
        assert!(output.contains("requires exactly one root"));
        assert!(!target.exists());
    }
}

#[test]
fn toon_extension_is_detected_and_explicit_json_overrides_it() {
    let path = std::env::temp_dir().join(format!("tless-format-{}.toon", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    file.write_all(b"42").unwrap();
    let output = run(&[path.to_str().unwrap()], b"");
    #[cfg(feature = "toon")]
    {
        assert!(output.status.success());
        assert_eq!(output.stdout, b"42");
    }
    #[cfg(not(feature = "toon"))]
    {
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("without TOON support"));
    }
    assert_eq!(
        run(&["--json", path.to_str().unwrap()], b"").stdout,
        b"42\n"
    );
    std::fs::remove_file(path).unwrap();
}

#[cfg(feature = "toon")]
#[test]
fn toon_pipeline_passes_through_without_validation() {
    let input = "\u{feff}items[99]: a,b\r\n  \r\n".as_bytes();
    let output = run(&["--toon"], input);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, input);
}

#[cfg(feature = "toon")]
#[test]
fn output_io_failure_exits_nonzero() {
    use std::os::unix::io::FromRawFd;
    let mut pipe = [-1; 2];
    assert_eq!(unsafe { libc::pipe(pipe.as_mut_ptr()) }, 0);
    let reader = unsafe { std::fs::File::from_raw_fd(pipe[0]) };
    let writer = unsafe { std::fs::File::from_raw_fd(pipe[1]) };
    drop(reader);
    let mut child = Command::new(env!("CARGO_BIN_EXE_tless"))
        .arg("--toon")
        .stdin(Stdio::piped())
        .stdout(writer)
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"a: 1").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Unable to write output"));
}

#[cfg(feature = "toon")]
#[test]
fn format_conflicts_and_invalid_utf8_fail_without_output() {
    for args in [&["--toon", "--json"][..], &["--toon", "--yaml"][..]] {
        assert!(!run(args, b"").status.success());
    }
    let output = run(&["--toon"], &[0xff]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Unable to get input"));
    assert!(!run(&[], b"name: Ada").status.success());
}

#[cfg(not(feature = "toon"))]
#[test]
fn disabled_build_omits_toon_option_and_help() {
    assert!(!run(&["--toon"], b"").status.success());
    assert!(!String::from_utf8_lossy(&run(&["--help"], b"").stdout).contains("--toon"));
}

#[test]
fn input_limit_applies_to_stdin_and_files_without_partial_output() {
    let output = run(&["--max-input-bytes", "2"], b"123");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("input exceeds"));
    assert!(run(&["--max-input-bytes", "3"], b"123").status.success());
    assert!(run(&["--max-input-bytes", "0"], b"123").status.success());
    let path = std::env::temp_dir().join(format!("tless-limit-{}.json", std::process::id()));
    std::fs::write(&path, b"123").unwrap();
    let output = run(&["--max-input-bytes", "2", path.to_str().unwrap()], b"");
    std::fs::remove_file(path).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
}

#[test]
fn version_help_and_usage_error_follow_cli_contract() {
    assert_eq!(
        run(&["--version"], b"").stdout,
        concat!("tless ", env!("CARGO_PKG_VERSION"), "\n").as_bytes()
    );
    assert!(run(&["--help"], b"").status.success());
    let invalid = run(&["--max-input-bytes", "invalid"], b"");
    assert_eq!(invalid.status.code(), Some(2));
    assert!(invalid.stdout.is_empty());
    assert!(!invalid.stderr.is_empty());
}
