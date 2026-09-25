## Context

See [proposal.md](proposal.md) for motivation and the [responsive-viewer requirements](specs/responsive-viewer/spec.md) for lifecycle and performance acceptance.

The current JSON TUI path is input read -> `output::parse_input` -> `FlatJson` -> `JsonViewer` -> direct TOON layout. It does not encode JSON through the TOON codec. `main.rs` reads and parses before terminal initialization. `JsonViewer::with_roots_impl` builds at default dimensions, and `App::run` applies actual dimensions on first draw. `layout_for_view` rebuilds rendered content while growing the gutter; `Layout::build` can recursively rebuild when annotated inline arrays overflow. Visibility projection clones text and mappings; wrapping allocates physical rows across the visible document. `focused_line_index` and some source reveals use global searches.

An exploratory optimized run on an approximately 10 MB input measured 3 ms reading, 20 ms parsing, 310 ms for one layout, 629 ms constructing the default-size viewer, and 711 ms applying actual dimensions. A later resize took 668 ms and wrapping roughly 140 ms. These isolated measurements establish priorities, not portable benchmark results or expected post-change performance. No confidential input, path, or fixture is required for implementation or acceptance.

Current behavior is specified by `toon-rendering`, `toon-navigation`, `toon-sequences`, `toon-display-extensions`, `path-filter`, and `piped-output`. In particular, line addresses are fully expanded logical TOON addresses, relative motions count visible logical lines, and wrapped scrolling counts physical rows. Table keys can map many logical occurrences to one painted spelling. Preserve these distinctions.

## Goals / Non-Goals

**Goals:**

- Remove redundant whole-document work before introducing concurrency.
- Separate immutable parsed data and semantic analysis from geometry, visibility, and painted fragments.
- Keep all document-sized work off the terminal event loop; publish bounded usable viewports.
- Make correctness independent of worker timing, cache residency, and request completion order.
- Meet the reference latency gates with reproducible nonconfidential workloads and retain supported platform/MSRV behavior.

**Non-Goals:**

- Pre-EOF browsing, a partially valid document model, memory mapping, out-of-core storage, or a new parser/codec.
- An async runtime, thread pool, new workspace crate, or user-facing tuning switches for the initial implementation.
- Altering TOON grammar, numeric preservation, duplicate-key policy, regex semantics, output framing, or published codec behavior.
- Accelerating export/clipboard serialization or external help programs; their data and terminal ownership contracts still must survive the refactor.
- Promising fixed first-content latency for slow pipes, remote storage, or every permitted maximum-size input.

## Decisions

### 1. Retain complete parsing; progressively prepare display

Read and validate the whole input on a preparation worker before publishing an immutable document. Existing JSON/YAML/TOON parsing and path validation remain authoritative, including validation of excluded input. Array counts, table eligibility, duplicate occurrences, warning aggregation, and sequence totals are computed from the complete document, not a prefix.

This rejects streaming canonical TOON text into the UI: the UI needs node identities, source ranges, warnings, and arbitrary access, not just bytes. Complete parsing is the inexpensive stage in the observed workload. A loading state solves blocked/slow input responsiveness without introducing provisional grammar.

### 2. Separate ownership and expose a small preparation interface

Keep modules in the application crate. Introduce one document-preparation seam whose caller submits a document/view request and receives ready, pending, or failed results. Internal cache/index details do not escape into `App` or `ScreenWriter`.

- Immutable document: source spelling, parsed rows, parent/child/sibling links, stable node IDs, and typed values. Share with `Arc` after validation without cloning the document.
- Semantic metadata: immediate counts, table field relationships, duplicate occurrences, warning facts, and compact token/source descriptors. Reuse independently of width and collapse. Avoid eagerly retaining another full decoded/formatted copy of every scalar.
- View state: active roots, collapsed nodes/documents, explicit multiline choices, desired selection, committed selection, wrap/gutter settings, and source/grapheme viewport anchor. Move mutable collapse state out of `FlatJson` into this layer and migrate all consumers without a compatibility shim.
- Geometry index: scoped canonical logical counts/anchors, visibility counts, and lazily available physical counts. No painted strings or per-character mapping arrays disguised as index metadata.
- Viewport result: immutable bounded painted fragments, local source mappings/hit targets, exact applicable addresses, and stable anchors for a single generation.

