//! Warning placement and the shared spelling used for table measurement.
use super::{DisplayLine, FlatJson, Formatter, TokenRole};
use crate::toon_display::index::key;
use crate::toon_display::{WarningKind, quote_json_into};
use std::fmt::Write;

pub(crate) struct WarningPosition {
    pub(crate) node: usize,
    pub(crate) first: bool,
    pub(crate) field: bool,
}

pub(crate) struct WarningText<'a> {
    pub(crate) flat: &'a FlatJson,
    pub(crate) quoted: &'a mut String,
}

impl WarningText<'_> {
    // The prefix belongs to the visible row, while the message and locator
    // belong to the source node. Measurement can consume these same fragments.
    pub(crate) fn parts(
        &mut self,
        warning: WarningKind,
        pos: WarningPosition,
        mut emit: impl FnMut(&str, bool),
    ) {
        emit(if pos.first { "  # WARN " } else { "; " }, true);
        emit(warning.message(), false);
        if pos.field {
            emit(" at field ", false);
            quote_json_into(&key(self.flat, pos.node).unwrap_or_default(), self.quoted);
            emit(self.quoted, false);
        }
    }
}

struct Annotation<'a> {
    line: &'a mut DisplayLine,
    first: bool,
    quoted: String,
    message: String,
}

impl Formatter<'_> {
    pub(super) fn annotate(&self, line: &mut DisplayLine) {
        let nodes = self.warning_nodes(line);
        let mut annotation = Annotation {
            line,
            first: true,
            quoted: String::new(),
            message: String::new(),
        };
        for node in nodes {
            let mut warnings: Vec<_> = self.analysis.warnings(node).collect();
            warnings.sort();
            for warning in warnings {
                self.write_warning(&mut annotation, node, warning);
            }
        }
    }

    fn warning_nodes(&self, line: &DisplayLine) -> Vec<usize> {
        let mut nodes: Vec<_> = line
            .spans
            .iter()
            .filter(|span| {
                !matches!(span.role, TokenRole::Key | TokenRole::FieldDefinition)
                    || self.same_line(span.node, line.owner)
            })
            .map(|span| span.node)
            .collect();
        nodes.push(line.owner);
        if self.analysis.table_row(self.flat, line.owner) {
            nodes.extend(line.owner + 1..*self.flat.subtree_range(line.owner).end());
        }
        nodes.sort_unstable();
        nodes.dedup();
        nodes
    }

    fn write_warning(&self, annotation: &mut Annotation<'_>, node: usize, warning: WarningKind) {
        annotation.message.clear();
        let table_row = self.analysis.table_row_owner(self.flat, node);
        let pos = WarningPosition {
            node,
            first: annotation.first,
            field: !self.analysis.is_root(node) && table_row.is_some_and(|row| row != node),
        };
        let mut text = WarningText {
            flat: self.flat,
            quoted: &mut annotation.quoted,
        };
        text.parts(warning, pos, |part, prefix| {
            if prefix {
                annotation
                    .line
                    .token(part, annotation.line.owner, TokenRole::Warning, None);
            } else {
                annotation.message.push_str(part);
            }
        });
        annotation.first = false;
        if table_row.is_none() {
            if let Some(index) = self.array_warning_index(node, annotation.line.owner) {
                write!(annotation.message, " at [{index}]").unwrap();
            }
        }
        annotation
            .line
            .token(&annotation.message, node, TokenRole::Warning, None);
    }

    fn array_warning_index(&self, node: usize, owner: usize) -> Option<usize> {
        if self.analysis.is_root(node) {
            return None;
        }
        let row = &self.flat[node];
        let parent = row.parent.as_option()?;
        if self.flat[parent].is_array() && self.same_line(node, owner) {
            return Some(row.index_in_parent);
        }
        None
    }

    fn same_line(&self, node: usize, owner: usize) -> bool {
        self.geometry.position(self.flat, self.analysis, node).line
            == self
                .geometry
                .position(self.flat, self.analysis, owner)
                .body_line
    }
}
