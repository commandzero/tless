use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use yaml_rust::{Yaml, YamlLoader};

use crate::terminal::Color;
use crate::theme::{Theme, ThemeColor};

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Config {
    theme_colors: Vec<(ThemeColor, Color)>,
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let Some(path) = config_path() else {
            return Ok(Self::default());
        };

        Self::load_from(&path)
    }

    pub fn apply_to(&self, mut theme: Theme) -> Theme {
        for (key, color) in &self.theme_colors {
            theme = theme.with_color(*key, *color);
        }
        theme
    }

    fn load_from(path: &Path) -> Result<Self, String> {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => {
                return Err(format!(
                    "Unable to read config file {}: {error}",
                    path.display()
                ))
            }
        };

        Self::parse(&contents)
            .map_err(|error| format!("Invalid config file {}: {error}", path.display()))
    }

    fn parse(contents: &str) -> Result<Self, String> {
        let documents = YamlLoader::load_from_str(contents).map_err(|error| error.to_string())?;
        if documents.is_empty() {
            return Ok(Self::default());
        }
        if documents.len() != 1 {
            return Err("expected one YAML document".to_string());
        }

        let root = match &documents[0] {
            Yaml::Hash(root) => root,
            Yaml::Null => return Ok(Self::default()),
            _ => return Err("expected a YAML mapping at the document root".to_string()),
        };

        let mut config = Self::default();
        for (key, value) in root {
            let key = yaml_string(key).ok_or_else(|| "config keys must be strings".to_string())?;
            if key != "theme" {
                return Err(format!("unknown config key '{key}'"));
            }

            let theme = value
                .as_hash()
                .ok_or_else(|| "'theme' must be a mapping".to_string())?;
            for (key, value) in theme {
                let key = theme_key_string(key)
                    .ok_or_else(|| "theme keys must be strings".to_string())?;
                let theme_color = ThemeColor::from_config_key(key)
                    .ok_or_else(|| format!("unknown theme key '{key}'"))?;
                let value = yaml_string(value)
                    .ok_or_else(|| format!("color for '{key}' must be a string"))?;
                let color = parse_color(value)
                    .ok_or_else(|| format!("unknown color '{value}' for theme key '{key}'"))?;
                config.theme_colors.push((theme_color, color));
            }
        }

        Ok(config)
    }
}

fn config_path() -> Option<PathBuf> {
    config_path_from(
        std::env::var_os("XDG_CONFIG_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
    )
}

fn config_path_from(xdg_config_home: Option<&OsStr>, home: Option<&OsStr>) -> Option<PathBuf> {
    let config_home = xdg_config_home
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| home.map(|path| PathBuf::from(path).join(".config")))?;

    Some(config_home.join("jless").join("config.yaml"))
}

fn yaml_string(value: &Yaml) -> Option<&str> {
    value.as_str()
}

fn theme_key_string(value: &Yaml) -> Option<&str> {
    match value {
        // YAML parses an unquoted `null` mapping key as a null scalar. Accept
        // it because `null: light-black` is the natural theme syntax.
        Yaml::Null => Some("null"),
        _ => yaml_string(value),
    }
}

