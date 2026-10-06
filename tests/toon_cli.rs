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
    use unicode_width::UnicodeWidthChar;
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
    fn path_filter_confines_navigation_search_and_keeps_original_paths() {
        let input = r#"{"hits":{"name":"Ada"},"outside":"secret"}"#;
        let output = session(input, ":.hits\n999k[[^^]]999jG/outside\nq");
        let rows = rendered_rows(&output, 120, 24);
        let data = rows[..22].join("\n");
        assert!(data.contains("name: Ada"), "{rows:?}");
        assert!(
            !data.contains("outside") && !data.contains("secret"),
            "{rows:?}"
        );
        let output = session(input, ":.hits\nlpP q");
        assert!(
            strip_styles(&output).contains(".hits.name\r\n"),
            "{output:?}"
        );
    }

    #[test]
    fn path_filter_exports_scope_preserves_failures_and_resets() {
        let target =
            std::env::temp_dir().join(format!("tless-path-scope-{}.json", std::process::id()));
        let input = r#"{"hits":{"name":"Ada","age":37},"outside":true}"#;
        let commands = format!(":.hits\nl:.missing\n:wj! {}\nq", target.display());
        session(input, &commands);
        assert_eq!(
            std::fs::read(&target).unwrap(),
            b"{\n  \"name\": \"Ada\",\n  \"age\": 37\n}\n"
        );
        session(
            input,
            &format!(":.hits\n:.\n:write-json! {}\nq", target.display()),
        );
        assert_eq!(
            std::fs::read(&target).unwrap(),
            b"{\n  \"hits\": {\n    \"name\": \"Ada\",\n    \"age\": 37\n  },\n  \"outside\": true\n}\n"
        );
        std::fs::remove_file(target).unwrap();
    }

    #[test]
    fn path_filter_accepts_yp_brackets_and_preserves_pointer_whitespace() {
        let target =
            std::env::temp_dir().join(format!("tless-path-keys-{}.json", std::process::id()));
        let input = r#"{"a.b":[{"name":"Ada"}]," ":7}"#;
        session(
            input,
            &format!(":[\"a.b\"][0].name\n:wj! {}\nq", target.display()),
        );
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "\"Ada\"\n");
        session(input, &format!(":./ \n:wj! {}\nq", target.display()));
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "7\n");
        std::fs::remove_file(target).unwrap();
    }

    #[test]
    fn path_filter_sequence_export_failure_does_not_truncate() {
        let target =
            std::env::temp_dir().join(format!("tless-path-atomic-{}.toon", std::process::id()));
        std::fs::write(&target, "preserve me").unwrap();
        session(
            r#"{"a":1} {"a":2}"#,
            &format!(":.a\n:wt! {}\nq", target.display()),
        );
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "preserve me");
        std::fs::remove_file(target).unwrap();
    }

    #[test]
    fn filtered_yaml_and_jsonl_keep_each_selected_root_and_array_shape() {
        let yaml = std::env::temp_dir().join(format!("tless-filtered-{}.yaml", std::process::id()));
        let jsonl =
            std::env::temp_dir().join(format!("tless-filtered-{}.jsonl", std::process::id()));
        let input = r#"{"outside":0,"hits":[{"name":"Ada"},2]} {"outside":1,"hits":[3,4]}"#;
        let output = session(
            input,
            &format!(
                ":.hits\n:wy {}\n:write-jsonl {}\nq",
                yaml.display(),
                jsonl.display()
            ),
        );
        assert!(output.matches("written").count() >= 2, "{output}");
        assert_eq!(
            std::fs::read(&yaml).unwrap(),
            b"---\n[\n  {\n    \"name\": \"Ada\"\n  },\n  2\n]\n---\n[\n  3,\n  4\n]\n"
        );
        assert_eq!(
            std::fs::read(&jsonl).unwrap(),
            b"[{\"name\":\"Ada\"},2]\n[3,4]\n"
        );
        std::fs::remove_file(yaml).unwrap();
        std::fs::remove_file(jsonl).unwrap();
    }

    #[test]
    fn filtered_json_writes_ignore_excluded_yaml_errors_but_reject_included_ones() {
        let existing =
            std::env::temp_dir().join(format!("tless-yaml-invalid-{}.json", std::process::id()));
        let missing =
            std::env::temp_dir().join(format!("tless-yaml-invalid-{}.jsonl", std::process::id()));
        std::fs::write(&existing, b"previous long contents").unwrap();
        let input = "good: 42\n? [bad]\n: .inf\n";
        let output = session_with_format(
            input,
            &format!(
                ":.good\n:write-json! {}\n:.\n:write-json! {}\n:write-jsonl! {}\nq",
                existing.display(),
                existing.display(),
                missing.display()
            ),
            Some("yaml"),
        );
        assert!(output.contains("written"), "{output}");
        assert!(
            output.contains("JSON output requires string mapping keys"),
            "{output}"
        );
        assert_eq!(std::fs::read(&existing).unwrap(), b"42\n");
        assert!(!missing.exists());
        std::fs::remove_file(existing).unwrap();
    }

    #[test]
    fn path_filter_startup_uses_root_presentation() {
        let output = session_with_width(
            r#"{"hits":{"name":"Ada"},"outside":true}"#,
            "q",
            Some("--path=.hits"),
            120,
        );
        let rows = rendered_rows(&output, 120, 24);
        let data = rows[..22].join("\n");
        assert!(data.contains("name: Ada"), "{rows:?}");
        assert!(
            !data.contains("outside") && !data.contains("hits:"),
            "{rows:?}"
        );
    }

    #[cfg(feature = "sexp")]
    #[test]
    fn path_filter_sexp_exports_nested_values_without_owning_key() {
        let target =
            std::env::temp_dir().join(format!("tless-path-sexp-{}.sexp", std::process::id()));
        session(
            r#"{"skip":0,"hits":{"a":1,"b":[true,"x y"]}}"#,
            &format!(":.hits\n:write-sexp {}\nq", target.display()),
        );
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "((a 1) (b (true \"x y\")))\n"
        );
        session(
            r#"{"hits":1} {"hits":2} {"hits":[]}"#,
            &format!(":.hits\n:ws! {}\nq", target.display()),
        );
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "1\n2\n()\n");
        std::fs::remove_file(target).unwrap();
    }
    #[test]
    fn command_marker_is_visible_only_while_command_input_is_active() {
        let output = session("[]", ":set nonumber\n:\x03q");
        let prompt_start = "\x1b[?2004h";
        let prompt_end = "\x1b[?2004l";
        let starts: Vec<_> = output
            .match_indices(prompt_start)
            .map(|(index, _)| index)
            .collect();
        let ends: Vec<_> = output
            .match_indices(prompt_end)
            .map(|(index, _)| index)
            .collect();
        assert_eq!(starts.len(), 2, "{output:?}");
        assert_eq!(ends.len(), 2, "{output:?}");

        let initial = rendered_rows(&output[..starts[0]], 120, 24);
        assert!(initial[23].is_empty(), "initial status row: {initial:?}");

        let command_prompt = strip_styles(&output[starts[0]..ends[0]]);
        assert!(
            command_prompt.contains(":set nonumber"),
            "{command_prompt:?}"
        );

        let after_command = rendered_rows(&output[..starts[1]], 120, 24);
        assert!(
            after_command[23].is_empty(),
            "status row after command: {after_command:?}"
        );

        let cancelled_prompt = strip_styles(&output[starts[1]..ends[1]]);
        assert!(cancelled_prompt.contains(':'), "{cancelled_prompt:?}");
        let after_cancel = rendered_rows(&output, 120, 24);
        assert!(
            after_cancel[23].is_empty(),
            "status row after cancellation: {after_cancel:?}"
        );
    }

    #[test]
    fn autocomplete_cycles_forward_backward_and_restores_input() {
        let cases = [
            (":write-j\t\nq", "write-json"),
            (":write-j\t\t\nq", "write-json!"),
            (":write-j\t\t\t\nq", "write-jsonl"),
            (":write-j\t\t\t\t\nq", "write-jsonl!"),
            (":write-j\t\t\t\t\t\nq", "write-j"),
            (":write-j\x1b[Z\nq", "write-jsonl!"),
            (":write-j\x1b[Z\x1b[Z\nq", "write-jsonl"),
            (":write-j\x1b[Z\x1b[Z\x1b[Z\nq", "write-json!"),
            (":write-j\x1b[Z\x1b[Z\x1b[Z\x1b[Z\nq", "write-json"),
            (":write-j\x1b[Z\x1b[Z\x1b[Z\x1b[Z\x1b[Z\nq", "write-j"),
            (":write-j\t\x1b[Z\nq", "write-j"),
            (":write-j\t\x1b\nq", "write-j"),
            (":write-j\t\x7f\t\nq", "write-json"),
            (":unknown\t\nq", "unknown"),
            (":set other\t\nq", "set other"),
        ];
        let error = regex::Regex::new(r"Unknown command: ([^\x1b\r\n]*)").unwrap();
        for (keys, expected) in cases {
            let output = session("{}", keys);
            let actual = error
                .captures(&output)
                .map(|capture| capture[1].trim_end().to_owned());
            assert_eq!(actual.as_deref(), Some(expected), "{keys:?}: {output:?}");
        }
    }

    #[test]
    fn autocomplete_offers_sexp_only_when_enabled() {
        let output = session("{}", ":write-s\t\nq");
        let expected = if cfg!(feature = "sexp") {
            "Unknown command: write-sexp"
        } else {
            "Unknown command: write-s"
        };
        assert!(strip_styles(&output).contains(expected), "{output}");
        let output = session("{}", ":write-s\x1b[Z\nq");
        let expected = if cfg!(feature = "sexp") {
            "Unknown command: write-sexp!"
        } else {
            "Unknown command: write-s"
        };
        assert!(strip_styles(&output).contains(expected), "{output}");
    }

    #[test]
    fn autocomplete_hints_require_acceptance_and_do_not_enter_search() {
        let output = session(r#"{"se":1}"#, ":se\n:se\x1b[C number\n/se\t\n?se\t\nq");
        assert!(output.contains("\x1b[2mt"), "{output:?}");
        let plain = strip_styles(&output);
        assert!(plain.contains("Unknown command: se"));
        assert!(!plain.contains("Unknown command: set number"));
        let search = output.split("/se").last().unwrap();
        assert!(!search.contains("\x1b[2mt"), "{search:?}");
        assert!(!plain.contains("/set"));
        assert!(!plain.contains("?set"));
    }

    #[test]
    fn autocomplete_preserves_arguments_and_cancellation_has_no_write_effect() {
        let path = std::env::temp_dir().join(format!("tless-complete-{}.json", std::process::id()));
        let filename = path.to_str().unwrap();
        let input = r#"{"a":1}"#;
        // Move from the end of an existing filename to the end of the command.
        let left = "\x1b[D".repeat(filename.len() + 1);
        let keys = format!(":  write-j {filename}{left}\t\nq");
        let output = session(input, &keys);
        assert!(strip_styles(&output).contains("written"));
        let original = std::fs::read(&path).unwrap();
        assert_eq!(original, b"{\n  \"a\": 1\n}\n");
        let keys = format!(":write-j\t\t {filename}\x03:write-j\x1b[Z\x03:se\nq");
        let output = strip_styles(&session(r#"{"changed":true}"#, &keys));
        assert!(output.contains("Unknown command: se"));
        assert_eq!(std::fs::read(&path).unwrap(), original);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn autocomplete_hints_clip_without_wrapping_and_clear_after_cancel() {
        let output = session_with_width("{}", ":wri\x03q", None, 6);
        assert!(output.contains("\x1b[2mt\x1b[0m"), "{output:?}");
        assert!(!output.contains("\x1b[2mte"));
        let output = session_with_width("{}", ":wri\x1b[C\nq", None, 6);
        assert!(strip_styles(&output).contains(":write"));
    }

    #[test]
    fn autocomplete_redraws_hints_on_resize_and_cursor_movement() {
        let output = session("{}", ":          wri\x12\x1b[D\x1b[C\x7f\x03q");
        assert!(
            output.contains("\x1b[2mte\x1b[0m"),
            "full hint before resize"
        );
        assert!(
            output.contains("\x1b[2mt\x1b[0m"),
            "clipped hint after resize"
        );
        let frames: Vec<_> = output
            .split("\x1b[?2026h")
            .skip(1)
            .filter_map(|frame| frame.split_once("\x1b[?2026l").map(|(frame, _)| frame))
            .collect();
        assert!(
            frames.iter().all(|frame| !frame.contains('\n')),
            "hint redraw must not wrap: {frames:?}"
        );
        assert!(
            frames
                .iter()
                .any(|frame| strip_styles(frame).contains(":          wri")
                    && !frame.contains("\x1b[2m")),
            "moving into the token removes the hint"
        );
        let last_close = output.rfind("\x1b[?2004l").unwrap();
        assert!(
            !output[last_close..].contains("\x1b[2m"),
            "cancel clears the hint"
        );
    }

    #[cfg(feature = "colorscheme")]
    #[test]
    fn autocomplete_keeps_theme_styles_after_switching() {
        let output = session(
            "{}",
            ":colorscheme borealis\n:se\x03:colorscheme default\n:se\t number\nq",
        );
        let frames: Vec<_> = output
            .split("\x1b[?2026h")
            .skip(1)
            .filter_map(|frame| frame.split_once("\x1b[?2026l").map(|(frame, _)| frame))
            .collect();
        assert!(
            frames.iter().any(|frame| {
                strip_styles(frame).contains(":set")
                    && frame.contains("\x1b[2mt")
                    && frame.contains("\x1b[38;2;202;211;226m")
                    && frame.contains("\x1b[48;2;5;15;33m")
            }),
            "Borealis foreground and background must survive hint rendering"
        );
        assert!(!strip_styles(&output).contains("Unknown command:"));
        assert!(!strip_styles(&output).contains("Unknown colorscheme"));
    }

    #[test]
    fn output_selection_leaves_terminal_view_and_json_print_unchanged() {
        for option in [
            "--output-format=json",
            "--output-format=yaml",
            "--output-format=toon",
        ] {
            let output = session_with_width(r#"{"a":1}"#, "lpp q", Some(option), 120);
            assert!(strip_styles(&output).contains("a: 1"));
            assert!(output.contains("1\r\n"));
        }
    }

    #[test]
    fn piped_input_with_controlling_pty_restores_interactive_input() {
        let output = piped_session(r#"{"a":1}"#, "q");
        assert!(strip_styles(&output).contains("a: 1"));
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

    fn piped_session(input: &str, commands: &str) -> String {
        session_with_source(input, commands, None, 120, true)
    }

    fn session_with_format(input: &str, commands: &str, format: Option<&str>) -> String {
        let format_arg = format.map(|format| format!("--input-format={format}"));
        session_with_width(input, commands, format_arg.as_deref(), 120)
    }

    fn session_with_width(input: &str, commands: &str, format: Option<&str>, width: u16) -> String {
        session_with_source(input, commands, format, width, false)
    }

    fn session_with_source(
        input: &str,
        commands: &str,
        format: Option<&str>,
        width: u16,
        piped: bool,
    ) -> String {
        let path = std::env::temp_dir().join(format!(
            "tless-pty-{}-{}.json",
            std::process::id(),
            NEXT_SESSION.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        if !piped {
            let mut input_file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .unwrap();
            input_file.write_all(input.as_bytes()).unwrap();
        }
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
        if !piped {
            command.arg(&path);
        }
        command
            .env("TERM", "xterm-256color")
            .stdout(slave.try_clone().unwrap())
            .stderr(slave.try_clone().unwrap());
        if piped {
            command.stdin(Stdio::piped());
        } else {
            command.stdin(slave.try_clone().unwrap());
        }
        unsafe {
            command.pre_exec(move || {
                if libc::setsid() == -1 || libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0) == -1 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command.spawn().unwrap();
        if piped {
            child
                .stdin
                .take()
                .unwrap()
                .write_all(input.as_bytes())
                .unwrap();
        }
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
        let mut keys = commands.bytes().peekable();
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
                    // The filename is clipped on narrow terminals. Cursor
                    // positioning marks the initial draw at every width.
                    let ready = output.windows(6).any(|bytes| bytes == b"\x1b[1;1H");
                    if !sent && ready {
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
                if recent.contains("\x1b[?2004l")
                    && (recent.contains("tless-pty-") || recent.contains("\x1b[1;1H"))
                {
                    waiting_for_redraw = None;
                }
            }
            if sent
                && waiting_for_prompt.is_none()
                && waiting_for_redraw.is_none()
                && Instant::now() >= next_key_at
            {
                if let Some(key) = keys.next() {
                    // ^R shrinks both dimensions; ^Q changes only the height
                    // so table viewport regressions cannot hide behind width.
                    if key == 0x12 || key == 0x11 {
                        if key == 0x12 {
                            size.ws_col = 16;
                        }
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
                    if key == b'\n' || (entering_command && key == 0x03) {
                        entering_command = false;
                        waiting_for_redraw = Some(output.len());
                    }
                    let mut input = vec![key];
                    // Keep xterm mouse sequences together. Sending one byte
                    // per tick can make termion see an incomplete CSI event
                    // before the rest of the sequence reaches the PTY.
                    if key == 0x1b && keys.peek() == Some(&b'[') {
                        for next in keys.by_ref() {
                            input.push(next);
                            if input.len() > 2 && (next == b'~' || next.is_ascii_alphabetic()) {
                                break;
                            }
                        }
                    }
                    master.write_all(&input).unwrap();
                    // Bound application-response waits, not the time spent
                    // pacing a finite script. Long literal temp paths can
                    // take more than ten seconds to type on loaded runners.
                    let sent_at = Instant::now();
                    deadline = sent_at + Duration::from_secs(10);
                    next_key_at = sent_at
                        + Duration::from_millis(if input == [0x1b] {
                            600
                        } else if key == b':' || key == b'\n' {
                            50
                        } else {
                            10
                        });
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
        if !piped {
            std::fs::remove_file(path).unwrap();
        }
        assert!(status.success(), "{}", String::from_utf8_lossy(&output));
        String::from_utf8(output).unwrap()
    }

    fn strip_styles(output: &str) -> String {
        regex::Regex::new(r"\x1b\[[0-?]*[ -/]*[@-~]")
            .unwrap()
            .replace_all(output, "")
            .into_owned()
    }

    // The PTY transcript contains cursor-addressed screen updates rather than
    // newline-delimited output. Keep a small screen model here so rendering
    // assertions can distinguish a continuation gutter from a later redraw.
    fn rendered_rows(output: &str, width: u16, height: u16) -> Vec<String> {
        let width = usize::from(width);
        let height = usize::from(height);
        let mut screen = vec![vec![' '; width]; height];
        let mut row = 0usize;
        let mut column = 0usize;
        let mut saved = (0usize, 0usize);
        let bytes = output.as_bytes();
        let mut index = 0usize;

        while index < bytes.len() {
            match bytes[index] {
                0x1b => {
                    if bytes.get(index + 1) == Some(&b'[') {
                        let mut end = index + 2;
                        while end < bytes.len() && !(0x40..=0x7e).contains(&bytes[end]) {
                            end += 1;
                        }
                        if end >= bytes.len() {
                            break;
                        }
                        let params = String::from_utf8_lossy(&bytes[index + 2..end]);
                        let mut numbers = params
                            .trim_start_matches('?')
                            .split(';')
                            .map(|part| part.parse::<usize>().unwrap_or(0));
                        let first = numbers.next().unwrap_or(0);
                        let second = numbers.next().unwrap_or(0);
                        match bytes[end] {
                            b'H' | b'f' => {
                                row = first.saturating_sub(1).min(height.saturating_sub(1));
                                column = second.saturating_sub(1).min(width.saturating_sub(1));
                            }
                            b'G' | b'`' => {
                                column = first.saturating_sub(1).min(width.saturating_sub(1));
                            }
                            b'A' => row = row.saturating_sub(first.max(1)),
                            b'B' | b'e' => {
                                row = row
                                    .saturating_add(first.max(1))
                                    .min(height.saturating_sub(1));
                            }
                            b'C' | b'a' => {
                                column = column
                                    .saturating_add(first.max(1))
                                    .min(width.saturating_sub(1));
                            }
                            b'D' => column = column.saturating_sub(first.max(1)),
                            b'J' => {
                                if first == 2 || first == 3 {
                                    for line in &mut screen {
                                        line.fill(' ');
                                    }
                                } else if first == 0 {
                                    for cell in screen[row][column..].iter_mut() {
                                        *cell = ' ';
                                    }
                                }
                            }
                            b'K' => match first {
                                1 => screen[row][..=column.min(width.saturating_sub(1))].fill(' '),
                                2 => screen[row].fill(' '),
                                _ => screen[row][column..].fill(' '),
                            },
                            b's' => saved = (row, column),
                            b'u' => (row, column) = saved,
                            _ => {}
                        }
                        index = end + 1;
                        continue;
                    }
                    // OSC and other two-byte escapes do not paint cells. OSC
                    // payloads end at BEL or ST; skipping the introducer is
                    // enough for the terminal sequences emitted by tless.
                    index = index.saturating_add(2);
                }
                b'\r' => {
                    column = 0;
                    index += 1;
                }
                b'\n' => {
                    row = row.saturating_add(1).min(height.saturating_sub(1));
                    index += 1;
                }
                0x08 => {
                    column = column.saturating_sub(1);
                    index += 1;
                }
                0x07 => index += 1,
                _ if row >= height || column >= width => {
                    if let Ok(text) = std::str::from_utf8(&bytes[index..]) {
                        if let Some(ch) = text.chars().next() {
                            index += ch.len_utf8();
                        } else {
                            index += 1;
                        }
                    } else {
                        index += 1;
                    }
                }
                _ => {
                    let text = std::str::from_utf8(&bytes[index..]).unwrap_or("�");
                    let ch = text.chars().next().unwrap_or('�');
                    let cells = UnicodeWidthChar::width(ch).unwrap_or(0);
                    if cells > 0 {
                        screen[row][column] = ch;
                        for cell in screen[row]
                            .iter_mut()
                            .skip(column + 1)
                            .take(cells.saturating_sub(1))
                        {
                            *cell = ' ';
                        }
                        column = column.saturating_add(cells).min(width);
                    }
                    index += ch.len_utf8();
                }
            }
        }

        screen
            .into_iter()
            .map(|line| line.into_iter().collect::<String>().trim_end().to_string())
            .collect()
    }

    #[test]
    fn every_input_uses_the_same_toon_document_lines() {
        let json =
            r#"{"tags":["rust","cli"],"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Lin"}]}"#;
        let yaml = "tags: [rust, cli]\nusers:\n  - {id: 1, name: Ada}\n  - {id: 2, name: Lin}\n";
        let cases = vec![
            (json, None),
            (yaml, Some("yaml")),
            (
                "tags[2]: rust,cli\nusers[2]{id,name}:\n  1,Ada\n  2,Lin",
                Some("toon"),
            ),
        ];
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
    fn toon_41_input_renders_nested_keyed_and_empty_shapes_like_json() {
        for (toon, json) in [
            (
                "items[2]{id,nested{x}}:\n  1,2\n  3,4",
                r#"{"items":[{"id":1,"nested":{"x":2}},{"id":3,"nested":{"x":4}}]}"#,
            ),
            (
                "scores[2:]{score}:\n  alice: 1\n  bob: 2",
                r#"{"scores":{"alice":{"score":1},"bob":{"score":2}}}"#,
            ),
            ("items: []", r#"{"items":[]}"#),
            ("[2]:\n  -\n  -", "[{},{}]"),
        ] {
            for width in [120, 30] {
                let actual = rendered_rows(
                    &session_with_width(toon, "q", Some("--input-format=toon"), width),
                    width,
                    24,
                );
                let reference =
                    rendered_rows(&session_with_width(json, "q", None, width), width, 24);
                assert_eq!(&actual[..22], &reference[..22], "{toon}, width {width}");
                if width == 120 {
                    let painted = actual[..22].join("\n");
                    if toon.starts_with("items[2]{") {
                        assert!(painted.contains("items[2]{id,nested{x}}:"), "{actual:?}");
                    } else if toon.starts_with("scores[2:]") {
                        assert!(painted.contains("scores[2:]{score}:"), "{actual:?}");
                    } else if toon == "items: []" {
                        assert!(painted.contains("items: []"), "{actual:?}");
                    }
                }
            }
        }
        for (toon, path, value) in [
            (
                "items[2]{id,nested{x}}:\n  1,2\n  3,4",
                ".items[1].nested.x",
                "4",
            ),
            (
                "scores[2:]{score}:\n  alice: 1\n  bob: 2",
                ".scores.bob.score",
                "2",
            ),
        ] {
            let output = session_with_format(toon, &format!(":{path}\npt q"), Some("toon"));
            assert!(output.contains(&format!("{value}\r\n")), "{output}");
        }
    }

    #[test]
    fn nested_columns_preserve_focus_search_alignment_and_paths() {
        let input = r#"{"items":[{"id":1,"nested":{"left":"Ada","right":2}},{"id":3,"nested":{"left":"Lin","right":4}}],"outside":true}"#;
        for width in [120, 35] {
            let rows = rendered_rows(&session_with_width(input, "l\tq", None, width), width, 24);
            assert!(rows[..22].join("\n").contains("nested{left"), "{rows:?}");
            assert!(rows[22].contains("Table aligned"), "{rows:?}");
            let group = session_with_width(input, "lllJ\tlpP pp q", None, width);
            assert!(group.contains(".items[0].nested.left\r\n"), "{group:?}");
            assert!(group.contains("\"Ada\"\r\n"), "{group:?}");
            let second = session_with_width(input, "lllJljpP pp q", None, width);
            assert!(second.contains(".items[1].nested.left\r\n"), "{second:?}");
            assert!(second.contains("\"Lin\"\r\n"), "{second:?}");
            let search = session_with_width(input, "l\t/left\npP pp q", None, width);
            assert!(search.contains(".items[0].nested.left\r\n"), "{search:?}");
            assert!(search.contains("\"Ada\"\r\n"), "{search:?}");
        }
        let ordinary = rendered_rows(&session(input, "J\tq"), 120, 24);
        assert!(!ordinary[22].contains("Table aligned"), "{ordinary:?}");
    }

    #[test]
    fn keyed_root_table_header_rows_groups_and_filter_restore() {
        let input =
            r#"{"alice":{"score":1,"meta":{"rank":2}},"bob":{"score":3,"meta":{"rank":4}}}"#;
        for width in [120, 35] {
            let rows = rendered_rows(&session_with_width(input, "\tq", None, width), width, 24);
            assert!(rows[..22].join("\n").contains("[2:]{"), "{rows:?}");
            assert!(rows[22].contains("Table aligned"), "{rows:?}");
            let nested = session_with_width(input, "llJlpP pp q", None, width);
            assert!(nested.contains(".alice.meta.rank\r\n"), "{nested:?}");
            assert!(nested.contains("2\r\n"), "{nested:?}");
            let next = session_with_width(input, "llJljpP pp q", None, width);
            assert!(next.contains(".bob.meta.rank\r\n"), "{next:?}");
            let searched = session_with_width(input, "\t/rank\npP pp q", None, width);
            assert!(searched.contains(".alice.meta.rank\r\n"), "{searched:?}");
        }
        for width in [120, 30] {
            let collapsed = rendered_rows(&session_with_width(input, " q", None, width), width, 24);
            assert!(collapsed[0].contains("▸ [2:]"), "{collapsed:?}");
            assert_eq!(collapsed[1].trim(), "~", "{collapsed:?}");
        }
        let restored = rendered_rows(&session(input, "\t:.bob.meta.rank\n:.\nq"), 120, 24);
        assert!(restored[..22].join("\n").contains("[2:]{"), "{restored:?}");
        assert!(restored[22].contains("Table aligned"), "{restored:?}");
        let filtered = rendered_rows(&session(input, "\t:.bob.meta.rank\nq"), 120, 24);
        assert!(!filtered[22].contains("Table aligned"), "{filtered:?}");
    }

    #[test]
    fn keyed_table_focused_print_and_filtered_write_keep_nested_source_values() {
        let input =
            r#"{"alice":{"score":1,"meta":{"rank":2}},"bob":{"score":3,"meta":{"rank":4}}}"#;
        let copied = session(input, "llJlpP pp q");
        assert!(copied.contains(".alice.meta.rank\r\n"), "{copied:?}");
        assert!(copied.contains("2\r\n"), "{copied:?}");
        let target =
            std::env::temp_dir().join(format!("tless-keyed-nested-{}.json", std::process::id()));
        session(
            input,
            &format!(":.bob.meta.rank\n:write-json! {}\nq", target.display()),
        );
        assert_eq!(std::fs::read(&target).unwrap(), b"4\n");
        session(input, &format!(":.\n:write-json! {}\nq", target.display()));
        assert_eq!(
            std::fs::read(&target).unwrap(),
            b"{\n  \"alice\": {\n    \"score\": 1,\n    \"meta\": {\n      \"rank\": 2\n    }\n  },\n  \"bob\": {\n    \"score\": 3,\n    \"meta\": {\n      \"rank\": 4\n    }\n  }\n}\n"
        );
        std::fs::remove_file(target).unwrap();
    }

    #[test]
    fn brackets_move_to_entries_at_the_parent_level() {
        for (motion, expected) in [("[", "\"x\": 20"), ("]", "30\r\n")] {
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
            assert!(output.contains(&format!("{}\r\n", expected)), "{}", output);
        }
    }

    #[test]
    fn sequence_headers_show_positions_and_collapse_only_their_document() {
        let input = "{\"name\":\"Ada\",\"active\":true}\n[10,20]\n7\n{}\n[]";
        for commands in ["q", " q", "  q"] {
            let output = session(input, commands);
            assert!(!output.contains("Multiple document roots"));
            let rows = rendered_rows(&output, 120, 24);
            let headers: Vec<_> = rows.iter().filter(|row| row.contains("--- (")).collect();
            assert_eq!(headers.len(), 5, "{:?}", rows);
            for (index, header) in headers.iter().enumerate() {
                assert!(header.contains(&format!("({} of 5)", index + 1)));
            }
            if commands == " q" {
                assert!(headers[0].contains("▸ --- (1 of 5) name: Ada; active: true"));
                assert!(!rows.iter().any(|row| row.trim_end().ends_with("active: true") && !row.contains("---")));
            } else {
                assert!(headers[0].ends_with("▾ --- (1 of 5)"), "{:?}", rows);
                let field = rows.iter().find(|row| row.contains("name: Ada")).unwrap();
                assert_eq!(
                    headers[0][..headers[0].find("---").unwrap()]
                        .chars()
                        .count(),
                    field[..field.find("name:").unwrap()].chars().count()
                );
            }
            assert!(headers[1].ends_with("▾ --- (2 of 5)"));
            assert!(rows.iter().any(|row| row.contains("[2]: 10,20")));
            assert_eq!(
                rows.iter()
                    .filter(|row| row.contains('▾') || row.contains('▸'))
                    .count(),
                6,
                "Only the five document headers and nonempty array body have arrows: {:?}",
                rows
            );
        }
    }

    #[test]
    fn sequence_navigation_search_and_mouse_keep_value_targets() {
        let input = "{\"name\":\"Ada\",\"details\":{\"needle\":\"MATCH\"}}\n[10,20]\n7\n{}\n[]";
        for (commands, expected) in [
            ("l]Jpp q", "7\r\n"),
            (
                " pp q",
                "{\n  \"name\": \"Ada\",\n  \"details\": {\n    \"needle\": \"MATCH\"\n  }\n}",
            ),
            (" /MATCH\npp q", "\"MATCH\"\r\n"),
            ("4Gpp q", "\"MATCH\"\r\n"),
        ] {
            let output = session(input, commands);
            assert!(
                output
                    .replace("\r\n", "\n")
                    .contains(&expected.replace("\r\n", "\n")),
                "{}",
                output
            );
        }
        // Disable numbers so the arrow is at column one. Collapse the first
        // document by mouse, then select the scalar document's header by click.
        let commands = ":set nonumber\n\x1b[<0;1;1M\x1b[<0;7;4Mpp q";
        let output = session(input, commands);
        assert!(strip_styles(&output).contains("▸ --- (1 of 5)"));
        assert!(output.contains("7\r\n"), "{}", output);
    }

    #[test]
    fn sequence_array_body_retains_inline_array_controls() {
        let output = session("[10,20] 7", "jlpp q");
        assert!(
            output.replace("\r\n", "\n").contains("[\n  10,\n  20\n]"),
            "{}",
            output
        );
        let rows = rendered_rows(&session("[10,20] 7", "j q"), 120, 24);
        assert!(rows.iter().any(|row| row.ends_with("▾ --- (1 of 2)")));
        assert!(rows.iter().any(|row| row.contains("  - 10")), "{:?}", rows);
    }

    #[test]
    fn sequence_search_ignores_positions_and_preserves_duplicate_identity() {
        let input = "{\"status\":\"queued\",\"status\":\"done\"} 7";
        let output = session(input, " /done\npp q");
        assert!(output.contains("\"done\"\r\n"));
        assert!(strip_styles(&output).contains("occurrence 2 of 2"));
        let output = strip_styles(&session(input, " /1 of 2\nq"));
        assert!(output.contains("Pattern not found: 1 of 2"), "{}", output);
    }

    #[test]
    fn sequence_document_collapse_preserves_whole_document_json_write() {
        let target = std::env::temp_dir().join(format!(
            "tless-sequence-export-{}-{}.json",
            std::process::id(),
            NEXT_SESSION.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let output = session(
            "{\"a\":1} [2,3] 7 {} []",
            &format!("c:write-json {}\nq", target.display()),
        );
        assert!(output.contains("written"), "{}", output);
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "{\n  \"a\": 1\n}\n[\n  2,\n  3\n]\n7\n{}\n[]\n"
        );
        std::fs::remove_file(target).unwrap();
    }

    #[test]
    fn sequence_bulk_collapse_from_body_and_header_click_keep_document_state() {
        let output = session("1 2 3", "jcq");
        let rows = rendered_rows(&output, 120, 24);
        for index in 1..=3 {
            assert!(
                rows.iter()
                    .any(|row| row.contains(&format!("▸ --- ({} of 3) {}", index, index))),
                "{:?}",
                rows
            );
        }
        let output = session("{\"name\":\"Ada\"} 7", ":set nonumber\n \x1b[<0;7;1Mq");
        let rows = rendered_rows(&output, 120, 24);
        assert!(rows[0].contains("▸ --- (1 of 2) name: Ada"), "{:?}", rows);
        assert!(rows[1].contains("▾ --- (2 of 2)"), "{:?}", rows);
    }

    #[test]
    fn right_expands_inline_arrays_into_navigable_lines() {
        let output = session(r#"["alpha","beta"]"#, "ljpp q");
        let clean = strip_styles(&output);
        assert!(clean.contains("[2]: alpha,beta"), "{}", clean);
        assert!(clean.contains("  - alpha"), "{}", clean);
        assert!(clean.contains("  - beta"), "{}", clean);
        assert!(output.contains("\"alpha\"\r\n"));
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
    fn alignment_tab_targets_header_row_and_cell_once_and_indicates_focus() {
        let input = r#"{"users":[{"id":1,"name":"Ada"},{"id":200,"name":"Lin"}],"plain":true}"#;
        for keys in ["l\tq", "ll\tq", "lll\tq", "l3\tq"] {
            let rows = rendered_rows(&session(input, keys), 120, 24);
            let text = rows[..22].join("\n");
            assert!(text.contains("users[2]{id  name}:"), "{keys:?}: {rows:?}");
            assert!(text.contains("           1 Ada"), "{keys:?}: {rows:?}");
            assert!(text.contains("         200 Lin"), "{keys:?}: {rows:?}");
            assert!(rows[22].contains("Table aligned"), "{keys:?}: {rows:?}");
        }
        let rows = rendered_rows(&session(input, "l\t\tq"), 120, 24);
        assert!(rows[..22].join("\n").contains("users[2]{id,name}:"));
        assert!(!rows[22].contains("Table aligned"), "{rows:?}");

        let collapsed = rendered_rows(&session(input, "l \tq"), 120, 24);
        assert!(collapsed[0].contains("users[2]"), "{collapsed:?}");
        assert!(!collapsed[..22].join("\n").contains("           1 Ada"));
        assert!(collapsed[22].contains("Table aligned"), "{collapsed:?}");
        let restored = rendered_rows(&session(input, "l \tlq"), 120, 24);
        assert!(restored[..22].join("\n").contains("         200 Lin"));
    }

    #[test]
    fn alignment_is_independent_and_ignores_ineligible_and_prompt_focus() {
        let input = r#"{"users":[{"id":1,"name":"Ada"},{"id":200,"name":"Lin"}],"other":[{"id":3,"name":"One"},{"id":4,"name":"Two"}],"scalar":9,"values":[1,2],"list":[{"a":1},{"b":2}]}"#;
        let first = rendered_rows(&session(input, "l\tJq"), 120, 24);
        assert!(first[..22].join("\n").contains("           1 Ada"));
        assert!(!first[22].contains("Table aligned"), "{first:?}");

        let both = rendered_rows(&session(input, "l\tJ\tq"), 120, 24);
        assert!(both[..22].join("\n").contains("           1 Ada"));
        assert!(both[22].contains("Table aligned"), "{both:?}");
        let only_first = rendered_rows(&session(input, "l\tJ\t\tKq"), 120, 24);
        assert!(only_first[..22].join("\n").contains("           1 Ada"));
        assert!(only_first[..22].join("\n").contains("other[2]{id,name}:"));
        assert!(only_first[22].contains("Table aligned"), "{only_first:?}");

        for keys in ["lJJ\tq", "lJJJ\tq", "lJJJJ\tq"] {
            let rows = rendered_rows(&session(input, keys), 120, 24);
            assert!(!rows[22].contains("Table aligned"), "{keys:?}: {rows:?}");
            assert!(rows[..22].join("\n").contains("users[2]{id,name}:"));
        }
        let sequence = r#"{"users":[{"id":1},{"id":200}]} {"plain":2}"#;
        let separator = rendered_rows(&session(sequence, "\tq"), 120, 24);
        assert!(!separator[22].contains("Table aligned"), "{separator:?}");
        assert!(
            separator[..22].join("\n").contains("users[2]{id}:"),
            "{separator:?}"
        );
        let output = session(input, "l\t:write-j\t\x03/name\t\nq");
        let rows = rendered_rows(&output, 120, 24);
        assert!(rows[22].contains("Table aligned"), "{rows:?}");
        assert!(output.contains("write-json"), "{output:?}");
    }

    #[test]
    fn alignment_preserves_complete_column_widths_and_source_spelling() {
        use unicode_width::UnicodeWidthStr;

        let mut input = String::from(r#"{"users":["#);
        for index in 0..28 {
            if index != 0 {
                input.push(',');
            }
            let value = if index == 27 {
                r#""é界👩‍💻\\n\\\"END""#
            } else {
                r#""Ada""#
            };
            input.push_str(&format!(r#"{{"id":{index},"name":{value},"flag":true}}"#));
        }
        input.push_str("]}");
        let output = session(&input, "l\tq");
        let rows = rendered_rows(&output, 120, 24);
        let header = rows.iter().find(|row| row.contains("users[28]{")).unwrap();
        let first = rows.iter().find(|row| row.contains("Ada")).unwrap();
        let header_col = UnicodeWidthStr::width(header.split("name").next().unwrap());
        let value_col = UnicodeWidthStr::width(first.split("Ada").next().unwrap());
        assert_eq!(header_col, value_col, "{rows:?}");
        let flag_header_col = UnicodeWidthStr::width(header.split("flag").next().unwrap());
        let flag_value_col = UnicodeWidthStr::width(first.split("true").next().unwrap());
        assert_eq!(flag_header_col, flag_value_col, "{rows:?}");
        let last = session(&input, "l\t/END\npp q");
        let clean = strip_styles(&last);
        assert!(clean.contains("users[27].name"), "{last:?}");
        assert!(clean.contains(r#"\\n\\\"END"#), "{last:?}");
    }

    #[test]
    fn wrapping_preserves_scrolled_aligned_header_rows_and_cell_focus() {
        let input = format!(
            r#"{{"users":[{{"identifier":"{}","name":"Ada"}},{{"identifier":"{}","name":"Lin"}}],"other":"{}"}}"#,
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
            "0123456789".repeat(8),
        );
        for (focus, path) in [
            ("", ".users"),
            ("j", ".users[0]"),
            ("jl", ".users[0].identifier"),
        ] {
            let before = rendered_rows(
                &session_with_width(&input, &format!("l\t{focus}.q"), None, 35),
                35,
                24,
            );
            assert!(before[22].contains(path), "{focus}: {before:?}");
            let wrapped = rendered_rows(
                &session_with_width(&input, &format!("l\t{focus}.\x0cq"), None, 35),
                35,
                24,
            );
            assert_eq!(&before[..3], &wrapped[..3], "{focus}: {wrapped:?}");
            assert!(wrapped[22].contains(path), "{focus}: {wrapped:?}");
            let unwrapped = rendered_rows(
                &session_with_width(&input, &format!("l\t{focus}.\x0c\x0cq"), None, 35),
                35,
                24,
            );
            assert_eq!(&before[..3], &unwrapped[..3], "{focus}: {unwrapped:?}");
            let count = |rows: &[String]| {
                rows[..22]
                    .iter()
                    .take_while(|row| row.as_str() != "~")
                    .count()
            };
            assert!(count(&wrapped) > count(&before), "{focus}: {wrapped:?}");
        }
    }

    #[test]
    fn aligned_offset_survives_vertical_motion_leave_return_and_collapse() {
        let input = format!(
            r#"{{"users":[{{"identifier":"{}","name":"Ada"}},{{"identifier":"{}","name":"Lin"}}],"other":0}}"#,
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789", "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
        );
        let viewport = |keys: &str| {
            let rows = rendered_rows(&session_with_width(&input, keys, None, 35), 35, 24);
            rows[..4].to_vec()
        };
        let cell = viewport("l\tlll.q");
        assert_ne!(
            cell,
            viewport("l\tlllq"),
            "counted scrolling must move the shared viewport"
        );
        for keys in ["l\tlll.jq", "l\tlll.jkkq", "l\tlll.jkkkjq", "l\tlll.jjkq"] {
            assert_eq!(viewport(keys), cell, "{keys}");
        }

        let header = viewport("l\t.q");
        assert_ne!(header, viewport("l\tq"), "header must scroll");
        for keys in ["l\t.  q", "l\t.jhq"] {
            assert_eq!(viewport(keys), header, "{keys}");
        }
        let selected = viewport("l\t.jlq");
        assert_ne!(
            selected, header,
            "horizontal selection must reveal its cell"
        );
        assert!(
            selected.iter().any(|row| row.contains("ABCDEFGHIJ")),
            "{selected:?}"
        );

        let nested = format!(r#"{{"nest":{input},"outside":0}}"#);
        let nested_view = |keys: &str| {
            let rows = rendered_rows(&session_with_width(&nested, keys, None, 35), 35, 24);
            rows[..5].to_vec()
        };
        let before = nested_view("ll\t.q");
        assert!(before.iter().any(|row| row.contains('…')), "{before:?}");
        assert_eq!(nested_view("ll\t.k  jq"), before);
    }

    #[test]
    fn horizontal_cell_selection_reveals_without_vertical_cell_reveal() {
        let input = r#"{"users":[{"identifier":"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789","name":"Ada"},{"identifier":"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789","name":"Lin"}]}"#;
        let viewport = |keys: &str| {
            let rows = rendered_rows(&session_with_width(input, keys, None, 35), 35, 24);
            rows[..3].to_vec()
        };
        let id = viewport("l\tlllq");
        let scrolled = viewport("l\tlll.q");
        let name = viewport("l\tlll.Jq");
        assert_ne!(
            name, scrolled,
            "sibling selection must reveal the name cell"
        );
        assert!(name.iter().any(|row| row.contains("Ada")), "{name:?}");
        assert_eq!(viewport("l\tlll.$q"), name);
        for keys in ["l\tlll.JKq", "l\tlll.J0q", "l\tlll.J^q"] {
            assert_eq!(viewport(keys), id, "{keys}");
        }

        let uneven = r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"}]}"#;
        let first = rendered_rows(&session_with_width(uneven, "l\tllJq", None, 35), 35, 24);
        let moved = rendered_rows(&session_with_width(uneven, "l\tllJjq", None, 35), 35, 24);
        assert_eq!(&moved[..3], &first[..3]);
        assert!(moved[22].contains(".users[1].name"), "{moved:?}");
    }

    #[test]
    fn resize_preserves_scrolled_header_and_reveals_newly_right_clipped_cell() {
        let input = r#"{"users":[{"identifier":"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789","name":"Ada"},{"identifier":"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789","name":"Lin"}]}"#;
        let before = rendered_rows(&session_with_width(input, "l\t.q", None, 35), 35, 24);
        let height = rendered_rows(&session_with_width(input, "l\t.\x11q", None, 35), 35, 8);
        assert_eq!(
            &height[..3],
            &before[..3],
            "height-only resize moved the table"
        );
        let narrow = rendered_rows(&session_with_width(input, "l\t.\x12q", None, 35), 16, 8);
        let already_narrow = rendered_rows(&session_with_width(input, "l\t.q", None, 16), 16, 24);
        assert_eq!(
            &narrow[..3],
            &already_narrow[..3],
            "width resize lost a valid offset"
        );

        let cell = r#"{"users":[{"id":"12345678","name":"Ada"},{"id":"87654321","name":"Lin"}]}"#;
        let resized = rendered_rows(&session_with_width(cell, "l\tjlJ\x12q", None, 35), 16, 8);
        assert!(resized[..6].join("\n").contains("Ada"), "{resized:?}");
    }

    #[test]
    fn hidden_aligned_table_restores_saturated_offset_after_filtering() {
        let input = r#"{"table_with_a_long_name":[{"identifier":"0123456789ABCDEFGHIJ","name":"Ada"},{"identifier":"x","name":"Lin"}],"other":"outside"}"#;
        let viewport = |keys: &str| {
            let rows = rendered_rows(&session_with_width(input, keys, None, 50), 50, 24);
            rows[..4].to_vec()
        };
        let before = viewport("l\t999999999.q");
        assert!(before.iter().any(|row| row.contains("…:")), "{before:?}");
        assert_eq!(viewport("l\t999999999.:.other\n:.\nq"), before);

        let nested = r#"{"nest":{"users":[{"identifier":"0123456789ABCDEFGHIJ","name":"Ada"},{"identifier":"x","name":"Lin"}]},"other":0}"#;
        let nested_view = |keys: &str| {
            let rows = rendered_rows(&session_with_width(nested, keys, None, 50), 50, 24);
            rows[..5].to_vec()
        };
        let ancestor = nested_view("ll\t999999999.q");
        assert_eq!(nested_view("ll\t999999999.k  jq"), ancestor);

        let sequence = r#"{"users":[{"identifier":"0123456789ABCDEFGHIJ","name":"Ada"},{"identifier":"x","name":"Lin"}],"a":1} {"a":2}"#;
        let original = rendered_rows(
            &session_with_width(sequence, "l\t999999999.q", None, 50),
            50,
            24,
        );
        let rows = rendered_rows(
            &session_with_width(sequence, "l\t999999999.:.a\n:.\nq", None, 50),
            50,
            24,
        );
        assert_eq!(&rows[..7], &original[..7]);
        assert!(rows[..22].join("\n").contains("--- (2 of 2)"), "{rows:?}");
    }

    #[test]
    fn alignment_toggle_clears_ordinary_offsets_on_shared_list_object_headers() {
        let input = r#"{"outer":[{"rows":[{"identifier":1,"description":"Lin"}],"flag":true}]}"#;
        let viewport = |keys: &str| {
            let rows = rendered_rows(&session_with_width(input, keys, None, 35), 35, 24);
            rows[..4].to_vec()
        };
        let ordinary = viewport("lllq");
        assert_ne!(viewport("lll.q")[1], ordinary[1]);
        assert_eq!(viewport("lll.\t\tq"), ordinary);
    }

    #[test]
    fn list_object_table_full_reduction_keeps_columns_and_mouse_identity() {
        use unicode_width::UnicodeWidthStr;
        let input =
            r#"{"list":[{"users":[{"id":1,"name":"Ada"},{"id":200,"name":"Lin"}]},{"x":1}]}"#;
        let keys = "lll\t<<";
        let rows = rendered_rows(&session(input, &format!("{keys}q")), 120, 24);
        let header = rows.iter().find(|row| row.contains("users[2]{")).unwrap();
        let first = rows.iter().find(|row| row.contains("Ada")).unwrap();
        let second = rows.iter().find(|row| row.contains("Lin")).unwrap();
        let column = UnicodeWidthStr::width(header.split("name").next().unwrap());
        assert_eq!(
            column,
            UnicodeWidthStr::width(first.split("Ada").next().unwrap()),
            "{rows:?}"
        );
        assert_eq!(
            column,
            UnicodeWidthStr::width(second.split("Lin").next().unwrap()),
            "{rows:?}"
        );
        let header_row = rows.iter().position(|row| row == header).unwrap() + 1;
        let value_row = rows.iter().position(|row| row == second).unwrap() + 1;
        for (row, expected) in [
            (header_row, ".list[0].users[0].name"),
            (value_row, ".list[0].users[1].name"),
        ] {
            let click = format!("\x1b[<0;{};{row}MpP q", column + 1);
            let output = session(input, &format!("{keys}{click}"));
            assert!(
                strip_styles(&output).contains(&format!("{expected}\r\n")),
                "{output:?}"
            );
        }
    }

    #[test]
    fn nested_header_and_row_mouse_hits_keep_their_source_identity() {
        use unicode_width::UnicodeWidthStr;
        let input = r#"{"items":[{"id":1,"nested":{"left":"Ada","right":2}},{"id":3,"nested":{"left":"Lin","right":4}}]}"#;
        let keys = "l\t";
        let rows = rendered_rows(&session(input, &format!("{keys}q")), 120, 24);
        let header_row = rows
            .iter()
            .position(|row| row.contains("nested{left"))
            .unwrap();
        let value_row = rows.iter().position(|row| row.contains("Lin")).unwrap();
        let header_column = UnicodeWidthStr::width(rows[header_row].split("left").next().unwrap());
        let value_column = UnicodeWidthStr::width(rows[value_row].split("Lin").next().unwrap());
        assert_eq!(header_column, value_column, "{rows:?}");
        for (row, expected) in [
            (header_row + 1, ".items[0].nested.left"),
            (value_row + 1, ".items[1].nested.left"),
        ] {
            let output = session(
                input,
                &format!("{keys}\x1b[<0;{};{row}MpP q", header_column + 1),
            );
            assert!(output.contains(&format!("{expected}\r\n")), "{output:?}");
        }
    }

    #[test]
    fn keyed_table_mouse_reaches_nested_header_and_second_entry() {
        use unicode_width::UnicodeWidthStr;
        let input = r#"{"alice":{"rank":1,"meta":{"label":"Ada"}},"bob":{"rank":2,"meta":{"label":"Lin"}}}"#;
        let keys = "\t";
        let rows = rendered_rows(&session(input, &format!("{keys}q")), 120, 24);
        let header_row = rows
            .iter()
            .position(|row| row.contains("meta{label"))
            .unwrap();
        let value_row = rows.iter().position(|row| row.contains("Lin")).unwrap();
        let header_column = UnicodeWidthStr::width(rows[header_row].split("label").next().unwrap());
        let value_column = UnicodeWidthStr::width(rows[value_row].split("Lin").next().unwrap());
        assert_eq!(header_column, value_column, "{rows:?}");
        for (row, expected) in [
            (header_row + 1, ".alice.meta.label"),
            (value_row + 1, ".bob.meta.label"),
        ] {
            let output = session(
                input,
                &format!("{keys}\x1b[<0;{};{row}MpP q", header_column + 1),
            );
            assert!(output.contains(&format!("{expected}\r\n")), "{output:?}");
        }
    }

    #[test]
    fn aligned_table_scrolling_and_wrapping_are_local() {
        let input = format!(
            r#"{{"users":[{{"id":1,"name":"Ada"}},{{"id":200,"name":"{}END"}}],"other":"{}"}}"#,
            "abcdefghijklmnopqrstuvwxyz".repeat(3),
            "0123456789".repeat(8),
        );
        let start = rendered_rows(&session_with_width(&input, "l\tq", None, 35), 35, 24);
        assert!(start[..22].join("\n").contains("users[2]{"), "{start:?}");
        let right = rendered_rows(&session_with_width(&input, "l\t2.q", None, 35), 35, 24);
        assert!(!right[..22].join("\n").contains("users[2]{"), "{right:?}");
        assert!(!right[..22].join("\n").contains(",Ada"), "{right:?}");
        let left = rendered_rows(&session_with_width(&input, "l\t2.,,q", None, 35), 35, 24);
        assert!(left[..22].join("\n").contains("users[2]{"), "{left:?}");
        let end = rendered_rows(&session_with_width(&input, "l\t;q", None, 35), 35, 24);
        assert!(end[..22].join("\n").contains("END"), "{end:?}");
        let beginning = rendered_rows(&session_with_width(&input, "l\t;;q", None, 35), 35, 24);
        assert!(
            beginning[..22].join("\n").contains("users[2]{"),
            "{beginning:?}"
        );
        let wrapped = rendered_rows(&session_with_width(&input, "l\x0c\t.q", None, 35), 35, 24);
        assert!(
            !wrapped[..22].join("\n").contains("users[2]{"),
            "{wrapped:?}"
        );
        assert!(wrapped[22].contains("Table aligned"), "{wrapped:?}");
        let reverted = rendered_rows(&session_with_width(&input, "l\x0c\t\tq", None, 35), 35, 24);
        assert!(!reverted[22].contains("Table aligned"), "{reverted:?}");
        let data_lines = |rows: &[String]| {
            rows[..22]
                .iter()
                .take_while(|row| row.as_str() != "~")
                .count()
        };
        assert!(data_lines(&reverted) > data_lines(&start), "{reverted:?}");
    }

    #[test]
    fn aligned_table_survives_filtering_collapse_gutters_and_resize() {
        let input =
            r#"{"nest":{"users":[{"id":1,"name":"Ada"},{"id":200,"name":"Lin"}]},"last":0}"#;
        let output = session(input, "ll\t:set nonumber\n:set relativenumber\n<\x12q");
        let rows = rendered_rows(&output, 16, 8);
        assert!(rows[6].contains("Align"), "{rows:?}");
        let restored = session(input, "ll\t:.nest.users[0]\n:.\nllq");
        let rows = rendered_rows(&restored, 120, 24);
        assert!(
            rows[..22].join("\n").contains("         200 Lin"),
            "{rows:?}"
        );
        assert!(rows[22].contains("Table aligned"), "{rows:?}");
        let filtered = rendered_rows(&session(input, "ll\t:.nest.users[0]\nq"), 120, 24);
        assert!(
            !filtered[..22].join("\n").contains("users[2]{"),
            "{filtered:?}"
        );
        assert!(!filtered[22].contains("Table aligned"), "{filtered:?}");
    }

    #[test]
    fn aligned_search_reveals_clipped_cell_and_shared_field_key_without_losing_identity() {
        let id = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let input =
            format!(r#"{{"users":[{{"id":"{id}","name":"Ada"}},{{"id":"{id}","name":"Lin"}}]}}"#);
        for (pattern, expected_path, expected_value) in [
            ("Lin", ".users[1].name", "\"Lin\""),
            ("name", ".users[0].name", "\"Ada\""),
        ] {
            let output = session_with_width(&input, &format!("l\t/{pattern}\npP pp q"), None, 35);
            assert!(
                strip_styles(&output).contains(&format!("{expected_path}\r\n")),
                "{pattern}: {output:?}"
            );
            assert!(
                output.contains(&format!("{expected_value}\r\n")),
                "{pattern}: {output:?}"
            );
            let rows = rendered_rows(&output, 35, 24);
            assert!(
                rows[..22].join("\n").contains(pattern),
                "{pattern}: {rows:?}"
            );
        }
    }

    #[test]
    fn aligned_scroll_bounds_include_row_warnings_and_saturate_counts() {
        let input = "users:\n  - id: 1\n    name: Ada\n  - id: 2\n    name: .inf\n";
        let format = Some("--input-format=yaml");
        let at_end = rendered_rows(&session_with_width(input, "l\t;q", format, 35), 35, 24);
        let saturated = rendered_rows(
            &session_with_width(input, "l\t999999999.q", format, 35),
            35,
            24,
        );
        let repeated = rendered_rows(
            &session_with_width(input, "l\t999999999.999999999.q", format, 35),
            35,
            24,
        );
        // `;` fits the end in the viewport; counted scrolling can instead
        // place the very last cell at the left edge. Both must reach the
        // warning's last cell, and repeated oversized counts stop there.
        assert_eq!(
            saturated[..22],
            repeated[..22],
            "{saturated:?}\n{repeated:?}"
        );
        assert!(saturated[2].ends_with('"'), "{saturated:?}");
        assert!(at_end[..22].join("\n").contains("number"), "{at_end:?}");
        let back = rendered_rows(
            &session_with_width(input, "l\t;999999999,q", format, 35),
            35,
            24,
        );
        assert!(back[..22].join("\n").contains("users[2]{"), "{back:?}");
        let reset = rendered_rows(&session_with_width(input, "l\t;;q", format, 35), 35, 24);
        assert!(reset[..22].join("\n").contains("users[2]{"), "{reset:?}");
    }

    #[test]
    fn aligned_mouse_targets_data_and_padding_after_shared_scroll() {
        let input = r#"{"users":[{"id":"ABCDEFGHIJKLMNO","name":"Ada"},{"id":"ABCDEFGHIJKLMNOP","name":"Lin"}]}"#;
        // With no number gutter, the second row's wider id leaves one
        // padding cell after the first row's id. At offset ten that cell is
        // terminal column 18 (including the arrow, spacer and ellipsis).
        let selected = session_with_width(input, ":set nonumber\nl\t.\x1b[<0;20;3MpP q", None, 35);
        assert!(
            strip_styles(&selected).contains(".users[1].name\r\n"),
            "{selected:?}"
        );
        let copied = session_with_width(input, ":set nonumber\nl\t.\x1b[<0;20;3Mpp q", None, 35);
        assert!(copied.contains("\"Lin\"\r\n"), "{copied:?}");
        let row_padding =
            session_with_width(input, ":set nonumber\nl\t.\x1b[<0;18;2MpP q", None, 35);
        assert!(
            strip_styles(&row_padding).contains(".users[0]\r\n"),
            "{row_padding:?}"
        );
        let header_padding =
            session_with_width(input, ":set nonumber\nl\t.\x1b[<0;10;1MpP q", None, 35);
        assert!(
            strip_styles(&header_padding).contains(".users\r\n"),
            "{header_padding:?}"
        );
    }

    #[test]
    fn aligned_copy_write_and_redirected_output_ignore_presentation_padding() {
        let input = r#"{"users":[{"id":1,"name":"Ada"},{"id":200,"name":"Lin"}]}"#;
        let cell = session(input, "lll\tjpP q");
        assert!(strip_styles(&cell).contains(".users[1].id\r\n"), "{cell:?}");
        let value = session(input, "lll\tjJpp q");
        assert!(value.contains("\"Lin\"\r\n"), "{value:?}");
        let toon = std::env::temp_dir().join(format!("tless-aligned-{}.toon", std::process::id()));
        let json = std::env::temp_dir().join(format!("tless-aligned-{}.json", std::process::id()));
        session(
            input,
            &format!(
                "l:.users\n:write {}\n:write-json {}\nq",
                toon.display(),
                json.display()
            ),
        );
        let baseline_toon = std::fs::read(&toon).unwrap();
        let baseline_json = std::fs::read(&json).unwrap();
        session(
            input,
            &format!(
                "l\t:.users\n:write! {}\n:write-json! {}\nq",
                toon.display(),
                json.display()
            ),
        );
        assert_eq!(std::fs::read(&toon).unwrap(), baseline_toon);
        assert_eq!(std::fs::read(&json).unwrap(), baseline_json);
        let json_data: serde_json::Value = serde_json::from_slice(&baseline_json).unwrap();
        assert_eq!(json_data[1]["name"], "Lin");
        std::fs::remove_file(toon).unwrap();
        std::fs::remove_file(json).unwrap();
        let redirected = run(&["-i", "json"], input.as_bytes());
        assert!(redirected.status.success());
        assert!(!String::from_utf8_lossy(&redirected.stdout).contains("Table aligned"));
        assert_eq!(redirected.stdout, b"users[2]{id,name}:\n  1,Ada\n  200,Lin");
    }

    #[test]
    fn wrapping_is_opt_in_and_numeric_prefix_toggles_once() {
        let input = r#"{"long":"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdefghijklmnopqrstuvwxyz","next":42}"#;

        let off = rendered_rows(&session_with_width(input, "q", None, 35), 35, 24);
        let unwrapped = off.iter().find(|row| row.contains("long:")).unwrap();
        assert!(unwrapped.contains('…'), "unwrapped rows: {:?}", off);

        for commands in ["\x0cq", "3\x0cq"] {
            let output = session_with_width(input, commands, None, 35);
            let rows = rendered_rows(&output, 35, 24);
            let first = rows.iter().position(|row| row.contains("long:")).unwrap();
            let next = rows.iter().position(|row| row.contains("next:")).unwrap();
            assert!(
                next > first + 1,
                "wrapped rows for {:?}: {:?}",
                commands,
                rows
            );
            assert!(
                rows.iter().any(|row| row.contains("Line wrapping on")),
                "{:?}",
                rows
            );
        }

        let output = session_with_width(input, "\x0c\x0cq", None, 35);
        let rows = rendered_rows(&output, 35, 24);
        let unwrapped_again = rows.iter().find(|row| row.contains("long:")).unwrap();
        assert!(
            unwrapped_again.contains('…'),
            "rows after disabling: {:?}",
            rows
        );
        assert!(
            rows.iter().any(|row| row.contains("Line wrapping off")),
            "{:?}",
            rows
        );
    }

    #[test]
    fn continuation_rows_keep_number_and_arrow_gutters_blank() {
        let input = r#"{"long":"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdefghijklmnopqrstuvwxyz","next":42}"#;
        let output = session_with_width(input, ":set number\n\x0cq", None, 35);
        let rows = rendered_rows(&output, 35, 24);
        let first = rows.iter().position(|row| row.contains("long:")).unwrap();
        let continuation = rows
            .iter()
            .skip(first + 1)
            .take_while(|row| !row.contains("next:"))
            .find(|row| !row.is_empty())
            .unwrap();
        let next = rows.iter().find(|row| row.contains("next:")).unwrap();

        assert!(
            rows[first].starts_with(" 1 "),
            "first row: {:?}",
            rows[first]
        );
        assert!(
            continuation.len() >= 5,
            "continuation row: {:?}",
            continuation
        );
        assert!(
            continuation[..3].chars().all(|cell| cell == ' '),
            "continuation gutter: {:?}",
            continuation
        );
        assert!(
            continuation.chars().nth(3) == Some(' '),
            "continuation arrow gutter: {:?}",
            continuation
        );
        assert!(next.starts_with(" 2 "), "next row: {:?}", next);
    }

    #[test]
    fn logical_motion_skips_wrapped_continuations() {
        let input = r#"{"long":"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdefghijklmnopqrstuvwxyz","next":42}"#;

        let down = session_with_width(input, "l\x0cjq", None, 35);
        assert!(
            rendered_rows(&down, 35, 24)
                .iter()
                .any(|row| row.contains(".next")),
            "{}",
            down
        );

        let up = session_with_width(input, "l\x0cjkq", None, 35);
        assert!(
            rendered_rows(&up, 35, 24)
                .iter()
                .any(|row| row.contains(".long")),
            "{}",
            up
        );
    }

    #[test]
    fn active_scalar_search_stays_visible_after_wrapping_and_resize() {
        let value = format!("{}NEEDLE", "a".repeat(120));
        let input = format!(r#"{{"long":"{}"}}"#, value);
        let output = session_with_width(&input, "/NEEDLE\n\x0c\x12pp q", None, 120);
        let rows = rendered_rows(&output, 16, 8);

        assert!(
            rows[..6].iter().any(|row| row.contains("NEEDLE")),
            "{:?}",
            rows
        );
        assert!(rows.iter().any(|row| row.contains(".long")), "{:?}", rows);
        assert!(output.contains(&format!("\"{}\"\r\n", value)), "{}", output);
    }

    #[test]
    fn active_shared_header_search_stays_visible_after_wrapping_and_resize() {
        let key = format!("field{}NEEDLE", "x".repeat(120));
        let input = format!(r#"{{"rows":[{{"{}":"A"}},{{"{}":"B"}}]}}"#, key, key);
        let output = session_with_width(&input, "/NEEDLE\n\x0c\x12pp q", None, 120);
        let rows = rendered_rows(&output, 16, 8);

        assert!(
            rows[..6].iter().any(|row| row.contains("NEEDLE")),
            "{:?}",
            rows
        );
        assert!(output.contains("\"A\"\r\n"), "{}", output);
    }

    #[test]
    fn horizontal_scroll_is_inert_while_wrapped_and_returns_after_toggle_off() {
        let input = r#""0123456789abcdefghijKLMNOPQRSTUVWXYZ0123456789abcdefghij""#;
        let wrapped = rendered_rows(&session_with_width(input, "\x0c.q", None, 35), 35, 24);
        assert!(
            !wrapped[..22].iter().any(|row| row.contains('…')),
            "wrapped rows: {:?}",
            wrapped
        );

        let unwrapped = rendered_rows(&session_with_width(input, "\x0c.\x0c.q", None, 35), 35, 24);
        assert!(
            unwrapped[..22]
                .iter()
                .any(|row| row.contains("…abcdefghij")),
            "unwrapped rows: {:?}",
            unwrapped
        );
    }

    #[test]
    fn collapsed_preview_stays_on_one_row_when_wrapping_is_enabled() {
        let input =
            r#"{"container":{"alpha":"abcdefghijklmnopqrstuvwxyz","beta":"0123456789"},"after":3}"#;
        let rows = rendered_rows(&session_with_width(input, "l\x0c q", None, 35), 35, 24);
        let container_rows = rows[..22]
            .iter()
            .filter(|row| row.contains("container"))
            .count();
        assert_eq!(container_rows, 1, "collapsed rows: {:?}", rows);
        assert!(rows.iter().any(|row| row.contains("after:")), "{:?}", rows);
    }

    #[test]
    fn physical_scrolling_reaches_a_tall_wrapped_value() {
        let input = format!(
            r#"{{"long":"{}TAIL"}}"#,
            "0123456789abcdefghijklmnopqrstuvwxyz".repeat(8)
        );
        // ^R resizes the isolated PTY to 16x8. The command sequence exercises
        // both single-row scroll commands and full-page scrolling while the
        // same logical value remains focused.
        let wheel_down = "\x1b[<65;1;4M";
        let wheel_up = "\x1b[<64;1;4M";
        let commands = format!("l\x0c\x12{wheel_down}{wheel_down}{wheel_up}\x059\x06q");
        let output = session_with_width(&input, &commands, None, 35);
        let rows = rendered_rows(&output, 16, 8);
        let tail_visible = rows
            .windows(2)
            .any(|pair| pair[0].ends_with("TAI") && pair[1].trim_start() == "L");
        assert!(tail_visible, "{:?}", rows);
        assert!(rows.iter().any(|row| row.contains(".long")), "{:?}", rows);
    }

    #[test]
    fn mouse_continuation_cell_keeps_the_value_copy_target() {
        let value = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let input = format!(r#"{{"first":1,"long":"{}"}}"#, value);
        // Row four is a continuation after the 16-column resize. Column eight
        // is in its document text, beyond the reserved gutter.
        let click = "\x1b[<0;8;4M";
        let output = session_with_width(&input, &format!("l\x0c\x12{click}pp q"), None, 35);
        assert!(output.contains(&format!("\"{}\"\r\n", value)), "{}", output);
    }

    #[test]
    fn deep_mouse_continuation_stays_visible_after_redraw() {
        let value = format!("{}CLICKED", "abcdefghijklmnopqrstuvwxyz".repeat(8));
        let input = format!(r#"{{"first":1,"long":"{}"}}"#, value);
        // Scroll to the end, then click the bottom viewer row on a continuation.
        let wheel_down = "\x1b[<65;1;4M";
        let click = "\x1b[<0;8;6M";
        let output = session_with_width(
            &input,
            &format!("l\x0c\x12{}{click}q", wheel_down.repeat(12)),
            None,
            35,
        );
        let rows = rendered_rows(&output, 16, 8);
        assert!(
            rows.windows(2)
                .any(|pair| pair[0].contains("CLICKE") && pair[1].contains('D')),
            "{:?}",
            rows
        );
    }

    #[test]
    fn hidden_search_reveals_and_prints_the_cell_without_annotations() {
        let output = session(
            r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Lin"}]}"#,
            "l /Lin\npp q",
        );
        assert!(output.contains("\"Lin\"\r\n"), "{}", output);
        assert!(strip_styles(&output).contains("users[1].name"));
    }

    #[test]
    fn resize_and_column_motion_preserve_the_parsed_selection() {
        let output = session(
            r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Lin"}]}"#,
            "lllJj\x12pp q",
        );
        assert!(output.contains("\"Lin\"\r\n"), "{}", output);
    }

    #[test]
    fn duplicate_warnings_and_occurrence_status_preserve_copy_identity() {
        let output = session(r#"{"status":"queued","status":"done"}"#, "lJpp q");
        let clean = strip_styles(&output);
        assert!(clean.contains("# WARN Duplicate key"));
        assert!(clean.contains("occurrence 2 of 2"));
        assert!(output.contains("\"done\"\r\n"));
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

    #[test]
    fn write_open_failure_reports_an_error_and_keeps_the_viewer_usable() {
        let target = std::env::temp_dir()
            .join(format!("tless-no-directory-{}", std::process::id()))
            .join("output.toon");
        let output = session("42", &format!(":wt {}\npt q", target.display()));
        assert!(output.contains("Error opening file for writing"));
        assert!(!output.contains(" written"));
        assert!(output.contains("42\r\n"));
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
    fn default_toon_commands_write_identical_bytes_to_literal_filenames() {
        for (index, command) in ["write", "w", "write-toon", "wt"].iter().enumerate() {
            let target = std::env::temp_dir()
                .join(format!("tless-default-toon-{}-{index}", std::process::id()));
            let output = session(
                r#"{"name":"Ada","scores":[1,2]}"#,
                &format!(":{command} {}\nq", target.display()),
            );
            assert!(output.contains("written"), "{output}");
            assert_eq!(
                std::fs::read(&target).unwrap(),
                b"name: Ada\nscores[2]: 1,2"
            );
            assert!(!target.with_extension("toon").exists());
            std::fs::remove_file(target).unwrap();
        }
    }

    #[test]
    fn yaml_and_json_writes_use_native_framing_for_multiple_roots() {
        let yaml = std::env::temp_dir().join(format!("tless-native-{}.yaml", std::process::id()));
        let json = std::env::temp_dir().join(format!("tless-native-{}.json", std::process::id()));
        let output = session(
            r#"{"a":1} {"b":[true,null]}"#,
            &format!(
                ":write-yaml {}\n:write-json {}\nq",
                yaml.display(),
                json.display()
            ),
        );
        assert!(output.matches("written").count() >= 2, "{output}");
        assert_eq!(
            std::fs::read(&yaml).unwrap(),
            b"---\n{\n  \"a\": 1\n}\n---\n{\n  \"b\": [\n    true,\n    null\n  ]\n}\n"
        );
        assert_eq!(
            std::fs::read(&json).unwrap(),
            b"{\n  \"a\": 1\n}\n{\n  \"b\": [\n    true,\n    null\n  ]\n}\n"
        );
        std::fs::remove_file(yaml).unwrap();
        std::fs::remove_file(json).unwrap();
    }

    #[test]
    fn jsonl_and_ndjson_write_identical_records_without_expanding_arrays() {
        let ndjson =
            std::env::temp_dir().join(format!("tless-records-{}.ndjson", std::process::id()));
        let jsonl =
            std::env::temp_dir().join(format!("tless-records-{}.jsonl", std::process::id()));
        let output = session(
            "{\"message\":\"a\\nb\",\"items\":[1,{\"n\":2}],\"n\":0.123456789012345678901,\"n\":1e1000000} [1,2] null",
            &format!(
                ":write-ndjson {}\n:write-jsonl {}\nq",
                ndjson.display(),
                jsonl.display()
            ),
        );
        assert!(output.matches("written").count() >= 2, "{output}");
        let expected = b"{\"message\":\"a\\nb\",\"items\":[1,{\"n\":2}],\"n\":0.123456789012345678901,\"n\":1e1000000}\n[1,2]\nnull\n";
        assert_eq!(std::fs::read(&ndjson).unwrap(), expected);
        assert_eq!(std::fs::read(&jsonl).unwrap(), expected);
        std::fs::remove_file(ndjson).unwrap();
        std::fs::remove_file(jsonl).unwrap();
    }

    #[test]
    fn format_shortcuts_refuse_existing_files_and_bang_truncates_them() {
        for (index, short, long, bytes) in [
            (0, "wj", "write-json", &b"{\n  \"x\": 1\n}\n"[..]),
            (1, "wy", "write-yaml", &b"---\n{\n  \"x\": 1\n}\n"[..]),
            (2, "wn", "write-ndjson", &b"{\"x\":1}\n"[..]),
        ] {
            let target = std::env::temp_dir().join(format!(
                "tless-shortcut-{}-{index}.toon",
                std::process::id()
            ));
            let long_target = std::env::temp_dir()
                .join(format!("tless-long-{}-{index}.toon", std::process::id()));
            std::fs::write(&target, b"original with trailing bytes").unwrap();
            let refusal = session(r#"{"x":1}"#, &format!(":{short} {}\nq", target.display()));
            assert!(
                refusal.contains("already exists (add ! to overwrite)"),
                "{refusal}"
            );
            assert_eq!(
                std::fs::read(&target).unwrap(),
                b"original with trailing bytes"
            );
            let output = session(
                r#"{"x":1}"#,
                &format!(
                    ":{short}! {}\n:{long} {}\nq",
                    target.display(),
                    long_target.display()
                ),
            );
            assert!(output.contains("written"), "{output}");
            assert_eq!(std::fs::read(&target).unwrap(), bytes);
            assert_eq!(std::fs::read(&long_target).unwrap(), bytes);
            std::fs::remove_file(target).unwrap();
            std::fs::remove_file(long_target).unwrap();
        }
    }

    #[test]
    fn writes_native_formats_from_yaml_and_toon_inputs() {
        let target = std::env::temp_dir().join(format!("tless-formats-{}.out", std::process::id()));
        for format in ["yaml", "toon"] {
            let output = session_with_format(
                "a: 1",
                &format!(":write-toon {}\nq", target.display()),
                Some(format),
            );
            assert!(output.contains("written"));
            assert_eq!(std::fs::read_to_string(&target).unwrap(), "a: 1");
            std::fs::remove_file(&target).unwrap();
            session_with_format(
                "a: 1",
                &format!(":write-json {}\nq", target.display()),
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
    fn wrapping_does_not_change_toon_export_or_printed_value() {
        let value = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let input = format!(r#"{{"long":"{}"}}"#, value);
        let target =
            std::env::temp_dir().join(format!("tless-wrapped-export-{}.toon", std::process::id()));
        let output = session_with_width(
            &input,
            &format!("\x0c:wt {}\nq", target.display()),
            None,
            35,
        );
        assert!(output.contains("written"), "{}", output);
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            format!("long: {}", value)
        );
        std::fs::remove_file(&target).unwrap();

        let output = session_with_width(&input, "l\x0clpt q", None, 35);
        assert!(output.contains(&format!("{}\r\n", value)), "{}", output);
    }

    #[test]
    fn prints_focused_canonical_toon_on_the_persistent_screen() {
        let output = session(r#"{"items":[1,2]}"#, "lpt q");
        assert!(output.contains("[2]: 1,2\r\n"), "{}", output);
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
        let output = session("1 2", &format!(" :write! {}\nq", target.display()));
        assert!(output.contains("requires exactly one root"));
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "original contents with a long suffix"
        );
        let output = session("42", &format!(":write {}\nq", target.display()));
        assert!(output.contains("already exists"));
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "original contents with a long suffix"
        );
        let output = session("42", &format!(":write-toon! {}\nq", target.display()));
        assert!(output.contains("written"));
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "42");
        session("{}", &format!(":w! {}\nq", target.display()));
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "");
        std::fs::remove_file(&target).unwrap();
        session("42", &format!(":w! {}\nq", target.display()));
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "42");
        std::fs::remove_file(&target).unwrap();
        let output = session("1 2", &format!(" :write-toon! {}\nq", target.display()));
        assert!(output.contains("requires exactly one root"));
        assert!(!target.exists());
    }

    #[test]
    fn writes_preserve_selected_focus_collapse_and_wrapping_state() {
        let target = std::env::temp_dir().join(format!("tless-state-{}.json", std::process::id()));
        let name = "long label ".repeat(30);
        let input = format!(r#"{{"hits":{{"items":[1,2],"name":"{name}"}},"outside":true}}"#);
        let baseline = session(&input, ":.hits\nl \x0cpp q");
        let output = session(
            &input,
            &format!(":.hits\nl \x0c:write-json {}\npp q", target.display()),
        );
        assert!(output.contains("written"), "{output}");
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            format!("{{\n  \"items\": [\n    1,\n    2\n  ],\n  \"name\": \"{name}\"\n}}\n")
        );
        assert!(
            output.replace("\r\n", "\n").contains("[\n  1,\n  2\n]\n"),
            "{output}"
        );
        let rows = rendered_rows(&output, 120, 24);
        let baseline_rows = rendered_rows(&baseline, 120, 24);
        assert_eq!(&rows[..22], &baseline_rows[..22]);
        assert!(
            rows.iter().any(|row| row.contains(".hits.items")),
            "{rows:?}"
        );
        std::fs::remove_file(target).unwrap();
    }

    #[test]
    fn unsupported_and_obsolete_write_names_never_create_destinations() {
        let target =
            std::env::temp_dir().join(format!("tless-removed-write-{}", std::process::id()));
        for command in [
            "writetoon",
            "writetoon!",
            "writesexp",
            "writesexp!",
            "write-csv",
        ] {
            let output = session("42", &format!(":{command} {}\nq", target.display()));
            assert!(
                strip_styles(&output).contains(&format!("Unknown command: {command}")),
                "{output}"
            );
            assert!(!target.exists(), "{command}");
        }
        let output = session(
            "42",
            &format!(":write\n:write-json\n:wy {} extra\nq", target.display()),
        );
        let plain = strip_styles(&output);
        assert!(plain.contains("Unknown command: write"), "{output}");
        assert!(plain.contains("Unknown command: write-json"), "{output}");
        assert!(plain.contains("Unknown command: wy"), "{output}");
        assert!(!target.exists());
    }

    #[cfg(not(feature = "sexp"))]
    #[test]
    fn sexp_write_names_are_unavailable_without_the_feature() {
        let target =
            std::env::temp_dir().join(format!("tless-disabled-sexp-{}", std::process::id()));
        for command in ["write-sexp", "write-sexp!", "ws", "ws!"] {
            let output = session("42", &format!(":{command} {}\nq", target.display()));
            assert!(
                strip_styles(&output).contains(&format!("Unknown command: {command}")),
                "{output}"
            );
            assert!(!target.exists(), "{command}");
        }
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
    assert!(output.status.success());
    assert_eq!(output.stdout, b"42");
    assert_eq!(
        run(
            &[
                "--input-format",
                "json",
                "-o",
                "json",
                path.to_str().unwrap(),
            ],
            b""
        )
        .stdout,
        b"42\n"
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn toon_pipeline_validates_before_output() {
    let input = "\u{feff}items[99]: a,b\r\n  \r\n".as_bytes();
    let output = run(&["--input-format", "toon"], input);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Unable to parse input"));
}

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
        .args(["--input-format", "toon"])
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

#[test]
fn format_conflicts_and_invalid_utf8_fail_without_output() {
    for args in [
        &["--input-format", "toon", "--input-format", "json"][..],
        &["--input-format", "toon", "--input-format", "yaml"][..],
    ] {
        assert!(!run(args, b"").status.success());
    }
    let output = run(&["--input-format", "toon"], &[0xff]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Unable to get input"));
    assert!(!run(&[], b"name: Ada").status.success());
}

#[test]
fn input_limit_applies_to_stdin_and_files_without_partial_output() {
    let output = run(&["--max-input-bytes", "2"], b"123");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("input exceeds"));
    assert!(
        run(&["-o", "json", "--max-input-bytes", "3"], b"123")
            .status
            .success()
    );
    assert!(
        run(&["-o", "json", "--max-input-bytes", "0"], b"123")
            .status
            .success()
    );
    let path = std::env::temp_dir().join(format!("tless-limit-{}.json", std::process::id()));
    std::fs::write(&path, b"123").unwrap();
    let output = run(&["--max-input-bytes", "2", path.to_str().unwrap()], b"");
    std::fs::remove_file(path).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
}

#[test]
fn invalid_input_limit_is_a_usage_error() {
    let invalid = run(&["--max-input-bytes", "invalid"], b"");
    assert_eq!(invalid.status.code(), Some(2));
    assert!(invalid.stdout.is_empty());
    assert!(!invalid.stderr.is_empty());
}

#[test]
fn legacy_format_options_are_usage_errors() {
    for legacy in ["--json", "--yaml", "--toon", "--input", "--output"] {
        let output = run(&[legacy], b"");
        assert_eq!(output.status.code(), Some(2), "{legacy}");
    }
}

#[test]
fn removed_modes_are_usage_errors() {
    for args in [
        &["--mode", "line"][..],
        &["--mode", "data"][..],
        &["-m", "data"][..],
    ] {
        let output = run(args, b"");
        assert_eq!(output.status.code(), Some(2));
    }
}