The UI commits view-state transitions; the preparation worker owns mutable indexes/caches. Requests carry small deltas or versioned snapshots, not an O(document-size) view-state clone per keystroke. Search shares the immutable document, never mutable viewer state. Large retired indexes and final document destruction remain off the UI hot path; a viewport swap must not synchronously free a former whole-document layout.

### 3. Count and index before painting

Initialize the view request with actual terminal dimensions and effective CLI gutter settings. Split semantic analysis, geometry counting, and fragment rendering. Compute annotated inline-array widths once per relevant geometry and decide overflow before rendering. Resolve gutter width to a fixed point over compact logical counts; repeats do not rerun semantic decoding, warning analysis, or full text rendering.

Use subtree/block counts with prefix/rank selection to locate absolute logical lines and visible entries. Preserve separate canonical and visible counts so collapse leaves absolute-number gaps. Direct node-to-anchor and source-to-node lookup replaces global scans during focus and match reveal. Logical coordinates remain independent of physical wrap coordinates.

Geometry counts can require an O(nodes) background pass; this design does not claim O(viewport) parsing or analysis. The first useful frame waits for the counts and semantic facts needed to give it exact grammar and addresses, but not rendered text for off-screen lines. `G` uses structural tail access, then prepares the destination region rather than rendering all intervening content. Exact numbered jumps wait explicitly if current geometry counts are unfinished.

Collapsed visibility is a subtree count/overlay change. Unchanged lines are referred to by identity rather than copied; only visible collapsed previews need synthesized fragments. With wrapping off, physical rows are an identity projection rather than a retained array of objects. With wrapping on, materialize nearby fragment boundaries and maintain block/checkpoint counts for distant positioning. Resolve physical locations relative to a stable node/source anchor; do not sum every earlier wrapped line on each motion.

### 4. Bound every presentation allocation, including pathological lines

Initial internal budgets are 32 MiB for retained presentation, including currently displayed and retiring viewport fragments, and 4 MiB total for in-flight result payloads. Each result batch is at most 256 KiB. Overscan targets at most one viewport before and after the current viewport, subject to those byte budgets; byte limits win over row targets. Budget accounting includes strings, spans, mapping vectors, capacities, and payload bookkeeping. Compact source/node/count indexes and search-match offsets have separate accounting and may scale with input.

Render source/token ranges into bounded fragments instead of creating an entire encoded display string for a giant value. Keep grapheme-safe checkpoints; long scans and escape decoding run on workers. Pathological graphemes must be scanned without buffering unbounded presentation bytes. Reconstruct evicted regions from immutable source descriptors. Never truncate the semantic value to make a cache fit.

Represent shared table-header field associations compactly and resolve the selected field occurrence on demand. Do not put one copied source-map vector for every table row into the visible header. Identity mappings use ranges; escaped mappings are produced only for painted or requested spans.

All producers respect byte credits/backpressure and cancellation while publishing. Progress is coalesced; it cannot fill the payload queue. Release large cache entries on the worker rather than transferring reclamation pauses to the UI. These are presentation budgets, not a bound on total RSS or maximum parsed input.

### 5. Use native workers and the existing descriptor-based event loop

Use one long-lived preparation worker and one bounded, on-demand search worker. The second worker prevents a long no-match regex scan from delaying viewport preparation. Do not create a worker per request. The existing codec helper with its required 16 MiB stack can remain nested in the preparation worker during codec operations; its blocking join is never on the UI thread. No general-purpose executor or async runtime is introduced.

Extend `TuiInput` to observe keyboard, resize, and a worker-notification descriptor using the existing macOS-safe `select`/high-descriptor `kqueue` and other-Unix fallback arrangements. Do not replace macOS `/dev/tty` handling with an unsupported polling path. Send data through bounded standard-library channels/mailboxes and use a nonblocking coalesced notification solely to wake the UI. Avoid missed wakeups between draining notifications and checking messages. Limit result adoption per event-loop turn and prioritize input before speculative work; never block the UI on a full request channel or a document mutex.

