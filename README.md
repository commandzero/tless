# tless

[![ci](https://github.com/CommandZero/tless/actions/workflows/ci.yml/badge.svg)](https://github.com/CommandZero/tless/actions/workflows/ci.yml)

View your data in the compact TOON format, including JSON and YAML files. Forked from the excellent [jless](https://github.com/PaulJuliusMartinez/jless). Expand and collapse data, navigate with vim-style keys, and search with regular expressions.

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
- `Tab` on a table header, row, or cell to toggle that table's space-separated aligned columns;
  numbers right-align, while strings and headers stay left-aligned;
  status shows `Table aligned` (`Align` on narrow screens) while focused there
- `,`/`.` (10 cells per numeric prefix) to scroll an aligned table together;
  `;` to jump to its shared end/start, including long warnings
- `ctrl+l` wraps other expanded lines but leaves aligned tables unwrapped and
  horizontally scrollable; disabling alignment restores the current wrap policy
- `/` to search
- `f1` or type `:help` for the help screen

## Command-line arguments

```sh
tless --help
tless --version
printf '{"answer":42}' | tless --input-format json -
tless --max-input-bytes 1073741824 large.json
tless --path '.hits[0].name' response.json
```

- No filename or `-` reads from `stdin`
- Redirecting output bypasses interactive viewing mode
- Use `-i <format>` or `--input-format <format>` for `stdin` or to override filename detection
- Use `-o <format>` or `--output-format <format>` to select the non-terminal output format
- Supported input/output formats are `toon`, `json`, and `yaml`
- Default input limit of 512 MiB, use `--max-input-bytes 0` to remove it
- Use `--path` to select a subtree before viewing or exporting; the full input is still parsed

```sh
cat file.json | tless | cat                 # TOON output
cat file.json | tless -o json | consumer    # preserve JSON pipeline behavior
tless --input-format yaml -o yaml file.yaml > normalized.yaml
tless --input-format toon --output-format=json file.toon > converted.json
```

## Interactive file writes

At the `:` prompt, give every write command one explicit filename. The command
selects the encoding regardless of the input format, CLI output selector, or
filename extension. These commands write all **active roots** (including
subtrees selected with `--path` or an interactive path filter), not just the
focused value or the annotated screen:

| Commands | Output |
| --- | --- |
| `:write`, `:w`, `:write-toon`, `:wt` | Standard TOON (the default) |
| `:write-json`, `:wj` | Pretty JSON |
| `:write-yaml`, `:wy` | YAML document stream |
| `:write-ndjson`, `:write-jsonl`, `:wn` | Compact JSON, one root per LF-terminated line |
| `:write-sexp`, `:ws` | S-expressions, **only** in builds with the optional `sexp` feature |

Each spelling also has a `!` overwrite form (`:write!`, `:w!`,
`:write-toon!`, `:wt!`, `:write-json!`, `:wj!`, `:write-yaml!`, `:wy!`,
`:write-ndjson!`, `:write-jsonl!`, `:wn!`, and, with `sexp`,
`:write-sexp!` and `:ws!`). For example, `:write report.toon` creates a
new TOON file and `:wj! report.json` replaces an existing JSON file.
Without `!`, an existing destination is left untouched. Every active root is
encoded before the destination is opened, so encoding errors never create or
truncate a file. File I/O failures after opening can leave partial output;
`!` is not an atomic replacement guarantee.

Filenames are literal: `:write report` creates `report`, not `report.toon`;
`:write-json report.toon` writes JSON to `report.toon`. Suffixes such as
`.toon`, `.json`, `.yaml`, `.ndjson`, `.jsonl`, and `.sexp` are conventions,
not format selectors. TOON requires exactly one active root and has no final
newline. JSON writes two-space pretty-printed values, one final LF per root,
in root order without an array wrapper. YAML puts `---` before each root
and ends each document with LF. NDJSON/JSONL writes one compact JSON value
and LF per active root: an array root stays one array record, not one record
per element. Reset a filter with `:.` to write all original roots.

**Breaking migration:** `:write`/`:w` previously wrote JSON; use
`:write-json report.json` or `:wj report.json` for JSON, and
`:write report.toon` for TOON. The old `:writetoon`/`:writetoon!`
names are removed; use `:write-toon`/`:write-toon!` (or `:wt`/`:wt!`).
With `sexp`, `:writesexp`/`:writesexp!` are removed; use
`:write-sexp`/`:write-sexp!` (or `:ws`/`:ws!`). Default builds do not
include s-expression output. CLI `-i`/`--input-format` and
`-o`/`--output-format` still accept only `toon`, `json`, and `yaml`;
interactive NDJSON/JSONL and s-expression write names do not add CLI formats.
See [document-view export details](docs/toon-view.md#copy-and-export).

## Path filtering

Use `--path '.hits.0.name'`, `--path '.hits[0].name'`, or the strict JSON pointer
`--path './hits/0/name'`. In the viewer, enter the same path at `:`, for example
`:.hits[0].name`. Paths copied with `yp` can be pasted unchanged into either
entry point, including `["a.b"][0].name` and root-array paths such as `[0].name`.
`yq` remains a jq query; `yb` remains an external-language path, not a promised
filter input.

`.` restores the original roots; CLI `--path ''` is equivalent. `./` instead
selects an empty object key. Filters are absolute, apply to every input document,
and fail atomically if any document cannot resolve the path. Navigation, search,
and whole-document exports use the selected subtrees. See the
[path-filter guide](docs/toon-view.md#path-filtering) for escaping and state rules.

## Color themes

Define your own colorscheme, or use one of the included schemes:

```sh
tless --theme desert data.json
tless --theme peachpuff data.json
tless --theme borealis data.json
```

Vim companion palettes require a 256-color terminal.  See the [Vim palette
audit and gallery](docs/vim-themes.md) for all 28 companion schemes. Borealis uses
24-bit RGB colors and requires a true-color terminal.

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

## Contribute and release

Run `scripts/preflight.sh` before submitting executable changes.
See [contributor guidance](docs/contributing.md) and the [release checklist](docs/release-checklist.md).
The supported targets and OS floors are listed in the [platform contract](docs/contributing.md#platform-contract).

## Attribution

A fork of [jless](https://github.com/PaulJuliusMartinez/jless)

[MIT license](LICENSE.md)
