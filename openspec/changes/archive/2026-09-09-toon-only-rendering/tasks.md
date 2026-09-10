## 1. Completion gate

- [x] 1.1 Inspect and pin the OpenSpec CLI used locally and in CI; verify native active, main-spec, and archived validation with telemetry disabled.
- [x] 1.2 Add one PR-scoped completion entry point shared by preflight and CI; verify added, edited, deleted, renamed, and explicitly associated changes, plus unrelated active changes.
- [x] 1.3 Require preserved archives, native validation, and delta-to-main-spec synchronization review before merge; verify deleted-only changes, skipped synchronization, already-synced specs, and multiple associated changes without duplicating the native validator.

## 2. Parsed-node layout

- [x] 2.1 Add an unconditional TOON display module with stable node identities, line ownership, spans, and container extents; verify mappings for nested objects, inline elements, and table cells without export-adapter conversion.
- [x] 2.2 Implement standard TOON object, primitive-array, table, and list layouts; verify retained grammar fixtures and exact outputs for root primitives, empty roots, nested arrays, and arrays of empty objects.
- [x] 2.3 Implement quoting, escaping, field-order preservation, and table eligibility; verify ambiguous strings, delimiters, duplicate fields, non-string keys, and differing row key order.
- [x] 2.4 Implement exact decimal normalization with checked sizing and the 4096-character fallback; verify decimal precision, negative zero, noncanonical YAML numeric spellings, exponent bounds, and no huge decimal allocation.

## 3. Display extensions

- [x] 3.1 Preserve and annotate duplicate entries using decoded key equality; verify all occurrences, escaped equivalent keys, order, identity, and immediate object counts.
- [x] 3.2 Render non-finite values and typed compact non-string keys with warnings; verify infinities, NaN, numeric versus string keys, and recursive complex keys without type coercion.
- [x] 3.3 Render multiple roots with warning separators; verify root shape, order, empty roots, absolute addresses, and separator selection behavior.
- [x] 3.4 Add deterministic warning records and shared-line comments with escaped locators; verify ordering, one final comment per line, source strings containing `# WARN`, and controls in field names.
- [x] 3.5 Cache descendant warning counts; verify collapsed ancestors retain their own warnings and count hidden semantic warnings without counting painted duplicates.

## 4. Collapse and viewport

- [x] 4.1 Project collapse state over TOON layout; verify header retention, table-row collapse, inline-array previews, empty containers, and descendant-state restoration.
- [x] 4.2 Apply syntax and annotation styles; verify expanded values are not subdued previews, arrays receive no duplicate count, and objects count immediate entries.
- [x] 4.3 Preserve optional gutters with TOON absolute addresses and relative visible-motion counts; verify shared-line values, collapsed gaps, separator skipping, and existing defaults.
- [x] 4.4 Add cell-aware clipping and horizontal scrolling; verify preview truncation, narrow terminals, wide/combining Unicode, escaped controls, reachable warnings, and resize-stable layouts.

## 5. Navigation and search

- [x] 5.1 Separate logical focus from display-line anchors; verify selection and paths for headers, inline elements, table objects/cells, implicit roots, and duplicate occurrences.
- [x] 5.2 Adapt vertical, counted, structural, page, and absolute-jump motions; verify table-column retention, same-line siblings, hidden targets, scrolloff, and boundaries.
- [x] 5.3 Map existing search matches to node/token spans; verify hidden matches reveal ancestors, shared field headers retain row identity, and warnings/previews do not add matches.
- [x] 5.4 Add row-and-column mouse hit testing and focus recovery; verify cell selection, arrow/warning ownership, resize behavior, and nearest-visible-ancestor fallback.

## 6. Switch the document view

- [x] 6.1 Replace line/data rendering and remove mode state, `--mode`, `-m`, interactive `m`, and closing-pair actions; verify unsupported flags and updated help while retaining unrelated commands.
- [x] 6.2 Keep parser/export features separate from unconditional rendering; verify JSON rendering without default features and unchanged `toon`/`sexp` command selection.
- [x] 6.3 Keep copy/export tied to parsed selections; verify cell/duplicate identity, annotation-free output, existing TOON normalization/errors, and unchanged redirected stdout.
- [x] 6.4 Update help, README, codec/acceptance docs, and Unreleased migration notes; verify extension examples, parsed-model fidelity, optional gutters, TOON line addresses, and display/export boundaries, then run `scripts/validate-docs.sh`.

## 7. Integration and completion

- [x] 7.1 Add terminal coverage for equivalent inputs, collapse/search/copy, resizing, and warnings; verify with terminal access and complete manual wide/narrow-terminal acceptance checks.
- [x] 7.2 Verify large-array redraws reuse layout without whole-document encoding and huge-number rendering stays bounded; record workload and observed behavior.
- [x] 7.3 Run `scripts/preflight.sh` and `TLESS_TOOLCHAIN=1.87.0 scripts/preflight.sh test`; verify supported feature profiles pass, licenses remain intact, and no vendored or patched codec enters the lockfile.
- [x] 7.4 Run `OPENSPEC_TELEMETRY=0 openspec validate toon-only-rendering --strict --no-interactive` and review every requirement/scenario against implementation; record and resolve findings before completion.
- [x] 7.5 At archive time, verify implementation completion, synchronize deltas into main specs, and archive; verify native active/main/archived validation and the PR-scoped completion gate before merge.
