## Context

Startup builds a complete TOON layout, projects visibility, and constructs physical rows. The original constructor used default dimensions; first draw then applied actual dimensions and rebuilt the layout. Gutter sizing also rebuilt the full layout after discovering the required number of digits.

The first implementation expanded into background workers, demand rendering, memory accounting, and incremental terminal output. The user rejected that complexity and explicitly selected a startup-focused fix. That implementation was preserved outside the checkout and removed; its performance numbers and verification do not describe this replacement.

## Goals / Non-Goals

- Reduce redundant startup work while preserving existing display and interaction semantics.
- Keep the change local to application initialization, viewer construction, and layout width adjustment.
- Do not introduce concurrency, asynchronous search/filtering, loading/error lifecycle changes, viewport caches, memory budgets, or a new terminal writer.
- Do not promise sub-500 ms startup, bounded process memory, or responsiveness during synchronous reflow.

## Decisions

### Initialize with actual geometry

`App::new` reads terminal dimensions and passes content dimensions and effective number visibility to the existing viewer constructor. The already-resolved roots remain authoritative; the existing distinction between normal startup and exposing filtered roots is preserved. `App::run` still refreshes terminal dimensions before first draw, so a resize during construction is not ignored.

### Reuse only when narrowing cannot change grammar

The gutter fixed-point loop retains its existing minimum-width calculation. After rendering, it computes the content width required by the actual line count. If every inline array's complete annotated text fits that width, reuse the layout and update its stored preview width. Narrowing cannot turn a multiline array into an inline array, and other expanded TOON grammar is width-independent. If any inline array stops fitting, use the original rebuild path.

This deliberately leaves the existing annotated-overflow fallback, visibility projection, and wrapping implementation intact. It is not a replacement counting/indexing subsystem.

## Risks / Tradeoffs

- Unicode cell width and annotations must participate in the reuse check, not UTF-8 byte length.
- Digit-boundary growth can change array layout and increase line count again; the existing loop remains necessary for those cases.
- Collapsed inline previews must use the narrowed width even when expanded grammar is reused.
- Full-document rendering and memory usage remain input-proportional. Resize, filtering, search, and terminal output retain their synchronous behavior.

## Verification

A release/default-feature run on native macOS arm64, Apple M4 Pro, Rust 1.97.1 used a synthetic 150,000-record, 10,238,903-byte JSON document at 140x40 terminal geometry. One warm-up was excluded, followed by 20 measured runs; p95 uses nearest rank.

| First useful content | Baseline | Startup-only fix |
| --- | ---: | ---: |
| Median | 3376.79 ms | 872.03 ms |
| p95 | 3413.25 ms | 879.17 ms |

Median startup improved approximately 3.9 times. The replacement's excluded cold/warm-up observation was 1210.95 ms. These observations do not establish performance on other hosts. The original baseline executable's compiler provenance was not recorded.

Measurement reused the preserved disposable PTY harness against the baseline and replacement executables; it verified rendered fixture content and focused destinations, not merely process launch or a loading message. The obsolete broad-scope latency thresholds still printed failures for startup and synchronous operations in that harness; they are explicitly not acceptance criteria for this narrowed change. No benchmark framework or synthetic large input is added to the repository.

The minimal-feature suite passed 264 tests after the replacement, including initial Unicode fit with/without numbers and the updated gutter digit-boundary case. Existing annotation, collapse, resize, search, and terminal cases remain in place.

A separate direct PTY smoke compared the entire startup/quit byte stream with the baseline at widths 18, 19, and 140, using absolute numbers, no numbers, and relative numbers with absolute numbers disabled. All nine cases were byte-identical and exited 0. The synthetic document included Unicode arrays, duplicate-key warnings, and more than 99 logical lines.

All five feature profiles passed on both Rust 1.97.1 and 1.87.0: 1,404 passed, zero failed per compiler. Tests ran serially through the preflight test commands; the combined runner hit its 300-second orchestration deadline during the MSRV default CLI suite, so that suite and the remaining MSRV profiles were completed separately with a longer deadline.
