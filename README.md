# tless

A terminal viewer for TOON, JSON, and YAML. Forked from the excellent [jless](https://github.com/PaulJuliusMartinez/jless).

[![ci](https://github.com/CommandZero/tless/actions/workflows/ci.yml/badge.svg)](https://github.com/CommandZero/tless/actions/workflows/ci.yml)

View your data in the compact TOON format, including JSON and YAML files.
Expand and collapse data, navigate with vim-style keys, and search with regular expressions.

Why [TOON](https://toonformat.dev/) format? Look at this short example from the TOON format [getting started](https://toonformat.dev/guide/getting-started.html) guide.

JSON:

```json
{
  "location": {
    "city": "Berlin",
    "country": "DE",
    "units": "metric"
  },
  "alerts": [
    "frost",
    "wind"
  ],
  "forecast": [
    {
      "day": "Mon",
      "temp": {
        "min": -2,
        "max": 4
      },
      "condition": "snow",
      "rainChance": 80
    },
    {
      "day": "Tue",
      "temp": {
        "min": 1,
        "max": 7
      },
      "condition": "cloudy",
      "rainChance": 20
    },
    {
      "day": "Wed",
      "temp": {
        "min": 3,
        "max": 11
      },
      "condition": "sunny",
      "rainChance": 5
    }
  ]
}
```

TOON:

```toon
location:
  city: Berlin
  country: DE
  units: metric
alerts[2]: frost,wind
forecast[3]:
  - day: Mon
    temp:
      min: -2
      max: 4
    condition: snow
    rainChance: 80
  - day: Tue
    temp:
      min: 1
      max: 7
    condition: cloudy
    rainChance: 20
  - day: Wed
    temp:
      min: 3
      max: 11
    condition: sunny
    rainChance: 5
```

Yes, that is the exact same data.

Also look at the differences between `examples/nato-phonetics.json` and `examples/nato-phonetics.toon`:

| Format | Lines | Tokens |
| --- | ---: | ---: |
| JSON | 249 |  1,604 |
| TOON | 47 | 548 |
| Saved | 202 | 1,056 |
| Saved % | 81.1% | 65.8% |

Token counts use the `o200k_base` tokenizer and include file metadata.

That is far fewer tokens for your LLM, [higher accuracy](https://toonformat.dev/guide/benchmarks.html#retrieval-accuracy), fewer lines for commit diffs, _and_ easier on your eyes.

Sign me up!

## Install

Install from Homebrew:

```sh
brew install commandzero/tools/tless
```

Or from Cargo:

```sh
cargo install tless
```

Then open any TOON, JSON, or YAML file with `tless`:

```sh
tless path/to/file.json
```

Or pipe in contents from stdin:

```sh
cat file.json | tless --input-format json
```

Some quick keys:
- `hjkl` vim motions, or arrows
- left/right to collapse/expand current entry
- `e` to expand, `shift+e` to expand all siblings
- `c` to collapse, `shift+c` to collapse all siblings
- `ctrl+l` to toggle line wrapping
- `/` to search
- `f1` or type `:help` for the help screen

## Command-line arguments

```sh
tless --help
tless --version
printf '{"answer":42}' | tless --input-format json -
tless --max-input-bytes 1073741824 large.json
```

A missing filename or `-` reads stdin. `-i <format>` or `--input-format <format>`
overrides filename detection; supported values are `json`, `yaml`, and `toon` in
all builds, including `--no-default-features` builds.
With non-terminal stdout, including pipes and redirected files, output defaults to
standard TOON. Select `-o json`, `-o yaml`, or `-o toon` with the `--output-format` flag.
The `--input-format` option selects parsing independently; `--output-format` does not force machine mode
when stdout is a terminal or change interactive copy/write commands.

```sh
cat file.json | tless | cat                 # TOON output
cat file.json | tless -o json | consumer    # preserve JSON pipeline behavior
tless --input-format yaml -o yaml file.yaml > normalized.yaml
tless --input-format toon --output-format=json file.toon > converted.json
```

Every input is parsed and reserialized, including matching input/output formats.
TOON has no final newline and requires one root; an empty object emits no bytes.
JSON uses two-space pretty-printing and a final LF per root, preserving multiple
roots as a sequence of JSON values. Original JSON number tokens and duplicate
entries survive `-o json`. YAML emits `---` before each root and a final LF per
document. It preserves parsed types and entries, including typed keys and
non-finite numbers. Source comments, anchors, and formatting are not preserved.
JSON conversion rejects non-string keys and non-finite numbers.
Empty JSON remains a parse error; zero-root YAML emits nothing with JSON/YAML
output and fails with TOON output. Empty TOON decodes to an empty object.

This default change is incompatible with scripts expecting JSON. Add `-o json`
to those scripts. `-o yaml` reserializes YAML; it does not restore byte-for-byte
pass-through. Standard TOON retains the normalization and known codec limitations
[documented below](#toon-support).
Machine mode emits no prompts or ANSI styling. Diagnostics go to stderr.

Exit status 0 means success or a normal viewer quit.
Status 1 means an input, parsing, or output error, including a broken pipe.
Status 2 means invalid command-line arguments.
In the viewer, `q` or Ctrl-C quits; Ctrl-C or Ctrl-D in a command prompt cancels that prompt.
Before interactive mode and in pipelines, signals use the operating system's normal termination behavior.

The viewer retains the complete input and its parsed representation.
The default input limit is 512 MiB, measured in bytes; `--max-input-bytes 0` removes it.
The reader consumes at most the limit plus 1 byte before rejecting an oversized input.
Parsed data and rendered output need additional memory; this is not a total-process memory limit.
JSON and YAML have no configurable depth bound. TOON has the limits below.
No output starts until input loading, parsing, and machine-output serialization finish.
An output error can leave a partial payload in the downstream consumer.

## Color themes

Define your own colorscheme, or use one of the included schemes:

```sh
tless --theme desert data.json
tless --theme peachpuff data.json
tless --theme borealis data.json
```

Vim companion palettes require a 256-color terminal. Borealis uses exact
24-bit RGB colors and requires a true-color terminal. See the [Vim palette
audit and gallery](docs/vim-themes.md) for all 28 companion schemes.

Define named themes in `$XDG_CONFIG_HOME/tless/config.yaml` when
`XDG_CONFIG_HOME` is set and non-empty; otherwise use `~/.config/tless/config.yaml`.

```yaml
colorscheme: navy # set a default colorscheme
themes: # define custom colorschemes
  navy:
    object-key: blue
    object-key-focused: light-blue
    string: cyan
    status-bar-background: 18
    status-bar-foreground: cyan
```

See [examples/config.yaml](examples/config.yaml) for a complete example and
[Color schemes](docs/colorschemes.md) for configuration, tokens, and precedence.

## Platform contract

| Target | Release test host and support floor |
| --- | --- |
| aarch64-apple-darwin | Native macOS 15 arm64 |
| x86_64-apple-darwin | Native macOS 15 Intel |
| x86_64-unknown-linux-gnu | Native Ubuntu 24.04, glibc 2.39 |
| aarch64-unknown-linux-gnu | Native Ubuntu 24.04 arm64, glibc 2.39 |

Each release must pass native tests and an extracted-binary smoke test on all 4 hosts.
These are release gates, not a claim that an unpublished release has passed them.
Other Linux distributions and older operating systems are unverified.
Windows and musl are not supported.
Linux clipboard access requires a usable X11 or XWayland display session.

## TOON support

TOON input and canonical output are included in every build:

```sh
cargo install --path . --locked
tless data.toon
producer | tless --input-format toon
```

TOON support cannot be disabled. `--no-default-features` only omits the
colorscheme feature and can still be combined with `sexp`. All builds recognize
the three input and output values, and `--input-format json` or
`--input-format yaml` override filename detection.

This implementation targets `toon-spec: 3.0`, not TOON 4.x. It uses strict
two-space decoding with literal dotted keys and supports declared comma, tab,
and pipe delimiters. The viewer renders with commas and two-space indentation.
Searches operate on normalized JSON text, not the original TOON spelling.

Use `yt` to copy or `pt` to print the complete focused value as canonical TOON.
Use `:wt file` or `:writetoon file` to write the whole document. Add `!` to create
or replace a target. Encoding finishes before the file is opened, but an I/O
failure during writing does not promise rollback. Existing `yy`, `pp`, and
`:write` commands still produce JSON.

Canonical output uses two spaces, comma delimiters, no key folding, and no final
newline. An empty root object produces an empty payload. Whole-document output
requires one root; focused export also works with multi-root JSON input.
TOON behavior follows the published `toon-format` 0.5.0 crate.
Duplicate object keys use the last value. Decimal conversion can round, and very
large numeric literals can become strings or change value. Table encoding can
reorder object keys. These conversions apply to standard TOON exports. The
document view preserves the entries, order, and numeric precision in the parsed
model and marks display extensions with `# WARN` comments. Extended display
text is not standard TOON or a new export format.
See [codec behavior and known limitations](docs/toon-codec.md) for concrete examples.
Export rejects non-string YAML keys, non-finite numbers, and more than 256 nested
containers relative to the selected root. Input depth follows the codec's own bound.

TOON input accepts CRLF and trailing blank lines, but rejects an initial
BOM and blank rows inside arrays. Non-terminal output also validates TOON input
and reports parse failures before writing stdout.
Invalid UTF-8 is always an input error.

Builds resolve the published codec through Cargo.lock, with its CLI features disabled.
Keep `--locked` when building. The codec license is included in NOTICES.md.

Clipboard access uses arboard with text support only. Linux builds no longer
require libxcb development packages.

## Contribute and release

Run `scripts/preflight.sh` before submitting executable changes.
See [contributor guidance](docs/contributing.md) and the [release checklist](docs/release-checklist.md).

## Attribution

A fork of [jless](https://github.com/PaulJuliusMartinez/jless)

[MIT license](LICENSE.md)

See [third-party notices](NOTICES.md) for upstream, codec, fixture, and artwork attribution.
