#![allow(dead_code)]

use std::fmt::{Result, Write};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Color {
    C16(u8),
    #[cfg(feature = "colorscheme")]
    C256(u8),
    #[cfg(feature = "colorscheme")]
    Rgb(u8, u8, u8),
    Default,
}

impl Color {
    pub fn bright(self) -> Self {
        match self {
            Color::C16(index @ 0..=7) => Color::C16(index + 8),
            #[cfg(feature = "colorscheme")]
            Color::C256(index) => Color::C256(index),
            Color::C16(8) | Color::Default => LIGHT_WHITE,
            color => color,
        }
    }
}

// Commented out colors are unused.
pub const BLACK: Color = Color::C16(0);
pub const RED: Color = Color::C16(1);
pub const GREEN: Color = Color::C16(2);
pub const YELLOW: Color = Color::C16(3);
pub const BLUE: Color = Color::C16(4);
pub const MAGENTA: Color = Color::C16(5);
pub const CYAN: Color = Color::C16(6);
pub const WHITE: Color = Color::C16(7);
pub const LIGHT_BLACK: Color = Color::C16(8);
// pub const LIGHT_RED: Color = Color::C16(9);
// pub const LIGHT_GREEN: Color = Color::C16(10);
pub const LIGHT_YELLOW: Color = Color::C16(11);
pub const LIGHT_BLUE: Color = Color::C16(12);
// pub const LIGHT_MAGENTA: Color = Color::C16(13);
pub const LIGHT_WHITE: Color = Color::C16(15);
pub const LIGHT_CYAN: Color = Color::C16(14);
pub const DEFAULT: Color = Color::Default;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub inverted: bool,
    pub bold: bool,
    pub dimmed: bool,
    pub underlined: bool,
}

impl Style {
    pub const fn default() -> Self {
        Style {
            fg: Color::Default,
            bg: Color::Default,
            inverted: false,
            bold: false,
            dimmed: false,
            underlined: false,
        }
    }
}

impl Default for Style {
    fn default() -> Self {
        Style::default()
    }
}

pub trait Terminal: Write {
    #[allow(dead_code)]
    fn clear_screen(&mut self) -> Result;
    fn clear_line(&mut self) -> Result;

    fn position_cursor(&mut self, col: u16, row: u16) -> Result;
    fn position_cursor_col(&mut self, col: u16) -> Result;

    fn set_style(&mut self, style: &Style) -> Result;
    fn reset_style(&mut self) -> Result;

    fn set_fg(&mut self, color: Color) -> Result;
    fn set_bg(&mut self, color: Color) -> Result;
    fn set_inverted(&mut self, inverted: bool) -> Result;
    fn set_bold(&mut self, bold: bool) -> Result;
    fn set_dimmed(&mut self, dimmed: bool) -> Result;
    fn set_underlined(&mut self, underlined: bool) -> Result;

    #[allow(dead_code)]
    fn output(&self) -> &str;

    // Only used for testing.
    #[allow(dead_code)]
    fn clear_output(&mut self);
}

pub struct AnsiTerminal {
    pub output: String,
    pub style: Style,
}

impl AnsiTerminal {
    pub fn new(output: String) -> Self {
        AnsiTerminal {
            output,
            style: Style::default(),
        }
    }

    pub fn flush_contents<W: std::io::Write>(&mut self, out: &mut W) -> std::io::Result<usize> {
        let bytes = out.write(self.output.as_bytes())?;
        out.flush()?;
        self.output.clear();
        Ok(bytes)
    }
}

impl Write for AnsiTerminal {
    fn write_str(&mut self, s: &str) -> Result {
        self.output.write_str(s)
    }
}

impl Terminal for AnsiTerminal {
    fn clear_screen(&mut self) -> Result {
        write!(self, "\x1b[2J")
    }

    fn clear_line(&mut self) -> Result {
        write!(self, "\x1b[2K")
    }

    fn position_cursor(&mut self, col: u16, row: u16) -> Result {
        write!(self, "\x1b[{row};{col}H")?;
        self.reset_style()
    }

    fn position_cursor_col(&mut self, col: u16) -> Result {
        write!(self, "\x1b[{col}G")?;
        self.reset_style()
    }

    fn set_style(&mut self, style: &Style) -> Result {
        self.set_fg(style.fg)?;
        self.set_bg(style.bg)?;
        self.set_inverted(style.inverted)?;
        self.set_bold(style.bold)?;
        self.set_dimmed(style.dimmed)?;
        self.set_underlined(style.underlined)?;
        Ok(())
    }

    fn reset_style(&mut self) -> Result {
        self.style = Style::default();
        write!(self, "\x1b[0m")
    }

