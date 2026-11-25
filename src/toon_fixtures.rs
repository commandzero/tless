use serde_json::Value;
use std::fs::{self, DirEntry};
use std::path::Path;

// Upstream toon-format/spec v4.4.0, commit d09c3d33e37389345a74bd96981eb96dd9be6c5e.
// The original toon-v4/tests/fixtures and LICENSE remain the historical 4.1 snapshot.
// Upstream files retain their 4.0 baseline; individual cases can require up to 4.4.
const PINNED_SPEC_VERSION: (u32, u32) = (4, 4);

pub(crate) struct FixtureCase {
    pub group: String,
    pub metadata: Value,
    pub toon: String,
}

pub(crate) fn load_fixtures(category: &str) -> Vec<FixtureCase> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/toon-v4/v4.4.0/tests/fixtures")
        .join(category);
    let groups = sorted_entries(&root);
    let group_names: Vec<_> = groups
        .iter()
        .map(|entry| entry.file_name().into_string().unwrap())
        .collect();
    assert_eq!(
        group_names,
        expected_groups(category),
        "unexpected fixture groups: {root:?}"
    );

    groups
        .into_iter()
        .flat_map(|group| load_group(category, group))
        .collect()
}

fn expected_groups(category: &str) -> &'static [&'static str] {
    match category {
        "encode" => &[
            "arrays-nested",
            "arrays-objects",
            "arrays-primitive",
            "arrays-tabular",
            "delimiters",
            "objects",
            "objects-keyed",
            "primitives",
            "whitespace",
        ],
        "decode" => &[
            "arrays-nested",
            "arrays-primitive",
            "arrays-tabular",
            "blank-lines",
            "comments",
            "delimiters",
            "indentation-errors",
            "numbers",
            "objects",
            "objects-keyed",
            "primitives",
            "root-form",
            "validation-errors",
            "whitespace",
        ],
        _ => panic!("unknown fixture category: {category}"),
    }
}

fn sorted_entries(directory: &Path) -> Vec<DirEntry> {
    let mut entries: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap())
        .collect();
    entries.sort_by_key(DirEntry::file_name);
    entries
}

fn load_group(category: &str, group: DirEntry) -> Vec<FixtureCase> {
    assert!(
        group.file_type().unwrap().is_dir(),
        "unexpected fixture file: {:?}",
        group.path()
    );
    let group_name = group.file_name().into_string().unwrap();
    let entries = sorted_entries(&group.path());
    assert!(!entries.is_empty(), "empty fixture group: {group_name}");
    entries
        .into_iter()
        .map(|file| load_case(category, &group_name, file))
        .collect()
}

fn load_case(category: &str, group: &str, file: DirEntry) -> FixtureCase {
    let path = file.path();
    assert!(
        file.file_type().unwrap().is_file(),
        "unexpected fixture entry: {path:?}"
    );
    assert_eq!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("toon"),
        "unexpected fixture file: {path:?}"
    );
    let content = fs::read_to_string(&path).unwrap();
    parse_fixture(category, group, &path, &content)
}

fn parse_fixture(category: &str, group: &str, path: &Path, content: &str) -> FixtureCase {
    let (comments, toon) = content
        .split_once("\n# ---\n")
        .unwrap_or_else(|| panic!("missing fixture header terminator: {path:?}"));
    let mut header = String::with_capacity(comments.len());
    for line in comments.split('\n') {
        let line = line
            .strip_prefix("# ")
            .unwrap_or_else(|| panic!("malformed fixture header comment: {path:?}"));
        header.push_str(line);
        header.push('\n');
    }
    let metadata: Value = toon_format::decode_strict(&header)
        .unwrap_or_else(|error| panic!("invalid fixture metadata: {path:?}: {error:?}"));
    assert!(
        metadata.is_object(),
        "fixture metadata must be an object: {path:?}"
    );
    let fixture = &metadata["fixture"];
    assert!(fixture.is_object(), "missing fixture metadata: {path:?}");
    assert_eq!(
        fixture["version"], "4.0",
        "fixture baseline changed: {path:?}"
    );
    assert_eq!(
        fixture["category"], category,
        "fixture category changed: {path:?}"
    );
    if let Some(version) = metadata.get("minSpecVersion") {
        let (major, minor) = version.as_str().unwrap().split_once('.').unwrap();
        let required = (major.parse::<u32>().unwrap(), minor.parse::<u32>().unwrap());
        assert!(
            required <= PINNED_SPEC_VERSION,
            "{path:?} requires a newer specification: {version}"
        );
    }
    validate_model(category, path, &metadata);
    FixtureCase {
        group: group.to_owned(),
        metadata,
        toon: toon.to_owned(),
    }
}

