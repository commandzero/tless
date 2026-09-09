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

Pending full preflight, Rust 1.87 feature matrix, standards/spec review, synchronization, and archive validation.
