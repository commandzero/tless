use super::pty::{SessionOptions, cursor_requests, session_with_options};
use super::screen::strip_styles;

#[test]
fn piped_input_with_controlling_pty_restores_interactive_input() {
    let output = session_with_options(
        r#"{"a":1}"#,
        "q",
        SessionOptions {
            piped: true,
            ..SessionOptions::default()
        },
    );
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
