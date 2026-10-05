---
type: Guide
title: TOON acceptance checks
description: Automated coverage and manual release acceptance checks for TOON input, output, and document rows.
status: draft
generated: { by: openai-codex/gpt-6-sol, at: 2026-10-05T03:48:34Z }
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

Pipeline argument-error cases supply no stdin payload: validation can exit before
reading input. Resolution-error cases supply documents and assert status 1 rather
than the status 2 used for invalid arguments.

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
   must list `:write`/`:w`, `:write-toon`/`:wt`, `:write-json`/`:wj`,
   `:write-yaml`/`:wy`, and `:write-ndjson`/`:write-jsonl`/`:wn` with `!`
   variants. Confirm default writes TOON and only `sexp` builds accept
   `:write-sexp`/`:ws` (including `!`). The old `:writetoon` and
   `:writesexp` names must not work. Check TOON 3.0 and 4.x limitations.
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
   Search inside a collapsed sequence document and confirm that its document
   row expands to reveal the match. Source strings containing `# WARN` must
   remain ordinary searchable data; generated headers, positions, warning
   comments, counts, and previews must not create search matches.
5. Open duplicate JSON keys, a precise decimal, `1e1000000`, and YAML with
   non-finite numbers and non-string keys. Confirm each warning and preserved
   parsed value. Open equivalent multi-root NDJSON, JSONL, concatenated JSON,
   and YAML stream inputs. Confirm each has ordered, selectable root-owned
   rows with expanded headers `--- (i of n)`, no multiple-roots warning, and a
   subdued contents preview only after collapse. Confirm the document fields
   start in the same content column as `---`, each row collapses independently,
   `[` from a top-level field selects its row, and `]` selects the next row.
   Vertical motion includes visible rows, absolute jumps select document rows,
   and selecting a row for copy or print returns its parsed root without
   generated metadata. Confirm single-root presentation and standard export
   restrictions remain unchanged.
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
9. With disposable destinations, use `:write report` (no inferred suffix),
   `:write-json report.toon` (JSON despite the suffix),
   `:write-yaml! report.yaml`, `:write-ndjson report.ndjson`, and
   `:write-jsonl report.jsonl` on filtered roots. Compare each destination's
   bytes: JSON has pretty roots plus final LF without an array wrapper,
   YAML has one `---` and final LF per root, and NDJSON/JSONL agree
   byte-for-byte with one compact root and LF per line. An array root stays
   a single array record. Confirm `:write report.toon` rejects multiple active roots
   and encoding failures leave existing files unchanged and missing files
   absent; without `!`, existing files are refused. A file-operation error
   after opening may leave partial output; no atomic replacement is promised.

## Aligned-table acceptance

Run in an isolated PTY at 120 and 30 columns, repeating with absolute,
relative, both, and no number gutters. Record actual frames, terminal-cell
column starts, selected paths, and horizontal transitions against the
implementation commit; also resize to 16 columns and change indentation with
`<`/`>`. Use this JSON as a starting point, with the last row placed below the
viewport when testing width measurement:

```json
{"users":[{"id":1,"name":"Ada"},{"id":200,"name":"Lin"}],"other":[{"id":7,"name":"X"},{"id":8,"name":"Y"}],"nested":{"rows":[{"id":1,"name":"é界👩‍💻"},{"id":22,"name":"line\nbreak"}]},"plain":"outside"}
```

1. Press Tab on `users` and check `users[2]{id ,name}:`, `         1  ,Ada`,
   `         200,Lin` without gutters. Check the first value and header start
   in the same terminal column and that pressing `3` then Tab toggles only once.
   Move among header, row, and cell: status keeps `Table aligned`; at a narrow
   width it shows `Align`. Leave for `plain` and return: alignment persists
   but its status label appears only while focus belongs to that table.
2. Enable the second and nested table independently. Compare widths with a
   widest escaped or wide/combining/emoji cell off-screen. Confirm no padding
   within quotes or graphemes and no table conversion for lists or primitive
   arrays. Collapse a table and toggle Tab: its preview stays ordinary and
   collapsed; reopening restores the grid. Collapse an ancestor, then reopen.
   Filter to a row or cell using `:` and confirm no inherited grid; reset with
   `:.` and confirm restored alignment and original paths.
3. At 30 columns, use `2.`, `,`, and `;` on a header, short row, and cell.
   Confirm header and every row move together, including newly exposed rows;
   a long warning remains reachable even from a short row. Confirm end/start
   bounds, saturated counts, focus and unrelated-line isolation. Search for a
   clipped value and then a field key: both become visible under the same
   horizontal transform while keeping their row-field identity. Click a token
   after scrolling and check `pP` and `pp`; click generated row and header
   padding and check their row/table owner. Confirm line numbers do not change.
