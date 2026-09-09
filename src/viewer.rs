use crate::flatjson::{FlatJson, Index, OptionIndex};
use crate::toon_display::{normalize_node, Layout, VisibleLine};
use crate::types::TTYDimensions;
#[cfg(test)]
use unicode_width::UnicodeWidthStr;

/// Parsed node focus is independent of its painted line (inline values and table cells).
pub struct JsonViewer {
    pub flatjson: FlatJson,
    pub layout: Layout,
    pub visible: Vec<VisibleLine>,
    /// Index into the visibility projection, not the parsed node list.
    pub top_row: Index,
    pub focused_row: Index,
    pub anchor_line: usize,
    jump_distance: Option<usize>,
    desired_depth: usize,
    pub dimensions: TTYDimensions,
    pub scrolloff_setting: u16,
}

impl JsonViewer {
    pub fn new(flatjson: FlatJson) -> Self {
        let layout = Layout::new(&flatjson);
        let visible = layout.project(&flatjson);
        let anchor_line = layout.nodes[0].line;
        Self {
            flatjson,
            layout,
            visible,
            top_row: 0,
            focused_row: 0,
            anchor_line,
            jump_distance: None,
            desired_depth: 0,
            dimensions: TTYDimensions::default(),
            scrolloff_setting: 3,
        }
    }

    pub fn focused_line_index(&self) -> usize {
        self.visible
            .iter()
            .position(|v| v.absolute == self.anchor_line)
            .unwrap_or(0)
    }

    #[cfg(test)]
    pub fn index_of_focused_row_on_screen(&self) -> u16 {
        self.focused_line_index().saturating_sub(self.top_row) as u16
    }

    fn focus(&mut self, node: usize) {
        self.focused_row = normalize_node(&self.flatjson, node);
        self.focused_row = self.flatjson.first_visible_ancestor(self.focused_row);
        self.anchor_line = self.layout.nodes[self.focused_row].line;
    }

    fn reveal(&mut self, node: usize, source: Option<usize>) {
        let node = normalize_node(&self.flatjson, node);
        let mut current = node;
        let mut changed = false;
        while let OptionIndex::Index(parent) = self.flatjson[current].parent {
            changed |= self.flatjson[parent].is_collapsed();
            self.flatjson.expand(parent);
            current = parent;
        }
        if changed {
            self.refresh_projection();
        }
        self.focus(node);
        if let Some(source) = source {
            if let Some((line, _)) = self.layout.lines.iter().enumerate().find(|(_, line)| {
                line.spans.iter().any(|span| {
                    span.node == node
                        && span
                            .source
                            .as_ref()
                            .is_some_and(|range| range.contains(&source))
                })
            }) {
                self.anchor_line = line;
            }
        }
    }

    fn refresh_projection(&mut self) {
        self.visible = self.layout.project(&self.flatjson);
        let recovered = self.flatjson.first_visible_ancestor(self.focused_row);
        if recovered != self.focused_row {
            self.focus(recovered);
        }
        self.top_row = self.top_row.min(self.visible.len().saturating_sub(1));
    }

    fn ensure_visible(&mut self) {
        let index = self.focused_line_index();
        let height = usize::from(self.dimensions.height).max(1);
        let padding = usize::from(self.scrolloff_setting).min((height - 1) / 2);
        if index < self.top_row + padding {
            self.top_row = index.saturating_sub(padding);
        } else if index >= self.top_row + height - padding {
            self.top_row = (index + padding + 1)
                .saturating_sub(height)
                .min(self.visible.len().saturating_sub(height));
        }
    }

    fn vertical(&mut self, count: usize, down: bool) {
        let mut index = self.focused_line_index();
        for _ in 0..count {
            let next = if down {
                ((index + 1)..self.visible.len()).find(|&i| !self.visible[i].line.separator)
            } else {
                (0..index).rev().find(|&i| !self.visible[i].line.separator)
            };
            let Some(next) = next else { break };
            index = next;
        }
        self.focus_line(index, true);
    }

