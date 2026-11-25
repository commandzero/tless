use super::pty::session;
use super::pty::session_with_width;
#[cfg(target_os = "linux")]
use super::pty::{SessionOptions, session_with_options};
use super::screen::rendered_rows;
use super::screen::strip_styles;
use std::io::Write;

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

#[cfg(target_os = "linux")]
#[test]
fn full_device_write_reports_failure_and_retains_navigation() {
    let output = session_with_options(
        r#"{"a":42,"b":99}"#,
        ":wt! /dev/full\nljpP pp q",
        SessionOptions {
            expected_status: Some(0),
            ..SessionOptions::default()
        },
    );
    let entered = output.find("\x1b[?1049h").unwrap();
    let failure = output
        .find("Error writing file:")
        .expect("write failure must be reported");
    assert!(
        entered < failure,
        "write failure must occur after terminal setup"
    );
    assert!(!output.contains("/dev/full written"), "{output}");
    assert!(strip_styles(&output).contains(".b\r\n"), "{output}");
    assert!(output.contains("99\r\n"), "{output}");
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
        let target =
            std::env::temp_dir().join(format!("tless-default-toon-{}-{index}", std::process::id()));
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
    let ndjson = std::env::temp_dir().join(format!("tless-records-{}.ndjson", std::process::id()));
    let jsonl = std::env::temp_dir().join(format!("tless-records-{}.jsonl", std::process::id()));
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
        let long_target =
            std::env::temp_dir().join(format!("tless-long-{}-{index}.toon", std::process::id()));
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
    for argument in ["--input-format=yaml", "--input-format=toon"] {
        let output = session_with_width(
            "a: 1",
            &format!(":write-toon {}\nq", target.display()),
            Some(argument),
            120,
        );
        assert!(output.contains("written"));
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "a: 1");
        std::fs::remove_file(&target).unwrap();
        session_with_width(
            "a: 1",
            &format!(":write-json {}\nq", target.display()),
            Some(argument),
            120,
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
    let target = std::env::temp_dir().join(format!("tless-replace-{}.toon", std::process::id()));
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
    let target = std::env::temp_dir().join(format!("tless-removed-write-{}", std::process::id()));
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
    let target = std::env::temp_dir().join(format!("tless-disabled-sexp-{}", std::process::id()));
    for command in ["write-sexp", "write-sexp!", "ws", "ws!"] {
        let output = session("42", &format!(":{command} {}\nq", target.display()));
        assert!(
            strip_styles(&output).contains(&format!("Unknown command: {command}")),
            "{output}"
        );
        assert!(!target.exists(), "{command}");
    }
}
