use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run(args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tless"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn success(args: &[&str], input: &str) -> String {
    let output = run(args, input);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn selectors_and_conversion_matrix() {
    let mut inputs = vec![("--json", "{\"a\":1}"), ("--yaml", "a: 1")];
    if cfg!(feature = "toon") {
        inputs.push(("--toon", "a: 1\n"));
    }
    for (flag, input) in inputs {
        assert_eq!(success(&[flag, "-o", "json"], input), "{\n  \"a\": 1\n}\n");
        assert_eq!(
            success(&[flag, "--output", "yaml"], input),
            "---\n{\n  \"a\": 1\n}\n"
        );
        if cfg!(feature = "toon") {
            assert_eq!(success(&[flag], input), "a: 1");
            assert_eq!(success(&[flag, "--output=toon"], input), "a: 1");
        }
    }
    for args in [&["-o"][..], &["--output", "xml"], &["-o", "JSON"]] {
        let output = run(args, "");
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    let help = success(&["--help"], "");
    for text in [
        "-o, --output",
        "non-terminal",
        "default: toon",
        "json",
        "yaml",
    ] {
        assert!(help.contains(text), "{}", help);
    }
}

#[test]
fn json_preserves_tokens_duplicates_and_root_framing() {
    assert_eq!(
        success(
            &["--yaml", "-o", "json"],
            "[.123456789012345678901, +1.5, 1.]"
        ),
        "[\n  0.123456789012345678901,\n  1.5,\n  1.0\n]\n"
    );
    assert_eq!(
        success(
            &["-o", "json", "-"],
            "{\"a\":0.123456789012345678901,\"a\":1e1000000} []"
        ),
        "{\n  \"a\": 0.123456789012345678901,\n  \"a\": 1e1000000\n}\n[]\n"
    );
    assert_eq!(success(&["--yaml", "-o", "json"], ""), "");
    assert_eq!(success(&["--yaml", "-o", "yaml"], ""), "");
}

#[test]
fn yaml_strings_are_json_escaped_and_types_survive_yaml_output() {
    let input = "---\n\"a\\\"\\\\\\t\\n\": \"b\\\"\\\\\\t\\n\\0\"\n---\n[true, \"true\", \".inf\", null, [], {}]\n";
    let json = success(&["--yaml", "-o", "json"], input);
    assert!(
        json.contains("\"a\\\"\\\\\\t\\n\": \"b\\\"\\\\\\t\\n\\u0000\""),
        "{}",
        json
    );
    let yaml = success(&["--yaml", "-o", "yaml"], input);
    assert_eq!(yaml.matches("---\n").count(), 2);
    assert_eq!(
        yaml_rust::YamlLoader::load_from_str(input).unwrap(),
        yaml_rust::YamlLoader::load_from_str(&yaml).unwrap()
    );
}

#[test]
fn yaml_preserves_typed_complex_keys_and_nonfinite_values() {
    let input =
        "1: number\n\"1\": string\n? [1, true]\n: .inf\n? {a: [null, \"x\"]}\n: -.inf\nnan: .nan\n";
    let yaml = success(&["--yaml", "-o", "yaml"], input);
    assert_eq!(
        yaml_rust::YamlLoader::load_from_str(input).unwrap(),
        yaml_rust::YamlLoader::load_from_str(&yaml).unwrap(),
        "{yaml}"
    );
    let duplicates = success(&["-o", "yaml"], "{\"a\":1,\"a\":2}");
    assert!(duplicates.contains("\"a\": 1,\n  \"a\": 2"));
    for input in ["1: x", "x: .inf", "x: .nan", "[1, 2"] {
        let output = run(&["--yaml", "-o", "json"], input);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn file_detection_and_override_do_not_select_output() {
    let path = std::env::temp_dir().join(format!("tless-output-{}.yaml", std::process::id()));
    std::fs::write(&path, "a: 1").unwrap();
    assert_eq!(
        success(&[path.to_str().unwrap(), "-o", "json"], ""),
        "{\n  \"a\": 1\n}\n"
    );
    assert_eq!(
        run(&[path.to_str().unwrap(), "--json", "-o", "yaml"], "")
            .status
            .code(),
        Some(1)
    );
    std::fs::remove_file(path).unwrap();
}

#[cfg(not(feature = "toon"))]
#[test]
fn disabled_toon_output_is_actionable_without_fallback() {
    for args in [&[][..], &["-o", "toon"]] {
        let output = run(args, "{}");
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("--features toon")
                && error.contains("-o json")
                && error.contains("-o yaml")
        );
    }
}

#[cfg(feature = "toon")]
#[test]
fn toon_export_contract_and_failures() {
    let escaped_json = r#"{"a\"\\\n":"x\"\\\t\n","nested":[true,null,{"b":"c"}]}"#;
    let yaml = success(&["-o", "yaml"], escaped_json);
    assert_eq!(success(&["--yaml"], &yaml), success(&[], escaped_json));
    for (input, expected) in [
        ("{}", ""),
        ("{\"a\":1,\"a\":2}", "a: 2"),
        ("[{},{}]", "[2]{}:\n  \n  "),
        ("{\"tags\":[1,2]}", "tags[2]: 1,2"),
    ] {
        assert_eq!(success(&[], input), expected);
        assert_eq!(success(&["-o", "toon"], input), expected);
    }
    assert_eq!(success(&["--toon"], ""), "");
    assert_eq!(success(&["--toon"], "a: 1\r\n"), "a: 1");
    for (flag, input) in [
        ("--json", ""),
        ("--json", "1 2"),
        ("--yaml", "1: x"),
        ("--yaml", "a: .inf"),
        ("--json", "1e1025"),
        ("--toon", "items[99]: a,b"),
        ("--yaml", "[bad"),
    ] {
        let output = run(&[flag], input);
        assert_eq!(output.status.code(), Some(1), "{flag} {input}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    let deep = format!("{}0{}", "[".repeat(257), "]".repeat(257));
    let output = run(&[], &deep);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
}

#[test]
fn machine_output_needs_no_controlling_terminal() {
    use std::os::unix::process::CommandExt;
    let mut command = Command::new(env!("CARGO_BIN_EXE_tless"));
    command
        .args(["-o", "json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // A fresh session has no controlling terminal; all descriptors are pipes.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().unwrap();
    child.stdin.take().unwrap().write_all(b"42").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"42\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn broken_pipe_fails_for_json_and_yaml_in_every_build() {
    use std::net::Shutdown;
    use std::os::fd::OwnedFd;
    use std::os::unix::net::UnixStream;
    for format in ["json", "yaml"] {
        let (reader, writer) = UnixStream::pair().unwrap();
        reader.shutdown(Shutdown::Both).unwrap();
        drop(reader);
        let mut child = Command::new(env!("CARGO_BIN_EXE_tless"))
            .args(["-o", format])
            .stdin(Stdio::piped())
            .stdout(OwnedFd::from(writer))
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(b"42").unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("Unable to write output"));
    }
}
