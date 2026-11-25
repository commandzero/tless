use std::fmt;
#[cfg(test)]
use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// This module provides functionality for truncating strings,
/// displaying them, and manipulating which portion of the string
/// is visible.
///
/// A TruncatedStrView represents an attempt to fit a string within
/// a given amount of available space. When `range` is None, it
/// signifies that the string cannot be represented at all in the
/// given space.
///
/// If the available space is negative, no string can be represented.
/// If there is 0 available space, then only an empty string can be
/// represented. Above that, every string can be represented with
/// ellipses representing truncated content.
///
/// When the string is representable, the view will also track how
/// much space the view takes up, including ellipses.
#[derive(Debug, Copy, Clone)]
pub struct TruncatedStrView {
    pub range: Option<TruncatedRange>,
    #[cfg(test)]
    available_space: isize,
}

/// A TruncatedRange is a range intended to represent a slice of
/// a string, along with some additional metadata that is useful
/// when manipulating a range.
///
/// The visible portion of the string is represented by the
/// range [start..end).
///
/// When we are representing a string in just 2 or 3 columns,
/// wide characters are unrepresentable. For example, while we can
/// represent the prefix of the string "abc🦀" in just two columns
/// ("a…"), we cannot represent the suffix because the '🦀'
/// character is two columns wide, so we wouldn't have room for the
/// ellipsis.
///
/// In this situation we choose to display the Unicode replacement
/// character: '�', which is to be used for unrepresentable
/// characters.
///
/// A similar situation occurs when a wide character appears in the
/// middle of a string and we have just 3 columns to display the
/// character.
///
/// Since showing a replacement character is less than ideal, when
/// we are showing other characters, we will opt for not using the
/// entire available space, rather than including the replacement
/// character.
///
/// This range also keeps track of how much space it takes up.
#[derive(Debug, Copy, Clone)]
pub struct TruncatedRange {
    pub start: usize,
    pub end: usize,
    pub showing_replacement_character: bool,
    used_space: isize,
}

/// A TruncatedStrView doesn't keep a reference to the `str` it
/// is intended to display.
///
/// This is a helper struct used to actually generate the printed
/// intended representation of the TruncatedStrView.
pub struct TruncatedStrSlice<'a, 'b> {
    pub s: &'a str,
    pub truncated_view: &'b TruncatedStrView,
}

// When manipulating a TruncatedStrView, we use this helper struct
// to keep track of state and maintain a reference to the string
// the view is representing.
//
// Note that the RangeAdjuster does *not* keep track of whether it is
// showing a replacement character. We can determine whether or
// not to display a replacement character when we decide we want to
// "materialize" the RangeAdjuster into a TruncatedStrView based on
// whether the visible portion is "empty" and there is space to
// display a replacement character.
#[derive(Clone, Debug)]
struct RangeAdjuster<'a> {
    s: &'a str,
    used_space: isize,
    available_space: isize,

    start: usize,
    end: usize,
}

impl TruncatedRange {
    // Create a RangeAdjuster representing the current state of the
    // TruncatedRange.
    #[cfg(test)]
    fn adjuster<'a>(&self, s: &'a str, available_space: isize) -> RangeAdjuster<'a> {
        let mut used_space = self.used_space;
        // The adjuster doesn't keep track of the replacement character.
        if self.showing_replacement_character {
            used_space -= 1;
        }

        RangeAdjuster {
            s,
            used_space,
            available_space,
            start: self.start,
            end: self.end,
        }
    }

    /// Check whether this is a view of a string that is totally elided,
    /// that is, it is represented by a single ellipsis.
    pub fn is_completely_elided(&self) -> bool {
        self.used_space == 1 && self.start == self.end
    }

    // Status clipping uses the retained range directly.
}

