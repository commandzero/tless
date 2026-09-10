## 1. CLI and machine-output routing

- [ ] 1.1 Add independent `-o` / `--output` selection with TOON default; verify short, long, equals, missing-value, invalid-value, and help behavior with CLI tests.
- [ ] 1.2 Replace redirected pass-through with parse/encode/write routing before terminal setup; verify stdin, `-`, filenames, extension overrides, and execution without a controlling terminal.
- [ ] 1.3 Handle TOON output availability in the machine path; verify default and explicit TOON failures in minimal and sexp-only builds, successful JSON/YAML overrides, and continued interactive startup.

## 2. Serialization

- [ ] 2.1 Connect the existing whole-document TOON encoder; verify default and explicit output match interactive standard-export bytes for representative supported values, normalization cases, empty objects, and the documented empty-object-array limitation.
- [ ] 2.2 Implement JSON conversion using decoded values and safe escaping while preserving JSON-input pretty-print behavior; verify duplicate entries, exact numeric tokens, multiple roots, YAML string escaping, and rejection of typed keys and non-finite numbers.
- [ ] 2.3 Implement YAML serialization of ordered parsed entries with document framing; verify strings resembling scalars, control escapes, empty containers, typed scalar and complex keys, non-finite values, duplicate-entry handling, and multiple roots.
- [ ] 2.4 Complete failure handling before output and during write/flush; verify malformed same-format input, unsupported TOON roots/values/depth, input limits, empty input by parser, and broken-pipe diagnostics without panics or premature stdout.

## 3. Integration and documentation

- [ ] 3.1 Replace obsolete pass-through/default assertions and add the supported input/output matrix; verify exact framing and absence of terminal annotations, with PTY checks that output selection leaves the viewer and interactive exports unchanged.
- [ ] 3.2 Update CLI help, README, affected TOON guides and acceptance checks, and Unreleased migration notes; verify documented examples against CLI tests and run `scripts/validate-docs.sh`.
- [ ] 3.3 Run `scripts/preflight.sh` and `TLESS_TOOLCHAIN=1.87.0 scripts/preflight.sh test`; resolve failures and record results for all four feature profiles without skipping terminal tests.

## 4. OpenSpec completion before merge

- [ ] 4.1 Verify implementation against every piped-output requirement and scenario; record correspondence and any codec limitations without claiming universal round-trip correctness.
- [ ] 4.2 Synchronize the capability into main specs and archive the completed change; verify native strict change/spec validation and archived-task validation using OpenSpec 1.11.0 with telemetry disabled.
- [ ] 4.3 Review archived deltas against main specs, add the required OpenSpec association and sync-review statements to the implementation PR description, and run the committed-head PR-scoped gate through the repository entry point; verify that the associated change passes before merge.
