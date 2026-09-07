use serde_json::Value;

#[test]
fn pinned_encode_profile() {
    let fixtures = [
        (
            "tests/fixtures/encode/arrays-nested.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/arrays-nested.json"),
        ),
        (
            "tests/fixtures/encode/arrays-objects.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/arrays-objects.json"),
        ),
        (
            "tests/fixtures/encode/arrays-primitive.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/arrays-primitive.json"),
        ),
        (
            "tests/fixtures/encode/arrays-tabular.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/arrays-tabular.json"),
        ),
        (
            "tests/fixtures/encode/delimiters.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/delimiters.json"),
        ),
        (
            "tests/fixtures/encode/key-folding.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/key-folding.json"),
        ),
        (
            "tests/fixtures/encode/objects.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/objects.json"),
        ),
        (
            "tests/fixtures/encode/primitives.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/primitives.json"),
        ),
        (
            "tests/fixtures/encode/whitespace.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/encode/whitespace.json"),
        ),
    ];
    let mut failures = Vec::new();
    for (file, json) in fixtures {
        let fixture: Value = serde_json::from_str(json).unwrap();
        for case in fixture["tests"].as_array().unwrap() {
            let options = &case["options"];
            if (options["indent"].is_number() && options["indent"] != 2)
                || (options["delimiter"].is_string() && options["delimiter"] != ",")
                || (options["keyFolding"].is_string() && options["keyFolding"] != "off")
            {
                eprintln!(
                    "EXCLUDED {} / {}: outside two-space comma no-folding profile",
                    file, case["name"]
                );
                continue;
            }
            let doc = crate::flatjson::parse_top_level_json(case["input"].to_string()).unwrap();
            let actual = super::encode_document(&doc, super::EncodeOptions::default());
            let expected = case["expected"].as_str().unwrap();
            if actual.as_ref().map(|s| s.as_str()).ok() != Some(expected) {
                failures.push(format!(
                    "{} / {}: {:?}, expected {:?}",
                    file, case["name"], actual, expected
                ));
            }
            // Published 0.5.0 decodes this out-of-u64 decimal token as a string.
            // Keep the fixture expectation intact and pin the known limitation.
            if file == "tests/fixtures/encode/primitives.json"
                && case["name"] == "encodes large number"
            {
                let decoded = super::parse(actual.as_ref().unwrap()).unwrap();
                let value: Value = serde_json::from_str(&decoded.1).unwrap();
                assert_eq!(value, serde_json::json!("100000000000000000000"));
                continue;
            }
            if let Ok(encoded) = actual {
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
    assert!(failures.is_empty(), "{}", failures.join("\n"));
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
            "tests/fixtures/decode/arrays-nested.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/decode/arrays-nested.json"),
        ),
        (
            "tests/fixtures/decode/arrays-primitive.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/decode/arrays-primitive.json"),
        ),
        (
            "tests/fixtures/decode/arrays-tabular.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/decode/arrays-tabular.json"),
        ),
        (
            "tests/fixtures/decode/blank-lines.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/decode/blank-lines.json"),
        ),
        (
            "tests/fixtures/decode/delimiters.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/decode/delimiters.json"),
        ),
        (
            "tests/fixtures/decode/indentation-errors.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/decode/indentation-errors.json"),
        ),
        (
            "tests/fixtures/decode/numbers.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/decode/numbers.json"),
        ),
        (
            "tests/fixtures/decode/objects.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/decode/objects.json"),
        ),
        (
            "tests/fixtures/decode/path-expansion.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/decode/path-expansion.json"),
        ),
        (
            "tests/fixtures/decode/primitives.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/decode/primitives.json"),
        ),
        (
            "tests/fixtures/decode/root-form.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/decode/root-form.json"),
        ),
        (
            "tests/fixtures/decode/validation-errors.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/decode/validation-errors.json"),
        ),
        (
            "tests/fixtures/decode/whitespace.json",
            include_str!("../tests/fixtures/toon-v3/tests/fixtures/decode/whitespace.json"),
        ),
    ];
    let mut failures = Vec::new();
    for (file, json) in fixtures {
        let fixture: Value = serde_json::from_str(json).unwrap();
        for case in fixture["tests"].as_array().unwrap() {
            let options = &case["options"];
            if options["strict"] == false
                || (options["indent"].is_number() && options["indent"] != 2)
                || (options["expandPaths"].is_string() && options["expandPaths"] != "off")
            {
                eprintln!(
                    "EXCLUDED {} / {}: outside strict two-space, no-expansion profile",
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
