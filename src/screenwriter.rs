use std::collections::HashMap;
use std::fmt::Write;
use std::ops::Range;

use rustyline::Editor;
use termion::raw::RawTerminal;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::app::MAX_BUFFER_SIZE;
use crate::flatjson::{Index, PathType};
use crate::lineprinter as lp;
use crate::options::Opt;
use crate::search::SearchState;
use crate::terminal;
use crate::terminal::{AnsiTerminal, Terminal};
use crate::truncatedstrview::{TruncatedStrSlice, TruncatedStrView};
use crate::types::TTYDimensions;
use crate::viewer::{Action, JsonViewer};

pub struct ScreenWriter {
    pub stdout: RawTerminal<Box<dyn std::io::Write>>,
    pub command_editor: Editor<()>,
    pub dimensions: TTYDimensions,
    pub terminal: AnsiTerminal,

    pub show_line_numbers: bool,
    pub show_relative_line_numbers: bool,

    indentation_reduction: u16,
    last_focus: Option<(usize, usize, u16, usize)>,
    layout_generation: usize,
    horizontal_offsets: HashMap<Index, usize>,
}

pub enum MessageSeverity {
    Info,
    Warn,
    Error,
}

impl MessageSeverity {
    pub fn color(&self) -> terminal::Color {
        match self {
            MessageSeverity::Info => terminal::WHITE,
            MessageSeverity::Warn => terminal::YELLOW,
            MessageSeverity::Error => terminal::RED,
        }
    }
}

const SPACE_BETWEEN_PATH_AND_FILENAME: isize = 3;

impl ScreenWriter {
    pub fn init(
        options: &Opt,
        stdout: RawTerminal<Box<dyn std::io::Write>>,
        command_editor: Editor<()>,
        dimensions: TTYDimensions,
    ) -> Self {
        ScreenWriter {
            stdout,
            command_editor,
            dimensions,
            terminal: AnsiTerminal::new(String::new()),
            show_line_numbers: options.show_line_numbers,
            show_relative_line_numbers: options.show_relative_line_numbers,
            indentation_reduction: 0,
            last_focus: None,
            layout_generation: 0,
            horizontal_offsets: HashMap::new(),
        }
    }

    pub fn print(
        &mut self,
        viewer: &mut JsonViewer,
        input_buffer: &[u8],
        input_filename: &str,
        search_state: &SearchState,
        message: &Option<(String, MessageSeverity)>,
    ) {
        self.print_viewer(viewer, search_state);
        self.print_status_bar(viewer, input_buffer, input_filename, search_state, message);
    }

    fn sync_layout(&mut self, viewer: &JsonViewer) {
        if self.layout_generation != viewer.layout_generation {
            self.horizontal_offsets.clear();
            self.last_focus = None;
            self.layout_generation = viewer.layout_generation;
        }
    }

