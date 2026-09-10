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
        session_with_width(input, commands, format, 120)
    }

    fn session_with_width(input: &str, commands: &str, format: Option<&str>, width: u16) -> String {
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
            ws_col: width,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        // libc exposes a mutable window-size pointer on macOS and a const pointer on Linux.
        let size_ptr = std::ptr::addr_of_mut!(size);
        // Each child gets its own controlling terminal, never the user's terminal.
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    size_ptr,
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
        // Newly built binaries can start slowly on macOS. Keep interaction
        // checks bounded separately once the first screen is available.
        let mut deadline = Instant::now() + Duration::from_secs(90);
        let mut output = Vec::new();
        let mut sent = false;
        let mut keys = commands.bytes();
        let mut next_key_at = Instant::now();
        let mut waiting_for_prompt = None;
        let mut entering_command = false;
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
                        deadline = Instant::now() + Duration::from_secs(10);
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
                    if key == 0x12 {
                        size.ws_col = 16;
                        size.ws_row = 8;
                        assert_ne!(
                            unsafe { libc::ioctl(master.as_raw_fd(), libc::TIOCSWINSZ, &size) },
                            -1
                        );
                        next_key_at = Instant::now() + Duration::from_millis(100);
                        continue;
                    }
                    if !entering_command && matches!(key, b':' | b'/' | b'?') {
                        entering_command = true;
                        waiting_for_prompt = Some(output.len());
                    }
                    if key == b'\n' {
                        entering_command = false;
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

    fn strip_styles(output: &str) -> String {
        regex::Regex::new(r"\x1b\[[0-?]*[ -/]*[@-~]")
            .unwrap()
            .replace_all(output, "")
            .into_owned()
    }

    #[test]
    fn every_input_uses_the_same_toon_document_lines() {
        let json =
            r#"{"tags":["rust","cli"],"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Lin"}]}"#;
        let yaml = "tags: [rust, cli]\nusers:\n  - {id: 1, name: Ada}\n  - {id: 2, name: Lin}\n";
        let mut cases = vec![(json, None), (yaml, Some("--yaml"))];
        if cfg!(feature = "toon") {
            cases.push((
                "tags[2]: rust,cli\nusers[2]{id,name}:\n  1,Ada\n  2,Lin",
                Some("--toon"),
            ));
        }
        for (input, format) in cases {
            let output = strip_styles(&session_with_format(input, "q", format));
            for expected in ["tags[2]: rust,cli", "users[2]{id,name}:", "1,Ada", "2,Lin"] {
                assert!(
                    output.contains(expected),
                    "missing {}: {}",
                    expected,
                    output
                );
            }
        }
    }

    #[test]
    fn brackets_move_to_entries_at_the_parent_level() {
        for (motion, expected) in [
            ("[", "\"x\": 20"),
            ("]", "30\r\n\r\nPress any key to continue."),
        ] {
            let output = session(
                r#"{"a":10,"b":{"x":20},"c":30}"#,
                &format!("ljl{}pp q", motion),
            );
            assert!(output.contains(expected), "{}", output);
        }
    }

    #[test]
    fn bracket_fallbacks_select_siblings_when_parent_targets_are_missing() {
        for (input, commands, expected) in [
            (r#"{"a":{"x":1,"y":2},"b":3}"#, "ll]pp q", "3"),
            (r#"{"only":{"x":1,"y":2}}"#, "ll]]pp q", "2"),
            ("1 2 3", "]pp q", "2"),
            ("1 2 3", "][pp q", "1"),
        ] {
            let output = session(input, commands);
            assert!(
                output.contains(&format!("{}\r\n\r\nPress any key to continue.", expected)),
                "{}",
                output
            );
        }
    }

    #[test]
    fn right_expands_inline_arrays_into_navigable_lines() {
        let output = session(r#"["alpha","beta"]"#, "ljpp q");
        let clean = strip_styles(&output);
        assert!(clean.contains("[2]: alpha,beta"), "{}", clean);
        assert!(clean.contains("  - alpha"), "{}", clean);
        assert!(clean.contains("  - beta"), "{}", clean);
        assert!(output.contains("\"alpha\"\r\n\r\nPress any key to continue."));
    }

    #[test]
    fn terminal_resize_reflows_arrays_before_another_keypress() {
        // The helper resizes on ^R without sending that byte to the app.
        // If resize waits for input, q exits before a multiline redraw occurs.
        let output = strip_styles(&session(r#"["alpha","beta"]"#, "\x12q"));
        assert!(output.contains("[2]: alpha,beta"), "{}", output);
        assert!(output.contains("  - alpha"), "{}", output);
        assert!(output.contains("  - beta"), "{}", output);
    }

    #[test]
    fn arrays_start_multiline_when_large_or_too_wide() {
        for (input, width, last) in [
            ("[1,2,3,4,5,6]", 120, "  - 6"),
            (r#"{"items":["alpha","beta"]}"#, 20, "  - beta"),
        ] {
            let output = strip_styles(&session_with_width(input, "q", None, width));
            assert!(output.contains(last), "{}", output);
        }
    }

    #[test]
    fn hidden_search_reveals_and_prints_the_cell_without_annotations() {
        let output = session(
            r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Lin"}]}"#,
            "l /Lin\npp q",
        );
        assert!(
            output.contains("\"Lin\"\r\n\r\nPress any key to continue."),
            "{}",
            output
        );
        assert!(strip_styles(&output).contains("users[1].name"));
    }

    #[test]
    fn resize_and_column_motion_preserve_the_parsed_selection() {
        let output = session(
            r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Lin"}]}"#,
            "lllJj\x12pp q",
        );
        assert!(
            output.contains("\"Lin\"\r\n\r\nPress any key to continue."),
            "{}",
            output
        );
    }

    #[test]
    fn duplicate_warnings_and_occurrence_status_preserve_copy_identity() {
        let output = session(r#"{"status":"queued","status":"done"}"#, "lJpp q");
        let clean = strip_styles(&output);
        assert!(clean.contains("# WARN Duplicate key"));
        assert!(clean.contains("occurrence 2 of 2"));
        assert!(output.contains("\"done\"\r\n\r\nPress any key to continue."));
    }

    #[test]
    fn search_in_the_last_fully_visible_column_does_not_scroll() {
        let value = format!("{}Z", "a".repeat(26));
        let input = format!("{{\"x\":\"{}\"}}", value);
        let output = strip_styles(&session_with_width(&input, "/Z\nq", None, 35));
        let line = format!("x: {}", value);
        assert!(output.matches(&line).count() >= 2, "{}", output);
        assert!(!output.contains("…Z"), "{}", output);
    }

    #[test]
    fn long_string_search_reveals_the_match_inside_its_token() {
        let input = format!("{{\"value\":\"{}NEEDLE\"}}", "a".repeat(150));
        let output = strip_styles(&session_with_width(&input, "/NEEDLE\nq", None, 35));
        assert!(output.contains("…NEEDLE"), "{}", output);
    }

    #[test]
    fn horizontal_keys_move_in_ten_cell_increments() {
        let input = r#""0123456789abcdefghijKLMNOPQRSTUVWXYZ0123456789abcdefghij""#;
        for (commands, expected) in [
            (".q", "…abcdefghij"),
            ("2.q", "…KLMNOPQRST"),
            ("2.,q", "…abcdefghij"),
        ] {
            let output = strip_styles(&session_with_width(input, commands, None, 35));
            assert!(output.contains(expected), "{}", output);
        }
    }

    #[test]
    fn semicolon_reaches_the_end_from_an_intermediate_horizontal_offset() {
        let input = format!("\"{}TAIL\"", "a".repeat(150));
        let output = strip_styles(&session_with_width(&input, "10.;q", None, 35));
        assert!(output.contains("TAIL"), "{}", output);
    }

    #[cfg(feature = "toon")]
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

    #[cfg(feature = "toon")]
    #[test]
    fn writes_canonical_toon_through_the_viewer_command() {
        let target = std::env::temp_dir().join(format!("tless-write-{}.toon", std::process::id()));
        let output = session(r#"{"a":1}"#, &format!(":wt {}\nq", target.display()));
        assert!(output.contains("written"), "{}", output);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "a: 1");
        std::fs::remove_file(target).unwrap();
    }

    #[cfg(feature = "toon")]
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

    #[cfg(feature = "toon")]
    #[test]
    fn prints_focused_canonical_toon_on_the_persistent_screen() {
        let output = session(r#"{"items":[1,2]}"#, "lpt q");
        assert!(
            output.contains("[2]: 1,2\r\n\r\nPress any key to continue."),
            "{}",
            output
        );
    }

    #[cfg(feature = "toon")]
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
    use std::net::Shutdown;
    use std::os::fd::OwnedFd;
    use std::os::unix::net::UnixStream;

    let (reader, writer) = UnixStream::pair().unwrap();
    // Shutdown also affects copies inherited by concurrently spawned terminal
    // tests, so an inherited reader cannot temporarily make the write succeed.
    reader.shutdown(Shutdown::Both).unwrap();
    drop(reader);
    let mut child = Command::new(env!("CARGO_BIN_EXE_tless"))
        .arg("--toon")
        .stdin(Stdio::piped())
        .stdout(OwnedFd::from(writer))
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

#[test]
fn removed_modes_are_usage_errors_and_help_describes_toon_addresses() {
    for args in [
        &["--mode", "line"][..],
        &["--mode", "data"][..],
        &["-m", "data"][..],
    ] {
        let output = run(args, b"");
        assert_eq!(output.status.code(), Some(2));
    }
    let help = String::from_utf8(run(&["--help"], b"").stdout).unwrap();
    assert!(!help.contains("--mode"));
    assert!(help.contains("TOON line addresses"));
}
