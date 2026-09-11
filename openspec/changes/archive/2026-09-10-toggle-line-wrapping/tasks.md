## 1. Physical row layout

- [x] 1.1 Add the shared physical-row projection after logical TOON layout and collapse projection; verify logical owners, absolute addresses, and primitive-array choices remain unchanged when only wrapping changes.
- [x] 1.2 Implement grapheme-aware segmentation using available document cells and indentation reduction; test exact fits, wide characters, combining sequences, emoji, escaped controls, zero width, and overwide-grapheme placeholders.
- [x] 1.3 Cache row boundaries by layout and geometry with no copied row text; verify invalidation after width, gutters, indentation, and collapse changes and forward progress on a large value in a narrow viewport.

## 2. Rendering and input

- [x] 2.1 Add session wrapping state and document-view Ctrl+L dispatch with on/off messages; verify default-off, repeated toggles, numeric prefixes, help return, and unchanged prompt-editor handling.
- [x] 2.2 Paint physical row slices using original token spans and explicit row bounds; verify first-row-only numbers and arrows, blank continuation gutters, styling across boundaries, and no writes into status or command rows.
- [x] 2.3 Keep collapsed previews, counts, warnings, and syntax-colored collapsed inline arrays on the one-row clipping path; test preview fitting, horizontal access, and restoration after expansion.
- [x] 2.4 Implement horizontal-offset reset and disabled horizontal commands for wrapped expanded lines; verify toggle-off reveals the selected span or active match and collapsed lines retain horizontal controls.

## 3. Navigation and viewport integration

- [x] 3.1 Preserve logical motions, table-field retention, relative distances, and numbered jumps; test one-step and counted navigation past wrapped values and jumps around collapsed descendants.
- [x] 3.2 Convert explicit scrolling, wheel input, paging, screen-position selection, and repositioning to physical coordinates; test a value taller than the viewport, scroll padding, document boundaries, and large counts without changing focus inside the same line.
- [x] 3.3 Map search reveal and mouse hits through row slices; test matches crossing wrap boundaries, long shared table headers, continuation-cell selection and copy targets, and inert continuation collapse gutters.
- [x] 3.4 Preserve selection, match identity, and collapse state across reflow; test resize, gutter toggles, indentation changes, wrapping toggles, and explicit multiline-array restoration.

## 4. Documentation and verification

- [x] 4.1 Update interactive help, relevant README guidance, TOON acceptance documentation, and the Unreleased changelog; verify the toggle and scrolling behavior are discoverable and run `scripts/validate-docs.sh`.
- [x] 4.2 Add an isolated terminal integration test for Ctrl+L, continuation gutters, tall-value scrolling, resize, and toggle-off; verify it passes with terminal access and that copy/export/piped-output regression tests still pass.
- [x] 4.3 Run `scripts/preflight.sh` and `TLESS_TOOLCHAIN=1.87.0 scripts/preflight.sh test`; record results across required feature profiles and verify implementation correspondence against every changed or added scenario.
- [x] 4.4 Synchronize both delta specs, review their correspondence with main specs, and archive the completed change; verify native strict change/spec validation, archived-task validation, and the committed-head PR-scoped completion gate with the required association and sync-review fields.
