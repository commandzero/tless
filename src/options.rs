use std::path::PathBuf;

use clap::{ArgAction, Parser, ValueEnum};

#[derive(PartialEq, Eq, Copy, Clone, Debug, ValueEnum)]
pub enum DataFormat {
    Json,
    Yaml,
    #[cfg(feature = "toon")]
    Toon,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug, ValueEnum)]
pub enum OutputFormat {
    Json,
    Yaml,
    Toon,
}

/// A pager for JSON, YAML, and TOON data
#[derive(Debug, Parser)]
#[command(name = "tless", version)]
pub struct Opt {
    /// Input file. tless will read from stdin if no input file is
    /// provided, or '-' is specified. If a filename is provided, tless
    /// will check the extension to determine what the input format is,
    /// and by default will assume JSON. Use --input <format> to select a
    /// format explicitly.
    #[arg(value_name = "FILE")]
    pub file: Option<PathBuf>,

    /// Format for non-terminal stdout. TOON requires a toon-enabled build.
    /// Input format and interactive commands are unchanged.
    #[arg(short = 'o', long, value_enum, default_value = "toon")]
    pub output: OutputFormat,

    /// Maximum input bytes. Use 0 for unlimited input. The complete input stays in memory.
    #[arg(long, default_value_t = 536_870_912)]
    pub max_input_bytes: u64,

    /// Built-in or configured theme name. Overrides colorscheme in config.yaml.
    #[cfg(feature = "colorscheme")]
    #[arg(long)]
    pub theme: Option<String>,

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

    /// Parse input as FORMAT, regardless of file extension.
    #[cfg_attr(
        feature = "toon",
        doc = "Supported formats are json, yaml, and toon (with TOON support enabled)."
    )]
    #[cfg_attr(
        not(feature = "toon"),
        doc = "Supported formats are json and yaml; toon requires a TOON-enabled build."
    )]
    #[arg(
        long = "input",
        value_enum,
        value_name = "FORMAT",
        display_order = 1000
    )]
    pub input_format: Option<DataFormat>,
}

#[cfg(all(test, feature = "colorscheme"))]
mod colorscheme_tests {
    use clap::Parser;

    use super::Opt;

    #[test]
    fn absent_theme_defers_to_configuration() {
        assert_eq!(Opt::try_parse_from(["tless"]).unwrap().theme, None);
    }

    #[test]
    fn accepts_named_themes() {
        for name in ["default", "vim", "delek"] {
            assert_eq!(
                Opt::try_parse_from(["tless", "--theme", name])
                    .unwrap()
                    .theme
                    .as_deref(),
                Some(name)
            );
        }
    }
}

impl Opt {
    pub fn data_format(&self) -> Option<DataFormat> {
        self.input_format
    }
}
