use super::pty::{SessionOptions, session_with_options};
use super::screen::strip_styles;
use crate::run;

#[test]
#[ignore = "requires native less; selected by scripts/preflight.sh test"]
fn native_help_pager_returns_to_selected_value() {
    let output = session_with_options(
        r#"{"a":1,"b":"雪"}"#,
        "lj\x1b[11~qpP pp q",
        SessionOptions {
            expected_status: Some(0),
            ..SessionOptions::default()
        },
    );
    assert!(strip_styles(&output).contains(".b\r\n"), "{output:?}");
    assert!(output.contains("\"雪\"\r\n"), "{output:?}");
}

#[test]
#[ignore = "native startup-parse acceptance via scripts/preflight.sh test"]
fn native_malformed_toon_preserves_startup_terminal_state() {
    let input = "items[2]: a";
    let output = session_with_options(
        input,
        "",
        SessionOptions {
            argument: Some("--input-format=toon"),
            expected_status: Some(1),
            ..SessionOptions::default()
        },
    );
    assert!(output.contains("Unable to parse input"), "{output:?}");
    assert!(!output.contains("\x1b[?1049h"), "{output:?}");
    let redirected = run(&["-i", "toon", "-o", "toon"], input.as_bytes());
    assert_eq!(redirected.status.code(), Some(1));
    assert!(redirected.stdout.is_empty(), "{redirected:?}");
    assert!(
        String::from_utf8_lossy(&redirected.stderr).contains("Unable to parse input"),
        "{redirected:?}"
    );
}
