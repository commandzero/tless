use crate::flatjson::{ContainerType, FlatJson, Value};

/// Return the non-closing rows that begin each original document.
pub fn document_roots(flat: &FlatJson) -> Vec<usize> {
    let mut roots = Vec::new();
    let mut node = 0;
    while node < flat.0.len() {
        roots.push(node);
        node = flat.subtree_range(node).end() + 1;
    }
    roots
}

/// A parsed path selection. Empty `tokens` means the original document roots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathFilter {
    tokens: Vec<Token>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Token {
    Key(String),
    /// A concrete bracket index, which requires an array.
    ArrayIndex(String),
    /// A concrete quoted bracket key, which requires an object.
    QuotedKey(String),
}

impl PathFilter {
    /// Parse either friendly dot/`yp` syntax or an RFC 6901 string pointer.
    pub fn parse(path: &str) -> Result<Self, String> {
        if path.is_empty() || path == "." {
            return Ok(Self { tokens: Vec::new() });
        }

        if path.starts_with('/') {
            return Self::parse_pointer(path);
        }
        if path.starts_with("./") {
            // `./` is the strict pointer spelling of the empty-string key. Only
            // the initial dot is removed; everything else is RFC 6901 input.
            return Self::parse_pointer(&path[1..]);
        }
        if path.starts_with('.') || path.starts_with('[') {
            return Self::parse_friendly(path);
        }

        Err(Self::syntax(
            "path must start with '.', '/', or a concrete bracket selector",
        ))
    }

    /// Resolve this path against every original document root, atomically.
    pub fn resolve(&self, flat: &FlatJson) -> Result<Vec<usize>, String> {
        let roots = document_roots(flat);
        if self.tokens.is_empty() {
            return Ok(roots);
        }
        if roots.is_empty() {
            return Err("path cannot resolve against an empty input (no documents)".to_owned());
        }

        let mut selected = Vec::with_capacity(roots.len());
        for (document, root) in roots.into_iter().enumerate() {
            let mut node = root;
            for token in &self.tokens {
                node = resolve_token(flat, node, token)
                    .map_err(|reason| format!("document {}: {reason}", document + 1))?;
            }
            selected.push(node);
        }
        Ok(selected)
    }

    fn parse_pointer(path: &str) -> Result<Self, String> {
        debug_assert!(path.starts_with('/'));
        let mut tokens = Vec::new();
        for raw in path[1..].split('/') {
            let decoded = decode_pointer_token(raw)?;
            tokens.push(Token::Key(decoded));
        }
        Ok(Self { tokens })
    }

    fn parse_friendly(path: &str) -> Result<Self, String> {
        let mut offset = usize::from(path.starts_with('.'));
        let mut tokens = Vec::new();
        while offset < path.len() {
            let (token, end) = match path.as_bytes()[offset] {
                b'[' => parse_bracket(path, offset)?,
                b'.' => return Err(Self::syntax("empty dot token")),
                _ => parse_dot_key(path, offset)?,
            };
            tokens.push(token);
            offset = next_component(path, end)?;
        }
        Ok(Self { tokens })
    }

    fn syntax(reason: &str) -> String {
        format!("invalid path syntax: {reason}")
    }
}

fn parse_dot_key(path: &str, start: usize) -> Result<(Token, usize), String> {
    let remaining = &path[start..];
    let length = remaining.find(['.', '[']).unwrap_or(remaining.len());
    let key = &remaining[..length];
    for (offset, ch) in key.char_indices() {
        if matches!(ch, ']' | '/' | '\\') || ch.is_whitespace() {
            return Err(PathFilter::syntax(&format!(
                "invalid character in dot token at byte {}",
                start + offset
            )));
        }
    }
    Ok((Token::Key(key.to_owned()), start + length))
}

fn next_component(path: &str, end: usize) -> Result<usize, String> {
    match path.as_bytes().get(end) {
        None | Some(b'[') => Ok(end),
        Some(b'.') if end + 1 < path.len() => Ok(end + 1),
        Some(b'.') => Err(PathFilter::syntax("a trailing '.' is not a path token")),
        _ => Err(PathFilter::syntax(&format!(
            "unexpected character at byte {end}"
        ))),
    }
}

fn parse_bracket(path: &str, start: usize) -> Result<(Token, usize), String> {
    let content = &path[start + 1..];
    if content.starts_with('"') {
        return parse_quoted_key(content, start + 1);
    }

    let end = content
        .find(']')
        .ok_or_else(|| PathFilter::syntax("unterminated bracket selector"))?;
    let text = &content[..end];
    if text.is_empty() {
        return Err(PathFilter::syntax("empty bracket selector"));
    }
    if !valid_index_text(text) {
        return Err(PathFilter::syntax(
            "bracket selector must be a zero-based decimal index",
        ));
    }
    Ok((Token::ArrayIndex(text.to_owned()), start + end + 2))
}

