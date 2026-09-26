## Context

See [proposal.md](proposal.md) for motivation and [the rendering delta](specs/toon-rendering/spec.md) for acceptance requirements. This is a replacement of the interactive presentation model, not another optimization layer over it.

### Historical seam and measured baseline

- At jless `21dd610` (0.9.0), and still at upstream `e6cdef7`, `JsonViewer` stores the parsed document and navigation state. `ScreenWriter::print_screen_impl` walks at most the terminal's document height using `FlatJson::next_item`; `LinePrinter` formats those rows from source ranges.
- Native rendering commit `4890b2c` introduces document-wide lines, previews, source maps, and cloned visibility. Current `9bb9568` avoids redundant startup builds but retains that eager model. Wrapping adds physical-row projection later.
- Investigation on native macOS arm64, Rust 1.97.1 release/default features, used the delta's synthetic fixture. Five post-warm-up stage measurements yielded medians of 43.6 ms parsing, 784.7 ms layout/gutter adjustment, 87.9 ms visibility projection, and 4.7 ms initial unwrapped physical rows. Total preparation median was 920.4 ms. These stage measurements exclude terminal painting and some application setup.
- That fixture produces 1,800,004 parsed rows, 750,001 rendered lines, 3,000,003 spans, and 1,650,001 source-map entries before projection copies. Further probes attribute roughly 188 ms to key/value/duplicate preparation, 286 ms to recursive rendering including previews, and 151 ms to annotation/source-map/index construction.
- A separate real PTY smoke, one warm-up and three measured launches, yielded 44.4 ms median for installed jless 0.9.0 and 911.1 ms for current tless. The installed jless compiler provenance was not established. These diagnostic results are motivation, not the required 20-run acceptance comparison or proof of the proposed targets.

## Goals / Non-Goals

**Goals:**

- Retain jless's separation between document storage and viewport formatting while representing TOON's many-nodes-per-line and generated-header cases explicitly.
- Keep semantic decisions complete and deterministic before painting; make presentation allocation proportional to the viewport rather than document length.
- Keep one synchronous implementation with local ownership of row construction, source mapping, and navigation translation.

**Non-Goals:**

- Streaming parsing, changing the parsed data model, or promising constant total memory.
- Workers, cancellation protocols, loading states, async runtimes, generalized caches, configurable memory budgets, or product telemetry.
- Restoring jless's data syntax, parsed-row numbering, or removed mode switches.
- Rewriting the codec, changing export behavior, adding dependencies, or enforcing wall-clock limits in ordinary CI.
- A guarantee that pathological individual values or every document shape can render in a fixed number of milliseconds.

## Decisions

### 1. Separate structural indexing from presentation

Keep `FlatJson` as the authoritative data and source-range store. Replace the interactive `Layout`'s owned strings and span graphs with compact structural metadata: active-root membership, display-row kind/owner, table membership and field position, child counts, duplicate occurrence/warning facts, inline-versus-multiline decisions, and logical line counts/anchors.

Use flat records and compact shared tables where variable metadata is needed. Do not reproduce the current per-parsed-row collections of strings, previews, and vectors under another type name. Closing delimiter rows refer to their opening node instead of cloning its presentation metadata. Iterate sibling links rather than repeatedly allocating `children()` vectors.

Full-data passes remain necessary for table eligibility, duplicate decoded keys, warning summaries, and exact line counts. Decode or normalize transiently where analysis requires it; retain semantic facts, not display strings for every value. Borrow existing parsed strings/source ranges when possible. Data-dependent string work remains real; this design does not claim all inspection is viewport-bounded.

**Alternative rejected:** moving `Layout::build` to a worker. It retains the measured cost, memory multiplication, and lifecycle complexity instead of removing the work.

### 2. Compute exact TOON addresses without rendering the document

Classify containers and compute expanded logical-line counts bottom-up. A table contributes its header and data rows; inline arrays contribute one line; list objects account for the merged hyphen/first-field row; sequence roots account separately for document headers and bodies. Store enough anchors/counts to translate both node-to-row and absolute-address-to-owner without scanning rendered spans.

Calculate fitting-array widths from token lengths in terminal cells, indentation, separators, and warning metadata. The display profile permits at most five inline elements, so stop width accumulation once overflow is proven. Width measurement and row formatting must share quoting/normalization rules rather than implement conflicting grammars. Do not construct a complete document just to measure candidate lines.

Resolve the existing monotone gutter fixed point using structural counts and fit decisions. Include annotation widths before accepting an inline decision. Reflow may revisit structural metadata across the selected roots; it must not regenerate every row's text. Explicit multiline choices remain separate from automatic fit state.

**Alternative rejected:** using parsed-row numbers or a permanently oversized gutter. Both simplify indexing by changing the current user contract.

### 3. Give the viewer logical row positions and the painter frame-local rows

Represent a logical position by its structural owner and row kind, distinguishing document header, ordinary body, table header/row, and inline-array row. Logical focus remains a parsed node, independent of the row it occupies. Traversal skips collapsed subtrees using their structure and preserves selected-root boundaries.

At draw time, resolve the viewport's logical positions and format them into frame-owned row text and spans. Retain the current frame for mouse hit testing and horizontal positioning; replace it on viewport/layout changes. A requested off-screen target can be formatted transiently to locate a matching span or continuation. No visited-row cache or general cache manager is required.

Reuse current token styling and clipping behavior behind this seam where possible. `ScreenWriter` should not know how to classify tables or decode keys. The formatter should not own navigation state or terminal lifecycle.

**Alternative rejected:** a vector of borrowed visible lines over an eager layout. It removes cloning but leaves the roughly 785 ms dominant layout cost intact.