impl TruncatedStrView {
    /// If we have a least one column, we can always represent a string
    /// by at least an ellipsis that represents the entirety of the string.
    ///
    /// If we have exactly 0 columns, then only an empty string can be
    /// represented. (We take the conservative approach here and ignore
    /// the possibility of strings solely consisting of zero-width spaces
    /// and other zero-width Unicode oddities.)
    ///
    /// The `range` of a TruncatedStrView should be present if and only if
    /// this function, when called on the string the TruncatedStrView is
    /// associated with, returns true.
    pub fn can_str_fit_at_all(s: &str, available_space: isize) -> bool {
        available_space > 0 || (available_space == 0 && s.is_empty())
    }

    /// Show the beginning of the string, eliding its end if necessary.
    pub fn init_start(s: &str, available_space: isize) -> TruncatedStrView {
        Self::init(s, available_space, Side::Left)
    }

    /// Show the end of the string, eliding its beginning if necessary.
    pub fn init_back(s: &str, available_space: isize) -> TruncatedStrView {
        Self::init(s, available_space, Side::Right)
    }

    fn init(s: &str, available_space: isize, side: Side) -> TruncatedStrView {
        if !Self::can_str_fit_at_all(s, available_space) {
            return Self::init_no_view(available_space);
        }
        let mut adjuster = RangeAdjuster::init_at_index(s, available_space, side.limit(s));
        adjuster.fill(side.opposite());
        adjuster.to_view()
    }

    // Create a TruncatedStrView that indicates that the string cannot
    // be represented in the available space.
    fn init_no_view(_available_space: isize) -> TruncatedStrView {
        TruncatedStrView {
            range: None,
            #[cfg(test)]
            available_space: _available_space,
        }
    }

    /// Return the amount of space used by a string view, if the string
    /// is representable.
    pub fn used_space(&self) -> Option<isize> {
        self.range
            .map(|TruncatedRange { used_space, .. }| used_space)
    }

    /// Check whether this is a view of a string that fits in the available
    /// space and shows at least one character (i.e., isn't totally elided).
    pub fn any_contents_visible(&self) -> bool {
        self.range.is_some_and(|r| !r.is_completely_elided())
    }