fn parse_quoted_key(content: &str, start: usize) -> Result<(Token, usize), String> {
    let mut escaped = false;
    let closing_quote = content
        .bytes()
        .enumerate()
        .skip(1)
        .find_map(|(offset, byte)| {
            if escaped {
                escaped = false;
                return None;
            }
            escaped = byte == b'\\';
            (byte == b'"').then_some(offset)
        })
        .ok_or_else(|| PathFilter::syntax("unterminated JSON string in bracket selector"))?;
    if content.as_bytes().get(closing_quote + 1) != Some(&b']') {
        return Err(PathFilter::syntax(
            "quoted bracket selector must end with ]",
        ));
    }
    let key = serde_json::from_str::<String>(&content[..=closing_quote])
        .map_err(|error| PathFilter::syntax(&format!("invalid quoted key: {error}")))?;
    Ok((Token::QuotedKey(key), start + closing_quote + 2))
}

fn decode_pointer_token(raw: &str) -> Result<String, String> {
    let mut decoded = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch != '~' {
            decoded.push(ch);
            continue;
        }
        match chars.next() {
            Some('0') => decoded.push('~'),
            Some('1') => decoded.push('/'),
            Some(other) => {
                return Err(PathFilter::syntax(&format!(
                    "invalid RFC 6901 escape {other:?}"
                )));
            }
            None => return Err(PathFilter::syntax("trailing '~' in RFC 6901 token")),
        }
    }
    Ok(decoded)
}

fn valid_index_text(text: &str) -> bool {
    match text.as_bytes() {
        [b'0'] => true,
        [b'1'..=b'9', tail @ ..] => tail.iter().all(u8::is_ascii_digit),
        _ => false,
    }
}

fn resolve_token(flat: &FlatJson, node: usize, token: &Token) -> Result<usize, String> {
    let container = match flat[node].value {
        Value::OpenContainer { container_type, .. } => Some(container_type),
        Value::EmptyArray => Some(ContainerType::Array),
        Value::EmptyObject => Some(ContainerType::Object),
        _ => None,
    };

    match (container, token) {
        (Some(ContainerType::Array), Token::Key(text) | Token::ArrayIndex(text)) => {
            let index = checked_index(text)?;
            child_at_index(flat, node, index)
                .ok_or_else(|| format!("array index {text:?} is out of range"))
        }
        (Some(ContainerType::Object), Token::Key(key) | Token::QuotedKey(key)) => {
            child_for_key(flat, node, key)
        }
        (Some(ContainerType::Object), Token::ArrayIndex(text)) => {
            Err(format!("array index selector {text:?} requires an array"))
        }
        (Some(ContainerType::Array), Token::QuotedKey(key)) => {
            Err(format!("quoted key selector {key:?} requires an object"))
        }
        (None, Token::Key(text) | Token::ArrayIndex(text) | Token::QuotedKey(text)) => {
            Err(format!("cannot traverse scalar with token {text:?}"))
        }
    }
}

fn checked_index(text: &str) -> Result<usize, String> {
    if !valid_index_text(text) {
        return Err(format!("invalid array index {text:?}"));
    }
    text.parse::<usize>()
        .map_err(|_| format!("invalid array index {text:?}: value overflows usize"))
}

fn child_at_index(flat: &FlatJson, node: usize, index: usize) -> Option<usize> {
    let mut child = flat[node].first_child();
    while let Some(child_index) = child.as_option() {
        if flat[child_index].index_in_parent == index {
            return Some(child_index);
        }
        child = flat[child_index].next_sibling;
    }
    None
}