4. Enable Ctrl+L while aligned: only other expanded lines wrap, with blank
   continuation gutters. Check the aligned table still scrolls horizontally;
   Ctrl+L again retains its offset. Turn off alignment while wrapping is on
   and check that the table follows the current wrapping policy. Resize,
   toggle gutters and indentation, and confirm column starts, focus, and
   bounds remain valid without painting over the status or command row.
5. Copy an aligned cell with `yt` (and inspect `pt`), then `:write` the table
   as TOON and `:write-json` it as JSON using a disposable filtered table
   root. Compare parsed content against unaligned output; no grid padding,
   status label, or warning comment is serialized. Compare redirected stdout
   with alignment disabled; opening an aligned terminal session must not
   modify the command-line output contract. Check document Tab does nothing
   on a scalar, primitive array, or list; command Tab still completes, and
   search-prompt Tab retains its previous input behavior. Verify both in-app
   help screens describe the controls and the local wrapping override.

## Startup and presentation resource checks

Ordinary tests cover late table disqualifiers, hidden warning totals, distant
escaped table-key and filtered inline-element matches, and bounded presentation
across substantially different tails and repeated distant navigation. They
assert row-materialization and retained span/map bounds, not wall-clock limits.
Large YAML cases also exercise distant typed-key/decoded-string lookup, selected
JSON export, and filtering across storage chunks. Collapse/reopen coverage
includes repeated operations through paired delimiters and exact logical
addresses before and after restoring an expanded projection.
Filtered first fields retain their own selection identity; out-of-order selected
roots preserve exact addresses across reflow and collapse. Repeated and shortened
key shapes still recheck changed keys, escaped aliases, duplicate extensions,
and scalar warnings.

The native release comparison is opt-in. Build jless 0.9.0 and tless at
`9bb9568` from identifiable source revisions in separate output directories.
Use the same compiler and default features, and record any reference build
adjustments. Set these environment variables before running:

| Variable | Value |
| --- | --- |
| `JLESS_REFERENCE` | Absolute path to the jless 0.9.0 release executable |
| `TLESS_REFERENCE` | Absolute path to the frozen `9bb9568` release executable |
| `TLESS_BUILD_PROVENANCE` | Compiler, host, revisions, features, and build commands for all three executables |

```sh
cargo build --release --locked
TLESS_CANDIDATE="$PWD/target/release/tless" \
TLESS_PERFORMANCE_REPORT=/tmp/tless-first-frame.json \
cargo test --release --locked --test performance -- --ignored --nocapture
```

The benchmark is ignored during ordinary test execution, but still compiles and
must pass the Linux/macOS build and Clippy checks applied to test targets.

The harness generates the 10,238,903-byte, 150,000-record synthetic input,
isolates user configuration, and opens each executable in a 140×40 PTY with
numbers enabled and wrapping disabled. It excludes one warm-up, rotates
executable order across 20 launches, responds to cursor-position queries, and
requires document content plus filename/status on the final terminal row
before timing successful startup. Every launch must quit successfully.

The JSON report retains warm-ups, individual observations, executable hashes,
median, and nearest-rank p95 (the nineteenth sorted observation). Acceptance
requires both candidate statistics to be at most three times jless and the
candidate median to be at most one fifth of frozen tless. Run without concurrent
builds or profiling. These thresholds are comparison gates, not a latency
promise for every document or a timing assertion for shared CI.

Memory is measured in a separate launch with `ps` RSS samples at least 5 ms
apart, stopping at useful-frame detection. Report the sampled pre-frame peak
in KiB; it is neither an exact high-water mark nor whole-session memory.
The memory launch samples before its first PTY read, so a complete initial frame
delivered in one read still has an RSS observation.
Record navigation, resize, wrapping, and search timings separately as diagnostics.

For direct parsed-node traversal, also run a separate 20-round private-input
comparison against both explicit jless modes: `--mode data` and `--mode line`.
Use the same host, release compiler, default features, isolated configuration,
140×40 terminal, warm-up, rotating launch order, useful-frame criterion, and
separate RSS sampling described above. Against **each** mode, the candidate must
have median and nearest-rank p95 startup time at most **1.5×** the reference and
sampled startup peak RSS at most **1.2×** the reference. This is additional to,
not a replacement for, the synthetic gate.
The committed `tests/performance.rs` enforces the synthetic gates only; this
private-input comparison is a separate local acceptance procedure, not an
assertion made by that test or ordinary CI.

Exercise broad objects, deep nesting, table-heavy arrays, and width-sensitive
Unicode arrays through the actual terminal. Check first/last absolute-line jumps,
collapse/reopen, wrapping, narrow/wide resize, search, filtering, successful exit,
and terminal restoration. Compare rendered output with the preceding version
where the display contract is unchanged. Keep private contents, source paths,
identifiers, and screen captures out of committed evidence.

## Release checks

Follow [the release checklist](release-checklist.md) for native packaging and
supported-host checks. The arboard migration removes the former block 0.1.6 and
xcb 0.8.2 dependencies and their future-compatibility notices.
