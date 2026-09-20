## Context

Issue: https://github.com/commandzero/tless/issues/10

The current `FlatJson` model retains parsed rows, parent/sibling links, source ranges, and duplicate occurrences. `Layout::build` discovers original roots and builds TOON presentation; `JsonViewer` owns logical focus and viewport projection. `App::get_content_target_data` uses original nodes for value and path copies. Interactive file writes currently export the complete model, while `output::serialize` parses and exports the complete input independently of the viewer.

Filtering therefore cannot be implemented by hiding painted lines alone: navigation, search, root rendering, warnings, numbering, and serialization all need the same selection. Replacing the parsed document with a copied subtree would lose the original ancestry needed by the status bar.

## Goals / Non-Goals

**Goals:**

- Easy dot paths, direct reuse of the `yp` representation, and an unambiguous strict JSON Pointer escape hatch.
- One shared resolver and one ordered selected-root representation across CLI, TUI, and export.
- Root-relative presentation with original node identity, reversible filtering, and atomic errors.
- Apply a path to all documents without silently discarding unmatched documents.
- Preserve existing behavior when selection is the original roots.

**Non-Goals:**

- jq/JSONPath evaluation, wildcards, recursive descent, relative navigation expressions, general bracket-expression support, or path completion.
- Selecting a document by a synthetic array index or inventing duplicate-key occurrence syntax.
- Incremental parsing, reduced input limits, data redaction, codec upgrades, or a new export format.
- A round-trip promise for `yb`, or changing `yq`/`pq` jq-query output. Syntax shared with `yp` can be accepted without making every bracketed representation a supported input format.

## Decisions

### 1. Classify syntax once; never fall back on lookup failure

| Input | Interpretation |
| --- | --- |
| CLI empty argument, or `.` | Original roots |
| `.hits.hits.0` | Friendly tokens `hits`, `hits`, `0` |
| `.hits[0].name` | Concrete `yp` path with an array index |
| `["a.b"][0].name` | `yp` path beginning with a quoted special key |
| `[0].name` | `yp` path into a root array |
| `./hits/hits/0` | Strip the initial dot, then evaluate RFC 6901 `/hits/hits/0` |
| CLI `/hits/hits/0` | RFC 6901 string pointer |
| `./a.b` | Literal dotted key `a.b` |
| `./` | Empty-string member, not reset |

