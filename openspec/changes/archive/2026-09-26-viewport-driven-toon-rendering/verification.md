# Verification

## Compiler and feature matrix

The final semantic-scan implementation passed all five feature profiles on both Rust 1.87.0 and 1.97.1, using `scripts/preflight.sh test` with `RUST_TEST_THREADS=1` and a separate MSRV target directory:

| Profile | Unit | Pipeline | CLI/PTY | Total passed |
| --- | ---: | ---: | ---: | ---: |
| Minimal | 196 | 13 | 57 | 266 |
| Default | 221 | 13 | 58 | 292 |
| S-expression only | 198 | 13 | 58 | 269 |
| Colorscheme only | 221 | 13 | 58 | 292 |
| All features | 223 | 13 | 59 | 295 |

The timing benchmark remains intentionally ignored in ordinary profile runs and was executed separately as described below. Documentation validation passed with zero errors and warnings; its existing SVG-link informational message is unchanged.

## Implementation correspondence

- `src/toon_display/index.rs` performs complete semantic analysis, including late table disqualifiers, decoded duplicate keys, warning multiplicity and selected-root identities. Object key decoding is shared by warning and duplicate analysis.
- `geometry.rs` retains logical addresses and resolves gutter/inline width. `format.rs` produces requested rows; `layout.rs` projects structural visibility without retained display strings.
- `src/viewer.rs` retains frame-local formatted rows and viewport physical rows. `src/wrapped_view.rs` walks continuations lazily. Search targets structural source identities, including never-painted rows and shared table headers; shared-header highlighting coalesces ranges rather than retaining aliases for every record.
- Existing eager rendering and document-wide physical-row construction were removed. No new dependency, background worker, loading state or compatibility renderer was introduced.

## Behavioral proof

The existing renderer, viewer, painter, wrapping, pipeline and CLI suites cover table/list decisions, duplicate identities, warnings, exact-fit Unicode, logical numbering, sequence/filter boundaries, copy/export, themes and terminal behavior. Four focused viewer tests additionally exercise late disqualifiers and warning totals, bounded frame retention across longer tails and repeated distant navigation, distant escaped shared-header matches without alias fanout, and filtered inline/distant wrapped search without formatting intervening rows.

`pty-proof.json` retains actual screens and diagnostic interaction timings from the final default-feature release binary:

- 2,000-record table: wrapping; search into the never-painted final long Unicode field; resize preserving `.rows[1999].text`; filtering to `.rows`; numbering off; mouse collapse/reopen; `3j`; `1500G` selecting `.rows[1498]`.
- Two-document YAML sequence: list rendering, non-finite/control-character warnings, collapsed hidden-warning annotation, Unicode inline values, and search selecting `[1].long`.
- Both sessions exited zero and restored terminal attributes, alternate screen and cursor. On macOS the final attributes were read from the PTY master because the exited controlling session revokes the slave.

The disposable Python driver used pyte 0.8.2, isolated configuration and cursor-query replies throughout, including quit. Its timings include terminal-emulator overhead and are diagnostic only. No private input was used.

## Native performance acceptance

Apple M4 Pro, 48 GiB RAM, macOS arm64. Rust 1.97.1 release builds with the committed lockfile and default features. Reference revisions: jless `21dd610` (0.9.0), tless `9bb9568`. Candidate source is this implementation; binary hashes, build provenance and every raw sample are in `performance.json`.

The opt-in `tests/performance.rs` harness generated exactly 10,238,903 bytes, used isolated 140×40 PTYs, timed before spawn through meaningful content and status output, answered cursor queries and required successful quit. One warmup and 20 rotating-order timed launches per binary. Nearest-rank p95; separate startup RSS sampling, not instrumented timing or exit high-water memory.

| Build | Median ms | p95 ms | Sampled startup peak KiB |
| --- | ---: | ---: | ---: |
| jless | 47.6439 | 50.6723 | 261,200 |
| `9bb9568` | 960.9399 | 974.8242 | 2,415,760 |
| Candidate | 135.7625 | 139.7731 | 641,008 |

Candidate median/p95 are 2.850×/2.758× jless; median is 0.1413× baseline (7.08× faster). Both required gates passed. Raw evidence retains the jless 735.68 ms outlier and the candidate 414.42 ms warmup; neither was silently discarded beyond the prescribed warmup exclusion and percentile calculation. An earlier candidate missed the 3× gate; the final semantic scan removes duplicate key decoding rather than accepting that result.

Presentation lifetime is bounded by visible/requested rows, not total visited rows. Input parsing, semantic metadata and logical geometry still scale with document size. A single long row still requires row-local measurement and wrapping work. RSS is sampled, not an exact maximum or a constant-memory claim.

## Spec synchronization

Compared both branch-associated deltas with `openspec/specs/toon-rendering/spec.md`:

- `viewport-driven-toon-rendering`: all three added requirements and eight scenarios match verbatim.
- Archived `responsive-large-file-viewing`: its added requirement and three scenarios match verbatim.

Neither delta includes modifications, removals or renames. Existing main requirements remain intact. The PR-scoped completion body declares both changes and both synchronization reviews; maintainer confirmation is still required before merge.
