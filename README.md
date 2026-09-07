# tless

A terminal viewer for JSON, YAML, and TOON, maintained by CommandZero.
This is an independent fork of [jless](https://github.com/PaulJuliusMartinez/jless).

[![ci](https://github.com/CommandZero/tless/actions/workflows/ci.yml/badge.svg)](https://github.com/CommandZero/tless/actions/workflows/ci.yml)

Expand and collapse data, navigate with vim-style keys, and search with regular expressions.
Press F1 or enter `:help` for in-app help.

## Install

Build from this checkout with Rust 1.87 or newer:

```sh
cargo install --path . --locked
tless data.json
tless data.yaml
tless data.toon
producer | tless --toon
```

The first CommandZero release is planned as 0.10.0.
Binary downloads will appear on the [releases page](https://github.com/CommandZero/tless/releases) after validation and maintainer publication.
No crates.io or Homebrew installation for this fork is advertised yet.
The codec comes from crates.io. Registry publication of tless remains a separate release decision.

The executable is `tless`. Update scripts and aliases that should use this fork.
The upstream `jless` executable can remain installed alongside it.

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
Linux clipboard support requires X11 and libxcb; clipboard access also needs a usable display session.

## Command-line contract

```sh
tless --help
tless --version
printf '{"answer":42}' | tless --json -
tless --max-input-bytes 1073741824 large.json
```

A missing filename or `-` reads stdin. Format flags override filename detection.
With redirected stdout, JSON is pretty-printed; YAML and TOON pass through as UTF-8 bytes without syntax validation.
JSON output can contain multiple top-level values separated by newlines.
TOON pass-through preserves the input's framing, including a missing final newline.
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
No output starts until input loading and any machine-mode JSON parsing finish.
An output error can leave a partial payload in the downstream consumer.

## TOON support

Build from this checkout to enable TOON input and canonical output:

```sh
cargo install --path . --locked --features toon
tless data.toon
producer | tless --toon
```

The `toon` Cargo feature is enabled by default. Use `--no-default-features`
for a JSON/YAML-only source build. It can be combined with `sexp`.
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
TOON behavior follows the published `toon-format` 0.5.0 crate.
Duplicate object keys use the last value. Decimal conversion can round, and very
large numeric literals can become strings or change value. Table encoding can
reorder object keys. This viewer is not an exact TOON data-conversion tool.
See [codec behavior and known limitations](docs/toon-codec.md) for concrete examples.
Export rejects non-string YAML keys, non-finite numbers, and more than 256 nested
containers relative to the selected root. Input depth follows the codec's own bound.

Interactive input accepts CRLF and trailing blank lines, but rejects an initial
BOM and blank rows inside arrays. With non-terminal stdout, selected TOON text
passes through unchanged without parsing, even if its syntax is malformed.
Invalid UTF-8 is always an input error.

Builds resolve the published codec through Cargo.lock, with its CLI features disabled.
Keep `--locked` when building. The codec license is included in NOTICES.md.

On Linux systems, X11 libraries are needed to build clipboard access if
building from source. On Ubuntu you can install these using:

```
sudo apt-get install libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
```


## Contribute and release

Run `scripts/preflight.sh` before submitting executable changes.
See [contributor guidance](docs/contributing.md) and the [release checklist](docs/release-checklist.md).

## Attribution

The upstream viewer, its mascot Jules, and its historical release notes remain attributed to their authors.
Jules artwork is by [annatgraphics](https://www.fiverr.com/annatgraphics).
The code retains the [MIT license](LICENSE.md).
See [third-party notices](NOTICES.md) for the codec and specification fixtures.
