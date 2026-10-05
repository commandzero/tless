## 1. Command contract

- [x] 1.1 Update the existing registry and `WriteFormat` for the TOON default, explicit JSON/YAML/TOON, canonical NDJSON with JSONL spelling, and feature-gated s-expressions; remove obsolete long names and retain specified short aliases, including `wj` for JSON, `wy` for YAML, and `wn` for NDJSON/JSONL with bang variants. Verify parser behavior covers formats, bang variants, missing/extra arguments, unsupported commands, feature gating, and rejection of removed names.
- [x] 1.2 Migrate command completion to the registered hyphenated names without a second candidate list. Verify prefix ordering, JSON/JSONL discovery, first-token argument preservation, cycling/cancellation, no automatic bang hint on exact names, and optional-command availability through the existing completion behavior tests.

## 2. Encoding and file dispatch

- [x] 2.1 Add compact JSON root encoding alongside the existing pretty traversal, sharing string/key/number conversion without building a second document model. Verify exact LF record framing, escaped controls and line breaks, nested structures, array/scalar roots, duplicate-entry order, precise numeric spellings, YAML numeric conversion, and failures for non-string keys/non-finite numbers; verify existing pretty JSON and YAML output coverage remains passing.
- [x] 2.2 Route all interactive formats through their native serializers and active roots while preserving serialization-before-open and existing create/overwrite behavior. Verify actual pseudoterminal writes produce TOON by default, JSON/YAML when explicit, byte-identical NDJSON/JSONL, and s-expressions only with the feature; verify literal filenames regardless of suffix and unchanged CLI selector behavior.
- [x] 2.3 Extend existing file-export behavior coverage for filtered scope across formats, array-preserving record counts, overwrite refusal and explicit truncation, encoding failures for existing and missing destinations, and TOON multiple-root failures. Verify destination bytes and unchanged viewer state rather than mock forwarding or source-text assertions.

## 3. User guidance and integrated verification

- [x] 3.1 Update `src/tless.help`, `src/toon.help`, README, relevant export/acceptance docs, and Unreleased changelog with the command table, optional `sexp`, literal filenames, root framing, overwrite guarantees, and breaking migration examples. Verify obsolete long names occur only in historical or explicitly labeled migration guidance, current examples agree with the specs, and `scripts/validate-docs.sh` passes.
- [x] 3.2 Smoke-run the actual viewer in a disposable pseudoterminal, execute each available format command and representative filtered/failed writes, exercise completion and help, and inspect destination bytes and diagnostics. Verify all six encodings/spellings follow the interactive-write contract, no display decorations leak into output, and existing CLI JSON/YAML/TOON framing is unchanged.
- [x] 3.3 Run `scripts/preflight.sh` with OpenSpec 1.11.0 available on PATH and `TLESS_TOOLCHAIN=1.87.0 scripts/preflight.sh test`. Verify all configured feature profiles pass and compatibility uses the committed lockfile; report any prerequisite failure rather than skipping coverage.

Verification:

- All five feature-profile test suites passed on Rust 1.97.1 and Rust 1.87.0 with the locked dependencies. The existing frozen-release-binary performance benchmark remains ignored by these suites.
- Final formatting and all-target Clippy passed for minimal and all-feature profiles. Documentation validation and strict OpenSpec 1.11.0 change validation passed.
- Disposable actual-viewer smoke runs verified native formats and aliases, compact record fidelity, filtered scope, literal filenames, overwrite refusal/truncation, encoding-failure file preservation, optional s-expression availability, completion, help, and unchanged pipeline selectors/framing.
- Full preflight ran its lint and gate regression checks, then stopped at the committed-HEAD OpenSpec completion gate because the planning commit still contains this active change. At that run, implementation was uncommitted. This result is not a full preflight pass.
