## Why

tless still renders through the inherited JSON line and data modes even when reading TOON. Its document view should use TOON syntax directly, with collapse annotations and explicit warnings where the parsed data needs an extension.

## What Changes

- **BREAKING**: Replace both rendering modes with one TOON document view. Remove `--mode`, `-m`, the `m` toggle, and closing-delimiter navigation.
- Render the existing TOON 3.0 profile with syntax colors, inline primitive arrays, tabular arrays, list arrays, and required quoting.
- Restrict ordinary document annotations to collapse arrows, subdued collapsed previews, and immediate-entry counts for collapsed objects. Arrays retain their TOON counts.
- Preserve optional absolute and relative line-number gutters outside document text.
- Add a documented tless display extension that preserves duplicate entries, non-finite numbers, non-string YAML keys, and multiple roots. Show subdued inline `# WARN ...` comments instead of silently converting or rejecting these parsed values.
- Preserve logical value identity, paths, search, and selection when several values share one TOON line.
- Make rendering available in every feature profile while retaining existing input and export feature selection.
- Establish the OpenSpec completion gate for this repository before the associated implementation PR merges.

## Capabilities

### New Capabilities

- `toon-rendering`: The sole document layout, syntax styling, collapse annotations, gutters, and viewport behavior.
- `toon-display-extensions`: Faithful display of parsed values outside the standard TOON data model, with deterministic warning comments and explicit export boundaries.
- `toon-navigation`: Logical focus, motion, search, selection, and paths over TOON lines, inline elements, and table cells.

### Modified Capabilities

None. There are no existing main OpenSpec capability specs in this checkout.

## Impact

The implementation affects `src/lineprinter.rs`, `src/viewer.rs`, `src/screenwriter.rs`, `src/search.rs`, `src/app.rs`, `src/options.rs`, the TOON adapter, help text, and renderer/terminal tests. It needs a mapping from parsed nodes to rendered lines and spans; changing the current mode enum alone is insufficient.

Input parsers and machine-output contracts remain unchanged. Fidelity starts at the parsed model: this change does not recover information an existing parser has already discarded. Existing exports remain separate from the extended display and retain their documented conversion behavior.

Retain Rust 1.87 compatibility, the committed lockfile, and the published codec with default features disabled. Do not vendor codec source or add a Cargo patch. Update user documentation and Unreleased notes during implementation. Planning artifacts stay outside the `docs/` OKF bundle.
