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
            // jless's order-preserving profile intentionally differs from §9.3.
            // Keep the upstream fixture intact and exercise its input with our
            // documented list-form expectation instead of excluding the case.
            let expected = if file == "tests/fixtures/encode/arrays-objects.json"
                && case["name"] == "uses field order from first object for tabular headers"
            {
                "items[2]:\n  - a: 1\n    b: 2\n    c: 3\n  - c: 30\n    b: 20\n    a: 10"
            } else {
                case["expected"].as_str().unwrap()
            };
            if actual.as_ref().map(|s| s.as_str()).ok() != Some(expected) {
                failures.push(format!(
                    "{} / {}: {:?}, expected {:?}",
                    file, case["name"], actual, expected
                ));
            }
            if let Ok(encoded) = actual {
                let decoded = super::parse(&encoded).unwrap();
                let value: Value = serde_json::from_str(&decoded.1).unwrap();
                assert!(
                    equal_values(&value, &case["input"]),
                    "ordered round trip: {} / {}",
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
            toon_format::utils::number::normalized_decimal(&a.to_string())
                == toon_format::utils::number::normalized_decimal(&b.to_string())
        }
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| equal_values(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|((ak, av), (bk, bv))| ak == bk && equal_values(av, bv))
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
                actual.as_ref().map_or(false, |doc| {
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
