//! Accepted command spellings and the long names offered by the command prompt.

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WriteFormat {
    Json,
    Toon,
    #[cfg(feature = "sexp")]
    Sexp,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    #[cfg(feature = "colorscheme")]
    Colorscheme(String),
    Quit,
    Help,
    SetShowLineNumber(Option<bool>),
    SetShowRelativeLineNumber(Option<bool>),
    WriteFile {
        filename: String,
        overwrite_existing: bool,
        write_format: WriteFormat,
    },
    Unknown,
}

#[derive(Copy, Clone)]
enum Kind {
    #[cfg(feature = "colorscheme")]
    Colorscheme,
    Quit,
    Help,
    Set,
    Write(WriteFormat, bool),
}

struct Name {
    long: &'static str,
    aliases: &'static [&'static str],
    kind: Kind,
}

const NAMES: &[Name] = &[
    #[cfg(feature = "colorscheme")]
    Name {
        long: "colorscheme",
        aliases: &[],
        kind: Kind::Colorscheme,
    },
    Name {
        long: "exit",
        aliases: &["exit()"],
        kind: Kind::Quit,
    },
    Name {
        long: "help",
        aliases: &["h"],
        kind: Kind::Help,
    },
    Name {
        long: "quit",
        aliases: &["q", "quit()"],
        kind: Kind::Quit,
    },
    Name {
        long: "set",
        aliases: &[],
        kind: Kind::Set,
    },
    Name {
        long: "write",
        aliases: &["w"],
        kind: Kind::Write(WriteFormat::Json, false),
    },
    Name {
        long: "write!",
        aliases: &["w!"],
        kind: Kind::Write(WriteFormat::Json, true),
    },
    #[cfg(feature = "sexp")]
    Name {
        long: "writesexp",
        aliases: &["ws"],
        kind: Kind::Write(WriteFormat::Sexp, false),
    },
    #[cfg(feature = "sexp")]
    Name {
        long: "writesexp!",
        aliases: &["ws!"],
        kind: Kind::Write(WriteFormat::Sexp, true),
    },
    Name {
        long: "writetoon",
        aliases: &["wt"],
        kind: Kind::Write(WriteFormat::Toon, false),
    },
    Name {
        long: "writetoon!",
        aliases: &["wt!"],
        kind: Kind::Write(WriteFormat::Toon, true),
    },
];

fn kind(name: &str) -> Option<Kind> {
    NAMES
        .iter()
        .find(|entry| entry.long == name || entry.aliases.contains(&name))
        .map(|entry| entry.kind)
}

pub fn matching_names(prefix: &str) -> Vec<&'static str> {
    NAMES
        .iter()
        .map(|entry| entry.long)
        .filter(|name| name.starts_with(prefix))
        .collect()
}

