use std::path::PathBuf;

use clap::{ArgAction, Parser, ValueEnum};

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

    // This godforsaken configuration to get both --line-numbers and --no-line-numbers to
    // work (with --line-numbers as the default) and --relative-line-numbers and
    // --no-relative-line-numbers to work (with --no-relative-line-numbers as the default)
    // was taken from here:
    //
    // https://jwodder.github.io/kbits/posts/clap-bool-negate/
    /// Don't show line numbers.
    #[arg(short = 'N', long = "no-line-numbers", action = ArgAction::SetFalse)]
    pub show_line_numbers: bool,

    /// Show absolute expanded TOON line addresses (default). Collapsed contents
    /// leave gaps; inline values and table cells share their line's address.
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
