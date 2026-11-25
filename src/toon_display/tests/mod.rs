use super::fixture::Fixture;
use super::*;
use std::collections::HashSet;

mod documents;
mod projection;
mod scalars;
mod search;
mod tables;
mod warnings;

fn json(input: &str) -> FlatJson {
    crate::flatjson::parse_top_level_json(input.into()).unwrap()
}
fn yaml(input: &str) -> FlatJson {
    crate::flatjson::parse_top_level_yaml(input.into()).unwrap()
}
fn text(flat: &FlatJson) -> String {
    Fixture::canonical(flat)
        .lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