Each result carries a document ID, scope revision, geometry revision, and request ID. Keep independent lanes for viewport, filter, and search work so unrelated valid results are not accidentally invalidated. Geometry publication and its hit targets are atomic. Discard mismatched results before they can change selection or counts. Retain at most the current and one latest pending request per lane; coalesce absolute destinations and resize frames, not ordered relative motion semantics. Apply relative commands to desired logical state or accumulate their exact motion before replacing a rendering request.

Preparation runs in resumable bounded batches with cancellation/generation checks, including inside long scalar scans. Requested viewport work outranks speculative indexing. A filter candidate is prepared against the latest view state and geometry before atomic publication, so edits made while it was pending survive. Prompt editing keeps terminal ownership; defer document result adoption/painting until the editor returns while coalescing notifications safely.

### 6. Make pending state honest and keep regex semantics

Maintain committed and desired view states separately. During a pending destination or geometry transition, keep a safely clipped committed frame with a status indicator, or show a loading surface if it cannot be presented safely. Copy/export uses committed identity and scope. Ignore geometry-dependent clicks while stale, with pending feedback. Escape cancels the pending command and retains the committed view; the latest actual terminal geometry still must be prepared and cannot be undone by Escape. A new absolute destination supersedes the previous pending destination.

Search runs on the immutable existing canonical search text and permitted scope ranges with the same regex configuration. Publish the completed match index atomically; until then display `Searching`, not zero or a final count. Do not split regex input naively into chunks: cross-boundary patterns, anchors, and empty matches must retain their current meaning. Check cancellation between matches/ranges and discard stale results after an indivisible regex call returns. A library scan without an interruption point may delay its replacement search, but cannot delay UI input or the separate preparation worker. Only one search runs with one latest pending replacement, and no abandoned search threads accumulate.

Search-match storage can remain O(matches), separate from the presentation cache. Source-to-node indexes and local fragment mappings resolve an eventual match without whole-document display scans. Successful filter publication clears prior search and invalidates pending search from the old scope; failed/cancelled filtering leaves the committed search intact.

### 7. Enter and leave the terminal deliberately

Keep argument and path-syntax validation before terminal setup. Branch on terminal stdout early; machine mode retains its current synchronous complete-validation/serialization contract without viewer workers or progress UI. Existing codec helper threads retain their current behavior.

For interactive mode, capture file input or duplicate the original stdin descriptor into an owned input source before remapping stdin to `/dev/tty`. Do not retain a global `Stdin` handle whose descriptor can be redirected underneath a worker. Initialize theme, actual dimensions, terminal guard, and loading surface before dispatching potentially slow reading/parsing. Failure to open the controlling terminal remains an ordinary startup error.

Use explicit loading, ready, pending, and failed application states. Loading permits resize, quit, and interruption, but not hidden queued document commands. Interactive read/parse/initial path failures restore terminal state, then report stderr and exit 1; syntax failures still exit 2 before setup. Preserve terminal-safe diagnostics. A recoverable request failure after a document is ready retains the prior committed view and reports an error; a fatal worker failure restores the terminal and exits rather than hanging.

Quit signals cancellation, closes request acceptance, restores the terminal, and exits without joining a worker blocked in a read or third-party codec. Normal process termination ends remaining native threads; do not attempt unsafe thread cancellation. Background workers never own terminal output or external writes, so this cannot leave a worker writing after terminal restoration. Use normal cooperative cleanup when work is interruptible, but do not let a scoped-thread join or large destructor delay restoration. Preserve existing suspend/resume behavior in loading and ready states.

### 8. Measure useful behavior and keep the workload public

Implement a deterministic fixture generator under tests/support or the existing test fixture conventions, not a checked-in multi-megabyte blob. The primary JSON fixture is compact UTF-8 with one root `records` array and 150,000 records in ascending order. Each record has ordered fields `id` (its zero-based integer), `nested` (`ok: true`, then `label: "record"`), and `values` (`[1,2,3]`). It is approximately 10 MiB, contains short inline candidates and nested list content, and has known first/last values for validating displayed destinations. Parameterize record count for scaling comparisons without including private input.

