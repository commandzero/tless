use super::*;
use crate::flatjson::{PathType, parse_top_level_json, parse_top_level_yaml};
use unicode_width::UnicodeWidthStr;

mod alignment;
mod arrays;
mod navigation;
mod projection_cost;
mod roots;
mod sequence;
mod tables;
mod wrapping;

fn viewer(input: &str) -> JsonViewer {
    JsonViewer::new(parse_top_level_json(input.into()).unwrap())
}

fn path(v: &JsonViewer) -> String {
    v.flatjson
        .build_path_to_node(PathType::Dot, v.focused_node)
        .unwrap()
}

fn act(v: &mut JsonViewer, actions: &[Action]) {
    for &action in actions {
        v.perform_action(action);
    }
}
