use std::fmt;
use std::iter::Peekable;
use std::ops::Range;

use crate::search::MatchRangeIter;
use crate::terminal::Terminal;
use crate::theme::{SearchState, StyleRole, StyleState, Theme};
use crate::truncatedstrview::TruncatedStrView;

#[allow(clippy::too_many_arguments)]
pub fn highlight_truncated_str_view(
    out: &mut dyn Terminal,
    mut s: &str,
    str_view: &TruncatedStrView,
    mut str_range_start: Option<usize>,
    theme: &Theme,
    role: StyleRole,
    state: StyleState,
    matches_iter: &mut Option<&mut Peekable<MatchRangeIter<'_>>>,
    focused_search_match: &Range<usize>,
) -> fmt::Result {
    let mut leading_ellipsis = false;
    let mut replacement_character = false;
    let mut trailing_ellipsis = false;

    if let Some(tr) = str_view.range {
        leading_ellipsis = tr.print_leading_ellipsis();
        replacement_character = tr.showing_replacement_character;
        trailing_ellipsis = tr.print_trailing_ellipsis(s);
        s = &s[tr.start..tr.end];
        str_range_start = str_range_start.map(|start| start + tr.start);
    }

    if leading_ellipsis {
        out.set_style(&theme.style(StyleRole::Ellipsis, state))?;
        out.write_char('…')?;
    }

    if replacement_character {
        out.set_style(&theme.style(role, state))?;
        out.write_char('�')?;
    }

    highlight_matches(
        out,
        s,
        str_range_start,
        theme,
        role,
        state,
        matches_iter,
        focused_search_match,
    )?;

    if trailing_ellipsis {
        out.set_style(&theme.style(StyleRole::Ellipsis, state))?;
        out.write_char('…')?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn highlight_matches(
    out: &mut dyn Terminal,
    mut s: &str,
    str_range_start: Option<usize>,
    theme: &Theme,
    role: StyleRole,
    state: StyleState,
    matches_iter: &mut Option<&mut Peekable<MatchRangeIter<'_>>>,
    focused_search_match: &Range<usize>,
) -> fmt::Result {
    let style = theme.style(role, state);

    if str_range_start.is_none() {
        out.set_style(&style)?;
        write!(out, "{s}")?;
        return Ok(());
    }

    let mut start_index = str_range_start.unwrap();

    while !s.is_empty() {
        let string_end = start_index + s.len();
        let mut match_start = string_end;
        let mut match_end = string_end;
        let mut match_is_focused_match = false;

        while let Some(range) = matches_iter.as_mut().and_then(|i| i.peek()) {
            if start_index < range.end {
                if *range == focused_search_match {
                    match_is_focused_match = true;
                }

                match_start = range.start.clamp(start_index, string_end);
                match_end = range.end.clamp(start_index, string_end);
                break;
            }
            matches_iter.as_mut().unwrap().next();
        }

        if start_index < match_start {
            let print_end = match_start - start_index;
            out.set_style(&style)?;
            write!(out, "{}", &s[..print_end])?;
        }

        if match_start < string_end {
            let search = if match_is_focused_match {
                SearchState::CurrentMatch
            } else {
                SearchState::Match
            };
            out.set_style(&theme.style(role, state.search(search)))?;
            let print_start = match_start - start_index;
            let print_end = match_end - start_index;
            write!(out, "{}", &s[print_start..print_end])?;
        }

        s = &s[(match_end - start_index)..];
        start_index = match_end;
    }

    Ok(())
}
