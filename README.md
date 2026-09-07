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

`jless` currently supports macOS and Linux. Windows support is planned.

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

### Optional TOON support

Build from this checkout to enable TOON input and canonical output:

```sh
cargo install --path . --locked --features toon
jless data.toon
producer | jless --toon
```

The `toon` Cargo feature is disabled by default. It can be combined with `sexp`.
Disabled builds omit TOON commands and help; opening a `.toon` filename explains
how to enable support. `--json` and `--yaml` override filename detection.

This implementation targets `toon-spec: 3.0`, not TOON 4.x. It uses strict
two-space decoding with literal dotted keys and supports declared comma, tab,
and pipe delimiters. The viewer still uses its existing JSON Line/Data views.
Searches operate on normalized JSON text, not the original TOON spelling.

Use `yt` to copy or `pt` to print the complete focused value as canonical TOON.
Use `:wt file` or `:writetoon file` to write the whole document. Add `!` to create
or replace a target. Encoding finishes before the file is opened, but an I/O
failure during writing does not promise rollback. Existing `yy`, `pp`, and
`:write` commands still produce JSON.

Canonical output uses two spaces, comma delimiters, no key folding, and no final
newline. An empty root object produces an empty payload. Whole-document output
requires one root; focused export also works with multi-root JSON input.
Object key order is preserved. Rows with differing key orders use list form,
an intentional exception to TOON 3.0 canonical table selection. Rows with the
same non-empty key sequence can still use tables.

TOON input and export reject duplicate keys. Export rejects non-string YAML
keys and non-finite numbers. Numeric conversion preserves exact decimal value
or fails; numeric spelling may change and negative zero becomes zero. Numbers
are limited to 1,024 coefficient digits and exponent magnitude 1,024. Nesting is
limited to 256 containers relative to the selected root.

Interactive input accepts CRLF and trailing blank lines, but rejects an initial
BOM and blank rows inside arrays. With non-terminal stdout, selected TOON text
passes through unchanged without parsing, even if its syntax is malformed.
Invalid UTF-8 is always an input error.

The source build uses an isolated vendored `toon-format` 0.5.0 backport for Rust
1.67 compatibility and fidelity checks. Keep `--locked` when building. The
vendored codec and pinned specification fixtures retain their upstream licenses.

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
