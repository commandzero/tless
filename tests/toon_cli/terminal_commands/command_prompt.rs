use super::pty::session;
use super::pty::session_with_width;
use super::screen::rendered_rows;
use super::screen::strip_styles;

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
