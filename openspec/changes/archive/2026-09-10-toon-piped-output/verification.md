# Implementation verification

## Completeness

All 13 tasks are complete. The seven piped-output requirements have implementation and automated coverage. Both compiler matrices and full committed-head preflight passed. The implementation is published in [PR #11](https://github.com/commandzero/tless/pull/11). Maintainer review remains required before merge.

## Correctness

| Requirement and scenarios | Implementation | Evidence |
| --- | --- | --- |
| Independent output selection: default pipeline, explicit conversion, invalid selector | `src/options.rs`, `src/main.rs`, `src/output.rs` | `selectors_and_conversion_matrix`, `file_detection_and_override_do_not_select_output`, existing TOON-extension override test |
| Terminal stdout | Machine branch remains before terminal setup; output option is unused by viewer | `output_selection_leaves_terminal_view_and_json_print_unchanged`, retained interactive write/print tests |
| Parsed TOON: equivalent inputs, empty object, unsupported document, duplicate keys | Existing codec adapter, with safe YAML-to-JSON adapter input | `toon_export_contract_and_failures`, format matrix, retained codec normalization and depth tests |
| JSON: compatibility override, escaped YAML strings, values outside JSON | Existing JSON-input pretty-printer; decoded-value serializer for conversions | `json_preserves_tokens_duplicates_and_root_framing`, `yaml_strings_are_json_escaped_and_types_survive_yaml_output`, `yaml_preserves_typed_complex_keys_and_nonfinite_values` |
| YAML: document stream, typed values and keys | Ordered row traversal, quoted strings, explicit typed keys, per-root separators | YAML tests above cover stream framing, scalar and complex keys, infinity, NaN, strings resembling literals, duplicate entries, and empty containers |
| Feature availability: minimal build | Output enum exists in every profile; codec availability checked only for machine output | `disabled_toon_output_is_actionable_without_fallback`, matrix tests and terminal-selection test in minimal and sexp-only profiles |
| Failures: invalid same-format input, resource or downstream failure | Parse/encode before locked stdout write and flush | `toon_pipeline_validates_before_output`, TOON failure matrix, `input_limit_applies_to_stdin_and_files_without_partial_output`, broken-pipe tests, `machine_output_needs_no_controlling_terminal` |
| Help and migration guidance | Option help, README, TOON guides, Unreleased entry linked to issue 7 | Help assertions, format/framing tests, `scripts/validate-docs.sh` |

## Coherence

The new output module has no terminal dependencies. JSON/YAML serialization does not require the optional codec or convert numbers through floating point. YAML output uses flow collections with two-space layout; the contract specifies data and document framing rather than block-style collections. Existing interactive exports and input parsers are unchanged. No dependency, lockfile, compiler minimum, or release version changed.

TOON still uses the pinned published encoder. Its duplicate-key normalization, numeric conversion, table ordering, and invalid empty-object-array output remain documented and tested. No universal validity or exact round-trip claim is made. YAML duplicate entries are preserved in emitted text; downstream parsers can normalize duplicates differently.

## Validation

- Rust 1.87 and Rust 1.97.1: all four feature profiles passed on each compiler, including isolated terminal tests, with no skips. Each final matrix ran 508 tests. This total includes the isolated terminal tests in `tests/toon_cli.rs`.
- Strict active-change validation and all four main-spec validations passed with OpenSpec 1.11.0.
- Complete docs bundle validation passed with zero errors or warnings.
- The initial development preflight passed lint, workflow checks, and 20 OpenSpec gate fixtures, then correctly rejected the still-active committed planning change. Final committed-head validation passed after archival.
- A sandbox terminal-access failure was rerun with terminal access. Concurrent compiler matrices caused a CLI executable replacement race; final matrices run sequentially.

- Full final preflight passed, including formatting, minimal/all-feature Clippy, ShellCheck, actionlint 1.7.12, release tests, 20 OpenSpec gate fixtures, the associated-change gate, docs validation, and 508 feature-matrix tests, including isolated terminal tests.
- Native archived-task validation passed for both archives. The associated-change gate passed native delta, archive-task, and main-spec validation with the prepared PR description.

The final counts below apply to each compiler. Earlier runs had 504 tests; adding the long-key regression added one test to each of the four feature profiles.

| Feature profile | Unit tests | Piped-output tests | CLI tests, including isolated terminal tests | Total |
| --- | --- | --- | --- | --- |
| Minimal | 84 | 9 | 20 | 113 |
| Default TOON | 103 | 9 | 27 | 139 |
| S-expression only | 86 | 9 | 20 | 115 |
| All features | 105 | 9 | 27 | 141 |
| Per compiler | 378 | 36 | 94 | 508 |

## Synchronization review

`piped-output` is a new capability. Reviewed all seven added requirements and their scenarios against the new main spec. Their bodies match exactly. The purpose is preserved; the main spec uses `## Requirements` rather than a delta-operation header. No existing capability is modified, removed, or renamed.

Rechecked synchronization and archival on 2026-09-10 at commit `f593ee8b3f78668333a2381a7d917dc675f7763b`. The archived delta still matches the main spec, all tasks are complete, and the PR-scoped completion gate passed native change, archived-task, and main-spec validation. The existing archive required no further move or spec edits.

## Long-key verification follow-up

A subsequent verification found that string keys with more than 1024 emitted characters produced invalid YAML. The first failing ASCII case had 1023 key characters plus two quotes. The new `yaml_long_and_escaped_string_keys_round_trip` test reproduced the failure before the fix. The serializer now uses explicit key syntax when the quoted and escaped representation exceeds that limit.

Regression cases cover 1021, 1022, 1023, 1024, and 1100 ASCII characters, escaped quotes and controls, and multibyte Unicode. Each runs through both JSON and explicit-key YAML input, reparses the emitted YAML, checks nested values and siblings, and compares conversion back to JSON. The added scenario is synchronized between the archived delta and the main spec. This closes the gap in the earlier verification.
