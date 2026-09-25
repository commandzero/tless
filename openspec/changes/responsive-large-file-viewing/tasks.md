## 1. Establish reproducible behavior and performance probes

- [ ] 1.1 Add deterministic synthetic fixture generation for the reference nested-record workload and table, duplicate-key, sequence, filtered-root, and oversized-string variants described in design.md; verify generated sizes, known first/last values, and format validity without retaining confidential inputs or multi-megabyte fixture blobs.
- [ ] 1.2 Add an opt-in release PTY profiling harness that interprets frames, responds to terminal queries, and separates loading, useful content, acknowledgement, destination completion, quit restoration, and resource measurements; run it against the current implementation to capture an honest baseline and prove that a loading message or echoed key cannot satisfy content/navigation assertions.

## 2. Remove redundant layout construction and separate immutable analysis

- [ ] 2.1 Pass actual terminal dimensions and effective gutter settings into initial viewer construction; verify first-frame geometry at default and nondefault widths, including line-number settings and exact-fit inline arrays.
- [ ] 2.2 Separate width-independent semantic analysis from geometry and painted text, retaining stable node/source identities, table eligibility, duplicate ordinals, and warning aggregation; verify existing grammar/extension fixtures and late-table-incompatibility behavior.
- [ ] 2.3 Replace full-render gutter retries and recursive annotated-array overflow rebuilds with count/width decisions before painting; verify digit-boundary gutters, Unicode exact fits, annotation overflow, and explicit multiline persistence, and compare stage timings with the baseline.
- [ ] 2.4 Move parsed-row collapse mutation into view state and migrate every affected consumer, including filters, search reveal, sequence rows, copy, and export; verify descendant restoration, sequence-root independence, and unchanged serialized values without retaining old mutation shims.

## 3. Build indexed viewport preparation

- [ ] 3.1 Implement scoped node/source anchors and canonical/visible subtree or block counts; verify absolute collapse gaps, filtered numbering, counted motions, shared-line cells, and direct top/end destinations without preparing intervening text.
- [ ] 3.2 Replace retained whole-document painted lines and cloned visibility projection with demand-prepared viewport fragments and collapsed overlays; verify identical committed TOON text/styles and original identities after visiting and evicting distant regions.
- [ ] 3.3 Implement byte-budgeted presentation caching and in-flight result accounting with the design's initial budgets; verify high-water bounds across repeated jumps and that worker-side eviction/reclamation does not stall terminal input.
- [ ] 3.4 Render giant values through bounded source/token fragments and compact shared-header mappings; verify Unicode/escape boundary reconstruction, distant source-match reveal, table-field occurrence selection, and complete copy/export for values exceeding the cache budget.
- [ ] 3.5 Replace unwrapped physical-row materialization with identity projection and add lazy wrapped-region checkpoints/counts; verify physical paging through tall values, blank continuation gutters, tiny widths, cache eviction/revisit, and unchanged logical addresses.
- [ ] 3.6 Migrate screen painting, hit testing, focus lookup, and match reveal to the preparation/index interface and remove obsolete global scans/eager rendering paths; verify existing navigation and terminal behavior suites and smoke-test real top/end, wrap, resize, and mouse interactions.

## 4. Integrate native background work and terminal lifecycle

- [ ] 4.1 Add explicit loading/ready/pending/failed application states and owned input sources that preserve piped stdin before terminal remapping; verify files, stdin, `-`, delayed EOF, and keyboard input remain distinct through isolated PTYs.
- [ ] 4.2 Add the native preparation worker, bounded requests/results, byte backpressure, resumable work batches, and coalesced notification descriptor; verify a full queue or long scalar scan does not block input and that repeated notifications cannot lose a completed result.
- [ ] 4.3 Extend existing keyboard/resize multiplexing for worker notifications on macOS and Linux, retaining high-descriptor handling; verify terminal input, resize, and worker completion arriving together, plus descriptor cleanup and no busy waiting at idle.
- [ ] 4.4 Implement per-lane document/scope/geometry/request generations and atomic viewport publication; verify delayed old results cannot revert a newer resize, scope, destination, or collapse state and that finite current requests complete without starvation.
- [ ] 4.5 Implement prompt-safe result adoption and preserve suspend/resume behavior; verify command/search editing, cancellation, and cursor position while results arrive, plus terminal restoration/reentry across suspension.
- [ ] 4.6 Implement loading quit/interruption and failure handling without joining blocked input/codec work before restoration; verify `q` exits 0, Ctrl+C exits 130, worker/read/parse failures exit 1 after restoration, and no worker writes terminal output after exit begins.

