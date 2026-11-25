use super::pty::CopyEvent;
use super::pty::NEXT_SESSION;
use super::pty::{SessionOptions, session_with_options};
use std::io;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

struct XclipOwner(std::process::Child);

impl Drop for XclipOwner {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn read_x11_clipboard(deadline: Instant) -> io::Result<String> {
    let mut reader = Command::new("xclip")
        .args(["-selection", "clipboard", "-out", "-target", "UTF8_STRING"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    loop {
        if reader.try_wait()?.is_some() {
            let output = reader.wait_with_output()?;
            if !output.status.success() {
                return Err(io::Error::other(format!(
                    "xclip reader failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
            return String::from_utf8(output.stdout)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error));
        }
        if Instant::now() >= deadline {
            reader.kill()?;
            reader.wait()?;
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "xclip did not receive the X11 clipboard selection",
            ));
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn seed_x11_clipboard(sentinel: &str) -> io::Result<XclipOwner> {
    // Foreground ownership keeps the seed alive until tless replaces it.
    let mut owner = XclipOwner(
        Command::new("xclip")
            .args(["-selection", "clipboard", "-in", "-quiet"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?,
    );
    owner
        .0
        .stdin
        .take()
        .unwrap()
        .write_all(sentinel.as_bytes())?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if read_x11_clipboard(deadline).is_ok_and(|value| value == sentinel) {
            return Ok(owner);
        }
        if Instant::now() >= deadline || owner.0.try_wait()?.is_some() {
            return Err(io::Error::other(
                "xclip did not establish the sentinel clipboard selection",
            ));
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

enum CopyExpected<'a> {
    Text(&'a str),
    Json(&'a str),
}

fn assert_native_clipboard_session(
    input: &str,
    commands: &str,
    format: Option<&str>,
    expected: &[CopyExpected<'_>],
) {
    let mut pasted = Vec::new();
    let mut seed = None;
    let mut observe_copy = |event| {
        match event {
            CopyEvent::Before => {
                let sentinel = format!(
                    "tless-native-sentinel-{}-{}",
                    std::process::id(),
                    NEXT_SESSION.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                );
                seed = Some(seed_x11_clipboard(&sentinel)?);
            }
            CopyEvent::Complete => {
                pasted.push(read_x11_clipboard(
                    Instant::now() + Duration::from_secs(10),
                )?);
                drop(seed.take());
            }
        }
        Ok(())
    };
    let output = session_with_options(
        input,
        commands,
        SessionOptions {
            argument: format,
            observe_copy: Some(&mut observe_copy),
            ..SessionOptions::default()
        },
    );
    assert_eq!(pasted.len(), expected.len(), "{output:?}");
    for (actual, expected) in pasted.iter().zip(expected) {
        match expected {
            CopyExpected::Text(expected) => assert_eq!(actual, expected, "{output:?}"),
            CopyExpected::Json(expected) => assert_eq!(
                serde_json::from_str::<serde_json::Value>(actual).unwrap(),
                serde_json::from_str::<serde_json::Value>(expected).unwrap(),
                "{output:?}"
            ),
        }
    }
    let printed: Vec<_> = output
        .split("\x1b[?1049l")
        .skip(1)
        .filter_map(|screen| screen.split_once("\x1b[2J\x1b[1;1H"))
        .filter_map(|(_, text)| text.split_once("\r\n\r\nPress any key to continue."))
        .map(|(value, _)| value.replace("\r\n", "\n"))
        .collect();
    assert_eq!(printed, pasted, "{output:?}");
}

#[test]
#[ignore = "requires isolated authenticated X11 via scripts/preflight.sh test"]
fn native_x11_clipboard_roundtrips_preserve_values_and_focus() {
    use CopyExpected::{Json, Text};
    assert_eq!(
        std::env::var("TLESS_NATIVE_X11").as_deref(),
        Ok("1"),
        "run through scripts/preflight.sh test, never the desktop display"
    );
    let display = std::env::var("DISPLAY").expect("the isolated Xvfb display is required");
    assert!(
        display
            .strip_prefix(':')
            .and_then(|number| number.parse::<u16>().ok())
            .is_some(),
        "xvfb-run must provide a local X11 display"
    );
    let authority =
        std::env::var_os("XAUTHORITY").expect("xvfb-run must provide a private Xauthority file");
    use std::os::unix::fs::PermissionsExt;
    let authority = std::fs::symlink_metadata(authority).unwrap();
    assert!(authority.is_file() && authority.len() > 0);
    assert_eq!(authority.permissions().mode() & 0o077, 0);

    let nested_toon = "message: Ada\nitems[2]: 1,2";
    assert_native_clipboard_session(
        r#"{"nested":{"message":"Ada","items":[1,2]},"after":99}"#,
        "lytpt yypp  ytpt jyppP yypp q",
        None,
        &[
            Text(nested_toon),
            Json(r#"{"message":"Ada","items":[1,2]}"#),
            Text(nested_toon),
            Text(".after"),
            Text("99"),
        ],
    );
    assert_native_clipboard_session(
        r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"雪"}]}"#,
        "lll\tjJytpt yppP yypp q",
        None,
        &[Text("雪"), Text(".users[1].name"), Text("\"雪\"")],
    );
    assert_native_clipboard_session(
        "\"é.\\\"key\":\n  rows[2]: \"line\\n雪\",雪",
        "lykpk yppP lllysps yppP jysps yppP q",
        Some("--input-format=toon"),
        &[
            Text("\"é.\\\"key\""),
            Text("[\"é.\\\"key\"]"),
            Text("line\n雪"),
            Text("[\"é.\\\"key\"].rows[0]"),
            Text("雪"),
            Text("[\"é.\\\"key\"].rows[1]"),
        ],
    );
    assert_native_clipboard_session(
        "0.123456789012345678901",
        "ytpt q",
        None,
        &[Text("0.12345678901234568")],
    );
}