    fn set_fg(&mut self, color: Color) -> Result {
        if self.style.fg != color {
            match color {
                Color::C16(c) => write!(self, "\x1b[38;5;{c}m")?,
                #[cfg(feature = "colorscheme")]
                Color::C256(c) => write!(self, "\x1b[38;5;{c}m")?,
                #[cfg(feature = "colorscheme")]
                Color::Rgb(r, g, b) => write!(self, "\x1b[38;2;{r};{g};{b}m")?,
                Color::Default => write!(self, "\x1b[39m")?,
            }
            self.style.fg = color;
        }
        Ok(())
    }

    fn set_bg(&mut self, color: Color) -> Result {
        if self.style.bg != color {
            match color {
                Color::C16(c) => write!(self, "\x1b[48;5;{c}m")?,
                #[cfg(feature = "colorscheme")]
                Color::C256(c) => write!(self, "\x1b[48;5;{c}m")?,
                #[cfg(feature = "colorscheme")]
                Color::Rgb(r, g, b) => write!(self, "\x1b[48;2;{r};{g};{b}m")?,
                Color::Default => write!(self, "\x1b[49m")?,
            }
            self.style.bg = color;
        }
        Ok(())
    }

    fn set_inverted(&mut self, inverted: bool) -> Result {
        if self.style.inverted != inverted {
            if inverted {
                write!(self, "\x1b[7m")?;
            } else {
                write!(self, "\x1b[27m")?;
            }
            self.style.inverted = inverted;
        }
        Ok(())
    }

    fn set_bold(&mut self, bold: bool) -> Result {
        if self.style.bold != bold {
            if bold {
                write!(self, "\x1b[1m")?;
            } else {
                write!(self, "\x1b[22m")?;
                // Also resets dimmed, so set that if we need to
                if self.style.dimmed {
                    write!(self, "\x1b[2m")?;
                }
            }
            self.style.bold = bold;
        }
        Ok(())
    }

    fn set_dimmed(&mut self, dimmed: bool) -> Result {
        if self.style.dimmed != dimmed {
            if dimmed {
                write!(self, "\x1b[2m")?;
            } else {
                write!(self, "\x1b[22m")?;
                // Also resets bold, so set that if we need to
                if self.style.bold {
                    write!(self, "\x1b[1m")?;
                }
            }
            self.style.dimmed = dimmed;
        }
        Ok(())
    }

    fn set_underlined(&mut self, underlined: bool) -> Result {
        if self.style.underlined != underlined {
            write!(self, "\x1b[{}m", if underlined { 4 } else { 24 })?;
            self.style.underlined = underlined;
        }
        Ok(())
    }

    fn output(&self) -> &str {
        &self.output
    }

    fn clear_output(&mut self) {
        self.output.clear()
    }
}

#[cfg(test)]
pub mod test {
    #[test]
    #[cfg(feature = "colorscheme")]
    fn rgb_colors_transition_to_indexed_and_reset() -> std::fmt::Result {
        let mut terminal = AnsiTerminal::new(String::new());
        terminal.set_fg(Color::Rgb(69, 168, 255))?;
        terminal.set_bg(Color::Rgb(5, 15, 33))?;
        terminal.set_fg(Color::Rgb(69, 168, 255))?;
        terminal.set_fg(Color::C256(123))?;
        terminal.set_bg(Color::Default)?;
        terminal.reset_style()?;
        assert_eq!(
            terminal.output(),
            "\x1b[38;2;69;168;255m\x1b[48;2;5;15;33m\x1b[38;5;123m\x1b[49m\x1b[0m"
        );
        assert_eq!(terminal.style, Style::default());
        Ok(())
    }

    use super::*;

    const COLOR_NAMES: [&str; 16] = [
        "Black",
        "Red",
        "Green",
        "Yellow",
        "Blue",
        "Magenta",
        "Cyan",
        "White",
        "LightBlack",
        "LightRed",
        "LightGreen",
        "LightYellow",
        "LightBlue",
        "LightMagenta",
        "LightCyan",
        "LightWhite",
    ];

