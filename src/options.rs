use std::path::PathBuf;

use clap::{ArgAction, Parser, ValueEnum};

use crate::theme::ThemeName;
use crate::viewer::Mode;

#[derive(PartialEq, Eq, Copy, Clone, Debug, ValueEnum)]
pub enum DataFormat {
    Json,
    Yaml,
    #[cfg(feature = "toon")]
    Toon,
}

/// A pager for JSON (or YAML) data
#[derive(Debug, Parser)]
#[command(name = "tless", version)]
pub struct Opt {
    /// Input file. tless will read from stdin if no input file is
    /// provided, or '-' is specified. If a filename is provided, tless
    /// will check the extension to determine what the input format is,
    /// and by default will assume JSON. Can specify input format
    /// explicitly using --json or --yaml.
    pub input: Option<PathBuf>,

    /// Maximum input bytes. Use 0 for unlimited input. The complete input stays in memory.
    #[arg(long, default_value_t = 536_870_912)]
    pub max_input_bytes: u64,

    /// Initial viewing mode. In line mode (--mode line), opening
    /// and closing curly and square brackets are shown and all
    /// Object keys are quoted. In data mode (--mode data; the default),
    /// closing braces, commas, and quotes around Object keys are elided.
    /// The active mode can be toggled by pressing 'm'.
    #[arg(short, long, value_enum, hide_possible_values = true, default_value_t = Mode::Data)]
    pub mode: Mode,

    /// Color theme used for terminal rendering.
    #[arg(long, value_enum, default_value_t = ThemeName::Classic)]
    pub theme: ThemeName,

    // This godforsaken configuration to get both --line-numbers and --no-line-numbers to
    // work (with --line-numbers as the default) and --relative-line-numbers and
    // --no-relative-line-numbers to work (with --no-relative-line-numbers as the default)
    // was taken from here:
    //
    // https://jwodder.github.io/kbits/posts/clap-bool-negate/
    /// Don't show line numbers.
    #[arg(short = 'N', long = "no-line-numbers", action = ArgAction::SetFalse)]
    pub show_line_numbers: bool,

    /// Show "line" numbers (default). Line numbers are determined by
    /// the line number of a given line if the document were pretty printed.
    /// These means there are discontinuities when viewing in data mode
    /// because the lines containing closing brackets and braces aren't displayed.
    #[arg(
        short = 'n',
        long = "line-numbers",
        overrides_with = "show_line_numbers"
    )]
    pub _show_line_numbers_hidden: bool,

    /// Show the line number relative to the currently focused line. Relative line
    /// numbers help you use a count with vertical motion commands (j k) without
    /// having to count.
    #[arg(
        short = 'r',
        long = "relative-line-numbers",
        overrides_with = "_show_relative_line_numbers_hidden"
    )]
    pub show_relative_line_numbers: bool,

    /// Don't show relative line numbers (default).
    #[arg(short = 'R', long = "no-relative-line-numbers")]
    _show_relative_line_numbers_hidden: bool,

    /// Number of lines to maintain as padding between the currently
    /// focused row and the top or bottom of the screen. Setting this to
    /// a large value will keep the focused in the middle of the screen
    /// (except at the start or end of a file).
    #[arg(long = "scrolloff", default_value_t = 3)]
    pub scrolloff: u16,

    /// Parse input as JSON, regardless of file extension.
    #[arg(long = "json", group = "data-format", display_order = 1000)]
    pub json: bool,

    /// Parse input as YAML, regardless of file extension.
    #[arg(long = "yaml", group = "data-format", display_order = 1000)]
    pub yaml: bool,

    /// Read TOON 3.0, regardless of file extension. Requires a toon-enabled build.
    #[cfg(feature = "toon")]
    #[arg(long = "toon", group = "data-format", display_order = 1000)]
    pub toon: bool,
}

impl Opt {
    pub fn data_format(&self) -> Option<DataFormat> {
        #[cfg(feature = "toon")]
        if self.toon {
            return Some(DataFormat::Toon);
        }
        if self.json {
            Some(DataFormat::Json)
        } else if self.yaml {
            Some(DataFormat::Yaml)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[test]
    fn theme_defaults_to_classic() {
        let options = Opt::try_parse_from(["jless"]).unwrap();

        assert_eq!(options.theme, ThemeName::Classic);
    }

    #[test]
    fn accepts_built_in_theme_names() {
        for (name, expected) in [("classic", ThemeName::Classic), ("cyan", ThemeName::Cyan)] {
            let options = Opt::try_parse_from(["jless", "--theme", name]).unwrap();
            assert_eq!(options.theme, expected);
        }
    }

    #[test]
    fn rejects_unknown_theme_name() {
        let error = Opt::try_parse_from(["jless", "--theme", "unknown"]).unwrap_err();

        assert_eq!(error.kind(), clap::error::ErrorKind::InvalidValue);
        assert!(error.to_string().contains("unknown"), "{}", error);
    }
}
