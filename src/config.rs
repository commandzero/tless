use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use clap::ValueEnum;
use yaml_rust::{Yaml, YamlLoader};

use crate::terminal::Color;
use crate::theme::{Theme, ThemeColor, ThemeName};

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Config {
    colorscheme: Option<String>,
    themes: BTreeMap<String, Vec<(ThemeColor, Color)>>,
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let Some(path) = config_path() else {
            return Ok(Self::default());
        };

        Self::load_from(&path)
    }

    pub fn resolve_startup(&self, requested: Option<&str>) -> Result<Theme, String> {
        self.resolve(
            requested
                .or(self.colorscheme.as_deref())
                .unwrap_or("default"),
        )
    }

    pub fn resolve(&self, name: &str) -> Result<Theme, String> {
        let base = ThemeName::value_variants()
            .iter()
            .copied()
            .find(|variant| {
                variant
                    .to_possible_value()
                    .is_some_and(|value| value.matches(name, false))
            })
            .or_else(|| self.themes.contains_key(name).then_some(ThemeName::Default))
            .ok_or_else(|| {
                let mut names: Vec<String> = ThemeName::value_variants()
                    .iter()
                    .filter_map(|variant| variant.to_possible_value())
                    .map(|value| value.get_name().to_owned())
                    .collect();
                names.extend(self.themes.keys().cloned());
                names.sort_unstable();
                names.dedup();
                format!(
                    "Unknown theme '{name}'. Available themes: {}",
                    names.join(", ")
                )
            })?;

        let mut theme = Theme::built_in(base);
        if let Some(colors) = self.themes.get(name) {
            for (key, color) in colors {
                theme = theme.with_color(*key, *color);
            }
        }
        Ok(theme)
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
            if key == "colorscheme" {
                config.colorscheme = Some(
                    value
                        .as_str()
                        .filter(|name| !name.trim().is_empty())
                        .ok_or_else(|| "'colorscheme' must be a non-empty string".to_string())?
                        .to_string(),
                );
                continue;
            }
            if key != "themes" {
                return Err(format!("unknown config key '{key}'"));
            }

            let themes = value
                .as_hash()
                .ok_or_else(|| "'themes' must be a mapping".to_string())?;
            for (name, value) in themes {
                let name = name
                    .as_str()
                    .filter(|name| !name.trim().is_empty())
                    .ok_or_else(|| "theme names must be non-empty strings".to_string())?;
                let colors = Self::parse_colors(value)
                    .map_err(|error| format!("theme '{name}': {error}"))?;
                config.themes.insert(name.to_string(), colors);
            }
        }

        Ok(config)
    }

    fn parse_colors(value: &Yaml) -> Result<Vec<(ThemeColor, Color)>, String> {
        let theme = value
            .as_hash()
            .ok_or_else(|| "expected a color mapping".to_string())?;
        let mut colors = Vec::new();
        for (key, value) in theme {
            let key =
                theme_key_string(key).ok_or_else(|| "theme keys must be strings".to_string())?;
            let theme_color = ThemeColor::from_config_key(key)
                .ok_or_else(|| format!("unknown theme key '{key}'"))?;
            let color = match value {
                Yaml::Integer(index) if (0..=255).contains(index) => Color::C256(*index as u8),
                Yaml::Integer(index) => {
                    return Err(format!(
                        "palette index {index} for theme key '{key}' must be between 0 and 255"
                    ))
                }
                Yaml::String(name) => parse_color(name)
                    .ok_or_else(|| format!("unknown color '{name}' for theme key '{key}'"))?,
                _ => {
                    return Err(format!(
                        "color for '{key}' must be a name or an integer palette index from 0 to 255"
                    ))
                }
            };
            colors.push((theme_color, color));
        }
        Ok(colors)
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

    Some(config_home.join("tless").join("config.yaml"))
}

fn yaml_string(value: &Yaml) -> Option<&str> {
    value.as_str()
}

fn theme_key_string(value: &Yaml) -> Option<&str> {
    match value {
        // YAML parses an unquoted `null` mapping key as a null scalar.
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
    use crate::theme::{JsonValueKind, StyleRole, StyleState};

    #[test]
    fn startup_selection_precedence() {
        let config = Config::parse("colorscheme: navy\nthemes: {navy: {string: blue}}").unwrap();
        assert_eq!(
            config.resolve_startup(None).unwrap(),
            config.resolve("navy").unwrap()
        );
        for name in ["default", "classic", "cyan", "borealis", "delek", "navy"] {
            assert_eq!(
                config.resolve_startup(Some(name)).unwrap(),
                config.resolve(name).unwrap()
            );
        }
        assert_eq!(
            Config::default().resolve_startup(None).unwrap(),
            Theme::default()
        );
    }

    #[test]
    fn parses_named_and_indexed_colors() {
        let config = Config::parse(
            "themes:\n  classic:\n    null: light-blue\n    string: 217\n    status-bar-background: 17\n",
        )
        .unwrap();
        let theme = config.resolve("classic").unwrap();
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
            Color::C256(217)
        );
        assert_eq!(
            theme.style(StyleRole::StatusBar, StyleState::main()).bg,
            Color::C256(17)
        );
    }

    #[test]
    fn rejects_invalid_definitions() {
        for input in [
            "colorscheme: ''",
            "themes: []",
            "themes: {navy: blue}",
            "themes: {navy: {string: ultraviolet}}",
            "themes: {navy: {string: 256}}",
            "theme: {}",
        ] {
            assert!(Config::parse(input).is_err(), "accepted {}", input);
        }
    }

    #[test]
    fn reports_available_theme_names() {
        let config =
            Config::parse("themes: {navy: {string: cyan}, sunset: {string: red}}").unwrap();
        let error = config.resolve("missing").unwrap_err();
        assert!(error.contains("default"));
        assert!(error.contains("vim"));
        assert!(error.contains("delek"));
        assert!(error.contains("navy"));
        assert!(error.contains("sunset"));
    }

    #[test]
    fn default_and_vim_are_distinct_built_ins() {
        let default = Config::default().resolve("default").unwrap();
        let vim = Config::default().resolve("vim").unwrap();
        assert_eq!(default, Theme::default());
        assert_eq!(vim.document_style().fg, Color::C256(7));
        assert_eq!(vim.document_style().bg, Color::C256(0));
    }

    #[test]
    fn resolves_xdg_and_home_paths() {
        assert_eq!(
            config_path_from(Some(OsStr::new("/tmp/xdg")), Some(OsStr::new("/home/me"))),
            Some(PathBuf::from("/tmp/xdg/tless/config.yaml"))
        );
        assert_eq!(
            config_path_from(None, Some(OsStr::new("/home/me"))),
            Some(PathBuf::from("/home/me/.config/tless/config.yaml"))
        );
    }
}
