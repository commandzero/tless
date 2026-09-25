## Why

Large documents wait for repeated full-document TOON display construction before the first screen, and resizing, wrapping, and visibility changes repeat work that blocks input. Profiling distinguishes this from codec conversion: JSON parsing is inexpensive relative to eager display layout, so moving the existing conversion wholesale to a thread would leave the main cost intact.

## What Changes

- Prepare the first useful viewport without rendering and copying every display line; reuse document analysis across width, gutter, wrapping, and collapse changes.
- Initialize geometry from the actual terminal and replace repeated full-render gutter/inline-fit passes with indexed geometry calculations.
- Run input loading and expensive preparation on native worker threads, integrated into the existing terminal event loop without an async runtime.
- Keep loading, uncached navigation, reflow, filtering, and search cancellable or supersedable; retain stable parsed identities and reject obsolete results.
- Bound presentation caches and background queues, including giant scalar and table-header cases. Retain complete input parsing and an in-memory document; this is not pre-EOF document streaming or out-of-core storage.
- Preserve TOON grammar, source-based search, logical/physical navigation, warnings, duplicate occurrences, copy/export behavior, and non-terminal output contracts.
- **BREAKING**: interactive startup may enter a responsive loading screen before input validation and CLI path resolution finish. Those failures restore the terminal before reporting stderr and exiting 1. Argument/path-syntax failures still occur before terminal setup and exit 2; non-terminal output remains free of terminal controls and partial validation output.
- Add reproducible synthetic performance scenarios, behavioral terminal coverage, and separate time-to-loading-screen, time-to-useful-content, interaction, and resource acceptance gates.

## Capabilities

### New Capabilities

- `responsive-viewer`: Responsive loading and worker lifecycle, bounded presentation work, scheduling, cancellation, stale-result protection, and performance acceptance.

### Modified Capabilities

- `toon-rendering`: Demand-prepared viewport content and atomic geometry publication without changing completed TOON presentation.
- `toon-navigation`: Indexed, nonblocking navigation and search with explicit pending behavior and stable identity across reflow.
- `path-filter`: Atomic background filter preparation and the revised interactive startup error lifecycle.

## Impact

- Primary modules: `src/main.rs`, `src/input.rs`, `src/app.rs`, `src/flatjson.rs`, `src/viewer.rs`, `src/toon_display.rs`, `src/wrapped_view.rs`, `src/screenwriter.rs`, and `src/search.rs`; filter resolution and codec calls participate where background lifecycle requires it.
- Keep one application crate, Rust 1.87 compatibility, the committed lockfile, and the published `toon-format` dependency with default features disabled. No async runtime, vendored codec, or Cargo patch.
- Update affected terminal/navigation tests, add deterministic synthetic fixture generation and an opt-in release profiling harness, and update user documentation/help and the Unreleased changelog during implementation.
- Timing targets apply to a documented controlled native reference run, not arbitrary CI hosts or remote/slow input sources. Structural resource and interaction correctness checks remain deterministic CI coverage.
- Existing main specifications remain authoritative except for the deltas in this change. This proposal contains no implementation and does not synchronize or archive its deltas yet.