The scenario matrix also includes a uniform primitive-field table and a late-incompatible last row, duplicate/escaped keys and warning-bearing numbers, sequences, filtered roots, one giant string exceeding 32 MiB with Unicode/escape boundaries and a known distant match, tiny terminal widths, malformed tails, input-limit failures, and a stalled pipe. Use deterministic generated values and isolated PTYs.

Record OS, CPU architecture/model, Rust toolchain, feature profile, fixture bytes/nodes, terminal dimensions, and warm/cold conditions in the implementation verification report; exclude personal hostnames, paths, and input data. Use an otherwise idle native macOS arm64 reference host with a release default-feature build and locked dependencies. Run one warm-up and at least 20 measurements; report nearest-rank p95 plus median. Native Linux verifies terminal/concurrency correctness and reports timings separately, not an unmeasured portability claim.

The PTY harness must consume and interpret actual terminal output, respond to terminal queries, and verify expected cells/focus before recording useful-frame or destination completion. Measure acknowledgement from the event injection to the actual new frame or pending indicator, not an input echo. Measure quit through restored terminal flags and process completion. Exercise cached motion repeatedly for stable p95 observations. Report input read, parse, analysis/counting, first viewport, uncached destination, wrapping/reflow, search completion, presentation high-water bytes, queue occupancy, and total peak RSS separately.

The thresholds in `responsive-viewer` are release acceptance criteria, not achievements of this planning change. Keep ordinary CI deterministic with state/resource assertions and generous hang timeouts. Existing terminal tests remain the behavioral oracle; add regressions for stale results, cancellation, geometry publication, oversized lines, and source-map identity, not implementation wiring or incidental defaults.

## Risks / Trade-offs

- Whole input and parsed indexes still consume O(input) memory -> state that limitation explicitly; preserve the existing input-byte limit and report RSS separately from bounded presentation.
- Exact layout metadata still needs document-wide analysis -> perform it once or as count-only geometry passes off the UI thread; do not claim immediate content for arbitrary input sizes.
- Lazy mapping can lose duplicate/table occurrence identity -> keep node IDs and source descriptors authoritative and verify uncached search/copy/hit-testing cases.
- Cache eviction or result adoption can move large deallocation costs to the UI -> budget results and retire large structures on workers.
- Long regex/codec calls cannot always be preempted -> isolate them from the terminal and viewport worker where needed, bound worker count, and never join them before quit restoration.
- Interleaved filter, resize, search, and navigation can publish mixed state -> use request lanes, generations, atomic viewport commits, and deterministic delayed-result tests.
- Terminal lifecycle changes an existing documented contract -> modify the path-filter delta explicitly, document restoration-first errors, and leave machine-mode and syntax behavior unchanged.
- Hardware-sensitive thresholds can produce noisy CI failures -> run the numerical gate on the documented native reference setup and keep resource/state invariants in normal CI.

## Migration Plan

1. Establish reproducible baseline scenarios, then remove default-size/full-render retries and separate semantic analysis from geometry without changing visible semantics.
2. Replace full display/projection/physical-row ownership with the indexed bounded preparation seam and migrate rendering, navigation, search reveal, and hit-testing consumers. Remove obsolete eager paths rather than leaving two rendering modes.
3. Integrate loading and request workers, event notifications, generations, cancellation, stdin ownership, and terminal restoration. Preserve non-terminal behavior in its existing path.
4. Complete behavioral compatibility checks, native PTY smoke runs, reference performance/resource acceptance, Rust 1.87 checks, documentation/help, and changelog. A spinner-only improvement does not complete this change.
5. Synchronize deltas and archive only after implementation verification; run the repository's PR-scoped completion gate before merge. Because interactive startup error ordering is marked breaking, apply the repository's minor-version policy in the associated release rather than promising patch compatibility.

There is no persisted user-state migration. Rollback is a revert of the implementation release; do not retain an eager-rendering feature flag or compatibility renderer as a second permanent path.
