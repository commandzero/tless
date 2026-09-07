---
type: Guide
title: TOON acceptance checks
description: Automated coverage and manual release acceptance checks.
status: draft
generated: { by: codex/gpt-6, at: 2026-09-07T18:15:35Z }
---

# TOON acceptance checks

Use [published codec behavior](toon_codec.md) for the conversion contract.
The former vendored codec's exact-number, duplicate-key rejection, key-order,
and linear scanner-work guarantees no longer apply.

## Automated checks

Run `scripts/preflight.sh` with the pinned development compiler, then
`TLESS_TOOLCHAIN=1.87.0 scripts/preflight.sh test` for the minimum compiler.
Both exercise minimal, default TOON, S-expression-only, and combined features.
The two fixture tests exercise 180 decode cases and 114 encode cases.
The known large-number round-trip failure has an explicit string-result assertion.

| Coverage | Evidence |
| --- | --- |
| Published strict decoder and default encoder | `toon::fixtures::pinned_decode_profile`, `pinned_encode_profile` |
| Last-value-wins duplicate keys and numeric conversion | `duplicate_keys_follow_published_last_value_wins`, `export_duplicate_keys_follow_last_value_wins`, `decimal_decoding_follows_published_numeric_conversion`, `export_numbers_follow_published_numeric_conversion` |
| Known invalid empty-object array output | `empty_object_arrays_expose_published_codec_limitation` |
| Nesting bounds, empty input, CRLF, BOM rejection | `input_depth_follows_published_codec_boundary`, `enforces_container_depth_and_accepts_blank_lines`, `accepts_empty_input_and_crlf_but_rejects_bom` |
| Focused export, unsupported YAML, multiple roots | `focused_export_includes_collapsed_children_and_normalizes_closing_rows`, `unsupported_yaml_does_not_block_a_supported_focused_value`, `export_depth_is_relative_to_the_selected_subtree` |
| Navigation, search, paths, collapse/expand | `decoded_navigation_search_and_paths_match_json_with_escaped_unicode` |
| Format options, disabled builds, input limits, byte-preserving pipelines | `tests/toon_cli.rs` |
| TOON writes and prints, overwrite refusal, replacement, encoding/open failures | `tests/toon_cli.rs::terminal_commands` |

Tests use isolated pseudoterminals and disposable files. They do not write to the
user's clipboard or controlling terminal. Rerun terminal-setup permission failures
with terminal access; do not count them as passing or skipped tests.

## Manual release checks

These checks remain required before publication. Automated results do not establish
that the desktop clipboard, help pager, or Linux full-device checks passed.
Record the actual host and results in the release PR.

1. Copy a nested value using `yt`, paste it into a text editor, and compare it
   with `pt`. Repeat on its closing row in Line mode and after collapsing it.
   Verify `yy`, string/key copy, and path copy with escaped and non-ASCII text.
2. Open `0.123456789012345678901` as JSON and use `yt`. Confirm that it copies
   the published encoder's rounded value, matching `pt`. Numeric precision loss
   is accepted; the former `UnsupportedNumber` expectation is removed.
3. Open in-app help in default and `--no-default-features` builds. Only the
   default build may list TOON commands. In the disabled build, `:wt`, `yt`, and
   `pt` must remain unavailable/unbound. Check the documented TOON 3.0 profile
   and 4.x limitation.
4. On Linux, use a disposable session to write to `/dev/full` with `:wt!`.
   Confirm a write error, no success message, and continued navigation.
   The shared writer propagates write and flush errors. Standard `File::flush`
   has no buffer to flush; no separate failing-file-flush test is claimed.
5. Open malformed TOON interactively. Confirm a useful parse diagnostic and
   normal terminal restoration. Non-terminal TOON pass-through must preserve
   malformed input bytes without entering the decoder.

## Release checks

Follow [the release checklist](release_checklist.md) for native packaging and
supported-host checks. The inherited block 0.1.6 on macOS and xcb 0.8.2 on Linux
emit Cargo future-compatibility notices on the development compiler. These are
separate from current compiler and Clippy failures.