    pub fn print_viewer(&mut self, viewer: &mut JsonViewer, search_state: &SearchState) {
        self.sync_layout(viewer);
        let focus = (
            viewer.focused_node,
            viewer.absolute_anchor_line,
            self.dimensions.width,
            viewer.physical_generation,
        );
        if self.last_focus != Some(focus) {
            if search_state.active_search_state().is_some() {
                self.scroll_line_to_search_match(viewer, search_state.current_match_range());
            } else {
                self.reveal_focused_span(viewer);
            }
            self.last_focus = Some(focus);
        }
        match self.print_screen_impl(viewer, search_state) {
            Ok(_) => match self.terminal.flush_contents(&mut self.stdout) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error while printing viewer: {e}");
                }
            },
            Err(e) => {
                eprintln!("Error while printing viewer: {e}");
            }
        }
    }

    pub fn print_status_bar(
        &mut self,
        viewer: &JsonViewer,
        input_buffer: &[u8],
        input_filename: &str,
        search_state: &SearchState,
        message: &Option<(String, MessageSeverity)>,
    ) {
        match self.print_status_bar_impl(
            viewer,
            input_buffer,
            input_filename,
            search_state,
            message,
        ) {
            Ok(_) => match self.terminal.flush_contents(&mut self.stdout) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error while printing status bar: {e}");
                }
            },
            Err(e) => {
                eprintln!("Error while printing status bar: {e}");
            }
        }
    }

    pub fn sync_wrap_geometry(&self, viewer: &mut JsonViewer) -> bool {
        let width =
            usize::from(self.dimensions.width).saturating_sub(self.number_width(viewer) + 2);
        viewer.set_wrap_geometry(width, usize::from(self.indentation_reduction) * 2)
    }

    /// Explicit viewport scrolling has already chosen its physical position.
    pub fn accept_viewport_focus(&mut self, viewer: &JsonViewer) {
        self.last_focus = Some((
            viewer.focused_node,
            viewer.absolute_anchor_line,
            self.dimensions.width,
            viewer.physical_generation,
        ));
    }

    pub fn invalidate_focus(&mut self) {
        self.last_focus = None;
    }

    pub fn reset_horizontal_offsets(&mut self, viewer: &JsonViewer) {
        self.horizontal_offsets.retain(|absolute, _| {
            viewer
                .layout
                .lines
                .get(*absolute)
                .is_some_and(|line| viewer.flatjson[line.owner].is_collapsed())
        });
        self.last_focus = None;
    }

    fn number_width(&self, viewer: &JsonViewer) -> usize {
        if self.show_line_numbers || self.show_relative_line_numbers {
            viewer.layout.lines.len().to_string().len().max(2) + 1
        } else {
            0
        }
    }

    fn line_viewport(&self, line: &crate::toon_display::VisibleLine) -> lp::LineViewport {
        lp::LineViewport::new(
            &line.line,
            self.horizontal_offsets
                .get(&line.absolute)
                .copied()
                .unwrap_or(0),
            usize::from(self.indentation_reduction) * 2,
        )
    }

    pub fn mouse_action(&self, viewer: &JsonViewer, row: u16, column: u16) -> Action {
        let Some(physical) = viewer.screen_row(usize::from(row.saturating_sub(1))) else {
            return Action::NoOp;
        };
        let index = physical.logical_line;
        let number_width = self.number_width(viewer);
        let column = usize::from(column.saturating_sub(1));
        if !physical.first && column < number_width + 2 {
            return Action::NoOp;
        }
        if viewer.is_wrapped_line(index) && column >= number_width + 2 {
            let line = &viewer.visible[index].line;
            let (node, source) =
                lp::hit_test_wrapped(line, physical, column.saturating_sub(number_width + 2));
            return Action::FocusNodeAt {
                node,
                source,
                display_range: (physical.bytes.start, physical.bytes.end),
            };
        }
        if column >= number_width && column < number_width + 2 {
            Action::ClickArrow(row)
        } else {
            let viewport = self.line_viewport(&viewer.visible[index]);
            let column = viewport.source_column(column.saturating_sub(number_width + 2));
            let line = &viewer.visible[index].line;
            let fitted = if viewport.horizontal_offset == 0 {
                lp::fit_annotations(
                    line,
                    usize::from(self.dimensions.width).saturating_sub(number_width + 2)
                        + viewport.removed_indentation,
                )
            } else {
                std::borrow::Cow::Borrowed(line)
            };
            let (node, source) = lp::hit_test(&fitted, column);
            Action::FocusNode { node, source }
        }
    }

    fn print_screen_impl(
        &mut self,
        viewer: &JsonViewer,
        search_state: &SearchState,
    ) -> std::fmt::Result {
        let matches = search_state.matches_iter(0).as_slice();
        let current = search_state.current_match_range();
        let focused = viewer.focused_line_index();
        let number_width = self.number_width(viewer);
        for screen in 0..viewer.dimensions.height {
            self.terminal.position_cursor(1, screen + 1)?;
            self.terminal.clear_line()?;
            self.terminal.reset_style()?;
            let Some(physical) = viewer.screen_row(usize::from(screen)) else {
                self.terminal.set_fg(terminal::LIGHT_BLACK)?;
                self.terminal.write_char('~')?;
                continue;
            };
            let index = physical.logical_line;
            let visible = &viewer.visible[index];
            let line = &visible.line;
            if number_width > 0 {
                let relative = viewer.visible[index.min(focused)..index.max(focused)]
                    .iter()
                    .filter(|line| !line.line.separator)
                    .count();
                let number = if self.show_relative_line_numbers
                    && (index != focused || !self.show_line_numbers)
                {
                    relative
                } else {
                    visible.absolute + 1
                };
                self.terminal.set_fg(if index == focused {
                    terminal::WHITE
                } else {
                    terminal::LIGHT_BLACK
                })?;
                let label = if physical.first {
                    format!("{:>width$} ", number, width = number_width - 1)
                } else {
                    " ".repeat(number_width)
                };
                self.terminal
                    .write_str(&label[..label.len().min(usize::from(self.dimensions.width))])?;
            }
            let available = usize::from(self.dimensions.width).saturating_sub(number_width);
            if available == 0 {
                continue;
            }
            self.terminal.reset_style()?;
            self.terminal.set_fg(if index == focused {
                terminal::WHITE
            } else {
                terminal::LIGHT_BLACK
            })?;
            let arrow =
                if physical.first && viewer.layout.nodes[line.owner].collapsible && !line.separator
                {
                    if viewer.flatjson[line.owner].is_collapsed()
                        || viewer.layout.nodes[line.owner].inline_array
                    {
                        '▸'
                    } else {
                        '▾'
                    }
                } else {
                    ' '
                };
            self.terminal.write_char(arrow)?;
            if available == 1 {
                continue;
            }
            self.terminal.write_char(' ')?;
            let focused_nodes = if index == focused {
                viewer.focused_node..match viewer.flatjson[viewer.focused_node].pair_index() {
                    crate::flatjson::OptionIndex::Index(end) => end + 1,
                    _ => viewer.focused_node + 1,
                }
            } else {
                0..0
            };
            if viewer.is_wrapped_line(index) {
                lp::paint_wrapped(
                    &mut self.terminal,
                    line,
                    focused_nodes,
                    physical,
                    available - 2,
                    matches,
                    &current,
                )?;
                continue;
            }
            let viewport = self.line_viewport(visible);
            let fitted = if viewport.horizontal_offset == 0 {
                lp::fit_annotations(line, available - 2 + viewport.removed_indentation)
            } else {
                std::borrow::Cow::Borrowed(line)
            };
            lp::paint(
                &mut self.terminal,
                &fitted,
                focused_nodes,
                viewport,
                available - 2,
                matches,
                &current,
            )?;
        }
        Ok(())
    }

    pub fn get_command(&mut self, prompt: &str) -> rustyline::Result<String> {
        write!(self.stdout, "{}", termion::cursor::Show)?;
        let _ = self.terminal.position_cursor(1, self.dimensions.height);
        self.terminal.flush_contents(&mut self.stdout)?;

        let result = self.command_editor.readline(prompt);
        write!(self.stdout, "{}", termion::cursor::Hide)?;

        let _ = self.terminal.position_cursor(1, self.dimensions.height);
        let _ = self.terminal.clear_line();
        self.terminal.flush_contents(&mut self.stdout)?;

        result
    }

    fn print_status_bar_impl(
        &mut self,
        viewer: &JsonViewer,
        input_buffer: &[u8],
        input_filename: &str,
        search_state: &SearchState,
        message: &Option<(String, MessageSeverity)>,
    ) -> std::fmt::Result {
        self.terminal
            .position_cursor(1, self.dimensions.height.saturating_sub(1).max(1))?;
        self.terminal.clear_line()?;
        self.terminal.set_style(&terminal::Style {
            fg: terminal::BLACK,
            bg: terminal::LIGHT_BLACK,
            ..terminal::Style::default()
        })?;
        // Need to print a line to ensure the entire bar with the path to
        // the node and the filename is highlighted.
        for _ in 0..self.dimensions.width {
            self.terminal.write_char(' ')?;
        }
        self.terminal.write_char('\r')?;

        let mut path_to_node = viewer
            .flatjson
            .build_path_to_node(PathType::DotWithTopLevelIndex, viewer.focused_node)
            .unwrap();
        if path_to_node.is_empty() {
            path_to_node.push('.');
        }
        let node = &viewer.layout.nodes[viewer.focused_node];
        if let (Some(occurrence), Some(total)) = (node.occurrence, node.occurrence_total) {
            write!(path_to_node, " (occurrence {occurrence} of {total})")?;
        }
        if viewer.visible[viewer.focused_line_index()]
            .line
            .text
            .is_empty()
        {
            path_to_node.push_str(" (empty object)");
        }
        self.print_path_to_node_and_file_name(
            &path_to_node,
            input_filename,
            viewer.dimensions.width as isize,
        )?;

        self.terminal.position_cursor(1, self.dimensions.height)?;
        self.terminal.clear_line()?;

        if let Some((contents, severity)) = message {
            self.terminal.set_style(&terminal::Style {
                fg: severity.color(),
                ..terminal::Style::default()
            })?;
            self.terminal.write_str(contents)?;
        } else if search_state.showing_matches() {
            self.terminal
                .write_char(search_state.direction.prompt_char())?;
            self.terminal.write_str(&search_state.search_term)?;

            if let Some((match_num, just_wrapped)) = search_state.active_search_state() {
                // Print out which match we're on:
                let match_tracker = format!("[{}/{}]", match_num + 1, search_state.num_matches());
                self.terminal.position_cursor(
                    self.dimensions
                        .width
                        .saturating_sub(1 + MAX_BUFFER_SIZE as u16 + 6 + match_tracker.len() as u16)
                        .max(1),
                    self.dimensions.height,
                )?;

                let wrapped_char = if just_wrapped { 'W' } else { ' ' };
                write!(self.terminal, " {wrapped_char} {match_tracker}")?;
            }
        } else {
            write!(self.terminal, ":")?;
        }

        self.terminal.position_cursor(
            // TODO: This can overflow on very skinny screens (2-3 columns).
            self.dimensions
                .width
                .saturating_sub(1 + MAX_BUFFER_SIZE as u16)
                .max(1),
            self.dimensions.height,
        )?;
        self.terminal
            .write_str(std::str::from_utf8(input_buffer).unwrap())?;

        // Position the cursor better for random debugging prints. (2 so it's after ':')
        self.terminal.position_cursor_col(2)?;

        Ok(())
    }

    fn print_path_to_node_and_file_name(
        &mut self,
        path_to_node: &str,
        filename: &str,
        width: isize,
    ) -> std::fmt::Result {
        let path_display_width = UnicodeWidthStr::width(path_to_node) as isize;
        let row = self.dimensions.height.saturating_sub(1).max(1);

        let space_available_for_filename =
            width - path_display_width - SPACE_BETWEEN_PATH_AND_FILENAME;

        let status_style = terminal::Style {
            fg: terminal::BLACK,
            bg: terminal::LIGHT_BLACK,
            ..terminal::Style::default()
        };

        let truncated_filename =
            TruncatedStrView::init_start(filename, space_available_for_filename);

        self.terminal.position_cursor(1, row)?;
        self.terminal.set_style(&status_style)?;
        let path_slice = TruncatedStrSlice {
            s: path_to_node,
            truncated_view: &TruncatedStrView::init_back(path_to_node, width),
        };
        write!(self.terminal, "{path_slice}")?;

        if truncated_filename.any_contents_visible() {
            let filename_width = truncated_filename.used_space().unwrap();

            self.terminal
                .position_cursor(self.dimensions.width - (filename_width as u16) + 1, row)?;
            self.terminal.set_style(&terminal::Style {
                fg: terminal::WHITE,
                ..status_style
            })?;

            let truncated_slice = TruncatedStrSlice {
                s: filename,
                truncated_view: &truncated_filename,
            };

            write!(self.terminal, "{truncated_slice}")?;
        }

        Ok(())
    }

    pub fn decrease_indentation_level(&mut self, max_depth: u16) {
        self.indentation_reduction = self.indentation_reduction.saturating_add(1).min(max_depth);
    }

    pub fn increase_indentation_level(&mut self) {
        self.indentation_reduction = self.indentation_reduction.saturating_sub(1)
    }

    fn reveal_focused_span(&mut self, viewer: &mut JsonViewer) {
        let line = &viewer.visible[viewer.focused_line_index()].line;
        if let Some(span) = line
            .spans
            .iter()
            .find(|span| span.node == viewer.focused_node && span.source.is_some())
        {
            self.reveal_byte_range(viewer, span.range.clone());
        }
    }

    fn reveal_byte_range(&mut self, viewer: &mut JsonViewer, range: Range<usize>) {
        if viewer.is_wrapped_line(viewer.focused_line_index()) {
            viewer.reveal_byte_range(range);
            return;
        }
        let line = &viewer.visible[viewer.focused_line_index()].line;
        let start_byte = line
            .text
            .grapheme_indices(true)
            .find(|(byte, text)| byte + text.len() > range.start)
            .map_or(range.start, |(byte, _)| byte);
        let end_byte = line
            .text
            .grapheme_indices(true)
            .find(|(byte, text)| byte + text.len() >= range.end)
            .map_or(range.end, |(byte, text)| byte + text.len());
        let viewport = self.line_viewport(&viewer.visible[viewer.focused_line_index()]);
        let start = viewport.reduced_column(UnicodeWidthStr::width(&line.text[..start_byte]));
        let end = viewport.reduced_column(UnicodeWidthStr::width(&line.text[..end_byte]));
        let document_width =
            usize::from(self.dimensions.width).saturating_sub(self.number_width(viewer) + 2);
        let visible_columns = viewport.visible_columns(line, document_width);
        let offset = self
            .horizontal_offsets
            .entry(viewer.absolute_anchor_line)
            .or_default();
        if start < visible_columns.start || end > visible_columns.end {
            *offset = start;
        }
    }

    pub fn scroll_focused_line_right(&mut self, viewer: &JsonViewer, count: usize) {
        self.scroll_focused_line(viewer, count, true);
    }

    pub fn scroll_focused_line_left(&mut self, viewer: &JsonViewer, count: usize) {
        self.scroll_focused_line(viewer, count, false);
    }

    fn scroll_focused_line(&mut self, viewer: &JsonViewer, count: usize, right: bool) {
        if viewer.is_wrapped_line(viewer.focused_line_index()) {
            return;
        }
        let absolute = viewer.absolute_anchor_line;
        let line = &viewer.visible[viewer.focused_line_index()];
        let width = self.line_viewport(line).content_width(&line.line);
        let offset = self.horizontal_offsets.entry(absolute).or_default();
        *offset = if right {
            offset.saturating_add(count).min(width.saturating_sub(1))
        } else {
            offset.saturating_sub(count)
        };
    }

    pub fn scroll_focused_line_to_an_end(&mut self, viewer: &JsonViewer) {
        if viewer.is_wrapped_line(viewer.focused_line_index()) {
            return;
        }
        let absolute = viewer.absolute_anchor_line;
        let line = &viewer.visible[viewer.focused_line_index()];
        let width = self.line_viewport(line).content_width(&line.line);
        let available =
            usize::from(self.dimensions.width).saturating_sub(self.number_width(viewer) + 2);
        let offset = self.horizontal_offsets.entry(absolute).or_default();
        let end = end_scroll_offset(width, available);
        *offset = if *offset < end { end } else { 0 };
    }

    pub fn scroll_line_to_search_match(&mut self, viewer: &mut JsonViewer, range: Range<usize>) {
        self.sync_layout(viewer);
        let line = &viewer.visible[viewer.focused_line_index()].line;
        let target = line
            .spans
            .iter()
            .filter(|span| span.node == viewer.focused_node)
            .find_map(|span| span.matching_ranges(&range).into_iter().next());
        if let Some(target) = target {
            self.reveal_byte_range(viewer, target);
            // The match may lie deep inside the token; generic node focus must
            // not move the next paint back to that token's beginning.
            self.last_focus = Some((
                viewer.focused_node,
                viewer.absolute_anchor_line,
                self.dimensions.width,
                viewer.physical_generation,
            ));
        }
    }
}

fn end_scroll_offset(width: usize, available: usize) -> usize {
    width
        .saturating_sub(available.saturating_sub(1))
        .min(width.saturating_sub(1))
}

#[cfg(test)]
mod tests {
    use super::end_scroll_offset;

    #[test]
    fn end_scroll_stays_inside_content_at_narrow_widths() {
        for available in [0, 1, 2] {
            assert_eq!(end_scroll_offset(10, available), 9);
        }
        assert_eq!(end_scroll_offset(0, 0), 0);
        assert_eq!(end_scroll_offset(1, 0), 0);
        assert_eq!(end_scroll_offset(10, 5), 6);
        assert_eq!(end_scroll_offset(10, 20), 0);
    }
}
