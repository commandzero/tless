use unicode_width::UnicodeWidthChar;

pub(super) fn strip_styles(output: &str) -> String {
    regex::Regex::new(r"\x1b\[[0-?]*[ -/]*[@-~]")
        .unwrap()
        .replace_all(output, "")
        .into_owned()
}

// The PTY transcript contains cursor-addressed screen updates rather than
// newline-delimited output. Reconstruct the cells used by rendering assertions.
pub(super) fn rendered_rows(mut output: &str, width: u16, height: u16) -> Vec<String> {
    let mut screen = Screen::new(width.into(), height.into());
    while let Some(character) = output.chars().next() {
        output = &output[character.len_utf8()..];
        match character {
            '\x1b' => output = screen.consume_escape(output),
            '\r' => screen.column = 0,
            '\n' => screen.row = screen.row.saturating_add(1).min(screen.last_row()),
            '\x08' => screen.column = screen.column.saturating_sub(1),
            '\x07' => {}
            _ => screen.paint(character),
        }
    }

    screen
        .rows
        .into_iter()
        .map(|line| line.into_iter().collect::<String>().trim_end().to_string())
        .collect()
}

struct Screen {
    rows: Vec<Vec<char>>,
    width: usize,
    row: usize,
    column: usize,
    saved_cursor: (usize, usize),
}

impl Screen {
    fn new(width: usize, height: usize) -> Self {
        Self {
            rows: vec![vec![' '; width]; height],
            width,
            row: 0,
            column: 0,
            saved_cursor: (0, 0),
        }
    }

    fn last_row(&self) -> usize {
        self.rows.len().saturating_sub(1)
    }

    fn last_column(&self) -> usize {
        self.width.saturating_sub(1)
    }

    fn consume_escape<'a>(&mut self, output: &'a str) -> &'a str {
        let Some(sequence) = output.strip_prefix('[') else {
            // Other escape introducers emitted by tless do not paint cells.
            return output.get(1..).unwrap_or_default();
        };
        let Some(end) = sequence
            .bytes()
            .position(|byte| (0x40..=0x7e).contains(&byte))
        else {
            return "";
        };
        let mut parameters = sequence[..end]
            .trim_start_matches('?')
            .split(';')
            .map(|part| part.parse::<usize>().unwrap_or(0));
        let first = parameters.next().unwrap_or(0);
        let second = parameters.next().unwrap_or(0);
        self.apply_csi(sequence.as_bytes()[end], first, second);
        &sequence[end + 1..]
    }

    fn apply_csi(&mut self, command: u8, first: usize, second: usize) {
        match command {
            b'H' | b'f' => {
                self.row = first.saturating_sub(1).min(self.last_row());
                self.column = second.saturating_sub(1).min(self.last_column());
            }
            b'G' | b'`' => self.column = first.saturating_sub(1).min(self.last_column()),
            b'A' => self.row = self.row.saturating_sub(first.max(1)),
            b'B' | b'e' => self.row = self.row.saturating_add(first.max(1)).min(self.last_row()),
            b'C' | b'a' => {
                self.column = self
                    .column
                    .saturating_add(first.max(1))
                    .min(self.last_column());
            }
            b'D' => self.column = self.column.saturating_sub(first.max(1)),
            b'J' => self.erase_display(first),
            b'K' => self.erase_line(first),
            b's' => self.saved_cursor = (self.row, self.column),
            b'u' => (self.row, self.column) = self.saved_cursor,
            _ => {}
        }
    }

    fn erase_display(&mut self, mode: usize) {
        match mode {
            0 => self.rows[self.row][self.column..].fill(' '),
            2 | 3 => self.rows.iter_mut().for_each(|line| line.fill(' ')),
            _ => {}
        }
    }

    fn erase_line(&mut self, mode: usize) {
        let last_column = self.last_column();
        let line = &mut self.rows[self.row];
        match mode {
            1 => line[..=self.column.min(last_column)].fill(' '),
            2 => line.fill(' '),
            _ => line[self.column..].fill(' '),
        }
    }

    fn paint(&mut self, character: char) {
        let Some(line) = self.rows.get_mut(self.row) else {
            return;
        };
        let cells = UnicodeWidthChar::width(character).unwrap_or(0);
        if cells > 0 && self.column < self.width {
            let end = self.column.saturating_add(cells).min(self.width);
            line[self.column] = character;
            line[self.column + 1..end].fill(' ');
            self.column = end;
        }
    }
}