## 5. Preserve navigation, filter, and search semantics during pending work

- [ ] 5.1 Separate desired and committed selection with explicit pending feedback, Escape cancellation, latest absolute destination wins, and ordered relative motion accumulation; verify numbered jumps during reflow, ten queued downward motions, copy from committed selection, and ignored stale-geometry clicks.
- [ ] 5.2 Prepare interactive filters in the background and publish scope plus its first viewport atomically against current geometry/state; verify all-document failure atomicity, reset behavior, Escape/supersession, navigation during preparation, and preservation of latest descendant collapse choices.
- [ ] 5.3 Add one on-demand native search worker with one latest replacement, retaining full permitted-range regex semantics and publishing complete match indexes; verify uncached/hidden matches, duplicate and shared-header occurrence identity, empty/cross-boundary patterns, long no-match scans, cancellation, and scope invalidation without starving viewport work.
- [ ] 5.4 Update interactive startup resolution failures to restore the terminal before final diagnostics while retaining syntax-before-terminal and machine-mode behavior; verify exit codes, escaped diagnostics, empty redirected stdout on failure, and no terminal requirement for pipelines.

## 6. Verify compatibility, responsiveness, and resource limits

- [ ] 6.1 Exercise the new lifecycle end to end with supported JSON/YAML/TOON, sequences, path filters, warnings, duplicate occurrences, themes, wrapping, search, mouse selection, and copy/export; keep deterministic consumer-visible regressions for races and identity errors and confirm actual PTY output rather than only internal assertions.
- [ ] 6.2 Run the reference release benchmark with fixed geometry, one warm-up, and at least 20 measured runs; record hardware/toolchain/profile, median and nearest-rank p95, first useful content, input acknowledgement, cached/uncached destinations, resize/wrap completion, and quit restoration, and satisfy every numerical responsive-viewer gate.
- [ ] 6.3 Stress repeated resize/jump/filter/search requests, stalled input, giant scalars, large table headers, and cache eviction; verify presentation/queue byte budgets and bounded worker/request counts, and report parsed/index/search storage and total peak RSS separately from presentation bytes.
- [ ] 6.4 Run native macOS and Linux PTY smoke scenarios plus `scripts/preflight.sh` and `TLESS_TOOLCHAIN=1.87.0 scripts/preflight.sh test`; record exercised platforms and results, with no claim that one host's timings prove another host's performance.

## 7. Document and complete the change

- [ ] 7.1 Update CLI/interactive help and existing user guides for loading, pending/cancel behavior, restoration-first startup errors, full-input parsing, bounded presentation versus total memory, and unchanged output semantics; update the Unreleased changelog under repository policy, mark the startup contract change as breaking, and validate the complete docs bundle with `scripts/validate-docs.sh`.
- [ ] 7.2 Record implementation verification and reference performance evidence without private paths/data, remove temporary instrumentation/scaffolding, and verify all requirements have behavior or measurement evidence rather than treating this checklist or syntax validation as implementation proof.
- [ ] 7.3 Synchronize the four capability deltas into main specifications, verify their complete correspondence, and archive the completed change using pinned OpenSpec 1.11.0 with telemetry disabled; validate affected main specs and archived tasks and satisfy the repository's associated-PR completion gate before merge.
