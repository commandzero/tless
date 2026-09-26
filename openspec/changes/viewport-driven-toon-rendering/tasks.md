## 1. Establish the comparison

- [ ] 1.1 Add the small synthetic fixture generator and opt-in PTY benchmark described in the rendering delta; verify the 10,238,903-byte input, useful-frame detector, raw timing output, and successful quit against identifiable release builds of `9bb9568` and jless 0.9.0 in separate output directories.
- [ ] 1.2 Capture current behavioral expectations for table/list decisions, shared-line selection, numbering/reflow, warnings, sequences, filtering, and wrapping; reuse existing suites and add only missing consumer-visible cases, with a recorded baseline PTY smoke before renderer changes.

## 2. Replace eager layout with structural analysis and row formatting

- [ ] 2.1 Implement compact TOON structural metadata over the parsed document, including table classification, duplicate occurrences, warnings, root scope, and generated document headers; verify late table disqualifiers, escaped duplicate keys, hidden-warning totals, and selected-root identities without building document-wide display strings.
- [ ] 2.2 Compute logical line counts/anchors and the gutter/inline-array fixed point from structural metadata and shared token measurement rules; verify exact-fit Unicode and annotation cases, digit-boundary reflow, merged list-object lines, shared-line addresses, and explicit multiline choices against the existing contract.
- [ ] 2.3 Implement frame-local logical-row formatting with source mappings and on-demand collapsed previews; verify exact TOON text, safe escaped output, token ownership, and normalized-string match ranges for ordinary rows, tables, inline arrays, and sequence headers.

## 3. Cut over interactive consumers

- [ ] 3.1 Migrate viewer traversal and screen painting to structural logical positions and frame-local rows; verify first draw, counted and structural motion, absolute jumps, collapse/reopen, resize, relative numbers, and mouse ownership through actual PTY interactions.
- [ ] 3.2 Migrate search reveal and shared-header field lookup away from eager span scans; verify never-painted distant cells and inline elements, distinct duplicate occurrences, table-key matches retaining their originating row field, and filtered sequence reveal boundaries.
- [ ] 3.3 Replace document-wide physical-row construction with logical-row/continuation viewport anchors; verify wrapping in both directions, distant match/end jumps, grapheme boundaries, zero-width geometry, continuation mouse selection, and focus preservation on wrap/resize without formatting intervening rows.
- [ ] 3.4 Migrate remaining layout consumers and remove the obsolete eager interactive layout, cloned visibility, and physical-row vectors; verify existing copy, piped output, export rejection, theme, filter, and sequence behavior and confirm no legacy renderer or compatibility shim remains.

## 4. Prove the completed behavior and performance

- [ ] 4.1 Add focused deterministic coverage for presentation lifetime and off-screen independence; verify that longer unrelated tails and repeated distant navigation do not increase retained formatted rows/source maps, while allowing document-wide structural metadata and row-local long-value work.
- [ ] 4.2 Run `scripts/preflight.sh` and the Rust 1.87 feature-profile tests, using serial PTY tests where required; exercise the completed CLI in real PTYs across the synthetic table/list/sequence/Unicode/warning cases and record preserved content, selection, and terminal restoration.
- [ ] 4.3 Run the uninstrumented 20-launch, same-host reference comparison; retain raw evidence proving median and p95 at most 3x jless and median at most one fifth of `9bb9568`, with separately measured startup RSS and diagnostic interaction timings. Do not mark complete on an architectural change or partial speedup alone.
- [ ] 4.4 After behavioral and performance proof, update relevant rendering guidance and the changelog, validate any docs changes with `scripts/validate-docs.sh`, then verify and synchronize the delta, archive the change, and run the pinned OpenSpec validation and committed PR-scoped completion gate required by `docs/contributing.md`.
