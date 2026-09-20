## Why

Large nested documents spend viewport space on ancestors and unrelated siblings. [Issue #10](https://github.com/commandzero/tless/issues/10) requests a selectable subtree that reduces indentation without losing the original path or navigation identity.

## What Changes

- Add `--path <path>` for initial selection in the viewer and machine output.
- Add friendly dot paths and strict JSON Pointers in the `:` prompt and CLI. Accept the concrete `yp` representation unchanged, including `.hits[0].name`, quoted special-key access, and leading brackets emitted by `yp`. Retain `.hits.0.name` shorthand, `./hits/0/name` strict syntax, and `.` reset. The CLI additionally accepts bare RFC 6901 pointers and the empty pointer.
- Render selected values as roots, with no ancestor headings or sibling content. Retain original parsed identity and full status/copy paths.
- Confine navigation, search, numbering, and interaction to the selected subtrees. New paths always resolve from original document roots, not the current filter.
- Apply the same path to every document in a stream, in input order. Resolve all documents before applying the selection; reject missing or ambiguous paths rather than dropping documents.
- Scope interactive file exports and machine output to selected roots, preserving existing per-format framing, normalization, limitations, and overwrite safeguards. Focused-value copy remains focused-value copy.
- Guarantee filter-input compatibility for `yp` paths, not the separate `yb` bracket representation. Preserve `yq` and its jq-query behavior as a compatibility promise; do not reinterpret its `[]` expressions as concrete locations.
- Preserve current behavior when no filter is supplied. This is opt-in; no existing CLI invocation changes meaning.

## Capabilities

### New Capabilities

- `path-filter`: Friendly/strict syntax, `yp` round trips, resolution, atomic state changes, subtree rendering, original identity, interactive exports, and help.

### Modified Capabilities

- `toon-navigation`: Define the active-root scope for structural motion, search, focus, and numbered jumps.
- `piped-output`: Apply path selection before serialization while retaining whole-input parsing and output safety guarantees.

## Impact

- `src/options.rs`, `src/main.rs`, and `src/command.rs`: path entry points and diagnostics.
- `src/flatjson.rs` and a focused path resolver: decoded-key lookup using existing parsed nodes, without JSON round trips or a second document model.
- `src/viewer.rs`, `src/toon_display.rs`, `src/search.rs`, `src/screenwriter.rs`, and `src/app.rs`: selected-root projection, interaction scoping, full-path status, and file export.
- `src/output.rs` and `src/toon.rs`: selected-root serialization using existing formats and codec restrictions.
- CLI/help documentation and behavior-focused tests: syntax disambiguation, error atomicity, sequence handling, navigation confinement, and export scope.
- No new crate, codec replacement, dependency upgrade, minimum compiler change, or release action is proposed. Rust 1.87 and the committed lockfile remain supported.