    // Creates a RangeAdjuster that represents the current state of
    // the TruncatedStrView. This should only be called when the string
    // is representable and we have a view.
    #[cfg(test)]
    fn range_adjuster<'a>(&self, s: &'a str) -> RangeAdjuster<'a> {
        debug_assert!(self.range.is_some());
        self.range.unwrap().adjuster(s, self.available_space)
    }

    /// Scroll by at least the requested grapheme count, up to the string's end.
    #[cfg(test)]
    pub fn scroll_right(&self, s: &str, count: usize) -> TruncatedStrView {
        self.scroll(s, count, Side::Right)
    }

    /// Scroll by at least the requested grapheme count, up to the string's start.
    #[cfg(test)]
    pub fn scroll_left(&self, s: &str, count: usize) -> TruncatedStrView {
        self.scroll(s, count, Side::Left)
    }

    #[cfg(test)]
    fn scroll(&self, s: &str, count: usize, side: Side) -> TruncatedStrView {
        if self.range.is_none() {
            return *self;
        }

        // Two columns cannot show a middle portion between ellipses.
        if self.available_space <= 2 {
            return Self::init(s, self.available_space, side);
        }

        let mut adjuster = self.range_adjuster(s);
        adjuster.extend(side, count);
        adjuster.shrink_to_fit(side.opposite());
        // Leave an empty view in place so it shows the overwide grapheme's
        // replacement character instead of skipping to the next grapheme.
        if adjuster.start != adjuster.end {
            adjuster.fill(side);
        }
        adjuster.to_view()
    }

    // Expand a view to fit into more available space.
    #[cfg(test)]
    fn expand(&self, s: &str, available_space: isize) -> TruncatedStrView {
        debug_assert!(available_space > self.available_space);
        let mut adjuster = self.range_adjuster(s);
        adjuster.available_space = available_space;

        adjuster.fill(Side::Right);
        // Fill on the left only after the suffix becomes visible. Otherwise,
        // a wide grapheme on the right should keep the viewport still.
        if adjuster.end == s.len() {
            adjuster.fill(Side::Left);
        }
        adjuster.to_view()
    }

    /// Shrink a view to fit into less available space.
    #[cfg(test)]
    fn shrink(&self, s: &str, available_space: isize) -> TruncatedStrView {
        debug_assert!(available_space < self.available_space);
        debug_assert!(self.range.is_some());

        // Won't be enough room for multiple ellipses and a middle character
        // so just init from the beginning (or end, if we're showing a suffix).
        if available_space < 3 {
            let TruncatedRange { start, end, .. } = self.range.unwrap();
            if start > 0 && end == s.len() {
                return Self::init_back(s, available_space);
            } else {
                return Self::init_start(s, available_space);
            }
        }

        let mut adjuster = self.range_adjuster(s);
        adjuster.available_space = available_space;

        // If we're showing a suffix of the string, shrink from the left
        // so we keep showing the end, otherwise shrink from the right.
        if adjuster.start > 0 && adjuster.end == s.len() {
            adjuster.shrink_to_fit(Side::Left);
        } else {
            adjuster.shrink_to_fit(Side::Right);
        }

        adjuster.to_view()
    }

    /// Scroll a view so that a particular subrange is shown.
    #[cfg(test)]
    pub fn focus(&self, s: &str, range: &Range<usize>) -> TruncatedStrView {
        if self.range.is_none() {
            return *self;
        }

        let Range { mut start, mut end } = *range;

        // Make sure our start isn't in-between a character boundary.
        while start != 0 && !s.is_char_boundary(start) {
            start -= 1;
        }
        end = end.min(s.len());

        let visible_range = self.range.unwrap();

        // If the entire match is already visible, don't do anything.
        if visible_range.start <= start && end <= visible_range.end {
            return *self;
        }

        // But otherwise, we'll just jump to the match and try to center it.
        let mut adjuster = RangeAdjuster::init_at_index(s, self.available_space, start);

        // Make sure to include entire match if possible.
        while adjuster.end < end && adjuster.used_space < self.available_space {
            adjuster.extend(Side::Right, 1);
        }

        // Then fill from both sides to keep it centered.
        adjuster.fill_from_both_sides();

        adjuster.to_view()
    }
}

#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
}

impl Side {
    fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    fn limit(self, text: &str) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => text.len(),
        }
    }

    fn take<'a>(self, graphemes: &mut unicode_segmentation::Graphemes<'a>) -> Option<&'a str> {
        match self {
            Self::Left => graphemes.next_back(),
            Self::Right => graphemes.next(),
        }
    }
}

impl<'a> RangeAdjuster<'a> {
    /// Initialize a RangeAdjuster at an arbitrary spot in a string, but
    /// is not showing any part of the string.
    pub fn init_at_index(s: &'a str, available_space: isize, index: usize) -> Self {
        let mut space_for_ellipses = 0;
        if index > 0 {
            // We have a leading ellipsis;
            space_for_ellipses += 1;
        }
        if index < s.len() {
            // We have a trailing ellipsis;
            space_for_ellipses += 1;
        }

        RangeAdjuster {
            s,
            used_space: space_for_ellipses,
            available_space,
            start: index,
            end: index,
        }
    }

