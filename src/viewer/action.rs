use super::{Action, JsonViewer};
use crate::flatjson::Index;

impl JsonViewer {
    pub fn perform_action(&mut self, action: Action) {
        match action {
            Action::NoOp => {}
            Action::MoveUp(count) => self.vertical(count, false),
            Action::MoveDown(count) => self.vertical(count, true),
            Action::MoveRight => self.move_right(),
            Action::MoveLeft => self.move_left(),
            Action::FocusParent => self.parent(),
            Action::FocusParentOrPreviousSibling => self.parent_or_previous_sibling(),
            Action::FocusNextAtParentLevel => self.next_at_parent_level(),
            Action::FocusPrevSibling(count) => self.sibling(count, false),
            Action::FocusNextSibling(count) => self.sibling(count, true),
            Action::FocusFirstSibling => self.focus_sibling_edge(false),
            Action::FocusLastSibling => self.focus_sibling_edge(true),
            Action::FocusTop => self.focus_top(),
            Action::FocusBottom => self.focus_line(self.visible.len() - 1, false),
            Action::MoveUpUntilDepthChange => self.move_until_depth_change(false),
            Action::MoveDownUntilDepthChange => self.move_until_depth_change(true),
            Action::JumpTo { line, make_visible } => self.jump_to_layout_line(line, make_visible),
            Action::FocusNode { node, source } => self.reveal(node, source),
            Action::FocusNodeAt {
                node,
                source,
                display_range,
            } => {
                self.reveal(node, source);
                self.focus_byte_range = Some(display_range.0..display_range.1);
            }
            Action::ScrollUp(count) => self.scroll(count, false),
            Action::ScrollDown(count) => self.scroll(count, true),
            Action::PageUp(count) => self.scroll(self.page_size(count), false),
            Action::PageDown(count) => self.scroll(self.page_size(count), true),
            Action::JumpUp(distance) => self.half_page_jump(distance, false),
            Action::JumpDown(distance) => self.half_page_jump(distance, true),
            Action::MoveFocusedLineToTop
            | Action::MoveFocusedLineToCenter
            | Action::MoveFocusedLineToBottom => self.reposition_focused_line(action),
            Action::ClickArrow(row) => {
                if !self.click_arrow(row) {
                    return;
                }
            }
            Action::ToggleCollapsed => self.toggle_collapsed(self.focused_node),
            Action::CollapseNodeAndSiblings => self.collapse_siblings(true, false),
            Action::DeepCollapseNodeAndSiblings => self.collapse_siblings(true, true),
            Action::ExpandNodeAndSiblings => self.collapse_siblings(false, false),
            Action::DeepExpandNodeAndSiblings => self.collapse_siblings(false, true),
            Action::ResizeViewerDimensions(dimensions) => {
                self.set_viewport(dimensions, self.line_numbers);
            }
        }
        self.finish_action(action);
    }

