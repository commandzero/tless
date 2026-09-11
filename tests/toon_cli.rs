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
    fn output_selection_leaves_terminal_view_and_json_print_unchanged() {
        for option in ["--output=json", "--output=yaml", "--output=toon"] {
            let output = session_with_format(r#"{"a":1}"#, "lpp q", Some(option));
            assert!(strip_styles(&output).contains("a: 1"));
            assert!(output.contains("1\r\n\r\nPress any key to continue."));
        }
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
        assert!(
            output.contains(&format!("\"{}\"\r\n\r\nPress any key to continue.", value)),
            "{}",
            output
        );
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
        assert!(
            output.contains("\"A\"\r\n\r\nPress any key to continue."),
            "{}",
            output
        );
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
        assert!(
            output.contains(&format!("\"{}\"\r\n\r\nPress any key to continue.", value)),
            "{}",
            output
        );
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
        assert!(
            output.contains(&format!("{}\r\n\r\nPress any key to continue.", value)),
            "{}",
            output
        );
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
        run(&["--json", "-o", "json", path.to_str().unwrap()], b"").stdout,
        b"42\n"
    );
    std::fs::remove_file(path).unwrap();
}

#[cfg(feature = "toon")]
#[test]
fn toon_pipeline_validates_before_output() {
    let input = "\u{feff}items[99]: a,b\r\n  \r\n".as_bytes();
    let output = run(&["--toon"], input);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Unable to parse input"));
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
    assert!(run(&["-o", "json", "--max-input-bytes", "3"], b"123")
        .status
        .success());
    assert!(run(&["-o", "json", "--max-input-bytes", "0"], b"123")
        .status
        .success());
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