    fn outside(&self, side: Side) -> unicode_segmentation::Graphemes<'a> {
        match side {
            Side::Left => self.s[..self.start].graphemes(true),
            Side::Right => self.s[self.end..].graphemes(true),
        }
    }

    fn edge(&mut self, side: Side) -> &mut usize {
        match side {
            Side::Left => &mut self.start,
            Side::Right => &mut self.end,
        }
    }

    fn expanded_edge(&self, side: Side, grapheme: &str) -> (usize, isize) {
        let position = match side {
            Side::Left => self.start - grapheme.len(),
            Side::Right => self.end + grapheme.len(),
        };
        // Reaching the boundary frees one cell for the ellipsis.
        let ellipsis = isize::from(position == side.limit(self.s));
        let used = self.used_space + UnicodeWidthStr::width(grapheme) as isize - ellipsis;
        (position, used)
    }

    fn add_if_fits(&mut self, side: Side, grapheme: &str) -> bool {
        let (position, used) = self.expanded_edge(side, grapheme);
        if used > self.available_space {
            return false;
        }
        *self.edge(side) = position;
        self.used_space = used;
        true
    }

    // Try the next grapheme even if there is no spare cell. It can replace
    // the ellipsis at the boundary with no increase in width.
    fn fill(&mut self, side: Side) {
        let mut graphemes = self.outside(side);
        while let Some(grapheme) = side.take(&mut graphemes) {
            if !self.add_if_fits(side, grapheme) {
                break;
            }
        }
    }

    #[cfg(test)]
    fn extend(&mut self, side: Side, count: usize) {
        let mut graphemes = self.outside(side);
        for _ in 0..count {
            let Some(grapheme) = side.take(&mut graphemes) else {
                break;
            };
            let (position, used) = self.expanded_edge(side, grapheme);
            *self.edge(side) = position;
            self.used_space = used;
        }
    }

    /// Add characters alternately by occupied width to keep the view centered.
    #[cfg(test)]
    fn fill_from_both_sides(&mut self) {
        let mut left = FillSide::new(self, Side::Left);
        let mut right = FillSide::new(self, Side::Right);
        while self.used_space <= self.available_space {
            let added_right = right.fill_until(self, &left, true);
            let added_left = left.fill_until(self, &right, false);
            if !added_right && !added_left {
                break;
            }
        }
    }

    #[cfg(test)]
    fn shrink_to_fit(&mut self, side: Side) {
        let mut visible = self.s[self.start..self.end].graphemes(true);
        let limit = side.limit(self.s);
        while self.used_space > self.available_space {
            debug_assert!(self.start < self.end);
            let grapheme = side.opposite().take(&mut visible).unwrap();
            let edge = self.edge(side);
            let ellipsis = isize::from(*edge == limit);
            match side {
                Side::Left => *edge += grapheme.len(),
                Side::Right => *edge -= grapheme.len(),
            }
            self.used_space += ellipsis - UnicodeWidthStr::width(grapheme) as isize;
        }
    }

    /// Convert a RangeAdjuster into a TruncatedStrView.
    pub fn to_view(&self) -> TruncatedStrView {
        debug_assert!(TruncatedStrView::can_str_fit_at_all(
            self.s,
            self.available_space
        ));

        // This DOESN'T consider the possibility that using a
        // replacement character would remove the need for an
        // ellipsis (because it's the last character), so
        // something like "🦀" is represented as "…", not "�".
        let showing_replacement_character =
            // We only show a replacement character if we're not
            // showing anything at all...
            self.start == self.end &&
                // But we have room to showing something...
                self.available_space > 1 &&
                    // And there's something to show.
                    !self.s.is_empty();

        let mut used_space = self.used_space;
        if showing_replacement_character {
            debug_assert!(used_space < self.available_space);
            used_space += 1;
        };

        TruncatedStrView {
            range: Some(TruncatedRange {
                start: self.start,
                end: self.end,
                showing_replacement_character,
                used_space,
            }),
            #[cfg(test)]
            available_space: self.available_space,
        }
    }
}

#[cfg(test)]
struct FillSide<'a> {
    side: Side,
    graphemes: unicode_segmentation::Graphemes<'a>,
    added_width: isize,
    has_more: bool,
}

#[cfg(test)]
impl<'a> FillSide<'a> {
    fn new(adjuster: &RangeAdjuster<'a>, side: Side) -> Self {
        Self {
            side,
            graphemes: adjuster.outside(side),
            added_width: 0,
            has_more: true,
        }
    }