impl Command {
    pub fn parse(command: &str) -> Self {
        // Colorscheme has historically accepted Unicode whitespace and names
        // containing spaces. Other commands split only on ASCII spaces.
        #[cfg(feature = "colorscheme")]
        if let Some((name, argument)) = command.trim().split_once(char::is_whitespace) {
            if matches!(kind(name), Some(Kind::Colorscheme)) && !argument.trim().is_empty() {
                return Self::Colorscheme(argument.trim().to_owned());
            }
        }
        let args: Vec<_> = command.split(' ').filter(|s| !s.is_empty()).collect();
        let Some(command_kind) = args.first().and_then(|name| kind(name)) else {
            return Self::Unknown;
        };
        match (command_kind, &args[1..]) {
            (Kind::Help, []) => Self::Help,
            (Kind::Quit, []) => Self::Quit,
            (Kind::Set, [argument]) => match *argument {
                "number" => Self::SetShowLineNumber(Some(true)),
                "number!" => Self::SetShowLineNumber(None),
                "nonumber" => Self::SetShowLineNumber(Some(false)),
                "relativenumber" => Self::SetShowRelativeLineNumber(Some(true)),
                "relativenumber!" => Self::SetShowRelativeLineNumber(None),
                "norelativenumber" => Self::SetShowRelativeLineNumber(Some(false)),
                _ => Self::Unknown,
            },
            (Kind::Write(write_format, overwrite_existing), [filename]) => Self::WriteFile {
                filename: (*filename).to_owned(),
                overwrite_existing,
                write_format,
            },
            _ => Self::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_names_and_aliases_keep_their_execution_contract() {
        for name in ["h", "help"] {
            assert_eq!(Command::parse(name), Command::Help);
        }
        for name in ["q", "quit", "quit()", "exit", "exit()"] {
            assert_eq!(Command::parse(name), Command::Quit);
        }
        let writes = [
            ("w", "write", WriteFormat::Json),
            ("wt", "writetoon", WriteFormat::Toon),
            #[cfg(feature = "sexp")]
            ("ws", "writesexp", WriteFormat::Sexp),
        ];
        for (short, long, format) in writes {
            for name in [short, long] {
                for bang in ["", "!"] {
                    assert_eq!(
                        Command::parse(&format!("  {name}{bang}  café.json  ")),
                        Command::WriteFile {
                            filename: "café.json".into(),
                            overwrite_existing: bang == "!",
                            write_format: format,
                        }
                    );
                }
            }
        }
        for input in [
            "",
            "hel",
            "HELP",
            "help extra",
            "quit extra",
            "write",
            "write a b",
            "write\ta",
            "set",
            "set other",
            "set number extra",
            "\thelp",
        ] {
            assert_eq!(Command::parse(input), Command::Unknown, "{input:?}");
        }
        for (name, expected) in [
            ("number", Some(true)),
            ("nonumber", Some(false)),
            ("number!", None),
        ] {
            assert_eq!(
                Command::parse(&format!("set {name}")),
                Command::SetShowLineNumber(expected)
            );
        }
        for (name, expected) in [
            ("relativenumber", Some(true)),
            ("norelativenumber", Some(false)),
            ("relativenumber!", None),
        ] {
            assert_eq!(
                Command::parse(&format!("set {name}")),
                Command::SetShowRelativeLineNumber(expected)
            );
        }
    }

    #[test]
    fn suggestions_are_sorted_long_names_available_in_this_build() {
        let names = matching_names("");
        assert!(names.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(matching_names("h"), ["help"]);
        assert_eq!(matching_names("q"), ["quit"]);
        assert!(matching_names("WR").is_empty());
        for alias in [
            "w", "w!", "wt", "wt!", "h", "q", "quit()", "exit()", "ws", "ws!",
        ] {
            assert!(!names.contains(&alias));
        }
        assert_eq!(
            names.contains(&"colorscheme"),
            cfg!(feature = "colorscheme")
        );
        assert_eq!(names.contains(&"writesexp"), cfg!(feature = "sexp"));
        for name in names {
            let arguments = match name {
                "set" => " number",
                "colorscheme" => " default",
                n if n.starts_with("write") => " out",
                _ => "",
            };
            assert_ne!(
                Command::parse(&format!("{name}{arguments}")),
                Command::Unknown
            );
        }
        #[cfg(not(feature = "sexp"))]
        for name in ["ws", "ws!", "writesexp", "writesexp!"] {
            assert_eq!(Command::parse(&format!("{name} out")), Command::Unknown);
        }
    }

    #[test]
    fn colorscheme_keeps_its_whitespace_and_feature_boundary() {
        for input in ["colorscheme", "colorscheme  ", "colorschemedefault"] {
            assert_eq!(Command::parse(input), Command::Unknown);
        }
        for input in [
            " colorscheme default  ",
            "\tcolorscheme\tdefault\n",
            "colorscheme\u{2003}default",
        ] {
            #[cfg(feature = "colorscheme")]
            assert_eq!(
                Command::parse(input),
                Command::Colorscheme("default".into())
            );
            #[cfg(not(feature = "colorscheme"))]
            assert_eq!(Command::parse(input), Command::Unknown);
        }
        #[cfg(feature = "colorscheme")]
        assert_eq!(
            Command::parse("colorscheme  my theme "),
            Command::Colorscheme("my theme".into())
        );
    }
}