fn parse_color(value: &str) -> Option<Color> {
    let color = match value {
        "black" => Color::C16(0),
        "red" => Color::C16(1),
        "green" => Color::C16(2),
        "yellow" => Color::C16(3),
        "blue" => Color::C16(4),
        "magenta" => Color::C16(5),
        "cyan" => Color::C16(6),
        "white" => Color::C16(7),
        "light-black" => Color::C16(8),
        "light-red" => Color::C16(9),
        "light-green" => Color::C16(10),
        "light-yellow" => Color::C16(11),
        "light-blue" => Color::C16(12),
        "light-magenta" => Color::C16(13),
        "light-cyan" => Color::C16(14),
        "light-white" => Color::C16(15),
        "default" => Color::Default,
        _ => return None,
    };
    Some(color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{JsonValueKind, StyleRole, StyleState, ThemeName};

    #[test]
    fn resolves_xdg_config_path() {
        assert_eq!(
            config_path_from(Some(OsStr::new("/tmp/xdg")), Some(OsStr::new("/home/me"))),
            Some(PathBuf::from("/tmp/xdg/jless/config.yaml"))
        );
    }

    #[test]
    fn falls_back_to_home_dot_config() {
        assert_eq!(
            config_path_from(None, Some(OsStr::new("/home/me"))),
            Some(PathBuf::from("/home/me/.config/jless/config.yaml"))
        );
    }

    #[test]
    fn parses_partial_theme_and_unquoted_null_key() {
        let config =
            Config::parse("theme:\n  null: light-blue\n  string: blue\n  object-key: light-cyan\n")
                .unwrap();
        let theme = config.apply_to(Theme::built_in(ThemeName::Classic));

        assert_eq!(
            theme
                .style(
                    StyleRole::JsonValue(JsonValueKind::Null),
                    StyleState::main()
                )
                .fg,
            Color::C16(12)
        );
        assert_eq!(
            theme
                .style(
                    StyleRole::JsonValue(JsonValueKind::String),
                    StyleState::main()
                )
                .fg,
            Color::C16(4)
        );
        assert_eq!(
            theme.style(StyleRole::ObjectKey, StyleState::main()).fg,
            Color::C16(14)
        );
        assert_eq!(
            theme
                .style(
                    StyleRole::JsonValue(JsonValueKind::Boolean),
                    StyleState::main()
                )
                .fg,
            Color::C16(3)
        );
    }

    #[test]
    fn config_overrides_named_theme() {
        let config = Config::parse("theme:\n  string: blue\n").unwrap();
        let theme = config.apply_to(Theme::built_in(ThemeName::Cyan));

        assert_eq!(
            theme
                .style(
                    StyleRole::JsonValue(JsonValueKind::String),
                    StyleState::main()
                )
                .fg,
            Color::C16(4)
        );
        assert_eq!(
            theme
                .style(
                    StyleRole::JsonValue(JsonValueKind::Null),
                    StyleState::main()
                )
                .fg,
            Color::C16(12)
        );
    }

    #[test]
    fn empty_config_is_allowed() {
        assert_eq!(Config::parse("").unwrap(), Config::default());
    }

    #[test]
    fn row_colors_override_reverse_video_and_preserve_message_severity() {
        let config = Config::parse("theme:\n  status-bar: red\n  status-text: yellow\n  status-bar-foreground: white\n  status-bar-background: blue\n  command-line-foreground: cyan\n  command-line-background: black\n").unwrap();
        for name in [ThemeName::Classic, ThemeName::Cyan] {
            let theme = config.apply_to(Theme::built_in(name));
            for role in [StyleRole::StatusBar, StyleRole::StatusPathBase] {
                let style = theme.style(role, StyleState::main());
                assert_eq!(style.fg, Color::C16(7));
                assert_eq!(style.bg, Color::C16(4));
                assert!(!style.inverted);
            }
            let style = theme.style(StyleRole::StatusText, StyleState::main());
            assert_eq!(style.fg, Color::C16(6));
            assert_eq!(style.bg, Color::C16(0));
            for (severity, color) in [
                (crate::theme::MessageSeverity::Info, 7),
                (crate::theme::MessageSeverity::Warn, 3),
                (crate::theme::MessageSeverity::Error, 1),
            ] {
                let style = theme.style(StyleRole::Message(severity), StyleState::main());
                assert_eq!(style.fg, Color::C16(color));
                assert_eq!(style.bg, Color::C16(0));
            }
        }
    }

    #[test]
    fn partial_row_colors_preserve_the_other_visible_color() {
        let theme =
            Config::parse("theme: {status-bar-background: blue, command-line-foreground: default}")
                .unwrap()
                .apply_to(Theme::default());
        let style = theme.style(StyleRole::StatusPathBase, StyleState::main());
        assert_eq!(style.fg, Color::C16(8));
        assert_eq!(style.bg, Color::C16(4));
        assert!(!style.inverted);
        assert_eq!(
            theme.style(StyleRole::StatusText, StyleState::main()),
            crate::terminal::Style::default()
        );
    }

    #[test]
    fn custom_severity_colors_override_command_foreground_and_keep_background() {
        let config = Config::parse(
            "theme:\n  message-info: light-blue\n  message-warning: light-yellow\n  message-error: light-red\n  command-line-foreground: green\n  command-line-background: blue\n",
        ).unwrap();

        for name in [ThemeName::Classic, ThemeName::Cyan] {
            let theme = config.apply_to(Theme::built_in(name));
            for (severity, color) in [
                (crate::theme::MessageSeverity::Info, 12),
                (crate::theme::MessageSeverity::Warn, 11),
                (crate::theme::MessageSeverity::Error, 9),
            ] {
                let style = theme.style(StyleRole::Message(severity), StyleState::main());
                assert_eq!(style.fg, Color::C16(color));
                assert_eq!(style.bg, Color::C16(4));
            }
            let command = theme.style(StyleRole::StatusText, StyleState::main());
            assert_eq!(command.fg, Color::C16(2));
            assert_eq!(command.bg, Color::C16(4));
        }
    }

    #[test]
    fn rejects_unknown_keys_and_colors() {
        let key_error = Config::parse("theme:\n  mystery: blue\n").unwrap_err();
        assert!(key_error.contains("unknown theme key 'mystery'"));

        let color_error = Config::parse("theme:\n  string: ultraviolet\n").unwrap_err();
        assert!(color_error.contains("unknown color 'ultraviolet'"));
    }

    #[test]
    fn rejects_invalid_yaml_and_non_mapping_theme_documents() {
        for input in [
            "theme: [",
            "theme: blue",
            "[blue]",
            "theme: {string: 42}",
            "{}\n---\n{}",
        ] {
            assert!(Config::parse(input).is_err(), "accepted {}", input);
        }
    }

    #[test]
    fn load_error_names_the_config_file() {
        let path = std::env::temp_dir().join(format!(
            "jless-invalid-config-{}-{}.yaml",
            std::process::id(),
            line!()
        ));
        fs::write(&path, "theme:\n  string: ultraviolet\n").unwrap();

        let error = Config::load_from(&path).unwrap_err();
        fs::remove_file(&path).unwrap();

        assert!(error.contains(&path.display().to_string()));
        assert!(error.contains("unknown color 'ultraviolet'"));
    }

    #[test]
    fn missing_file_is_empty_config() {
        let path = std::env::temp_dir().join(format!(
            "jless-missing-config-{}-{}.yaml",
            std::process::id(),
            line!()
        ));

        assert_eq!(Config::load_from(&path).unwrap(), Config::default());
    }
}
