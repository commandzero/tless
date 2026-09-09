# Implementation verification

Implementation scope: parsed-node TOON view, navigation, display extensions, and PR completion gate.

## Evidence recorded before final review

- Gate commit 3d71133: 17 native OpenSpec 1.11.0 regression scenarios pass under Bash 3.2, plus ShellCheck and actionlint 1.7.12.
- Layout commit 9791361: 11 focused tests pass, including more than 80 retained standard grammar cases, exact bounded numbers, node/span mappings, warning ownership/counts, collapse restoration, and a 300 KB parsed Unicode scalar.
- UI commit f487562: 69 default and 50 minimal unit tests pass. All-feature/all-target Clippy passes. Four new PTY scenarios pass for equivalent input formats, hidden-search cell printing, resize/column selection, and duplicate occurrence selection. Final combined feature matrix is recorded below after completion.
- Documentation validator: 8 concepts, zero errors or warnings.
- Native active-change validation passes with telemetry disabled.

## Interactive acceptance

Host: macOS 26.6.2 arm64. Checked debug UI build matching commit f487562 with layout 9791361 using disposable pseudoterminals and a pyte screen reader. No user clipboard or controlling terminal was touched.

At 120 and 30 columns, inspected inline arrays and tables, selected Ada/Lin cells, retained the selected field on vertical movement, printed the focused cell, resized without losing selection, collapsed the table, and searched for the hidden Lin cell. The search expanded the table and restored the cell path. Verified duplicate occurrence 1/2 status, exact decimal digits, noncanonical-number warnings, typed YAML keys/non-finite numbers, and multiple-root separators. All 12 acceptance assertions passed; sessions exited successfully. Separately verified horizontal warning scrolling after resize, including the final characters and leftward return. Release-only clipboard and Linux host checks remain in docs/toon-acceptance.md.

Cold newly built executables intermittently delayed startup before producing output, including --version. Stable executions subsequently returned --version in 6-13 ms and passed terminal checks. This environmental startup symptom is not counted as a passing test; final failed invocations must be rerun.

## Redraw workload

Debug build, 120x24 terminal, two-field object tables. Sent 100 downward motions and counted 100 complete frame starts. The 5,000-row input was 142,781 bytes and redraws took 0.085 s. The 20,000-row input was 597,781 bytes and redraws took 0.092 s. Both documents were ready within the 1-second polling interval and exited normally. These are observed smoke measurements, not a cross-platform performance promise.

Code inspection confirms Layout::new runs on document construction, while navigation and resize use the cached layout/projection. Collapse rebuilds the indexed visibility projection without codec encoding. Numeric expansion checks its 4096-character bound before allocating the decimal result; focused tests cover huge exponents. A large direct parsed scalar verifies bounded preview materialization independently of the existing parser.

## Final verification

Reviewed implementation: c7e35ae, merged at f502558. Warning-order clarification and regression: 19a8e8d, merged at ddc5218.

- Parallel standards and spec reviews identified 12 findings. A single fix implementer addressed all of them. Independent follow-up reviews closed every finding, including the final warning-order wording correction.
- Fixes cover source-to-display substring mapping and search reveal, typed complex YAML keys, explicit table membership, inline warning locators, retained header styling, container focus, scrolloff/end-scroll behavior, archive preservation, and retained navigation tests.
- Unsupported string controls now have an explicit terminal-safe display spelling and `Non-standard string escape` warning. Warning order is explicitly parsed-node order, then kind order within each node, with hidden summaries last.
- All four feature profiles passed on Rust 1.97.1 and Rust 1.87.0 after the behavioral fixes. Before the final additional warning-order regression, counts were 62/81/64/83 unit tests and 12/19/12/19 CLI/PTY tests for minimal/default/sexp-only/combined respectively. No ignored or skipped tests. `RUST_TEST_THREADS=1` was used with disposable terminal access.
- Repeated all 12 wide/narrow terminal acceptance assertions against the fixed build; all passed and every session exited successfully. Independent reviewer terminals also confirmed each reported behavior correction.
- All 20 completion-gate fixtures passed under Bash 3.2. Both Clippy profiles and the docs validator passed.
- Cargo.toml, Cargo.lock, and LICENSE have no changes relative to the PR base. The published codec dependency and existing licenses are preserved.
- Compared every requirement and scenario with implementation evidence. Main specs preserve all 17 requirements and 38 scenarios from the three deltas, including their Purpose text, with no pending delta operations. Native strict active-change and main-spec validation passed.

Full `scripts/preflight.sh` passed on committed archive 73648ee with the synchronization-review PR fields. This included formatting, both all-target Clippy profiles, shell syntax/ShellCheck, actionlint, release fixtures, all 20 OpenSpec gate fixtures, the committed-head completion gate, docs validation, and all four feature profiles. `TLESS_TOOLCHAIN=1.87.0 scripts/preflight.sh test` also passed after the final regression addition. Final counts on each compiler were 63/82/65/84 unit tests and 12/19/12/19 CLI/PTY tests, 356 tests per compiler with zero failures or ignored tests.

Repeated the redraw workload on the fixed build: 5,000 rows / 147,781 bytes and 20,000 rows / 617,781 bytes each produced all 100 requested frames in 0.095 seconds. Both sessions exited normally. The direct 1 MB parsed scalar test verifies coalesced source mapping independently of the pre-existing tokenizer's large-scalar throughput issue.

All implementation and completion tasks are verified. Native active validation passed before archival; native archived and main-spec validation passed after archival. All three archived deltas were explicitly compared with the main specs and have no remaining changes to apply.

## Hosted review follow-up

Hosted compatibility passed on macOS and Linux. The first hosted preflight exposed a test-only BrokenPipe race in obsolete-option rejection: the child correctly exited before unused input was written. Commit af7d174 removes that unnecessary input while retaining all exit-code assertions; ten repeated focused invocations passed.

The hosted review also identified indentation reduction incorrectly acting as global horizontal scrolling. Commit af7d174 adds one capped leading-space conversion shared by painting, mouse, search, and scrolling. Commit 54e40c9 aligns offsets with grapheme boundaries so clicking a value after a partially clipped wide character selects the painted value. Cached layout remains unchanged, and left-edge redraws avoid alignment scans.

Independent standards/spec follow-ups are clear. All five terminal indentation assertions passed on the rebuilt normal executable. The exact wide-character reproduction now selects `input.nested[1]` when clicking the visible `2` after `l12.`. General wide/narrow acceptance also passed after the indentation change. Final full preflight passed on ab059ca, including the committed-head gate and all four feature profiles. The Rust 1.87 matrix also passed. Both compilers ran 66/85/68/87 unit tests and 12/19/12/19 CLI/PTY tests, 368 tests per compiler with zero failures or ignored tests. Hosted CI runs the same entry points; its latest result is attached to PR #4.

A subsequent hosted review found unnecessary scrolling for an already-visible match in the last viewport column. Commit 2e1a6e7 shares the painter's bounded clipping-window calculation with search reveal, accounting for actual left/right markers. Independent terminal verification confirms that searching for `Z` at width 35 in an exactly fitting line preserves the whole line. Unit cases cover no markers, left only, right only, both markers, and zero width; the terminal regression also passes. Both narrow follow-up reviews are clear. Final matrix results are recorded after the committed-head check.