Recognize `./` before general friendly syntax. Retain nonempty dot tokens without slash, backslash, brackets, or whitespace, and extend the friendly grammar with the concrete selectors used by `yp`: `[N]` for an array index and `["key"]` for a JSON-escaped string key. Selectors can occur initially or after another selector/token. Decode quoted strings with JSON rules; never split dots, brackets, or whitespace inside them. A quoted numeric key requires an object; `[0]` requires an array. Numeric dot tokens remain container-dependent. Empty brackets, slices, expressions, negative indices, and general jq evaluation are not accepted. Only strict mode decodes tilde escapes. Strict paths follow [RFC 6901 sections 3–4](https://www.rfc-editor.org/rfc/rfc6901#section-3), not its URI-fragment representation.

The command parser recognizes dot-led input and leading concrete bracket selectors after optional leading ASCII spaces, passing the entire suffix unchanged to the resolver. Leading brackets are necessary because `yp` already emits them for root arrays and special root keys; accepting them in the `:` editor does not change normal-mode bracket navigation. Do not trim trailing whitespace or space-tokenize paths. Bare slash remains search syntax in normal viewing, not an interactive path command. The CLI accepts slash because it has no such ambiguity. Path input receives no command-name completion or hints.

The round-trip promise belongs to `yp` (and its matching printed representation), not `yb`. Test the real path formatter's output through both filter entry points, rather than testing only handwritten examples. Preserve concrete indices and original-document ancestry after filtering. Root-path copy can use `.`. For unambiguous string-key/index paths, the copied path must resolve to the same node in its source document; in streams it still resolves in every document under the existing atomic policy. Duplicate-key ambiguity, non-string YAML keys, and missing paths in other documents remain explicit resolution errors, not a license to select a different value. No new duplicate/document addressing language is introduced.

Retain `yq`/`pq` behavior unchanged, including jq array traversal with `[]`; do not replace those outputs with concrete indices or route them through the friendly formatter. Preserve `yb`/`pb` output for external consumers without promising filter compatibility. Shared syntax need not be artificially rejected just because `yb` can also emit it.

Alternative rejected: dot replacing the first pointer slash. It prevents the requested `.path.with.dots` form from behaving intuitively. Also rejected: trying both grammars until a key exists, because meaning would then vary with data.

### 2. Resolve into original node identities

Introduce a small path module whose interface parses a path and resolves it against the existing model's original roots. Return an ordered list of selected node indices or a structured error containing the failed token and document ordinal. Reuse existing decoded YAML key values and JSON string decoding rather than stringify-and-reparse or build a second tree. Keep traversal iterative; follow direct-child/sibling links, skipping container bodies when looking for peers. Check an array index without unchecked integer conversion.

Strict tokens and plain dot tokens use container-dependent lookup. Friendly bracket selectors retain their type: numeric brackets require arrays and quoted brackets require string-keyed objects. Duplicate matching decoded keys fail, as RFC 6901 specifies; choosing last-wins here would disagree with the viewer's preserved duplicate identities. Non-string YAML keys are never coerced. Unsupported values beneath a selected root remain subject to the renderer/exporter's existing behavior.

Resolve all documents before returning a candidate selection. An empty/root path restores original roots, including the empty sequence. A nonempty path against zero roots fails. This avoids document-count-dependent syntax and silent record loss.

### 3. Separate original roots from active view roots

Retain `FlatJson` unchanged. Store the selected roots and their source-document identities in viewer state. Centralize membership and effective-parent/root handling so navigation and search do not each invent a filter check. A selected root is parentless only for interaction and layout; status and copied paths still traverse original parents.

Build canonical and visible layouts from active roots, rendering their values at baseline depth without the owning key. Recompute root-sensitive table/inline-array presentation rather than slice lines out of an old layout. Only selected values generate visible warnings, counts, previews, or search spans. Preserve original indices in layout span ownership and node lookup; excluded nodes must have no usable layout anchor. Existing indexed caches may remain document-sized, but do not construct excluded display lines or clone document subtrees.

One selected value uses single-root presentation. Multiple values use the existing document-header presentation, carrying original document numbering and paths. Absolute line addresses restart at one in the expanded filtered layout; physical wrapping remains a projection of those logical lines.

Alternative rejected: cloning selected values into a replacement `FlatJson`. That duplicates input data, breaks paths and duplicate identities, and requires an avoidable reverse mapping.

### 4. Commit filter changes as one state transition

Prepare resolution and the new projection before committing selection. Failure or prompt cancellation leaves view and search state untouched. Success focuses the first selected root, resets horizontal and vertical offsets, and clears active search matches rather than retaining stale source-to-display mappings. Preserve node-keyed descendant collapse and multiline choices, while exposing selected roots expanded. Reset uses the same transition, not a separate history mechanism.

All motion paths, sibling fallbacks, counted motions, mouse selection, jumps, and search reveal use active membership. A selected sequence root may move to another selected root, but never to an excluded original ancestor/sibling. Searches include collapsed descendants but not the omitted owning key of a selected root. Scope search enumeration, not only painting, to keep counts and repeat-search correct.

### 5. Make selected roots the export seam

Pass the same ordered selected-root identities to existing serializers and TOON value/document adapters. Serialize selected values without parent keys and at baseline depth. Retain JSON source number spelling, duplicate entries, and ordering where currently promised; YAML retains its existing framing and type support. Keep standard TOON's one-root and conversion restrictions, applying them to selected data only. In particular, filter before the existing YAML-to-JSON TOON normalization so excluded non-string keys do not spuriously fail an otherwise exportable selection. Reuse required codec conversions; do not introduce a second conversion just for filtering.

Interactive JSON, TOON, and feature-gated s-expression file commands export selected roots, independent of focus/collapse. Focused-value/key copy remains unchanged; concrete path copies retain original-document ancestry and support the `yp` round-trip contract. The separate `yq` and `yb` representations retain their existing functions. Serialize before creating or truncating a destination; preserve overwrite guards. Pipeline selection likewise resolves every root and serializes before any stdout write. Existing write/flush errors may still leave a partial payload.

Alternative rejected: view-only filtering. The chosen contract explicitly scopes exports too; keeping whole-input writes would surprise users looking at a subtree.

### 6. Validate startup selection before terminal initialization

Parse CLI path syntax as argument validation (exit 2). Load and parse the entire input under existing limits, then resolve the selection (exit 1 on lookup failure). Share parsed input with viewer initialization rather than parse it twice. Move the needed preparation ahead of raw-mode/alternate-screen setup. Machine mode must remain independent of a controlling terminal. Render diagnostic tokens with safe escaping; distinguish syntax, lookup, duplicate ambiguity, index, and scalar-traversal errors.

Filtering is not lazy parsing: malformed excluded input still fails, and `--max-input-bytes` applies to the complete input. This prevents users from interpreting the feature as a resource-limit bypass.

## Risks / Trade-offs

- Navigation has many paths to focus changes → centralize membership/effective-parent logic and exercise parent/sibling fallbacks, counted jumps, search, and mouse transitions.
- Table and inline layouts share lines → rebuild selected-root layouts with original span owners; verify row, cell, scalar, and empty-root selections on the actual TUI.
- Existing serializers assume depth-zero original roots → make selected-root traversal explicit without changing unfiltered formatting or normalization.
- Strict whitespace is meaningful → avoid trimming paths, document `./ `, and keep command completion away from path buffers.
- `yp` has quoted and leading-bracket forms as well as dot access → parse its actual emitted representation and prove formatter-to-filter round trips; do not assume `yp` means bracket-free text.
- Query-copy and concrete-copy share implementation today → preserve `yq`/`pq` outputs with compatibility coverage while adapting concrete path formatting where needed.
- Every-document selection is strict on heterogeneous streams → report the failing document and preserve the previous view; do not silently omit data.
- Filtered multi-document default output still fails TOON → document `-o json` or `-o yaml` for stream output rather than expand the codec contract.
- Full parsing and original-node retention consume whole-input memory → document this limit; no performance claim beyond avoiding excluded display construction.

## Migration Plan

Implement resolver and selected-root traversal first, then wire CLI/export and TUI selection using the same contract. Update help and user documentation alongside the feature. No data or configuration migration is required; omitted/empty/root filters preserve current behavior. Rollback removes the new entry points and their scoped traversal without changing saved files or configuration formats.

Before implementation merges, complete the tasks, run compatibility/preflight and real terminal checks, synchronize the three capability deltas, archive this change, and satisfy the PR-scoped completion gate. Planning artifacts remain active until implementation is complete.

## Open Questions

None blocking this proposal. `yp` is the round-trip representation; `yq` retains its jq compatibility promise and `yb` remains outside the filter compatibility guarantee. Duplicate-member rejection, all-document atomicity, and focus/search reset remain the previously specified policies.
