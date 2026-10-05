# Tasks

## 1. Table state and aligned presentation

- [x] 1.1 Add identity-based per-table alignment state and compact column metrics using existing eligibility and scalar rendering rules; verify focused header/row/cell ownership, independent tables, non-tabular no-ops, and an off-screen widest Unicode or quoted value with behavior-focused unit coverage.
- [x] 1.2 Materialize aligned headers and rows with right padding outside token spans and row-leading grid padding; verify the specification's exact `users` example, terminal-cell starts for wide/combining/emoji tokens, warning placement, nested tables, indentation reduction, and all gutter settings using rendered-output and span-boundary checks.
- [x] 1.3 Keep column measurement separate from lazy row formatting and cache only per-column metrics and compact extents; verify with a large synthetic table that first enable includes its final row, redraw reuses metrics, unrelated off-screen rows are not materialized, and traversal does not accumulate row presentation.
- [x] 1.4 Add alignment transitions to wrap eligibility and presentation-cache invalidation; verify alignment overrides wrapping only for its expanded table, Ctrl+L preserves enabled state, collapse stays unchanged, and disabling alignment restores the current wrap policy without losing selected node or line addresses.
- [x] 1.5 Document the aligned text example, complete-table width rules, presentation-only boundary, and local wrapping override in `docs/toon-view.md`; verify examples against actual rendered output and run `scripts/validate-docs.sh`.

## 2. Shared horizontal scrolling and source identity

- [x] 2.1 Route screenwriter viewport lookup and counted `,`/`.`/`;` scrolling through one table-owned offset for expanded aligned members, with separate ordinary collapsed-preview offsets; verify header/all-row movement, newly exposed rows, short focused rows, long annotations, saturated counts, end/start bounds, and unrelated-line isolation.
- [x] 2.2 Apply the shared ownership and table coordinate transform to selected-span reveal, search reveal, clipping, and mouse mapping; verify clipped value and shared-field-key matches, original row-field paths and copy targets, clicks after nonzero scrolling, and padding clicks selecting the line owner.
- [x] 2.3 Preserve table settings and valid shared offsets across focus changes, collapse/reopen, resize, gutters, indentation reduction, and path-filter exclusion/restoration; verify restoration and bounds clamping, no inherited table grid on filtered cell/row roots, and zero-based enable/disable transitions followed by selected-span reveal.
- [x] 2.4 Document shared counted scrolling, end/start behavior, search reveal, and padding selection in `docs/toon-view.md`; verify documented interactions in a native PTY and run `scripts/validate-docs.sh`.

## 3. Tab interaction, indication, and end-to-end acceptance

- [x] 3.1 Bind document-view Tab to one toggle regardless of a numeric prefix and add a persistent textual focused-table status indicator with narrow-width handling; add PTY coverage proving Tab on header/row/cell, independent tables, non-tabular no-ops, preserved collapsed state, indicator persistence, and unchanged command/search prompt behavior.
- [x] 3.2 Exercise the running TUI with two tables, nested data, wide/escaped cells, an off-screen widest row, warnings, wrapping, resize, mouse selection, and search; retain consumer-visible regression coverage in the existing unit/PTY suites and record actual observed frames, column positions, paths, and scroll transitions for the acceptance scenarios.
- [x] 3.3 Verify serialization isolation through the running program: copy an aligned cell and write an aligned table in supported formats, then compare parsed results with the unaligned selections and check redirected stdout; retain regression coverage only for consumer-visible data or output differences.
- [ ] 3.4 Update `src/tless.help`, `src/toon.help`, relevant README controls, `docs/toon-acceptance.md`, and the Unreleased changelog for Tab, persistent indication, synchronized scrolling, and the local wrapping override; verify interactive help in the TUI, execute the documented acceptance steps, and run `scripts/validate-docs.sh`.

## 4. Integration and OpenSpec completion

- [ ] 4.1 Run `scripts/preflight.sh` and `TLESS_TOOLCHAIN=1.87.0 scripts/preflight.sh test`; verify all applicable checks pass and record native terminal smoke evidence separately from test results.
- [ ] 4.2 Synchronize both delta specs into the main capabilities, compare every added/modified requirement and scenario against the implemented behavior, and archive this change with the pinned OpenSpec CLI; verify strict native change/spec/archive validation and preserve all planning artifacts in the archive.
- [ ] 4.3 Before merging the implementation PR, declare `OpenSpec-Change: table-aligned-mode` and the reviewed synchronization field, commit the finished change, fetch the target branch, and run the PR-scoped completion command from `docs/contributing.md`; verify it passes for this change without selecting unrelated active proposals.
