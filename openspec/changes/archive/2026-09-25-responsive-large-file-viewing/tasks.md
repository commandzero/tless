## 1. Implement the startup-only optimization

- [x] 1.1 Initialize the existing viewer with actual content dimensions, effective number visibility, and already-resolved roots; preserve filtered-root exposure and first-draw resize handling.
- [x] 1.2 Reuse rendered layout when gutter growth leaves every annotated Unicode inline array fitting; update preview width and retain the rebuild fallback when grammar changes.

## 2. Verify behavior and improvement

- [x] 2.1 Verify initial Unicode exact-fit boundaries with and without numbers, digit-boundary fallback/reuse, and existing annotation, collapse, resize, search, filter, and terminal behavior.
- [x] 2.2 Compare release startup against the baseline with the same disposable synthetic workload and record the measured improvement and remaining synchronous/memory limitations in design.md.
- [x] 2.3 Run all five feature profiles on Rust 1.97.1 and Rust 1.87 without adding a permanent profiling framework or dependencies. The commit-aware preflight gate runs after this completed change is archived.

## 3. Complete the narrowed change

- [x] 3.1 Revise the proposal/design/specs and Unreleased changelog, remove superseded capability deltas, synchronize the one remaining rendering requirement, and validate its correspondence to the implementation.