### 4. Resolve search and table-key identity structurally

Search continues over parsed content. Map its node/source range through structural metadata to the correct body row or shared table header, then generate only the target row's display mapping. Store a table's column relationship, not one header span per field occurrence across every table row.

For a shared header, preserve the selected match's row-field identity and the existing highlight and mouse-selection behavior. A column spelling can correspond to many source occurrences without retaining every occurrence as a rendered span. Key/value source distinctions and duplicate occurrence IDs remain explicit.

Previews are generated when a visible container is collapsed. Hidden-warning totals come from semantic summaries; preview construction must not traverse or copy all children merely to take a short prefix. Escaped/normalized strings retain accurate source mapping, including terminal-safe control escapes.

### 5. Use row-local wrapping, not a document-wide physical projection

When wrapping is off, a logical row maps directly to a screen row; there is no `PhysicalRow` entry for every document line. When wrapping is on, keep the viewport anchor as a logical row plus a grapheme-safe continuation position. Enumerate enough continuations to fill the viewport and construct hit-test mappings for those rows.

Reverse scrolling can inspect the preceding logical row; distant search or end jumps resolve a logical target first and process that row's continuations, rather than constructing every intervening row. Counted vertical commands still count logical lines, while physical viewport operations count continuations. A very long individual row may require scanning that row to locate a distant continuation; this is not permission to materialize unrelated rows.

Geometry, filter, and collapse changes invalidate frame presentation. Preserve logical focus and recover viewport anchors using structural identities, not offsets into discarded rendered vectors.

### 6. Keep export traversal explicit

Audit current `Layout::canonical` and encoding callers before removal. Whole-document export can visit every selected value, but cannot force interactive startup to do the same. Reuse scalar spelling and grammar helpers where semantics match. Remove obsolete eager interactive types and consumers; do not leave a compatibility renderer or feature flag. Preserve existing output failures for data that standard TOON cannot represent.

### 7. Validate performance and semantics separately

The delta defines the authoritative reference startup protocol. Build identifiable release references from `9bb9568` and jless 0.9.0; record compiler, features, any reference build adjustments, host, and executable hashes. Keep separate target/output directories so probe/reference builds cannot replace the candidate binary. Use deterministic default configuration and the same PTY setup for all three. Alternate executable order across repetitions and avoid simultaneous build/profiling work.

A useful frame requires initial record content (`id: 0`, `nested`, and `record`, allowing jless's syntax) and the filename/status on the terminal's final row. Respond to terminal cursor-position queries. Record raw observations and require successful quit. Sample RSS only through frame detection for startup memory, with interval and platform units recorded; do not label whole-process exit maxima as startup memory. Timing limits apply to an uninstrumented executable. Structural/presentation probes run separately.

Proposed completion thresholds are at most 3x jless for median and p95, and at least 5x faster than frozen tless for median. These are deliberately stronger than a modest improvement over 911 ms. Failure means the implementation is not complete; do not relax the target or replace the reference fixture without revising the proposal explicitly.

Add deterministic behavioral/resource-invariant coverage where the existing suites lack it: unpainted shared-line targets, late table disqualifiers, hidden-warning counts, gutter/Unicode boundaries, and repeated distant navigation without retained presentation growth. Compare documents with the same first viewport but substantially different tails; distinguish allowed structural work from row-materialization counts. These are not elapsed-time CI assertions or tests of internal type names.

Exercise actual PTYs for startup and representative navigation, collapse/reopen, resize, wrap, filter, search reveal, mouse selection, and quit. Use synthetic table, list, sequence, duplicate-key, escaped-Unicode, and long-row cases. Existing contracts, not incidental terminal write batching, define equivalence. Record interaction timings as diagnostics without introducing a new global latency SLA.

## Risks / Trade-offs

- [Exact addresses still require global analysis] → Keep compact structural passes; do not claim startup is purely O(viewport height).
- [Analysis accidentally retains most eager allocation] → Measure stage timings and retained presentation separately; inspect per-node records and shared-header fan-out before integration.
- [Two implementations of token width diverge] → Share spelling/measurement rules and cover exact-fit, Unicode, annotations, and digit-boundary reflow.
- [On-demand navigation loses unpainted source identity] → Resolve node/key/value ownership structurally and test table header matches from distant rows, duplicate occurrences, and filtered sequences.
- [Wrapping introduces another global projection] → Require logical-plus-continuation anchors and direct distant-target scenarios; permit row-local scans, not whole-document formatting.
- [Host noise or mismatched reference builds distort ratios] → Preserve provenance, isolate the benchmark, retain all measurements, and compare all executables in one session. Do not use the historical diagnostic numbers as the acceptance baseline.
- [Refactor grows into another broad framework] → Keep the index, row formatter, and traversal in existing modules where practical. No worker subsystem, generic cache, lifecycle redesign, or unrelated cleanup.

## Migration Plan

1. Freeze the synthetic reference and behavior expectations against current tless; establish the benchmark before changing the renderer.
2. Implement structural analysis and row formatting, then migrate viewer, painter, search/mouse translation, wrapping, and export-sharing callers as one complete cutover.
3. Remove the eager interactive path and its obsolete projections. Preserve behavior tests; replace tests tied only to removed storage details rather than maintaining shims.
4. Run behavior suites and real PTY acceptance, then the uninstrumented reference comparison. Update contributor-facing rendering guidance/changelog after proof, synchronize the delta, and archive through the repository completion gate.

There is no persistent-data migration or new configuration. Rollback is a source revert to the previous renderer, not a permanent runtime fallback. This proposal does not authorize implementation.
