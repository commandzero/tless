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

- No filename or `-` reads from `stdin`
- Redirecting output bypasses interactive viewing mode
- Use `-i <format>` or `--input-format <format>` for `stdin` or to override filename detection
- Use `-o <format>` or `--output-format <format>` to select the non-terminal output format
- Supported input/output formats are `toon`, `json`, and `yaml`
- Default input limit of 512 MiB, use `--max-input-bytes 0` to remove it

```sh
cat file.json | tless | cat                 # TOON output
cat file.json | tless -o json | consumer    # preserve JSON pipeline behavior
tless --input-format yaml -o yaml file.yaml > normalized.yaml
tless --input-format toon --output-format=json file.toon > converted.json
```

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