fn validate_model(category: &str, path: &Path, metadata: &Value) {
    let model_field = match category {
        "encode" => "input-json",
        _ if metadata["shouldError"] == true && metadata.get("expected-json").is_none() => {
            return;
        }
        _ => "expected-json",
    };
    assert!(
        metadata[model_field].is_string(),
        "missing required {model_field} string: {path:?}"
    );
}

fn uses_other_indentation(options: &Value) -> bool {
    options["indentSize"].is_number() && options["indentSize"] != 2
}

fn uses_other_delimiter(options: &Value) -> bool {
    options["delimiter"].is_string() && options["delimiter"] != ","
}

#[test]
fn pinned_encode_profile() {
    let mut selected = 0;
    let mut excluded = 0;
    for fixture in load_fixtures("encode") {
        let file = &fixture.group;
        let case = &fixture.metadata;
        let options = &case["options"];
        if uses_other_indentation(options) || uses_other_delimiter(options) {
            excluded += 1;
            eprintln!(
                "EXCLUDED {} / {}: outside two-space comma profile",
                file, case["name"]
            );
            continue;
        }
        selected += 1;
        let input = case["input-json"].as_str().unwrap();
        let doc = crate::flatjson::parse_top_level_json(input.to_owned()).unwrap();
        let encoded = super::encode_document(&doc, super::EncodeOptions::default()).unwrap();
        assert_eq!(encoded, fixture.toon, "{file} / {}", case["name"]);
        let decoded = super::parse(&encoded).unwrap();
        let value: Value = serde_json::from_str(&decoded.1).unwrap();
        let expected: Value = serde_json::from_str(input).unwrap();
        assert!(
            equal_values(&value, &expected),
            "semantic round trip: {} / {}",
            file,
            case["name"]
        );
    }
    assert_eq!(
        (selected, excluded),
        (139, 21),
        "pinned encode profile counts"
    );
}

fn equal_values(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            if !a.is_f64() && !b.is_f64() {
                a == b
            } else {
                a.as_f64() == b.as_f64()
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| equal_values(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, value)| b.get(key).is_some_and(|other| equal_values(value, other)))
        }
        _ => a == b,
    }
}

#[test]
fn pinned_decode_profile() {
    let mut failures = Vec::new();
    let mut selected = 0;
    let mut excluded = 0;
    for fixture in load_fixtures("decode") {
        let file = &fixture.group;
        let case = &fixture.metadata;
        let options = &case["options"];
        if options["strict"] == false || uses_other_indentation(options) {
            excluded += 1;
            eprintln!(
                "EXCLUDED {} / {}: outside strict two-space profile",
                file, case["name"]
            );
            continue;
        }
        selected += 1;
        let actual = super::parse(&fixture.toon);
        let passes = if case["shouldError"] == true {
            actual.is_err()
        } else {
            let expected: Value =
                serde_json::from_str(case["expected-json"].as_str().unwrap()).unwrap();
            actual.as_ref().is_ok_and(|doc| {
                let value: Value = serde_json::from_str(&doc.1).unwrap();
                equal_values(&value, &expected)
            })
        };
        if !passes {
            failures.push(format!(
                "{} / {}: {:?}, expected {}",
                file,
                case["name"],
                actual.map(|d| d.1),
                case["expected-json"]
            ));
        }
    }
    assert_eq!(
        (selected, excluded),
        (348, 56),
        "pinned decode profile counts"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