    impl std::fmt::Display for Color {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Color::C16(c) => write!(f, "{}", COLOR_NAMES.get(*c as usize).unwrap_or(&"?")),
                #[cfg(feature = "colorscheme")]
                Color::C256(c) => write!(f, "C256({c})"),
                #[cfg(feature = "colorscheme")]
                Color::Rgb(r, g, b) => write!(f, "Rgb({r},{g},{b})"),
                Color::Default => write!(f, "Default"),
            }
        }
    }

    pub struct TextOnlyTerminal {
        pub output: String,
    }

    impl TextOnlyTerminal {
        pub fn new() -> Self {
            TextOnlyTerminal {
                output: String::new(),
            }
        }
    }

    impl Write for TextOnlyTerminal {
        fn write_str(&mut self, s: &str) -> Result {
            self.output.write_str(s)
        }
    }

    #[rustfmt::skip]
    impl Terminal for TextOnlyTerminal {
        fn clear_screen(&mut self) -> Result { Ok(()) }
        fn clear_line(&mut self) -> Result { Ok(()) }
        fn position_cursor(&mut self, _row: u16, _col: u16) -> Result { Ok(()) }
        fn position_cursor_col(&mut self, _col: u16) -> Result { Ok(()) }
        fn set_style(&mut self, _style: &Style) -> Result { Ok(()) }
        fn reset_style(&mut self) -> Result { Ok(()) }
        fn set_fg(&mut self, _color: Color) -> Result { Ok(()) }
        fn set_bg(&mut self, _color: Color) -> Result { Ok(()) }
        fn set_inverted(&mut self, _inverted: bool) -> Result { Ok(()) }
        fn set_bold(&mut self, _bold: bool) -> Result { Ok(()) }
        fn set_dimmed(&mut self, _bold: bool) -> Result { Ok(()) }
        fn set_underlined(&mut self, _underlined: bool) -> Result { Ok(()) }
        fn output(&self) -> &str { &self.output }
        fn clear_output(&mut self) { self.output.clear() }
    }

    pub struct VisibleEscapesTerminal {
        pub output: String,
        pub style: Style,
        pub pending_style: Style,
        pub show_position: bool,
        pub show_style: bool,
    }

    impl VisibleEscapesTerminal {
        pub fn new(show_position: bool, show_style: bool) -> Self {
            VisibleEscapesTerminal {
                output: String::new(),
                style: Style::default(),
                pending_style: Style::default(),
                show_position,
                show_style,
            }
        }
    }

    impl VisibleEscapesTerminal {
        fn write_pending_styles(&mut self) -> Result {
            if self.show_style {
                if self.style.fg != self.pending_style.fg {
                    write!(self.output, "_FG({})_", self.pending_style.fg)?;
                }
                if self.style.bg != self.pending_style.bg {
                    write!(self.output, "_BG({})_", self.pending_style.bg)?;
                }
                if self.style.inverted != self.pending_style.inverted {
                    if self.pending_style.inverted {
                        write!(self.output, "_INV_")?;
                    } else {
                        write!(self.output, "_!INV_")?;
                    }
                }
                if self.style.bold != self.pending_style.bold {
                    if self.pending_style.bold {
                        write!(self.output, "_B_")?;
                    } else {
                        write!(self.output, "_!B_")?;
                    }
                }
                if self.style.dimmed != self.pending_style.dimmed {
                    if self.pending_style.dimmed {
                        write!(self.output, "_D_")?;
                    } else {
                        write!(self.output, "_!D_")?;
                    }
                }
            }

            if self.show_style && self.style.underlined != self.pending_style.underlined {
                write!(
                    self.output,
                    "{}",
                    if self.pending_style.underlined {
                        "_U_"
                    } else {
                        "_!U_"
                    }
                )?;
            }
            self.style = self.pending_style;

            Ok(())
        }
    }

    impl Write for VisibleEscapesTerminal {
        fn write_str(&mut self, s: &str) -> Result {
            self.write_pending_styles()?;
            self.output.write_str(s)
        }
    }

    impl Terminal for VisibleEscapesTerminal {
        fn clear_screen(&mut self) -> Result {
            Ok(())
        }

        fn clear_line(&mut self) -> Result {
            Ok(())
        }

        fn position_cursor(&mut self, row: u16, col: u16) -> Result {
            if self.show_position {
                write!(self, "_RC({row},{col})_")
            } else {
                Ok(())
            }
        }

        fn position_cursor_col(&mut self, col: u16) -> Result {
            if self.show_position {
                write!(self, "_C({col})_")
            } else {
                Ok(())
            }
        }

        fn set_style(&mut self, style: &Style) -> Result {
            self.pending_style = *style;
            Ok(())
        }

        fn reset_style(&mut self) -> Result {
            self.style = Style::default();
            self.pending_style = Style::default();
            if self.show_style {
                write!(self, "_R_")?;
            }
            Ok(())
        }

        fn set_fg(&mut self, color: Color) -> Result {
            self.pending_style.fg = color;
            Ok(())
        }

        fn set_bg(&mut self, color: Color) -> Result {
            self.pending_style.bg = color;
            Ok(())
        }

        fn set_inverted(&mut self, inverted: bool) -> Result {
            self.pending_style.inverted = inverted;
            Ok(())
        }

        fn set_bold(&mut self, bold: bool) -> Result {
            self.pending_style.bold = bold;
            Ok(())
        }

        fn set_dimmed(&mut self, dimmed: bool) -> Result {
            self.pending_style.dimmed = dimmed;
            Ok(())
        }

        fn set_underlined(&mut self, underlined: bool) -> Result {
            self.pending_style.underlined = underlined;
            Ok(())
        }

        fn output(&self) -> &str {
            &self.output
        }

        fn clear_output(&mut self) {
            self.style = Style::default();
            self.pending_style = Style::default();
            self.output.clear()
        }
    }
}
