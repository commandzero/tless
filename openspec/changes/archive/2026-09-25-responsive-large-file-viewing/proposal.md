## Why

Large documents pay for repeated full-document display construction before the first useful screen. Parsing is comparatively inexpensive. Fix the redundant startup layout work rather than replacing the viewer's interaction architecture.

## What Changes

- Construct the initial viewer with actual terminal dimensions and effective absolute/relative line-number settings.
- Reuse a rendered layout when a wider number gutter does not change any inline-array fit; update the width used by collapsed previews. Rebuild normally when a fit actually changes.
- Preserve complete parsing, exact display text, warnings, source identities, navigation, wrapping, search, filtering, copy/export, and startup error behavior.
- Measure release startup on a disposable synthetic workload and retain focused geometry regressions.

The user explicitly narrowed this change after reviewing an over-complicated implementation. Background workers, loading/pending states, cancellation protocols, presentation-memory budgets, demand rendering, source-fragment output, and fixed interaction-latency guarantees are outside this change. There is no new 500 ms startup guarantee or total-memory claim.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `toon-rendering`: Initial layout uses real geometry and avoids redundant gutter renders when grammar is unchanged.

## Impact

Production changes are limited to `src/app.rs`, `src/viewer.rs`, and `src/toon_display.rs`. Existing renderer/navigation tests cover preserved behavior; focused tests cover initial Unicode fits and gutter digit boundaries. No new dependencies, threads, terminal event types, or persistent profiling framework. Keep Rust 1.87 compatibility, the committed lockfile, and the existing published codec dependency. Update the Unreleased changelog and synchronize the narrowed rendering requirement before completion.
