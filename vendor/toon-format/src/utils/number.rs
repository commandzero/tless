use crate::types::Number;

pub fn is_number_literal(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut pos = usize::from(bytes.first() == Some(&b'-'));
    if bytes.get(pos) == Some(&b'0') {
        pos += 1;
    } else {
        let start = pos;
        while bytes.get(pos).map_or(false, u8::is_ascii_digit) { pos += 1; }
        if pos == start { return false; }
    }
    if bytes.get(pos) == Some(&b'.') {
        pos += 1;
        let start = pos;
        while bytes.get(pos).map_or(false, u8::is_ascii_digit) { pos += 1; }
        if pos == start { return false; }
    }
    if matches!(bytes.get(pos), Some(b'e' | b'E')) {
        pos += 1;
        if matches!(bytes.get(pos), Some(b'+' | b'-')) { pos += 1; }
        let start = pos;
        while bytes.get(pos).map_or(false, u8::is_ascii_digit) { pos += 1; }
        if pos == start { return false; }
    }
    pos == bytes.len()
}

/// Compare decimal values before accepting a conversion through binary floats.
/// Bounds are checked before any exponent expansion or allocation.
pub fn checked_json_number(text: &str) -> crate::types::ToonResult<serde_json::Number> {
    let original = normalized_decimal(text).ok_or_else(|| {
        crate::types::ToonError::ProfileError { kind: crate::types::ProfileErrorKind::UnsupportedNumber, line: None, column: None, message: text.to_owned() }
    })?;
    let number = text.parse::<serde_json::Number>().map_err(|_| {
        crate::types::ToonError::ProfileError { kind: crate::types::ProfileErrorKind::UnsupportedNumber, line: None, column: None, message: text.to_owned() }
    })?;
    if normalized_decimal(&number.to_string()).as_ref() != Some(&original) {
        return Err(crate::types::ToonError::ProfileError { kind: crate::types::ProfileErrorKind::UnsupportedNumber, line: None, column: None, message: text.to_owned() });
    }
    if original.1.is_empty() {
        Ok(serde_json::Number::from(0))
    } else {
        Ok(number)
    }
}

pub fn normalized_decimal(text: &str) -> Option<(bool, String, i32)> {
    let (negative, unsigned) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let mut parts = unsigned.split(['e', 'E']);
    let mantissa = parts.next()?;
    let exponent = match parts.next() {
        Some(value) => value.parse::<i32>().ok()?,
        None => 0,
    };
    if parts.next().is_some() || !(-1024..=1024).contains(&exponent) {
        return None;
    }
    let mut pieces = mantissa.split('.');
    let integer = pieces.next()?;
    let fraction = pieces.next();
    if pieces.next().is_some() || integer.is_empty()
        || (integer.len() > 1 && integer.starts_with('0'))
        || !integer.bytes().all(|c| c.is_ascii_digit())
        || fraction.map_or(false, |s| s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit())) {
        return None;
    }
    let fraction = fraction.unwrap_or("");
    if integer.len() + fraction.len() > 1024 {
        return None;
    }
    let digits = format!("{integer}{fraction}");
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        return Some((false, String::new(), 0));
    }
    let significant = digits.trim_end_matches('0');
    Some((negative, significant.to_owned(), exponent - fraction.len() as i32 + (digits.len() - significant.len()) as i32))
}

/// Format a number in TOON canonical form (no exponents, no trailing zeros).
pub fn format_canonical_number(n: &Number) -> String {
    if let Some(i) = n.as_i64() {
        return i.to_string();
    }

    if let Some(u) = n.as_u64() {
        return u.to_string();
    }

    if let Some(f) = n.as_f64() {
        return format_f64_canonical(f);
    }

    n.to_string()
}

fn format_f64_canonical(f: f64) -> String {
    // Normalize integer-valued floats to integers
    if f.is_finite() && f.fract() == 0.0 && f.abs() <= i64::MAX as f64 {
        return format!("{}", f as i64);
    }

    let default_format = format!("{f}");

    // Handle cases where Rust would use exponential notation
    if default_format.contains('e') || default_format.contains('E') {
        format_without_exponent(f)
    } else {
        remove_trailing_zeros(&default_format)
    }
}

fn format_without_exponent(f: f64) -> String {
    if !f.is_finite() {
        return "0".to_string();
    }

    if f.abs() >= 1.0 {
        let abs_f = f.abs();
        let int_part = abs_f.trunc();
        let frac_part = abs_f.fract();

        if frac_part == 0.0 {
            format!("{}{}", if f < 0.0 { "-" } else { "" }, int_part as i64)
        } else {
            // High precision to avoid exponent, then trim trailing zeros
            let result = format!("{f:.17}");
            remove_trailing_zeros(&result)
        }
    } else if f == 0.0 {
        "0".to_string()
    } else {
        // Small numbers: use high precision to avoid exponent
        let result = format!("{f:.17}",);
        remove_trailing_zeros(&result)
    }
}

