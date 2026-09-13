## 1. Document layout and state

- [x] 1.1 Replace passive multi-root separators with root-owned document headers and remove multiple-roots warnings. Verify equivalent NDJSON, JSONL, concatenated JSON, and YAML sequences produce ordered headers while single-root snapshots remain unchanged.
- [x] 1.2 Add persistent subdued position spans and bounded contents previews only for collapsed document rows, including scalar and empty roots. Verify expanded rows show only the position after `---`, collapsed rows add the contents preview, expansion removes it, selected theme styles, Unicode truncation, and position priority at narrow widths.
- [x] 1.3 Add independent document collapse state and body projection without changing parsed values or indentation. Verify mixed root shapes, unchanged nested indentation, independent collapse, and restoration of descendant states.

## 2. Navigation and selection

- [x] 2.1 Anchor structural root selection at document headers and retain body-line selection for roots owning several lines. Verify `[` and `]` from top-level fields, root siblings, first/last boundaries, counted `J`/`K`, child motions, and scalar/array body navigation.
- [x] 2.2 Include document rows in visible vertical motions, absolute jumps, relative gutters, and mouse hit testing. Verify counted motions, collapsed address gaps, header jumps, header clicks, collapse arrows, and selection through wrapping, resizing, and viewport scrolling.
- [x] 2.3 Route shallow, deep, sibling, and ancestor collapse/expand commands through document state. Verify hidden selection moves to the document row and reopening preserves descendant state except when deep operations explicitly change it.

## 3. Data actions and integration

- [x] 3.1 Reveal hidden search matches through document collapse state while excluding generated positions and previews from search. Verify scalar, nested, table-cell, and duplicate-key matches retain their original identities and paths.
- [x] 3.2 Preserve selected-root copy and print, whole-document serialization, and remaining warning behavior. Verify copied JSON has no header or synthetic wrapper, standard TOON multi-root restrictions remain unchanged, and hidden data warnings survive document collapse without a multiple-roots warning.
- [x] 3.3 Add terminal integration coverage for a mixed sequence. Verify opening, selecting, collapsing, jumping to the next document, searching hidden content, and reopening works with keyboard and mouse controls.

## 4. Documentation and completion

- [x] 4.1 Update interactive help, `docs/toon-view.md`, `docs/toon-acceptance.md`, and the Unreleased changelog to describe document rows and remove obsolete separator rules. Verify examples show aligned content columns and run `scripts/validate-docs.sh`.
- [x] 4.2 Run `scripts/preflight.sh`, the required Rust 1.87 compatibility checks, and strict native OpenSpec validation with telemetry disabled. Record the results and resolve failures before declaring implementation complete.
- [x] 4.3 Verify implementation against every delta scenario, synchronize the deltas into main specs, and archive `toon-sequences`. Review delta-to-main correspondence and pass native archived validation and the repository's PR-scoped OpenSpec completion gate before merge.
