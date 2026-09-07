# TOON support acceptance

Scope: `openspec/changes/add-toon-support`. The separate TOON rendering change
is not implemented here. Build from this checkout with `--locked --features toon`.

The user selected order preservation. Rows with different key orders use list
form as a documented canonical-profile exception. The work is on the approved
`add-toon-support` branch. Desktop/manual release checks remain below.

## Automated coverage

| Capability | Evidence |
| --- | --- |
| Pinned strict v3 grammar, all root/array forms, delimiters, indentation, quotes, escapes, blank rows, literal dotted keys | `toon::fixtures::pinned_decode_profile`: 180 applicable cases |
| Canonical form selection, tables, ordered keys, Unicode/quoted keys, two-space comma output, no folding/final newline, empty root object | `toon::fixtures::pinned_encode_profile`: 113 upstream payloads plus one explicit order-preserving profile expectation; ordered round trips for all 114 cases |
| Explicit option exclusions and source/license attribution | `fixtures/toon-v3/PROFILE.md` and its case inventory |
| Empty input, CRLF, trailing blanks, initial BOM | `accepts_empty_input_and_crlf_but_rejects_bom` |
| Input decimal fidelity, overflowing integers remain numeric errors, leading-zero strings | `rejects_decimal_rounding_before_accepting_a_value`, `decoding_never_reclassifies_out_of_range_numbers_as_strings` |
| Duplicate keys/fields, one-based locations, huge counts, invalid array headers, strict blank rows | `rejects_duplicate_decoded_names`, `strict_diagnostics_are_located_and_huge_counts_fail_safely` |
| 256/257-container boundary, named-array parent counts, long blank runs | `enforces_container_depth_and_scans_long_blank_runs`, `input_depth_counts_the_object_around_a_named_array` |
| Deterministic linear scanner work | `blank_line_scanner_work_grows_linearly`: at most 2.2x visits for 200,000 versus 100,000 blank lines |
| Valid empty-object array output and typed diagnostics unaffected by key names | `empty_object_arrays_have_a_valid_list_round_trip`, `diagnostic_categories_do_not_depend_on_key_contents` |
| JSON-shaped backing text and row/key ranges | `reads_a_toon_object_as_existing_json_rows` |
| Navigation, collapse/expand, Line/Data modes, matching pairs, paths and search with escapes/Unicode | `decoded_navigation_search_and_paths_match_json_with_escaped_unicode` |
| Unique string-key export domain, duplicate entry ordinal/path, unsupported YAML and supported focused subtree | `export_rejects_duplicate_decoded_keys_without_losing_an_entry`, `unsupported_yaml_does_not_block_a_supported_focused_value` |
| Exact numeric export, i64/u64 boundaries, negative zero, exponent/precision rejection | `export_preserves_exact_numbers_or_rejects_them` |
| Multi-root document rejection, focused subtree allowance, closing-row equivalence and collapsed descendants | `focused_export_includes_collapsed_children_and_normalizes_closing_rows` |
| Export depth relative to the selected subtree, checked before recursive conversion | `export_depth_is_relative_to_the_selected_subtree` |
| Disabled option/help, extension detection, explicit overrides | `toon_cli::disabled_build_omits_toon_option_and_help`, `toon_extension_is_detected_and_explicit_json_overrides_it` |
| Format conflicts, stdin default, invalid UTF-8 | `format_conflicts_and_invalid_utf8_fail_without_output` |
| Byte-preserving non-terminal input, malformed syntax, BOM/CRLF, output failure status | `toon_pipeline_passes_through_without_validation`, `output_io_failure_exits_nonzero` |
| Write commands from JSON, YAML and TOON; existing JSON write output | Pseudoterminal tests `writes_canonical_toon_through_the_viewer_command`, `writes_toon_from_yaml_and_toon_inputs_without_changing_json_commands` |
| Refuse overwrite; replace shorter/empty files; create missing file with bang; encoding failure preserves existing file and does not create a missing file | `bang_writes_replace_the_entire_file_after_successful_encoding` |
| File-open failure produces no false success and leaves viewer usable | `write_open_failure_reports_an_error_and_keeps_the_viewer_usable` |
| Persistent focused TOON print and prompt separated from payload | `prints_focused_canonical_toon_on_the_persistent_screen` |
| Terminal byte-stream handling | `terminal_peer_answers_cursor_requests_across_read_boundaries`; output is drained to EOF/EIO |
| Default/TOON/S-expression/all-feature combinations and Rust 1.67 | `.github/workflows/ci.yml` feature matrix; local results recorded below |