    fn focus_line(&mut self, index: usize, retain_field: bool) {
        let owner = self.visible[index].line.owner;
        let mut node = owner;
        if retain_field && self.flatjson[self.focused_row].key_range.is_some() {
            // Match the logical column ordinal, including escaped-equivalent keys.
            if let OptionIndex::Index(parent) = self.flatjson[self.focused_row].parent {
                if self.layout.nodes[parent].line == self.layout.nodes[self.focused_row].line
                    && self.flatjson[owner].is_expanded()
                {
                    let mut ordinal = 0;
                    let mut child = self.flatjson[parent].first_child();
                    while let OptionIndex::Index(candidate) = child {
                        if candidate == self.focused_row {
                            break;
                        }
                        ordinal += 1;
                        child = self.flatjson[candidate].next_sibling;
                    }
                    child = self.flatjson[owner].first_child();
                    for _ in 0..ordinal {
                        if let OptionIndex::Index(candidate) = child {
                            child = self.flatjson[candidate].next_sibling;
                        }
                    }
                    if let OptionIndex::Index(candidate) = child {
                        if self.layout.nodes[candidate].line == self.visible[index].absolute {
                            node = candidate;
                        }
                    }
                }
            }
        }
        self.focus(node);
    }

    fn parent(&mut self) {
        if let OptionIndex::Index(parent) = self.flatjson[self.focused_row].parent {
            self.focus(parent);
        }
    }

    fn sibling(&mut self, count: usize, next: bool) {
        for _ in 0..count {
            let before = self.focused_row;
            let sibling = if next {
                self.flatjson[before].next_sibling
            } else {
                self.flatjson[before].prev_sibling
            };
            if let OptionIndex::Index(mut node) = sibling {
                while self.flatjson[node].depth < self.desired_depth
                    && self.flatjson[node].is_expanded()
                {
                    let child = if next {
                        self.flatjson[node].first_child()
                    } else if let OptionIndex::Index(pair) = self.flatjson[node].pair_index() {
                        self.flatjson[pair].last_child()
                    } else {
                        OptionIndex::Nil
                    };
                    if let OptionIndex::Index(child) = child {
                        node = child;
                    } else {
                        break;
                    }
                }
                self.focus(node);
            } else {
                self.parent();
            }
            if self.focused_row == before {
                break;
            }
        }
    }

    fn collapse(&mut self, node: usize, collapsed: bool) {
        if self.layout.nodes[node].collapsible {
            if collapsed {
                self.flatjson.collapse(node);
            } else {
                self.flatjson.expand(node);
            }
        }
    }

    fn collapse_siblings(&mut self, collapsed: bool, deep: bool) {
        let mut node = match self.flatjson[self.focused_row].parent {
            OptionIndex::Index(parent) => self.flatjson[parent].first_child(),
            OptionIndex::Nil => OptionIndex::Index(0),
        };
        while let OptionIndex::Index(current) = node {
            self.collapse(current, collapsed);
            if deep {
                if let OptionIndex::Index(end) = self.flatjson[current].pair_index() {
                    for descendant in current + 1..end {
                        self.collapse(descendant, collapsed);
                    }
                }
            }
            node = self.flatjson[current].next_sibling;
        }
        self.refresh_projection();
    }

    fn scroll(&mut self, count: usize, down: bool) {
        self.top_row = if down {
            self.top_row
                .saturating_add(count)
                .min(self.visible.len() - 1)
        } else {
            self.top_row.saturating_sub(count)
        };
        let height = usize::from(self.dimensions.height).max(1);
        let focus = self.focused_line_index();
        let index = focus.clamp(
            self.top_row,
            (self.top_row + height - 1).min(self.visible.len() - 1),
        );
        let index = (index..self.visible.len())
            .find(|&i| !self.visible[i].line.separator)
            .unwrap_or(index);
        if index != focus {
            self.focus_line(index, true);
        }
    }

