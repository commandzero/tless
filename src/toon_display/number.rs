//! Exact decimal normalization bounded by a display byte limit.
use super::WarningKind;

pub(super) fn number(raw: &str) -> (String, Option<WarningKind>) {
    if let Some(label) = non_finite_label(raw) {
        return (label.into(), Some(WarningKind::NonFiniteNumber));
    }

    lazy_static::lazy_static! {
        static ref DECIMAL: regex::Regex = regex::Regex::new(
            r"^(-?)(0|[1-9][0-9]*)(?:\.([0-9]+))?(?:[eE]([+-]?[0-9]+))?$"
        ).unwrap();
    }
    let Some(parts) = DECIMAL.captures(raw) else {
        return (raw.to_owned(), Some(WarningKind::NonCanonicalNumber));
    };
    if let Some(text) = normalize_decimal(&parts) {
        return (text, None);
    }

    let warning = parts
        .get(4)
        .is_none()
        .then_some(WarningKind::NonCanonicalNumber);
    (raw.to_owned(), warning)
}

fn non_finite_label(raw: &str) -> Option<&'static str> {
    match raw.to_ascii_lowercase().as_str() {
        ".inf" | "+.inf" => Some(".inf"),
        "-.inf" => Some("-.inf"),
        ".nan" | "+.nan" | "-.nan" => Some(".nan"),
        _ => None,
    }
}

fn normalize_decimal(parts: &regex::Captures<'_>) -> Option<String> {
    let integer = &parts[2];
    let fraction = parts.get(3).map_or("", |part| part.as_str());
    let digits = format!("{integer}{fraction}");
    let trimmed = digits.trim_start_matches('0');
    if trimmed.is_empty() {
        return Some("0".into());
    }

    let significant = trimmed.trim_end_matches('0');
    let leading = digits.len() - trimmed.len();
    let exponent = match parts.get(4) {
        Some(part) => part.as_str().parse::<i64>().ok()?,
        None => 0,
    };
    let point = (integer.len() as i64)
        .checked_add(exponent)?
        .checked_sub(leading as i64)?;
    let negative = &parts[1] == "-";
    let size = decimal_length(point, significant.len() as i64)?.checked_add(i64::from(negative))?;
    if size > 4096 {
        return None;
    }
    let mut text = String::with_capacity(size as usize);
    if negative {
        text.push('-');
    }
    write_decimal(&mut text, significant, point);
    Some(text)
}

fn decimal_length(point: i64, digits: i64) -> Option<i64> {
    if point <= 0 {
        return point.checked_neg()?.checked_add(2)?.checked_add(digits);
    }
    point.max(digits).checked_add(i64::from(point < digits))
}

fn write_decimal(text: &mut String, digits: &str, point: i64) {
    if point <= 0 {
        text.push_str("0.");
        text.extend(std::iter::repeat_n('0', (-point) as usize));
        text.push_str(digits);
    } else if point as usize >= digits.len() {
        text.push_str(digits);
        text.extend(std::iter::repeat_n('0', point as usize - digits.len()));
    } else {
        let (left, right) = digits.split_at(point as usize);
        text.push_str(left);
        text.push('.');
        text.push_str(right);
    }
}