    fn fill_until(
        &mut self,
        adjuster: &mut RangeAdjuster<'a>,
        other: &Self,
        take_ties: bool,
    ) -> bool {
        let mut added = false;
        let target = other.added_width + isize::from(take_ties);
        while !other.has_more || self.added_width < target {
            let before = adjuster.used_space;
            let fits = self
                .side
                .take(&mut self.graphemes)
                .is_some_and(|grapheme| adjuster.add_if_fits(self.side, grapheme));
            if !fits {
                self.has_more = false;
                break;
            }
            self.added_width += adjuster.used_space - before;
            added = true;
        }
        added
    }
}

impl<'a, 'b> fmt::Display for TruncatedStrSlice<'a, 'b> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.truncated_view.range.is_none() {
            return Ok(());
        }

        let TruncatedRange {
            start,
            end,
            showing_replacement_character,
            ..
        } = self.truncated_view.range.unwrap();

        if start != 0 {
            f.write_str("…")?;
        }

        if showing_replacement_character {
            f.write_str("�")?;
        }

        f.write_str(&self.s[start..end])?;

        if end != self.s.len() {
            f.write_str("…")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rendered(s: &str, truncated_view: &TruncatedStrView) -> String {
        format!("{}", TruncatedStrSlice { s, truncated_view })
    }

    #[test]
    fn test_init_start_and_init_back() {
        #[track_caller]
        fn assert_init_start(string: &str, space: isize, front: &str, used_space: Option<isize>) {
            let init_state = TruncatedStrView::init_start(string, space);
            assert_eq!(front, rendered(string, &init_state), "incorrect prefix");
            assert_eq!(
                used_space,
                init_state.used_space(),
                "incorrect prefix width"
            );
        }

        #[track_caller]
        fn assert_init_back(string: &str, space: isize, back: &str, used_space: Option<isize>) {
            let init_state = TruncatedStrView::init_back(string, space);
            assert_eq!(back, rendered(string, &init_state), "incorrect suffix");
            assert_eq!(
                used_space,
                init_state.used_space(),
                "incorrect suffix width"
            );
        }

        #[track_caller]
        fn assert_init_states(
            string: &str,
            space: isize,
            front: &str,
            back: &str,
            used_space: Option<isize>,
        ) {
            assert_init_start(string, space, front, used_space);
            assert_init_back(string, space, back, used_space);
        }

        assert_init_states("abcde", -1, "", "", None);

        assert_init_states("abcde", 0, "", "", None);
        assert_init_states("", 0, "", "", Some(0));

        assert_init_states("a", 1, "a", "a", Some(1));
        assert_init_states("abc", 1, "…", "…", Some(1));
        // Note comment in to_view to understand why
        // this is a single ellipsis instead of a
        // replacement character.
        assert_init_states("🦀", 1, "…", "…", Some(1));

        assert_init_states("abc", 2, "a…", "…c", Some(2));
        assert_init_states("ab", 2, "ab", "ab", Some(2));

        assert_init_states("🦀abc", 2, "�…", "…c", Some(2));
        assert_init_states("abc🦀", 2, "a…", "…�", Some(2));

        assert_init_states("abc", 3, "abc", "abc", Some(3));
        assert_init_states("abcd", 3, "ab…", "…cd", Some(3));

        assert_init_states("🦀🦀abc🦀🦀", 3, "🦀…", "…🦀", Some(3));
        assert_init_states("🦀🦀abc🦀🦀", 5, "🦀🦀…", "…🦀🦀", Some(5));

        // Since we're showing a normal character, these don't use the
        // replacement, so the lengths are different for front vs. back.
        assert_init_start("a🦀bc", 3, "a…", Some(2));
        assert_init_back("a🦀bc", 3, "…bc", Some(3));

        assert_init_start("ab🦀c", 3, "ab…", Some(3));
        assert_init_back("ab🦀c", 3, "…c", Some(2));
    }

    #[test]
    fn test_scroll_states() {
        let s = "abcdef";
        assert_scroll_states(s, 5, vec!["abcd…", "…cdef"]);

        let s = "abcdefgh";
        assert_scroll_states(s, 5, vec!["abcd…", "…cde…", "…def…", "…efgh"]);

        let s = "🦀bcde";
        assert_scroll_states(s, 5, vec!["🦀bc…", "…bcde"]);

        let s = "🦀bcdef";
        assert_scroll_states(s, 5, vec!["🦀bc…", "…bcd…", "…cdef"]);

        let s = "abcd🦀efghi";
        assert_scroll_states(s, 5, vec!["abcd…", "…d🦀…", "…🦀e…", "…efg…", "…fghi"]);

        let s = "abc🦀def";
        assert_scroll_states(s, 3, vec!["ab…", "…c…", "…�…", "…d…", "…ef"]);

        let s = "🦀z";
        assert_scroll_states(s, 2, vec!["�…", "…z"]);

        let s = "a🦀";
        assert_scroll_states(s, 2, vec!["a…", "…�"]);
    }

    #[track_caller]
    fn assert_scroll_states(s: &str, available_space: isize, states: Vec<&str>) {
        let mut curr_state = TruncatedStrView::init_start(s, available_space);
        let mut prev_formatted = rendered(s, &curr_state);
        assert_eq!(states[0], prev_formatted);

        for expected_state in states.iter().skip(1) {
            let next_state = curr_state.scroll_right(s, 1);
            let formatted = rendered(s, &next_state);

            assert_eq!(
                expected_state, &formatted,
                "expected scroll_right({}) to be {}",
                prev_formatted, expected_state,
            );
            curr_state = next_state;
            prev_formatted = formatted;
        }

        let mut curr_state = TruncatedStrView::init_back(s, available_space);
        let mut prev_formatted = rendered(s, &curr_state);
        assert_eq!(states.last().unwrap(), &prev_formatted);

        for expected_state in states.iter().rev().skip(1) {
            let next_state = curr_state.scroll_left(s, 1);
            let formatted = rendered(s, &next_state);

            assert_eq!(
                expected_state, &formatted,
                "expected scroll_left({}) to be {}",
                prev_formatted, expected_state,
            );
            curr_state = next_state;
            prev_formatted = formatted;
        }
    }

    #[test]
    fn test_expand() {
        let s = "abcdefghij";

        assert_resize(
            s,
            TruncatedStrView::init_start(s, 5),
            &[
                "abcd…",
                "abcde…",
                "abcdef…",
                "abcdefg…",
                "abcdefgh…",
                "abcdefghij",
            ],
            TruncatedStrView::expand,
            1,
        );

        let initial_state = TruncatedStrView::init_start(s, 5).scroll_right(s, 2);

        assert_resize(
            s,
            initial_state,
            &[
                "…def…",
                "…defg…",
                "…defgh…",
                "…defghij",
                "…cdefghij",
                "abcdefghij",
            ],
            TruncatedStrView::expand,
            1,
        );

        let s = "a👍b👀c😱d";
        assert_resize(
            s,
            TruncatedStrView::init_start(s, 5),
            &[
                "a👍b…",
                "a👍b…",
                "a👍b👀…",
                "a👍b👀c…",
                "a👍b👀c…",
                "a👍b👀c😱d",
            ],
            TruncatedStrView::expand,
            1,
        );
        let s = "a👍b👀c😱d";

        assert_resize(
            s,
            TruncatedStrView::init_start(s, 5).scroll_right(s, 2),
            &[
                "…👀c…",
                "…👀c…",
                "…👀c😱d",
                "…b👀c😱d",
                "…b👀c😱d",
                "a👍b👀c😱d",
            ],
            TruncatedStrView::expand,
            1,
        );
    }

    #[test]
    fn test_shrink_ascii() {
        let s = "abcdefghij";

        assert_resize(
            s,
            TruncatedStrView::init_start(s, 10),
            &[
                "abcdefghij",
                "abcdefgh…",
                "abcdefg…",
                "abcdef…",
                "abcde…",
                "abcd…",
                "abc…",
                "ab…",
                "a…",
                "…",
            ],
            TruncatedStrView::shrink,
            -1,
        );

        assert_resize(
            s,
            TruncatedStrView::init_start(s, 9).scroll_right(s, 1),
            &[
                "…cdefghij",
                "…defghij",
                "…efghij",
                "…fghij",
                "…ghij",
                "…hij",
                "…ij",
                "…j",
                "…",
            ],
            TruncatedStrView::shrink,
            -1,
        );

        assert_resize(
            s,
            TruncatedStrView::init_start(s, 8).scroll_right(s, 1),
            &[
                "…cdefgh…",
                "…cdefg…",
                "…cdef…",
                "…cde…",
                "…cd…",
                "…c…",
                "a…",
            ],
            TruncatedStrView::shrink,
            -1,
        );
    }

    #[test]
    fn test_shrink_wide_graphemes() {
        let s = "ab👍c👀d😱efg";
        assert_resize(
            s,
            TruncatedStrView::init_start(s, 11).scroll_right(s, 1),
            &[
                "…👍c👀d😱e…",
                "…👍c👀d😱…",
                "…👍c👀d…",
                "…👍c👀d…",
                "…👍c👀…",
                "…👍c…",
                "…👍c…",
                "…👍…",
                "…�…",
                "a…",
            ],
            TruncatedStrView::shrink,
            -1,
        );

        let s = "🦀abc";
        assert_resize(
            s,
            TruncatedStrView::init_start(s, 5),
            &["🦀abc", "🦀a…", "🦀…", "�…", "…"],
            TruncatedStrView::shrink,
            -1,
        );

        let s = "abc🦀";
        assert_resize(
            s,
            TruncatedStrView::init_back(s, 4),
            &["…c🦀", "…🦀", "…�", "…"],
            TruncatedStrView::shrink,
            -1,
        );
    }

    #[track_caller]
    fn assert_resize(
        string: &str,
        mut view: TruncatedStrView,
        states: &[&str],
        resize: fn(&TruncatedStrView, &str, isize) -> TruncatedStrView,
        step: isize,
    ) {
        assert_eq!(states[0], rendered(string, &view));
        for &expected in &states[1..] {
            let previous = rendered(string, &view);
            let width = view.available_space + step;
            view = resize(&view, string, width);
            assert_eq!(
                expected,
                rendered(string, &view),
                "resizing {previous:?} to {width}"
            );
        }
    }

    #[test]
    fn test_focus() {
        let s = "0123456789";
        let tsv = TruncatedStrView::init_start(s, 10);

        assert_focuses(
            s,
            tsv,
            vec![
                (&(0..1), "0123456789"),
                (&(4..7), "0123456789"),
                (&(8..12), "0123456789"),
            ],
        );

        let s = "0123456789abc";
        // "…34567…"
        let tsv = TruncatedStrView::init_start(s, 7).scroll_right(s, 2);

        assert_focuses(
            s,
            tsv,
            vec![
                // Focus range if not visible.
                (&(0..1), "012345…"),
                // Don't move if entire range is visible.
                (&(3..4), "…34567…"),
                (&(3..8), "…34567…"),
                (&(7..8), "…34567…"),
                // Center focused value if not all visible.
                (&(6..9), "…56789…"),
                // Start at beginning if can't fit whole focused range.
                (&(2..9), "…23456…"),
                // Focus range if not visible.
                (&(10..15), "…789abc"),
            ],
        );
    }

    #[track_caller]
    fn assert_focuses(
        string: &str,
        initial_state: TruncatedStrView,
        ranges_and_expected: Vec<(&Range<usize>, &str)>,
    ) {
        let initial_formatted = rendered(string, &initial_state);

        for (i, (range, expected_focused)) in ranges_and_expected.into_iter().enumerate() {
            let focused = initial_state.focus(string, range);
            let formatted = rendered(string, &focused);

            assert_eq!(
                expected_focused, &formatted,
                "Case {}: expected focus({}, {}..{}) to be {}",
                i, initial_formatted, range.start, range.end, expected_focused,
            );
        }
    }
}
