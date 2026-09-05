![jless logo and mascot](https://raw.githubusercontent.com/PaulJuliusMartinez/jless/master/logo/text-logo-with-mascot.svg)

[`jless`](https://jless.io) is a command-line JSON viewer. Use it as a
replacement for whatever combination of `less`, `jq`, `cat` and your
editor you currently use for viewing JSON files. It is written in Rust
and can be installed as a single standalone binary.

[![ci](https://github.com/PaulJuliusMartinez/jless/actions/workflows/ci.yml/badge.svg?branch=master&event=push)](https://github.com/PaulJuliusMartinez/jless/actions/workflows/ci.yml)

### Features

- Clean syntax highlighted display of JSON data, omitting quotes around
  object keys, closing object and array delimiters, and trailing commas.
- Expand and collapse objects and arrays so you can see both the high-
  and low-level structure of the data.
- A wealth of vim-inspired movement commands for efficiently moving
  around and viewing data.
- Full regex-based search for finding exactly the data you're looking
  for.
- Selectable `classic` and `cyan` terminal color themes through the
  `--theme` option.

`jless` currently supports macOS and Linux. Windows support is planned.

## Color themes

Use `--theme classic` or `--theme cyan` to select a built-in color theme.
`classic` is the default.

You can override individual colors in
`$XDG_CONFIG_HOME/jless/config.yaml`. If `XDG_CONFIG_HOME` is unset, jless
uses `$HOME/.config/jless/config.yaml`.

```yaml
theme:
  null: light-blue
  string: green
  object-key: light-cyan
  search-match-current: light-yellow
  status-bar-foreground: white
  status-bar-background: blue
  command-line-foreground: light-cyan
  command-line-background: black
  message-info: light-blue
  message-warning: light-yellow
  message-error: light-red
```

The file is optional. Each configured color replaces the matching color in
the selected built-in theme. Unlisted colors and style attributes keep their
built-in values.

Theme keys are `null`, `boolean`, `number`, `string`, `empty-container`,
`object-key`, `focused-object-key`, `array-index`, `punctuation`,
`primitive-trailing-comma`, `container-delimiter`,
`focused-container-delimiter`, `ellipsis`, `preview-text`, `preview-count`,
`line-number`, `focused-line-number`, `empty-row-marker`,
`truncation-indicator`, `status-bar`, `status-text`, `message-info`,
`message-warning`, `message-error`, `search-match`, `search-match-preview`, and
`search-match-current`.

`search-match` colors ordinary matches, `search-match-current` colors the selected
match, and `search-match-preview` colors matches inside collapsed previews.

Use `status-bar-foreground` and `status-bar-background` for the path and filename
row, and `command-line-foreground` and `command-line-background` for the bottom
row, including command and search editing. These keys name the visible text and
background colors, even when the built-in bar uses reverse video. Set
`message-info`, `message-warning`, and `message-error` to customize severity text
colors. These override `command-line-foreground` for messages and use the
`command-line-background`. Omitted severity colors use the built-in defaults.
The older `status-bar` and
`status-text` keys remain supported; explicit foreground/background keys take
precedence over them.

Colors are `default`, `black`, `red`, `green`, `yellow`, `blue`, `magenta`,
`cyan`, `white`, and the `light-` version of each named color.

## Installation

You can install `jless` using various package managers:

| Operating System / Package Manager | Command |
| ---------------------------------- | ------- |
| macOS - [HomeBrew](https://formulae.brew.sh/formula/jless) | `brew install jless`      |
| macOS - [MacPorts](https://ports.macports.org/port/jless/) | `sudo port install jless` |
| Linux - [HomeBrew](https://formulae.brew.sh/formula/jless) | `brew install jless`      |
| [Arch Linux](https://archlinux.org/packages/extra/x86_64/jless/)     | `pacman -S jless`         |
| [Void Linux](https://github.com/void-linux/void-packages/tree/master/srcpkgs/jless) | `sudo xbps-install jless` |
| [NetBSD](https://pkgsrc.se/textproc/jless/)                | `pkgin install jless`     |
| [FreeBSD](https://freshports.org/textproc/jless/)          | `pkg install jless`       |
| From source (Requires [Rust toolchain](https://www.rust-lang.org/tools/install))       | `cargo install jless`       |

The [releases](https://github.com/PaulJuliusMartinez/jless/releases)
page also contains links to binaries for various architectures.

## Dependencies

On Linux systems, X11 libraries are needed to build clipboard access if
building from source. On Ubuntu you can install these using:

```
sudo apt-get install libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

## Website

[jless.io](https://jless.io) is the official website for `jless`. Code
for the website is contained separately on the
[`website`](https://github.com/PaulJuliusMartinez/jless/tree/website) branch.

## Logo

The mascot of the `jless` project is Jules the jellyfish.

<img style="width: 250px;" alt="jless mascot" src="https://raw.githubusercontent.com/PaulJuliusMartinez/jless/master/logo/mascot.svg">

Art for Jules was created by
[`annatgraphics`](https://www.fiverr.com/annatgraphics).

## License

`jless` is released under the [MIT License](https://github.com/PaulJuliusMartinez/jless/blob/master/LICENSE).
