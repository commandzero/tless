---
type: Guide
title: TOON acceptance checks
description: Automated coverage and manual release acceptance checks.
status: draft
generated: { by: codex/gpt-6, at: 2026-09-11T05:42:30Z }
---

# TOON acceptance checks

Use [published codec behavior](toon-codec.md) for the conversion contract.
Use [the document view](toon-view.md) for the separate display contract.
The former vendored codec's exact-number, duplicate-key rejection, key-order,
and linear scanner-work guarantees no longer apply.

## Automated checks

Run `scripts/preflight.sh` with the pinned development compiler, then
`TLESS_TOOLCHAIN=1.87.0 scripts/preflight.sh test` for the minimum compiler.
Both exercise minimal and default builds, with TOON always enabled and optional
S-expression and colorscheme features.
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
| Format options, feature profiles, input limits, parsed pipelines | `tests/toon_cli.rs` |
| Output format matrix, framing, typed YAML, JSON compatibility, parse/encode failures | `tests/piped_output.rs` |
| TOON writes and prints, overwrite refusal, replacement, encoding/open failures | `tests/toon_cli.rs::terminal_commands` |

Tests use isolated pseudoterminals and disposable files. They do not write to the
user's clipboard or controlling terminal. Rerun terminal-setup permission failures
with terminal access; do not count them as passing or skipped tests.

## Manual release checks

These checks remain required before publication. Automated results do not establish
that the desktop clipboard, help pager, or Linux full-device checks passed.
Record the actual host and results in the release PR.

1. Copy a nested value using `yt`, paste it into a text editor, and compare it
   with `pt`. Repeat after collapsing it and with an individual table cell.
   Verify `yy`, string/key copy, and path copy with escaped and non-ASCII text.
2. Open `0.123456789012345678901` as JSON and use `yt`. Confirm that it copies
   the published encoder's rounded value, matching `pt`. Numeric precision loss
   is accepted; the former `UnsupportedNumber` expectation is removed.
3. Open in-app help in default and `--no-default-features` builds. Both builds
   must list the TOON commands because TOON support is always enabled. Check the
   documented TOON 3.0 profile and 4.x limitation.
4. On Linux, use a disposable session to write to `/dev/full` with `:wt!`.
   Confirm a write error, no success message, and continued navigation.
   The shared writer propagates write and flush errors. Standard `File::flush`
   has no buffer to flush; no separate failing-file-flush test is claimed.
5. Open malformed TOON interactively. Confirm a useful parse diagnostic and
   normal terminal restoration. Non-terminal TOON output must reject malformed
   input with status 1, a stderr diagnostic, and an empty stdout payload.

## Document-view acceptance

Run these checks at 120 columns, then at 30 columns, in an isolated terminal.
Record results against the implementation commit. Clipboard and host-specific
release checks above remain separate.

1. Open equivalent JSON, YAML, and TOON containing a primitive array and a
   uniform object table. Confirm identical expanded text and distinct key,
   string, number, boolean, and null styles. Check a minimal build with JSON.
2. Navigate into an inline element and a table cell. Move across siblings and
   vertically between table rows. Confirm paths, focused print output, parent
   motion, and selection after resize. Check absolute and relative gutters.
3. Collapse a table row, an inline array, and a containing object. Confirm
   subdued previews, immediate object counts, retained array counts, hidden
   warning counts, and restoration of descendant collapse states.
4. Search for a hidden table value and a table field key. Confirm ancestor
   expansion, row identity, header highlighting, and horizontal visibility.
   Source strings containing `# WARN` must remain ordinary searchable data;
   generated warning comments must not create search matches.
5. Open duplicate JSON keys, a precise decimal, `1e1000000`, and YAML with
   non-finite numbers and non-string keys. Confirm each warning and preserved
   parsed value. Open multiple roots and check that vertical motion skips
   separators while absolute jumps select the following root.
6. Scroll long values and warnings horizontally. Include wide and combining
   Unicode and escaped controls. Confirm clipping does not split terminal cells
   and narrowing the window does not change a table to a list.
7. At 30 columns, confirm expanded long values and table rows start unwrapped.
   Press Ctrl+L and confirm expanded rows wrap at terminal-cell boundaries,
   first rows keep their numbers and arrows, continuation rows have blank
   gutters, and collapsed previews stay on one row. Use up/down to move
   by logical entries. Use scrolling, paging, and wheel input to read every row
   of a value taller than the viewport while keeping it selected. Check
   `zz`/`zt`/`zb` positioning. Confirm `,`,
   `.`, and `;` are inert on wrapped expanded lines, work again after toggling
   wrapping off, and that resize, gutter, and indentation changes preserve the
   selected value after reflow.
8. Confirm `--mode` and `-m` fail with argument errors, `m` does not switch
   modes, and help has no closing-delimiter controls. Confirm JSON and standard
   TOON exports contain no generated annotations and keep codec behavior.

## Release checks

Follow [the release checklist](release-checklist.md) for native packaging and
supported-host checks. The arboard migration removes the former block 0.1.6 and
xcb 0.8.2 dependencies and their future-compatibility notices.
