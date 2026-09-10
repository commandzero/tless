# Implementation verification

## Completeness

All 13 tasks are complete locally. The seven piped-output requirements have implementation and automated coverage. Both compiler matrices and full committed-head preflight passed. The implementation PR description is prepared in `pr-description.md`; publishing a PR and maintainer review remain separate actions.

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

- Rust 1.87 and Rust 1.97.1: all four feature profiles passed on each compiler, including isolated terminal tests, with no skips. Each matrix ran 504 tests.
- Strict active-change validation and all four main-spec validations passed with OpenSpec 1.11.0.
- Complete docs bundle validation passed with zero errors or warnings.
- The initial development preflight passed lint, workflow checks, and 20 OpenSpec gate fixtures, then correctly rejected the still-active committed planning change. Final committed-head validation passed after archival.
- A sandbox terminal-access failure was rerun with terminal access. Concurrent compiler matrices caused a CLI executable replacement race; final matrices run sequentially.

- Full final preflight passed, including formatting, minimal/all-feature Clippy, ShellCheck, actionlint 1.7.12, release tests, 20 OpenSpec gate fixtures, the associated-change gate, docs validation, and 504 feature-matrix tests.
- Native archived-task validation passed for both archives. The associated-change gate passed native delta, archive-task, and main-spec validation with the prepared PR description.

## Synchronization review

`piped-output` is a new capability. Reviewed all seven added requirements and their scenarios against the new main spec. Their bodies match exactly. The purpose is preserved; the main spec uses `## Requirements` rather than a delta-operation header. No existing capability is modified, removed, or renamed.
