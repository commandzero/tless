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
forecast[3]{day,temp{min,max},condition,rainChance}:
  Mon,-2,4,snow,80
  Tue,1,7,cloudy,20
  Wed,3,11,sunny,5
```

Yes, that is the exact same data.

Also look at the differences between `examples/nato-phonetics.json` and `examples/nato-phonetics.toon`:

| Format | Lines | Tokens |
| --- | ---: | ---: |
| JSON | 249 |  1,602 |
| TOON | 47 | 546 |
| Saved | 202 | 1,056 |
| Saved % | 81.1% | 65.9% |

That is far fewer tokens for your LLM, [higher accuracy](https://toonformat.dev/guide/benchmarks.html#retrieval-accuracy), less lines for commit diffs, _and_ easier on your eyes.

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
cat <filename> | tless --input-format json
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
printf '{"answer":42}' | tless -i json -
tless --max-input-bytes 1073741824 large.json
```

The default input limit is 512 MiB, measured in bytes; `--max-input-bytes 0` removes it.

A missing filename or `-` reads stdin. Format flags override filename detection.
Output defaults to standard TOON. Select `-o json`, `-o yaml`, or `-o toon` with
the `--output-format` flag.

```sh
cat file.json | tless | cat                 # TOON output
cat file.json | tless -o json | consumer    # preserve JSON pipeline behavior
tless --output-format json file.toon > converted.json
tless -o yaml file.yaml > normalized.yaml
```

## Color themes

Define your own colorscheme, or use one of the included vim-inspired ones:

```sh
tless --theme desert data.json
tless --theme peachpuff data.json
tless --theme borealis data.json
```

Vim companion palettes require a 256-color terminal. Borealis uses exact
24-bit RGB colors and requires a true-color terminal. See the [Vim palette
audit and gallery](docs/vim-themes.md) for all 28 companion schemes.

Define named themes in `~/.config/tless/config.yaml`. (Or `$XDG_CONFIG_HOME` location)

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

See [examples/config.yaml](examples/config.yaml) for a complete example.

## Contribute and release

Run `scripts/preflight.sh` before submitting executable changes.
See [contributor guidance](docs/contributing.md) and the [release checklist](docs/release-checklist.md).

## Attribution

A fork of [jless](https://github.com/PaulJuliusMartinez/jless)

[MIT license](LICENSE.md)