    pub fn perform_action(&mut self, action: Action) {
        let mut track = true;
        match action {
            Action::NoOp => {}
            Action::MoveUp(n) => self.vertical(n, false),
            Action::MoveDown(n) => self.vertical(n, true),
            Action::MoveRight => {
                if self.flatjson[self.focused_row].is_collapsed() {
                    self.collapse(self.focused_row, false);
                    self.refresh_projection();
                } else if let OptionIndex::Index(child) =
                    self.flatjson[self.focused_row].first_child()
                {
                    self.focus(child);
                }
            }
            Action::MoveLeft => {
                if self.layout.nodes[self.focused_row].collapsible
                    && self.flatjson[self.focused_row].is_expanded()
                {
                    self.collapse(self.focused_row, true);
                    self.refresh_projection();
                } else {
                    self.parent();
                }
            }
            Action::FocusParent => self.parent(),
            Action::FocusPrevSibling(n) => self.sibling(n, false),
            Action::FocusNextSibling(n) => self.sibling(n, true),
            Action::FocusFirstSibling | Action::FocusLastSibling => {
                let last = matches!(action, Action::FocusLastSibling);
                let mut node = match self.flatjson[self.focused_row].parent {
                    OptionIndex::Index(parent) => self.flatjson[parent].first_child().unwrap(),
                    OptionIndex::Nil => 0,
                };
                if last {
                    if let OptionIndex::Index(parent) = self.flatjson[node].parent {
                        node = self.flatjson[self.flatjson[parent].pair_index().unwrap()]
                            .last_child()
                            .unwrap();
                    } else {
                        while let OptionIndex::Index(next) = self.flatjson[node].next_sibling {
                            node = next;
                        }
                    }
                }
                self.focus(node);
            }
            Action::FocusTop => {
                self.focus(0);
                self.top_row = 0;
            }
            Action::FocusBottom => {
                self.focus_line(self.visible.len() - 1, false);
            }
            Action::MoveUpUntilDepthChange | Action::MoveDownUntilDepthChange => {
                let down = matches!(action, Action::MoveDownUntilDepthChange);
                let depth = self.flatjson[self.focused_row].depth;
                loop {
                    let before = self.focused_line_index();
                    self.vertical(1, down);
                    if before == self.focused_line_index()
                        || self.flatjson[self.focused_row].depth != depth
                    {
                        break;
                    }
                }
            }
            Action::JumpTo { line, make_visible } => {
                let line = line.min(self.layout.lines.len() - 1);
                let node = self.layout.lines[line].owner;
                if make_visible {
                    self.reveal(node, None);
                } else {
                    self.focus(node);
                }
            }
            Action::FocusNode { node, source } => self.reveal(node, source),
            Action::ScrollUp(n) => {
                self.scroll(n, false);
                track = false;
            }
            Action::ScrollDown(n) => {
                self.scroll(n, true);
                track = false;
            }
            Action::PageUp(n) => {
                self.scroll(usize::from(self.dimensions.height).saturating_mul(n), false);
                track = false;
            }
            Action::PageDown(n) => {
                self.scroll(usize::from(self.dimensions.height).saturating_mul(n), true);
                track = false;
            }
            Action::JumpUp(n) | Action::JumpDown(n) => {
                self.jump_distance = n.or(self.jump_distance);
                let distance = self
                    .jump_distance
                    .unwrap_or((usize::from(self.dimensions.height) / 2).max(1));
                let down = matches!(action, Action::JumpDown(_));
                self.vertical(distance, down);
                self.scroll(distance, down);
                track = false;
            }
            Action::MoveFocusedLineToTop
            | Action::MoveFocusedLineToCenter
            | Action::MoveFocusedLineToBottom => {
                let height = usize::from(self.dimensions.height).max(1);
                let padding = match action {
                    Action::MoveFocusedLineToTop => {
                        usize::from(self.scrolloff_setting).min((height - 1) / 2)
                    }
                    Action::MoveFocusedLineToCenter => height / 2,
                    _ => height - 1,
                };
                self.top_row = self.focused_line_index().saturating_sub(padding);
                track = false;
            }
            Action::ClickArrow(row) => {
                let index =
                    (self.top_row + usize::from(row.saturating_sub(1))).min(self.visible.len() - 1);
                let node = self.visible[index].line.owner;
                self.focus(node);
                self.collapse(node, !self.flatjson[node].is_collapsed());
                self.refresh_projection();
            }
            Action::ToggleCollapsed => {
                self.collapse(
                    self.focused_row,
                    !self.flatjson[self.focused_row].is_collapsed(),
                );
                self.refresh_projection();
            }
            Action::CollapseNodeAndSiblings => self.collapse_siblings(true, false),
            Action::DeepCollapseNodeAndSiblings => self.collapse_siblings(true, true),
            Action::ExpandNodeAndSiblings => self.collapse_siblings(false, false),
            Action::DeepExpandNodeAndSiblings => self.collapse_siblings(false, true),
            Action::ResizeViewerDimensions(dimensions) => self.dimensions = dimensions,
        }
        if !matches!(
            action,
            Action::FocusPrevSibling(_)
                | Action::FocusNextSibling(_)
                | Action::NoOp
                | Action::ScrollUp(_)
                | Action::ScrollDown(_)
                | Action::MoveFocusedLineToTop
                | Action::MoveFocusedLineToCenter
                | Action::MoveFocusedLineToBottom
                | Action::ResizeViewerDimensions(_)
        ) {
            self.desired_depth = self.flatjson[self.focused_row].depth;
        }
        if track {
            self.ensure_visible();
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Action {
    // Does nothing, for debugging, shouldn't modify any state.
    #[allow(dead_code)]
    NoOp,

    MoveUp(usize),
    MoveDown(usize),
    MoveLeft,
    MoveRight,

    // TODO: Come up with better names for these. Their behavior is
    // a little subtle. When moving down it'll move forward until
    // the depth changes. If the depth increases (because it got to
    // an expanded container) it'll stop on the line of the opening
    // of the container, but if the depth decreases (because we moved
    // past the last child of the current container) it'll focus the
    // line after.
    MoveUpUntilDepthChange,
    MoveDownUntilDepthChange,

    FocusParent,

    // The behavior of these is subtle and stateful. These move to the
    // previous/next sibling of the focused element. If we are focused
    // on the first/last child, we will move to the parent, but we
    // will remember what depth we were at when we first performed
    // this action, and move back to that depth the next time we can.
    FocusPrevSibling(usize),
    FocusNextSibling(usize),

    FocusFirstSibling,
    FocusLastSibling,
    FocusTop,
    FocusBottom,

    ScrollUp(usize),
    ScrollDown(usize),

    // By default, these move by half a screen, and move the focus by
    // the same number of lines, so the focus doesn't appear to move
    // on the screen. When jumping down, it will not show lines past
    // the end of the file.
    //
    // When a count is provided, we'll move by that many *lines* (not
    // N half screen sizes). This count is stored in
    // JsonViewer.jump_distance and used for subsequent jumps, rather
    // than half a screen size.
    //
    // vim always moves both the viewing window and the focused line
    // by the appropriate lines, so the location of the focused line
    // on the screen will move when jumping past the end of the file
    // (or before the start).
    //
    // We'll implement a slight variation on this behavior. If the
    // viewing window moves, we'll keep the focused line in the same
    // vertical location, but once we're at the top of the file, and
    // the viewing window doesn't change at all, then we will change
    // the focused line by the expected count.
    //
    // These commands ignore the scrolloff option.
    JumpUp(Option<usize>),
    JumpDown(Option<usize>),

    JumpTo {
        line: Index,
        make_visible: bool,
    },

    FocusNode {
        node: Index,
        source: Option<usize>,
    },

    PageUp(usize),
    PageDown(usize),

    MoveFocusedLineToTop,
    MoveFocusedLineToCenter,
    MoveFocusedLineToBottom,

    ClickArrow(u16),

    ToggleCollapsed,
    CollapseNodeAndSiblings,
    DeepCollapseNodeAndSiblings,
    ExpandNodeAndSiblings,
    DeepExpandNodeAndSiblings,

    ResizeViewerDimensions(TTYDimensions),
}

#[cfg(test)]
impl OptionIndex {
    pub fn as_usize(&self) -> usize {
        match self {
            Self::Nil => crate::flatjson::NIL,
            Self::Index(i) => *i,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flatjson::{parse_top_level_json, parse_top_level_yaml, PathType};
    fn viewer(input: &str) -> JsonViewer {
        JsonViewer::new(parse_top_level_json(input.into()).unwrap())
    }
    fn path(v: &JsonViewer) -> String {
        v.flatjson
            .build_path_to_node(PathType::Dot, v.focused_row)
            .unwrap()
    }
    fn act(v: &mut JsonViewer, actions: &[Action]) {
        for &action in actions {
            v.perform_action(action);
        }
    }

    #[test]
    fn inline_values_retain_individual_identity_and_copy_ranges() {
        let mut v = viewer(r#"{"tags":["rust","cli"],"done":true}"#);
        act(&mut v, &[Action::MoveRight, Action::MoveRight]);
        assert_eq!(path(&v), ".tags[0]");
        let line = v.anchor_line;
        v.perform_action(Action::FocusNextSibling(1));
        assert_eq!(path(&v), ".tags[1]");
        assert_eq!(
            &v.flatjson.1[v.flatjson[v.focused_row].range.clone()],
            "\"cli\""
        );
        assert_eq!(line, v.anchor_line);
        v.perform_action(Action::MoveDown(1));
        assert_eq!(path(&v), ".done");
        assert_eq!(v.anchor_line, line + 1);
        v.perform_action(Action::MoveUp(1));
        assert_eq!(path(&v), ".tags");
    }

    #[test]
    fn table_cells_keep_columns_and_parent_structure() {
        let mut v = viewer(r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"\u006eame":"Lin"}]}"#);
        act(
            &mut v,
            &[
                Action::MoveRight,
                Action::MoveRight,
                Action::MoveRight,
                Action::FocusNextSibling(1),
            ],
        );
        assert_eq!(path(&v), ".users[0].name");
        v.perform_action(Action::MoveDown(1));
        assert_eq!(
            &v.flatjson.1[v.flatjson[v.focused_row].range.clone()],
            "\"Lin\""
        );
        let line = v.anchor_line;
        v.perform_action(Action::FocusParent);
        assert_eq!(path(&v), ".users[1]");
        assert_eq!(line, v.anchor_line);
        v.perform_action(Action::FocusParent);
        assert_eq!(path(&v), ".users");
    }

    #[test]
    fn collapse_preserves_child_state_and_implicit_root() {
        let mut v = viewer(r#"{"box":{"inner":{"value":1}},"last":2}"#);
        v.perform_action(Action::ToggleCollapsed);
        assert!(!v.flatjson[0].is_collapsed());
        act(
            &mut v,
            &[
                Action::MoveRight,
                Action::MoveRight,
                Action::ToggleCollapsed,
            ],
        );
        let child = v.focused_row;
        act(&mut v, &[Action::FocusParent, Action::ToggleCollapsed]);
        assert_eq!(v.visible.len(), 2);
        v.perform_action(Action::MoveRight);
        assert!(v.flatjson[child].is_collapsed());
        v.perform_action(Action::MoveRight);
        assert_eq!(v.focused_row, child);
        v.perform_action(Action::MoveRight);
        assert!(!v.flatjson[child].is_collapsed());
        v.perform_action(Action::MoveRight);
        assert_eq!(path(&v), ".box.inner.value");
    }

    #[test]
    fn absolute_jumps_and_search_reveal_distinguish_nodes_and_lines() {
        let mut v = viewer(r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Lin"}]}"#);
        act(&mut v, &[Action::MoveRight, Action::ToggleCollapsed]);
        v.perform_action(Action::JumpTo {
            line: 2,
            make_visible: false,
        });
        assert_eq!(path(&v), ".users");
        v.perform_action(Action::JumpTo {
            line: 2,
            make_visible: true,
        });
        assert_eq!(path(&v), ".users[1]");
        act(&mut v, &[Action::MoveRight, Action::FocusNextSibling(1)]);
        let node = v.focused_row;
        let key = v.flatjson[node].key_range.as_ref().unwrap().start;
        act(&mut v, &[Action::FocusParent, Action::ToggleCollapsed]);
        v.perform_action(Action::FocusNode {
            node,
            source: Some(key),
        });
        assert_eq!(v.focused_row, node);
        assert_eq!(v.anchor_line, 0, "shared key header is the display anchor");
        assert_eq!(path(&v), ".users[1].name");
    }

    #[test]
    fn separator_jumps_skip_annotations_and_empty_roots_remain_selectable() {
        let mut v = JsonViewer::new(parse_top_level_yaml("---\n{}\n---\n42\n".into()).unwrap());
        let first = v.focused_row;
        v.perform_action(Action::MoveDown(1));
        assert_ne!(first, v.focused_row);
        assert!(!v.visible[v.focused_line_index()].line.separator);
        v.perform_action(Action::MoveUp(1));
        assert_eq!(v.focused_row, first);
        v.perform_action(Action::JumpTo {
            line: 2,
            make_visible: false,
        });
        assert_ne!(v.focused_row, first);
        let mut empty = viewer("{}");
        act(
            &mut empty,
            &[
                Action::MoveRight,
                Action::ToggleCollapsed,
                Action::FocusBottom,
            ],
        );
        assert_eq!(empty.focused_row, 0);
        assert_eq!(empty.visible.len(), 1);
        assert!(empty.visible[0].line.text.is_empty());
    }

    #[test]
    fn mouse_hits_cells_and_resize_preserves_focus() {
        let mut v = viewer(r#"["界","é",3]"#);
        let child = v.flatjson[0].first_child().unwrap();
        let second = v.flatjson[child].next_sibling.unwrap();
        let span = v.layout.lines[0]
            .spans
            .iter()
            .find(|span| span.node == second)
            .unwrap();
        let col = UnicodeWidthStr::width(&v.layout.lines[0].text[..span.range.start]);
        let (node, source) = crate::lineprinter::hit_test(&v.visible[0].line, col);
        v.perform_action(Action::FocusNode { node, source });
        assert_eq!(v.focused_row, second);
        v.perform_action(Action::ResizeViewerDimensions(TTYDimensions {
            width: 2,
            height: 0,
        }));
        assert_eq!(v.focused_row, second);
        v.perform_action(Action::ClickArrow(1));
        assert_eq!(v.focused_row, 0);
        assert!(v.flatjson[0].is_collapsed());
    }

    #[test]
    fn counted_motions_scrolling_and_boundaries() {
        let input = format!(
            "{{{}}}",
            (0..30)
                .map(|i| format!("\"k{i}\":{i}"))
                .collect::<Vec<_>>()
                .join(",")
        );
        let mut v = viewer(&input);
        v.perform_action(Action::ResizeViewerDimensions(TTYDimensions {
            width: 20,
            height: 8,
        }));
        v.perform_action(Action::MoveDown(12));
        assert_eq!(v.anchor_line, 12);
        assert!(v.index_of_focused_row_on_screen() < 8);
        v.perform_action(Action::MoveFocusedLineToCenter);
        assert_eq!(v.index_of_focused_row_on_screen(), 4);
        v.perform_action(Action::PageDown(1));
        assert!(v.top_row >= 8);
        v.perform_action(Action::JumpDown(Some(3)));
        let before = v.anchor_line;
        v.perform_action(Action::JumpUp(None));
        assert_eq!(v.anchor_line, before - 3);
        v.perform_action(Action::MoveDown(usize::MAX));
        assert_eq!(v.anchor_line, 29);
        v.perform_action(Action::MoveUp(usize::MAX));
        assert_eq!(v.anchor_line, 0);
        v.perform_action(Action::PageDown(usize::MAX));
        assert!(v.top_row < v.visible.len());
    }
}