fn child_for_key(flat: &FlatJson, node: usize, key: &str) -> Result<usize, String> {
    let children = std::iter::successors(flat[node].first_child().as_option(), |&child| {
        flat[child].next_sibling.as_option()
    });
    let mut matched = None;
    for child in children {
        if flat.decoded_string_key(child)?.as_deref() != Some(key) {
            continue;
        }
        if matched.replace(child).is_some() {
            return Err(format!("ambiguous member {key:?}: duplicate key"));
        }
    }
    matched.ok_or_else(|| format!("missing member {key:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flatjson::{parse_top_level_json, parse_top_level_yaml};

    fn resolve(json: &str, path: &str) -> Vec<usize> {
        let flat = parse_top_level_json(json.to_owned()).unwrap();
        PathFilter::parse(path).unwrap().resolve(&flat).unwrap()
    }

    #[test]
    fn parses_both_syntaxes_without_fallback() {
        assert!(PathFilter::parse(".a.b").is_ok());
        assert!(PathFilter::parse("./a.b").is_ok());
        assert!(PathFilter::parse("/a~1b").is_ok());
        assert!(PathFilter::parse("[0].name").is_ok());
        for path in [".a..b", ".a.", ".a/b", ".a[]", ".a[0:2]", "./a~2b", "#/a"] {
            assert!(PathFilter::parse(path).is_err(), "{path}");
        }
    }

    #[test]
    fn strict_escapes_and_empty_key_are_distinct_from_root() {
        let flat =
            parse_top_level_json(r#"{"":7,"a.b":1,"a/b":2,"m~n":3,"~1":4}"#.to_owned()).unwrap();
        assert_eq!(
            PathFilter::parse(".").unwrap().resolve(&flat).unwrap(),
            vec![0]
        );
        assert_eq!(
            PathFilter::parse("./").unwrap().resolve(&flat).unwrap(),
            vec![1]
        );
        assert_eq!(
            PathFilter::parse("./a.b").unwrap().resolve(&flat).unwrap(),
            vec![2]
        );
        assert_eq!(
            PathFilter::parse("/a~1b").unwrap().resolve(&flat).unwrap(),
            vec![3]
        );
        assert_eq!(
            PathFilter::parse("/m~0n").unwrap().resolve(&flat).unwrap(),
            vec![4]
        );
        assert_eq!(
            PathFilter::parse("/~01").unwrap().resolve(&flat).unwrap(),
            vec![5]
        );
    }

    #[test]
    fn typed_brackets_and_numeric_object_keys() {
        let flat = parse_top_level_json(r#"{"01":8,"arr":[10,{"x":11}]}"#.to_owned()).unwrap();
        assert_eq!(resolve(r#"{"01":8,"arr":[10,{"x":11}]}"#, ".01"), vec![1]);
        assert_eq!(
            PathFilter::parse(r#"["arr"][1]["x"]"#)
                .unwrap()
                .resolve(&flat)
                .unwrap(),
            vec![5]
        );
        assert!(PathFilter::parse(r#"["arr"][01]"#).is_err());
        assert!(
            PathFilter::parse(r#"["arr"]["1"]"#)
                .unwrap()
                .resolve(&flat)
                .is_err()
        );
    }

    #[test]
    fn duplicate_matching_keys_and_streams_are_atomic() {
        let duplicate = parse_top_level_json(r#"{"a":1,"a":2,"b":3}"#.to_owned()).unwrap();
        let error = PathFilter::parse(".a")
            .unwrap()
            .resolve(&duplicate)
            .unwrap_err();
        assert!(error.contains("ambiguous") && error.contains("document 1"));

        let stream = parse_top_level_json(r#"{"a":1} {"a":2}"#.to_owned()).unwrap();
        assert_eq!(
            PathFilter::parse(".a").unwrap().resolve(&stream).unwrap(),
            vec![1, 4]
        );
        let missing = parse_top_level_json(r#"{"a":1} {"b":2}"#.to_owned()).unwrap();
        assert!(PathFilter::parse(".a").unwrap().resolve(&missing).is_err());
    }

    #[test]
    fn copied_yp_paths_select_original_nodes_through_both_entry_points() {
        use crate::{command::Command, flatjson::PathType, options::Opt, viewer::JsonViewer};
        use clap::Parser;

        for input in [
            r#"{"hits":[{"name":"Ada","a.b":[{"":"empty","quote\"\\\n\t😀":42}]}],"outside":0}"#,
            r#"[{"name":"Ada","a/b~c":1},[{"name":"Lin"}]]"#,
            r#"{"a.b":[{"name":"Ada"}],"0":"numeric object key"}"#,
        ] {
            let flat = parse_top_level_json(input.into()).unwrap();
            let mut viewer = JsonViewer::new(flat);
            for node in 0..viewer.flatjson.0.len() {
                if viewer.flatjson[node].is_closing_of_container() {
                    continue;
                }
                viewer.set_roots(vec![node]);
                let copied = viewer
                    .flatjson
                    .build_path_to_node(PathType::Dot, node)
                    .unwrap();
                let Command::Path(interactive) = Command::parse(&copied) else {
                    panic!("copied path was not accepted by the command prompt: {copied:?}");
                };
                let cli = Opt::try_parse_from(["tless", "--path", &copied]).unwrap();
                for path in [&interactive, cli.path.as_ref().unwrap()] {
                    assert_eq!(
                        PathFilter::parse(path)
                            .unwrap()
                            .resolve(&viewer.flatjson)
                            .unwrap(),
                        vec![node],
                        "copied path {copied:?}",
                    );
                }
            }
        }
    }

    #[test]
    fn yaml_typed_keys_are_not_coerced() {
        let flat = parse_top_level_yaml("---\n1: number\n\"1\": string\n".to_owned()).unwrap();
        assert_eq!(
            PathFilter::parse(".1")
                .unwrap()
                .resolve(&flat)
                .unwrap()
                .len(),
            1
        );
        assert!(
            PathFilter::parse(r#"["1"]"#)
                .unwrap()
                .resolve(&flat)
                .is_ok()
        );
    }
}
