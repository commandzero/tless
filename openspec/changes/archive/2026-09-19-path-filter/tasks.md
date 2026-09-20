## 1. Path parsing and resolution

- [x] 1.1 Implement shared friendly-dot/yp and RFC 6901 parsing, including concrete numeric/JSON-quoted selectors and leading brackets, with explicit syntax precedence, empty/root handling, literal whitespace preservation, and safe structured diagnostics.
- [x] 1.2 Resolve tokens against original FlatJson roots using decoded string keys and checked array indices; reject ambiguous duplicates, scalar traversal, non-string-key coercion, zero-document nonempty selection, and partial stream matches.
- [x] 1.3 Add focused behavioral coverage for syntax distinctions, typed bracket selectors, tilde decoding order, empty keys, Unicode/whitespace, numeric object keys versus array indices, duplicate ambiguity, and all-document atomicity.
- [x] 1.4 Prove actual `yp` formatter-to-filter round trips through both entry points for nested arrays, root arrays, special root keys, JSON escapes, and filtered-view absolute ancestry; support `.` for root copy and retain existing resolution-error policies. Preserve `yq`/`pq` jq-query behavior and `yb`/`pb` external representations without extending the round-trip promise to them.

## 2. Selected-root rendering and interaction

- [x] 2.1 Introduce ordered active roots retaining original document/node identity, with shared membership and effective-parent semantics; preserve the original parsed model and unfiltered behavior.
- [x] 2.2 Build root-relative TOON layouts and projections for selected objects, arrays, table rows/cells, scalars, and empty values; exclude ancestor/sibling content and warnings while preserving original status/copy paths.
- [x] 2.3 Confine structural/count/vertical motion, fallback navigation, mouse actions, absolute/relative numbering, wrapping, and viewport bounds to active roots; preserve sequence-root sibling navigation.
- [x] 2.4 Scope search enumeration, counts, repeat-search, and reveal to selected values, excluding omitted root keys and ancestors; verify collapsed selected descendants remain searchable.
- [x] 2.5 Add dot-led and leading-concrete-bracket path submission and atomic filter transitions, including failure/cancellation preservation, success focus/scroll/search reset, descendant presentation-state retention, and `.` restoration; retain command-completion/search-prompt isolation and normal-mode bracket navigation.

## 3. Startup and export integration

- [x] 3.1 Add `--path <path>` in all builds, validate syntax as argument errors, and prepare parsed input plus resolved roots once before terminal initialization; retain full-input limits and parse failures.
- [x] 3.2 Adapt machine JSON/YAML/TOON serialization to selected roots, preserving source fidelity where promised, framing, default TOON restrictions, selected-data validation, and serialize-before-write behavior.
- [x] 3.3 Adapt every interactive JSON/TOON/s-expression write alias and overwrite form to selected roots, preserving focused-copy behavior and serialization-before-file-open safety.
- [x] 3.4 Add behavior-focused regressions for excluded-data serialization restrictions, multi-document framing/failure atomicity, startup error exit codes without terminal sequences, and existing-file preservation on filtered export failure.

## 4. End-to-end verification and user guidance

- [x] 4.1 Exercise the actual TUI with nested objects, inline/table values, empty/scalar roots, and a document stream: enter both path forms, copy with `yp` and paste unchanged into the command prompt, inspect full paths, attempt boundary navigation/search, resize/wrap, export, reject a bad path, and reset.
- [x] 4.2 Smoke-test actual CLI pipelines for equivalent path forms, special keys, JSON/YAML framing, TOON single-root success and multi-root rejection, empty/root equivalence, malformed excluded input, and no-output failures; remove disposable fixtures afterward.
- [x] 4.3 Update CLI help, interactive help, and applicable user docs with both syntaxes, the `yp` round-trip promise and its numeric/quoted/leading bracket forms, unchanged `yq` compatibility, `yb` scope, root/empty-key distinction, strict escape examples, absolute/all-document semantics, filtered exports, and whole-input resource limits; add an Unreleased feature entry under repository changelog policy.
- [x] 4.4 Run `scripts/validate-docs.sh`, `scripts/preflight.sh`, and `TLESS_TOOLCHAIN=1.87.0 scripts/preflight.sh test`; resolve failures attributable to this implementation and record actual terminal verification.

## 5. Specification completion gate

- [x] 5.1 Verify implementation against every requirement/scenario in the path-filter, toon-navigation, and piped-output deltas, including preserved unfiltered contracts.
- [x] 5.2 Synchronize all three deltas into main specs and archive this completed change with the pinned OpenSpec CLI and telemetry disabled; validate affected specs/change and archived task completion in strict mode.
- [x] 5.3 Compare archived deltas with main specs, record the required synchronization-review statement, and run the committed implementation PR's scoped completion check before merge.

## Verification evidence

- Native macOS arm64, Rust 1.97.1: all-feature unit, pipeline, and actual pseudoterminal tests passed, including formatter-to-filter identity through both entry points, hidden matches, filtered exports, warning exclusion, and s-expression scalar-root framing.
- Rust 1.87.0: `TLESS_TOOLCHAIN=1.87.0 scripts/preflight.sh test` passed all five feature profiles.
- `scripts/validate-docs.sh` passed with zero errors or warnings.
- Actual compiled CLI smoke passed friendly/yp/pointer equivalence, special keys, JSON/YAML sequence framing, TOON single-root success and multi-root rejection, empty/root equivalence, excluded malformed input, whole-input byte limits, atomic failure, and YAML/TOON input conversion.
- Actual compiled TUI smoke passed nested object/table/inline/scalar/empty-root presentation, both filter syntaxes, clipboard `yp` paste unchanged (`.hits.users[0].name`), boundary motion and excluded search, resize/wrap, filtered export, bad-path preservation, reset, and selected-document sequence navigation/export. Disposable harness and fixtures were removed.
- Reviewed all seven path-filter requirements, three navigation modifications, and one pipeline modification against implementation and behavior evidence. Main-spec requirement/scenario blocks match the deltas exactly; unrelated main-spec requirements were preserved.
- Archived-delta synchronization review: all 11 requirement blocks and their scenarios match the main specs; no removals or renames were required.
- `scripts/preflight.sh` passed with Rust 1.97.1 and OpenSpec 1.11.0: formatting, both Clippy profiles, shell/workflow checks, release/OpenSpec gate regressions, documentation validation, and all five feature-profile test suites.
- The committed-HEAD completion check against fetched `origin/main` passed with the review fields below: archived change valid and complete, original artifacts preserved, and all three affected main specs strictly valid.

```text
OpenSpec-Change: path-filter
OpenSpec-Sync-Reviewed: path-filter
```
