// I don't like this rule because it changes the semantic
// structure of the code.
#![allow(clippy::collapsible_else_if)]
// Sometimes "x >= y + 1" is semantically clearer than "x > y"
#![allow(clippy::int_plus_one)]

extern crate lazy_static;
extern crate libc_stdhandle;

use std::fs::File;
use std::io;
use std::io::IsTerminal;
use std::io::Read;
use std::io::Write;

use clap::Parser;
use termion::cursor::HideCursor;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::IntoAlternateScreen;

mod app;
#[cfg(feature = "colorscheme")]
mod commandline;
#[cfg(feature = "colorscheme")]
mod config;
mod flatjson;
mod input;
mod jsonparser;
mod jsonstringunescaper;
mod jsontokenizer;
mod lineprinter;
mod options;
mod output;
mod screenwriter;
mod search;
mod terminal;
mod theme;
#[cfg(feature = "toon")]
mod toon;
mod toon_display;
mod truncatedstrview;
mod types;
mod viewer;
mod wrapped_view;
mod yamlparser;

use app::App;
#[cfg(feature = "colorscheme")]
use config::Config;
use options::{DataFormat, Opt};
#[cfg(not(feature = "colorscheme"))]
use theme::Theme;

fn main() {
    let opt = Opt::parse();

    let (input_string, input_filename) = match get_input_and_filename(&opt) {
        Ok(input_and_filename) => input_and_filename,
        Err(err) => {
            eprintln!("Unable to get input: {err}");
            std::process::exit(1);
        }
    };

    let data_format = match determine_data_format(opt.data_format(), &input_filename) {
        Ok(format) => format,
        Err(error) => {
            eprintln!("{}", error);
            std::process::exit(1);
        }
    };

    if !io::stdout().is_terminal() {
        if let Err(error) = print_output(input_string, data_format, opt.output) {
            eprintln!("{}", error);
            std::process::exit(1);
        }
        std::process::exit(0);
    }

    #[cfg(feature = "colorscheme")]
    let (config, theme) = match Config::load().and_then(|config| {
        let theme = config.resolve_startup(opt.theme.as_deref())?;
        Ok((config, theme))
    }) {
        Ok(selection) => selection,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };
    #[cfg(not(feature = "colorscheme"))]
    let theme = Theme::default();

    // We use freopen to remap /dev/tty to STDIN so that rustyline works when
    // JSON input is provided via STDIN. rustyline gets initialized when we
    // create the App, so by putting this before creating the app, we make
    // sure rustyline gets the /dev/tty input.
    input::remap_dev_tty_to_stdin();

    let raw_stdout = MouseTerminal::from(HideCursor::from(
        io::stdout()
            .into_raw_mode()
            .unwrap()
            .into_alternate_screen()
            .unwrap(),
    ));

    let mut app = match App::new(
        &opt,
        theme,
        #[cfg(feature = "colorscheme")]
        config,
        input_string,
        data_format,
        input_filename,
        raw_stdout,
    ) {
        Ok(jl) => jl,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };

    app.run(Box::new(input::get_input()));
}

fn print_output(
    input: String,
    data_format: DataFormat,
    output_format: options::OutputFormat,
) -> Result<(), String> {
    let output = output::serialize(input, data_format, output_format)?;
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    stdout
        .write_all(output.as_bytes())
        .and_then(|()| stdout.flush())
        .map_err(|error| format!("Unable to write output: {error}"))
}

fn get_input_and_filename(opt: &Opt) -> io::Result<(String, String)> {
    let mut input_string = String::new();
    let filename;

    match &opt.file {
        None => {
            if io::stdin().is_terminal() {
                eprintln!("Missing filename (\"tless --help\" for help)");
                std::process::exit(1);
            }
            filename = "STDIN".to_string();
            read_input(io::stdin().lock(), &mut input_string, opt.max_input_bytes)?;
        }
        Some(path) => {
            if path.as_os_str() == "-" {
                filename = "STDIN".to_string();
                read_input(io::stdin().lock(), &mut input_string, opt.max_input_bytes)?;
            } else {
                read_input(File::open(path)?, &mut input_string, opt.max_input_bytes)?;
                filename = String::from(path.file_name().unwrap().to_string_lossy());
            }
        }
    }

    Ok((input_string, filename))
}

fn read_input(reader: impl Read, output: &mut String, limit: u64) -> io::Result<()> {
    if limit == 0 {
        reader.take(u64::MAX).read_to_string(output)?;
    } else {
        reader
            .take(limit.saturating_add(1))
            .read_to_string(output)?;
        if output.len() as u64 > limit {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "input exceeds --max-input-bytes {limit}; raise the limit or use 0 for unlimited input"
                ),
            ));
        }
    }
    Ok(())
}

fn determine_data_format(
    format: Option<DataFormat>,
    filename: &str,
) -> Result<DataFormat, &'static str> {
    if let Some(format) = format {
        return Ok(format);
    }
    match std::path::Path::new(filename)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
    {
        Some("yml") | Some("yaml") => Ok(DataFormat::Yaml),
        #[cfg(feature = "toon")]
        Some("toon") => Ok(DataFormat::Toon),
        #[cfg(not(feature = "toon"))]
        Some("toon") => Err(
            "This binary was built without TOON support; rebuild with --features toon, or use --input json/--input yaml.",
        ),
        _ => Ok(DataFormat::Json),
    }
}