fn remove_trailing_zeros(s: &str) -> String {
    if !s.contains('.') {
        // No decimal point, return as-is
        return s.to_string();
    }

    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 2 {
        return s.to_string();
    }

    let int_part = parts[0];
    let mut frac_part = parts[1].to_string();

    frac_part = frac_part.trim_end_matches('0').to_string();

    if frac_part.is_empty() {
        // All zeros removed, return as integer
        int_part.to_string()
    } else {
        format!("{int_part}.{frac_part}")
    }
}

#[cfg(test)]
mod tests {
    use std::f64;

    use serde_json::json;

    use super::*;

    #[test]
    fn test_format_canonical_integers() {
        let n = Number::from(42i64);
        assert_eq!(format_canonical_number(&n), "42");

        let n = Number::from(-123i64);
        assert_eq!(format_canonical_number(&n), "-123");

        let n = Number::from(0i64);
        assert_eq!(format_canonical_number(&n), "0");
    }

    #[test]
    fn test_format_canonical_floats() {
        // Integer-valued floats
        let n = Number::from_f64(1.0).unwrap();
        assert_eq!(format_canonical_number(&n), "1");

        let n = Number::from_f64(42.0).unwrap();
        assert_eq!(format_canonical_number(&n), "42");

        // Non-integer floats
        let n = Number::from_f64(1.5).unwrap();
        assert_eq!(format_canonical_number(&n), "1.5");

        let n = Number::from_f64(f64::consts::PI).unwrap();
        let result = format_canonical_number(&n);
        assert!(result.starts_with("3.141592653589793"));
        assert!(!result.contains('e'));
        assert!(!result.contains('E'));
    }

    #[test]
    fn test_remove_trailing_zeros() {
        assert_eq!(remove_trailing_zeros("1.5000"), "1.5");
        assert_eq!(remove_trailing_zeros("1.0"), "1");
        assert_eq!(remove_trailing_zeros("1.500"), "1.5");
        assert_eq!(remove_trailing_zeros("42"), "42");
        assert_eq!(remove_trailing_zeros("0.0"), "0");
        assert_eq!(remove_trailing_zeros("1.23"), "1.23");
    }

    #[test]
    fn test_large_numbers_no_exponent() {
        // 1e6 should become 1000000
        let n = Number::from_f64(1_000_000.0).unwrap();
        let result = format_canonical_number(&n);
        assert_eq!(result, "1000000");
        assert!(!result.contains('e'));

        // 1e9
        let n = Number::from_f64(1_000_000_000.0).unwrap();
        let result = format_canonical_number(&n);
        assert_eq!(result, "1000000000");
        assert!(!result.contains('e'));
    }

    #[test]
    fn test_small_numbers_no_exponent() {
        // 1e-6 should become 0.000001
        let n = Number::from_f64(0.000001).unwrap();
        let result = format_canonical_number(&n);
        assert!(result.starts_with("0.000001"));
        assert!(!result.contains('e'));
        assert!(!result.contains('E'));

        // 1e-3
        let n = Number::from_f64(0.001).unwrap();
        let result = format_canonical_number(&n);
        assert_eq!(result, "0.001");
    }

    #[test]
    fn test_pi_formatting() {
        let n = Number::from_f64(std::f64::consts::PI).unwrap();
        let result = format_canonical_number(&n);

        // Should not have exponent
        assert!(!result.contains('e'));
        assert!(!result.contains('E'));

        // Should start with 3.14159...
        assert!(result.starts_with("3.14159"));
    }

    #[test]
    fn test_from_json_values() {
        // Test with actual JSON values
        let val = json!(1000000);
        if let Some(n) = val.as_i64() {
            let num = Number::from(n);
            assert_eq!(format_canonical_number(&num), "1000000");
        }

        let val = json!(1.5000);
        if let Some(f) = val.as_f64() {
            let num = Number::from_f64(f).unwrap();
            assert_eq!(format_canonical_number(&num), "1.5");
        }
    }

    #[test]
    fn test_negative_numbers() {
        let n = Number::from_f64(-1.5).unwrap();
        assert_eq!(format_canonical_number(&n), "-1.5");

        let n = Number::from(-42i64);
        assert_eq!(format_canonical_number(&n), "-42");

        let n = Number::from_f64(-1000000.0).unwrap();
        assert_eq!(format_canonical_number(&n), "-1000000");
    }
}
