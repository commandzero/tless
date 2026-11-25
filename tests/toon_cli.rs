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

#[path = "toon_cli/terminal_commands/mod.rs"]
mod terminal_commands;

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
