## Why

[Issue #7](https://github.com/commandzero/tless/issues/7) asks for redirected output to default to TOON, matching the viewer's default format. Today `cat file.json | tless | cat` emits JSON, while YAML and TOON inputs pass through without validation.

## What Changes

- **BREAKING**: Default non-terminal stdout to standard TOON serialization, regardless of input format.
- Add `-o` / `--output` with `json`, `yaml`, and `toon` values. Input selectors and filename detection remain independent.
- Parse and serialize redirected input before writing. Explicit JSON preserves the existing JSON-input pretty-printing contract; YAML output supports document streams.
- Reuse the published TOON export contract, including normalization, framing, resource limits, and known codec limitations. Do not serialize the interactive display.
- Retain the optional TOON input/export feature. A build without it reports an actionable error for redirected default or explicit TOON output; explicit JSON and YAML remain available.
- Document migration, output framing, failures, and the scope of the output selector. Terminal stdout continues to open the viewer.

## Capabilities

### New Capabilities

- `piped-output`: Non-terminal output selection, format conversion, framing, feature availability, and failure behavior.

### Modified Capabilities

None. Existing rendering and display-extension specs govern the viewer and existing interactive exports. This change adds a separate noninteractive serialization contract.

## Impact

Implementation touches `src/options.rs`, the redirected-output branch in `src/main.rs`, serialization helpers, and `tests/toon_cli.rs`. YAML serialization must use parsed values, including typed keys and decoded strings. TOON integration stays behind `src/toon.rs` with the published dependency and its default features disabled.

Update README, applicable TOON guides and acceptance checks, CLI help, and Unreleased release notes during implementation. This changes the public CLI default and belongs in a minor release under the repository's 0.x policy. Rust 1.87 compatibility, the committed lockfile, and all four feature profiles remain required. No registry publication or release is part of this proposal.
