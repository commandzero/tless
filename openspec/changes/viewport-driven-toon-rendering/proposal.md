## Why

Native TOON rendering replaced jless's viewport-driven formatting with document-wide rendered text, previews, source maps, and visibility copies. On the synthetic 10.24 MB reference input, current tless takes about 911 ms to display its first useful frame versus 44 ms for jless; measured parsing takes only 44 ms, while TOON layout and visibility projection consume about 873 ms.

## What Changes

- Restore viewport-driven, synchronous presentation over `FlatJson`, with a compact TOON structural index instead of eagerly rendered document text.
- Materialize text, styling spans, previews, source maps, and wrapped continuations for the current viewport or an explicitly requested navigation target, not every off-screen row.
- Preserve TOON grammar, absolute and relative numbering, table-cell and inline-element identity, search, mouse selection, collapse, wrapping, sequence documents, filtering, themes, and copy/export behavior.
- Require a first-useful-frame benchmark on synthetic input: median and p95 no more than three times jless data view, and median at least five times faster than the frozen current tless baseline on the same host. These are proposed acceptance targets, not achieved results.
- Remove the obsolete eager interactive layout and its dependent projections after migrating all consumers. Whole-document export remains an explicit separate operation.
- Do not add background workers, an async runtime, loading states, cache-budget machinery, new dependencies, or a new view mode.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `toon-rendering`: add demand-driven presentation and measured startup-performance requirements while retaining the existing rendering and interaction contracts.

## Impact

- Primary implementation: `src/toon_display.rs`, `src/viewer.rs`, `src/screenwriter.rs`, `src/lineprinter.rs`, and `src/wrapped_view.rs`.
- Integrations: node/source lookup for search reveal and mouse actions, app viewport updates, filtered roots, sequence metadata, and any export callers sharing current layout helpers.
- Validation: existing behavior suites, focused regressions for off-screen/shared-line identities and reflow, real PTY comparison, and a small synthetic performance harness.
- No CLI, input-format, output-format, parser, codec, dependency, or MSRV change. Main specs remain untouched until implementation is verified and synchronized.
