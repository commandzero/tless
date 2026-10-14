use serde_json::Value;
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
    let expected_groups: &[&str] = match category {
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
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/toon-v4/v4.4.0/tests/fixtures")
        .join(category);
    let mut groups: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap())
        .collect();
    groups.sort_by_key(|entry| entry.file_name());
    let group_names: Vec<_> = groups
        .iter()
        .map(|entry| entry.file_name().into_string().unwrap())
        .collect();
    assert_eq!(
        group_names, expected_groups,
        "unexpected fixture groups: {root:?}"
    );

    let mut fixtures = Vec::new();
    for group in groups {
        assert!(
            group.file_type().unwrap().is_dir(),
            "unexpected fixture file: {:?}",
            group.path()
        );
        let group_name = group.file_name().into_string().unwrap();
        let mut files: Vec<_> = std::fs::read_dir(group.path())
            .unwrap()
            .map(|entry| entry.unwrap())
            .collect();
        files.sort_by_key(|entry| entry.file_name());
        assert!(!files.is_empty(), "empty fixture group: {group_name}");
        for file in files {
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
            let content = std::fs::read_to_string(&path).unwrap();
            fixtures.push(parse_fixture(category, &group_name, &path, &content));
        }
    }
    fixtures
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
    let model_field = if category == "encode" {
        "input-json"
    } else {
        "expected-json"
    };
    if category == "encode"
        || metadata["shouldError"] != true
        || metadata.get(model_field).is_some()
    {
        assert!(
            metadata[model_field].is_string(),
            "missing required {model_field} string: {path:?}"
        );
    }
    FixtureCase {
        group: group.to_owned(),
        metadata,
        toon: toon.to_owned(),
    }
}

#[test]
fn pinned_encode_profile() {
    let mut selected = 0;
    let mut excluded = 0;
    for fixture in load_fixtures("encode") {
        let file = &fixture.group;
        let case = &fixture.metadata;
        let options = &case["options"];
        if (options["indentSize"].is_number() && options["indentSize"] != 2)
            || (options["delimiter"].is_string() && options["delimiter"] != ",")
        {
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
        if options["strict"] == false
            || (options["indentSize"].is_number() && options["indentSize"] != 2)
        {
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
