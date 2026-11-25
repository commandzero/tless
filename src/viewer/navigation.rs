use super::JsonViewer;
use crate::flatjson::Index;
use crate::toon_display::normalize_node;
use std::cmp::Ordering;

impl JsonViewer {
    pub(super) fn parent(&mut self) {
        if let Some(parent) = self.effective_parent(self.focused_node).as_option() {
            self.focus(parent);
        }
    }

    pub(super) fn parent_or_previous_sibling(&mut self) {
        let current = normalize_node(&self.flatjson, self.focused_node);
        let destination = self.effective_parent(current).as_option().or_else(|| {
            self.active_root_for(current)
                .and_then(|root| self.adjacent_root(root, false))
        });
        if let Some(node) = destination {
            self.focus(node);
        }
    }

    fn adjacent_root(&self, root: Index, next: bool) -> Option<Index> {
        let position = self
            .active_roots
            .iter()
            .position(|&candidate| candidate == root)?;
        let adjacent = if next {
            position + 1
        } else {
            position.checked_sub(1)?
        };
        self.active_roots.get(adjacent).copied()
    }

    fn next_sibling_in_view(&self, node: Index) -> Option<Index> {
        if let Some(position) = self.active_roots.iter().position(|&root| root == node) {
            self.active_roots.get(position + 1).copied()
        } else if self.contains_node(node) {
            self.flatjson[node].next_sibling.as_option()
        } else {
            None
        }
    }

    pub(super) fn next_at_parent_level(&mut self) {
        let current = normalize_node(&self.flatjson, self.focused_node);
        let destination = self
            .effective_parent(current)
            .as_option()
            .and_then(|parent| self.next_sibling_in_view(parent))
            .filter(|&node| self.contains_node(node))
            .or_else(|| self.next_sibling_in_view(current));
        if let Some(node) = destination.filter(|&node| self.contains_node(node)) {
            self.focus(node);
        }
    }

    pub(super) fn focus_sibling_edge(&mut self, last: bool) {
        let Some(root) = self.active_root_for(self.focused_node) else {
            return;
        };
        let node = if normalize_node(&self.flatjson, self.focused_node) == root {
            if last {
                self.active_roots.last().copied()
            } else {
                self.active_roots.first().copied()
            }
        } else {
            self.effective_parent(self.focused_node)
                .as_option()
                .and_then(|parent| self.edge_child(parent, last))
        };
        if let Some(node) = node.filter(|&node| self.contains_node(node)) {
            self.focus(node);
        }
    }

    fn edge_child(&self, node: Index, last: bool) -> Option<Index> {
        if last {
            let pair = self.flatjson[node].pair_index().as_option()?;
            self.flatjson[pair].last_child().as_option()
        } else {
            self.flatjson[node].first_child().as_option()
        }
    }

    fn adjacent_sibling(&self, node: Index, next: bool) -> Option<Index> {
        let root = self.active_root_for(node)?;
        if root == normalize_node(&self.flatjson, node) {
            self.adjacent_root(root, next)
        } else if next {
            self.flatjson[node].next_sibling.as_option()
        } else {
            self.flatjson[node].prev_sibling.as_option()
        }
    }

    fn descend_sibling(&self, mut node: Index, next: bool) -> Index {
        while self.flatjson[node].depth < self.desired_depth && self.flatjson[node].is_expanded() {
            let Some(child) = self
                .edge_child(node, !next)
                .filter(|&child| self.contains_node(child))
            else {
                break;
            };
            node = child;
        }
        node
    }

    pub(super) fn sibling(&mut self, count: usize, next: bool) {
        for _ in 0..count {
            let before = self.focused_node;
            match self.adjacent_sibling(before, next) {
                Some(node) if self.contains_node(node) => {
                    self.focus(self.descend_sibling(node, next))
                }
                None if !next => self.parent(),
                _ => break,
            }
            if self.focused_node == before {
                break;
            }
        }
    }

    pub(super) fn move_until_depth_change(&mut self, down: bool) {
        let mut depth = self.flatjson[self.focused_node].depth;
        let mut moved = false;
        loop {
            let previous_node = self.focused_node;
            let previous_line = self.absolute_anchor_line;
            self.vertical(1, down);
            if self.absolute_anchor_line == previous_line {
                break;
            }
            let next_depth = self.flatjson[self.focused_node].depth;
            match (next_depth.cmp(&depth), down, moved) {
                (Ordering::Equal, _, _) => {}
                // An initial upward step may enter the preceding container.
                (Ordering::Greater, false, false) => depth = next_depth,
                // Keep the last row at the retained depth when crossing this boundary.
                (Ordering::Greater, true, true) | (_, false, true) => {
                    self.focus(previous_node);
                    self.absolute_anchor_line = previous_line;
                    break;
                }
                _ => break,
            }
            moved = true;
        }
    }
}
