use super::pty::session;
use super::pty::session_with_width;
use super::screen::rendered_rows;
use super::screen::strip_styles;

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
    let target = std::env::temp_dir().join(format!("tless-path-scope-{}.json", std::process::id()));
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
    let target = std::env::temp_dir().join(format!("tless-path-keys-{}.json", std::process::id()));
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
    let jsonl = std::env::temp_dir().join(format!("tless-filtered-{}.jsonl", std::process::id()));
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
    let output = session_with_width(
        input,
        &format!(
            ":.good\n:write-json! {}\n:.\n:write-json! {}\n:write-jsonl! {}\nq",
            existing.display(),
            existing.display(),
            missing.display()
        ),
        Some("--input-format=yaml"),
        120,
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
    let target = std::env::temp_dir().join(format!("tless-path-sexp-{}.sexp", std::process::id()));
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
