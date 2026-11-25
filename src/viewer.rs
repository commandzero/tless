mod action;
mod alignment;
mod collapse;
mod focus;
mod frame;
mod navigation;
mod projection;
mod viewport;

#[cfg(test)]
use crate::flatjson::OptionIndex;
use crate::flatjson::{FlatJson, Index};
use crate::toon_display::DisplayLine;
use crate::toon_display::alignment::TableMetrics;
use crate::toon_display::layout::{Layout, Projection};
use crate::types::TTYDimensions;
use crate::wrapped_view::PhysicalRow;
use std::collections::{HashMap, HashSet};
use std::ops::Range;

/// Parsed node focus is independent of its painted line (inline values and table cells).
pub struct JsonViewer {
    pub flatjson: FlatJson,
    pub layout: Layout,
    pub visible: Projection,
    /// Index into the visibility projection, not the parsed node list.
    pub top_visible_line: Index,
    /// Continuation ordinal within the top logical line, never a document-wide row index.
    pub top_continuation: usize,
    /// Physical rows and text for the current frame only.
    pub physical_rows: Vec<PhysicalRow>,
    frame: Vec<(usize, DisplayLine)>,
    /// Session-only optional wrapping state. It starts disabled.
    pub wrapping_enabled: bool,
    pub focused_node: Index,
    pub absolute_anchor_line: usize,
    jump_distance: Option<usize>,
    desired_depth: usize,
    pub dimensions: TTYDimensions,
    pub scrolloff_setting: u16,
    expanded_arrays: HashSet<usize>,
    /// Collapse state for sequence document rows. This is deliberately kept
    /// separate from `FlatJson` so a root's document row can be collapsed
    /// independently from a root array's TOON body layout.
    document_collapsed: HashSet<Index>,
    active_roots: Vec<Index>,
    document_roots: Vec<Index>,
    line_numbers: bool,
    wrap_width: usize,
    wrap_indentation: usize,
    pub layout_generation: usize,
    pub physical_generation: usize,
    focus_byte_range: Option<Range<usize>>,
    /// Only enabled table identities retain column metrics, never formatted rows.
    aligned_tables: HashMap<Index, TableMetrics>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Cursor {
    logical: usize,
    continuation: usize,
}

impl JsonViewer {
    #[cfg(test)]
    pub fn new(flatjson: FlatJson) -> Self {
        let roots = crate::path_filter::document_roots(&flatjson);
        Self::with_roots(flatjson, roots, TTYDimensions::default(), true, false)
    }

    /// Construct directly from already-resolved active roots. This avoids
    /// briefly constructing a full excluded layout during startup filtering.
    pub fn with_roots(
        flatjson: FlatJson,
        roots: Vec<Index>,
        dimensions: TTYDimensions,
        line_numbers: bool,
        expand_roots: bool,
    ) -> Self {
        let mut flatjson = flatjson;
        if expand_roots {
            for &root in &roots {
                flatjson.expand(root);
            }
        }
        let expanded_arrays = HashSet::new();
        let document_collapsed = HashSet::new();
        let layout = Layout::new(
            &flatjson,
            &roots,
            usize::from(dimensions.width),
            line_numbers,
            &expanded_arrays,
        );
        let visible = layout.project_with_documents(&flatjson, &document_collapsed);
        let wrap_width = layout.geometry.width;
        let mut viewer = Self {
            flatjson,
            layout,
            visible,
            top_visible_line: 0,
            top_continuation: 0,
            physical_rows: Vec::new(),
            frame: Vec::new(),
            wrapping_enabled: false,
            focused_node: 0,
            absolute_anchor_line: 0,
            jump_distance: None,
            desired_depth: 0,
            dimensions,
            scrolloff_setting: 3,
            expanded_arrays,
            document_collapsed,
            document_roots: roots.clone(),
            active_roots: roots,
            line_numbers,
            wrap_width,
            wrap_indentation: 0,
            layout_generation: 0,
            physical_generation: 0,
            focus_byte_range: None,
            aligned_tables: HashMap::new(),
        };
        viewer.focus_first_root();
        viewer.refresh_frame();
        viewer
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
    FocusParentOrPreviousSibling,
    FocusNextAtParentLevel,

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
    FocusNodeAt {
        node: Index,
        source: Option<usize>,
        display_range: (usize, usize),
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
        self.as_option().unwrap_or(crate::flatjson::NIL)
    }
}

#[cfg(test)]
mod tests;