Tests use isolated pseudoterminals and uniquely named temporary files. They do
not write to the user's clipboard or controlling terminal. Terminal tests can
require permission to configure the pseudoterminal; a sandbox
`EPERM` is not an acceptable passing or skipped result.

## Manual release checks

These checks are documented but have not been performed against the user's
desktop clipboard or interactive help pager.

1. In a TOON-enabled build, copy a nested value using `yt`, paste it into a text
   editor, and compare it with `pt`. Repeat on its closing row in Line mode and
   after collapsing it. Verify `yy`, string/key copy, and path copy still match
   equivalent JSON input, including escaped and non-ASCII text.
2. Put a marker in the clipboard. Open `0.123456789012345678901` as JSON and use
   `yt`. Confirm an `UnsupportedNumber` warning and that the marker remains.
3. Open in-app help in enabled and disabled builds. Only the enabled build may
   list TOON commands. In the disabled build, `:wt`, `yt`, and `pt` must remain
   unavailable/unbound. Check the README's `toon-spec: 3.0` and 4.x limitation.
4. On Linux, use a disposable session to write to `/dev/full` with `:wt!`.
   Confirm a write error, no success message, and continued navigation. The
   shared writer propagates both write and flush errors. Standard `File::flush`
   has no buffer to flush; no separate failing-file-flush test is claimed.
5. Open malformed TOON interactively. Confirm a useful parse diagnostic and
   normal terminal restoration. Valid non-terminal malformed TOON pass-through
   is covered automatically and must not enter this parser path.

## Current compliance checks

The compliance update renames the executable to tless and enables TOON by default.
Use `--no-default-features` to exercise the disabled build.
The shared preflight validates all 4 profiles on Rust 1.97.1 and Rust 1.67.
After 2 new CLI contract tests, the profiles pass 71 tests without features,
99 with TOON, 73 with S-expression only, and 101 with both features.
The 294 applicable fixture cases remain inside 2 tests.
All-target Clippy with warnings denied, formatting, ShellCheck, workflow validation,
and release-gate regression tests pass. The macOS arm64 archive passes extraction,
version, JSON, TOON, and input-limit checks.
The inherited block 0.1.6 on macOS and xcb 0.8.2 on Linux emit Cargo's
future-compatibility notice on the pinned development compiler. It does not fail the current compiler or Clippy.
Manual release checks above remain pending; these local results do not replace them.

## Historical build notes

After review fixes, both the current compiler and Rust 1.67 passed 69 tests
without optional features, 97 with `toon`, 71 with `sexp`, and 99 with all
features. The fixture suites count as two tests; all 294 applicable fixture cases
run within them. Sandbox terminal-setup failures were rerun with isolated
pseudoterminal permission and passed; they were not counted as skipped tests.

Formatting and strict OpenSpec validation pass. Plain current-toolchain Clippy
fails on the pre-existing `clippy::never_loop` in `src/jsonparser.rs`. Allowing
that lint completes with existing warnings and no new TOON warnings. This change
does not claim a warning-free cleanup of the older codebase.

The adapter checks numeric text and duplicate names before inserting into
order-preserving JSON maps. It does not keep a second lifetime-long value tree.
The released codec needs a fixed 16 MiB worker stack in debug builds; the
structural depth checks run before entering each nested container.

The local Cargo patch is for source-checkout/binary distribution. Registry
publication requires a separately published compatible codec version because
Cargo removes local patches when publishing. No registry release is performed
by this change.
