use serde_json::Value;

#[test]
fn pinned_encode_profile() {
    let fixtures = [
        (
            "arrays-nested",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/encode/arrays-nested.json"),
        ),
        (
            "arrays-objects",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/encode/arrays-objects.json"),
        ),
        (
            "arrays-primitive",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/encode/arrays-primitive.json"),
        ),
        (
            "arrays-tabular",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/encode/arrays-tabular.json"),
        ),
        (
            "delimiters",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/encode/delimiters.json"),
        ),
        (
            "objects",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/encode/objects.json"),
        ),
        (
            "objects-keyed",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/encode/objects-keyed.json"),
        ),
        (
            "primitives",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/encode/primitives.json"),
        ),
        (
            "whitespace",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/encode/whitespace.json"),
        ),
    ];
    for (file, json) in fixtures {
        let fixture: Value = serde_json::from_str(json).unwrap();
        for case in fixture["tests"].as_array().unwrap() {
            let options = &case["options"];
            if (options["indentSize"].is_number() && options["indentSize"] != 2)
                || (options["delimiter"].is_string() && options["delimiter"] != ",")
            {
                eprintln!(
                    "EXCLUDED {} / {}: outside two-space comma profile",
                    file, case["name"]
                );
                continue;
            }
            let doc = crate::flatjson::parse_top_level_json(case["input"].to_string()).unwrap();
            let encoded = super::encode_document(&doc, super::EncodeOptions::default()).unwrap();
            assert_eq!(
                encoded,
                case["expected"].as_str().unwrap(),
                "{file} / {}",
                case["name"]
            );
            let decoded = super::parse(&encoded).unwrap();
            let value: Value = serde_json::from_str(&decoded.1).unwrap();
            assert!(
                equal_values(&value, &case["input"]),
                "semantic round trip: {} / {}",
                file,
                case["name"]
            );
        }
    }
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
    let fixtures = [
        (
            "arrays-nested",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/decode/arrays-nested.json"),
        ),
        (
            "arrays-primitive",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/decode/arrays-primitive.json"),
        ),
        (
            "arrays-tabular",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/decode/arrays-tabular.json"),
        ),
        (
            "blank-lines",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/decode/blank-lines.json"),
        ),
        (
            "comments",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/decode/comments.json"),
        ),
        (
            "delimiters",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/decode/delimiters.json"),
        ),
        (
            "indentation-errors",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/decode/indentation-errors.json"),
        ),
        (
            "numbers",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/decode/numbers.json"),
        ),
        (
            "objects",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/decode/objects.json"),
        ),
        (
            "objects-keyed",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/decode/objects-keyed.json"),
        ),
        (
            "primitives",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/decode/primitives.json"),
        ),
        (
            "root-form",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/decode/root-form.json"),
        ),
        (
            "validation-errors",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/decode/validation-errors.json"),
        ),
        (
            "whitespace",
            include_str!("../tests/fixtures/toon-v4/tests/fixtures/decode/whitespace.json"),
        ),
    ];
    let mut failures = Vec::new();
    for (file, json) in fixtures {
        let fixture: Value = serde_json::from_str(json).unwrap();
        for case in fixture["tests"].as_array().unwrap() {
            let options = &case["options"];
            if options["strict"] == false
                || (options["indentSize"].is_number() && options["indentSize"] != 2)
            {
                eprintln!(
                    "EXCLUDED {} / {}: outside strict two-space profile",
                    file, case["name"]
                );
                continue;
            }
            let actual = super::parse(case["input"].as_str().unwrap());
            let passes = if case["shouldError"] == true {
                actual.is_err()
            } else {
                actual.as_ref().is_ok_and(|doc| {
                    let value: Value = serde_json::from_str(&doc.1).unwrap();
                    equal_values(&value, &case["expected"])
                })
            };
            if !passes {
                failures.push(format!(
                    "{} / {}: {:?}, expected {}",
                    file,
                    case["name"],
                    actual.map(|d| d.1),
                    case["expected"]
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