    fn finish_action(&mut self, action: Action) {
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
            self.desired_depth = self.flatjson[self.focused_node].depth;
        }
        if matches!(
            action,
            Action::NoOp
                | Action::ScrollUp(_)
                | Action::ScrollDown(_)
                | Action::PageUp(_)
                | Action::PageDown(_)
                | Action::JumpUp(_)
                | Action::JumpDown(_)
                | Action::MoveFocusedLineToTop
                | Action::MoveFocusedLineToCenter
                | Action::MoveFocusedLineToBottom
        ) {
            self.refresh_frame();
        } else {
            self.ensure_visible();
        }
    }

    fn move_right(&mut self) {
        let node = self.focused_node;
        if self.is_document_root(node) && self.focused_document_header() {
            self.enter_document(node);
            return;
        }
        let layout = self.layout.node(&self.flatjson, node);
        if layout.collapsible && self.flatjson[node].is_collapsed() {
            self.collapse(node, false);
            if layout.inline_array {
                self.expand_inline_array(node);
            } else {
                self.refresh_projection();
            }
        } else if layout.inline_array {
            self.expand_inline_array(node);
        } else if let Some(child) = self.flatjson[node].first_child().as_option() {
            self.focus(child);
        }
    }

    fn enter_document(&mut self, node: Index) {
        if self.is_document_collapsed(node) {
            self.collapse(node, false);
            self.refresh_projection();
            return;
        }
        let Some(child) = self.flatjson[node].first_child().as_option() else {
            return;
        };
        let body_collapsed = self.flatjson[node].is_collapsed();
        if body_collapsed {
            self.flatjson.expand(node);
        }
        if self.layout.node(&self.flatjson, node).inline_array {
            self.expand_inline_array(node);
        } else if body_collapsed {
            self.refresh_projection();
        }
        self.focus(child);
    }

    fn expand_inline_array(&mut self, node: Index) {
        self.expanded_arrays.insert(node);
        self.rebuild_layout();
    }

    fn move_left(&mut self) {
        let node = self.focused_node;
        if self.is_document_root(node) && self.focused_document_header() {
            if !self.is_document_collapsed(node) {
                self.collapse(node, true);
                self.refresh_projection();
            }
            return;
        }
        if self.focused_body_is_collapsible() && self.flatjson[node].is_expanded() {
            self.collapse(node, true);
            self.refresh_projection();
        } else {
            self.parent();
        }
    }

    fn focused_body_is_collapsible(&self) -> bool {
        self.line_is_collapsible(self.focused_line_index())
            && self
                .layout
                .node(&self.flatjson, self.focused_node)
                .collapsible
    }

    fn focus_top(&mut self) {
        let root = self.document_roots().first().copied().unwrap_or(0);
        self.focus(root);
        if self.is_document_root(root) {
            self.absolute_anchor_line = self.document_header_absolute(root);
            self.desired_depth = self.flatjson[root].depth;
        }
        self.top_visible_line = 0;
        self.top_continuation = 0;
    }

    fn jump_to_layout_line(&mut self, line: usize, make_visible: bool) {
        let line = line.min(self.layout.line_count() - 1);
        let node = self.layout.row(&self.flatjson, line).unwrap().owner;
        if let Some(index) = self.visible.visible_index(line) {
            self.focus_line(index, false);
            return;
        }
        if !make_visible {
            self.focus(node);
            return;
        }
        self.reveal(node, None);
        if let Some(index) = self.visible.visible_index(line) {
            self.focus_line(index, false);
        }
    }

    fn page_size(&self, count: usize) -> usize {
        usize::from(self.dimensions.height).saturating_mul(count)
    }

    fn half_page_jump(&mut self, distance: Option<usize>, down: bool) {
        self.jump_distance = distance.or(self.jump_distance);
        let distance = self
            .jump_distance
            .unwrap_or((usize::from(self.dimensions.height) / 2).max(1));
        self.jump(distance, down);
    }

    fn reposition_focused_line(&mut self, action: Action) {
        let height = usize::from(self.dimensions.height).max(1);
        let padding = match action {
            Action::MoveFocusedLineToTop => {
                usize::from(self.scrolloff_setting).min((height - 1) / 2)
            }
            Action::MoveFocusedLineToCenter => height / 2,
            _ => height - 1,
        };
        if self.wrapping_enabled {
            self.set_top(self.walk(self.focused_cursor(), padding, false));
            self.clamp_wrapped_top();
        } else {
            self.top_visible_line = self.focused_line_index().saturating_sub(padding);
        }
    }

    fn click_arrow(&mut self, row: u16) -> bool {
        let Some(physical) = self.screen_row(usize::from(row.saturating_sub(1))) else {
            return false;
        };
        let line = physical.logical_line;
        if !physical.first || !self.line_is_collapsible(line) {
            return false;
        }
        let node = self
            .document_root_for_line(line)
            .unwrap_or(self.visible_line(line).unwrap().owner);
        if self.is_document_header(line) {
            self.focus(node);
        } else {
            self.focus_line(line, false);
        }
        self.toggle_collapsed(node);
        true
    }
}
