## ADDED Requirements

### Requirement: Dual path syntax

The CLI SHALL accept `--path <path>` in every build. An empty argument or `.` SHALL select the full input. A leading `/` SHALL select RFC 6901 string-pointer syntax; a leading `./` SHALL select the same syntax after removing only the initial dot. Otherwise a leading `.` or concrete bracket selector SHALL introduce friendly syntax. Friendly syntax SHALL accept nonempty dot tokens and the concrete selectors emitted by `yp`: `[N]` for a zero-based array index and `["key"]` for a JSON-escaped string key. These selectors SHALL be allowed at the beginning and after tokens/selectors. Bare dot tokens SHALL contain no slash, backslash, square bracket, or whitespace and SHALL be literal without tilde decoding. Quoted keys SHALL use JSON string decoding and preserve embedded dots, brackets, slashes, whitespace, Unicode, and escaped characters. Unsupported syntax SHALL be rejected without retrying another mode. Strict pointers SHALL support empty tokens, literal dots, whitespace, Unicode, `~0`, and `~1`; reject other tilde escapes; and perform neither percent decoding nor Unicode normalization. URI fragments, whole-path JSON-string unquoting, wildcards, empty brackets, slices, recursive descent, arbitrary bracket expressions, and relative paths SHALL NOT be supported.

The `:` prompt SHALL accept dot-led forms and leading concrete bracket selectors so `yp` output can be pasted unchanged. After optional leading ASCII spaces, the path command SHALL consume the entire remaining buffer without trimming or shell-style splitting. `.` SHALL reset the filter, while `./` SHALL select an empty-string member. Command-name completion SHALL NOT rewrite or suggest completions for path input. Normal-mode bracket navigation and `/` and `?` search prompts SHALL retain their meanings.

#### Scenario: Equivalent entry points

- **WHEN** a user selects `.hits.hits.0`, `./hits/hits/0`, or CLI-only `/hits/hits/0`
- **THEN** each SHALL resolve the same first array element under `hits.hits`
- **AND** the interactive forms SHALL be submitted by typing `:` followed by the dot-led path

#### Scenario: Literal special keys

- **WHEN** strict paths `./a.b`, `./a~1b`, `./m~0n`, `./~01`, or `./ ` are submitted
- **THEN** their single tokens SHALL respectively identify `a.b`, `a/b`, `m~n`, `~1`, and a single-space key
- **AND** `.a.b` SHALL instead traverse `a` then `b`

#### Scenario: Root and empty key differ

- **WHEN** input is `{"":7,"a":1}`
- **THEN** `.` and CLI `--path ''` SHALL select the complete object
- **AND** `./` and CLI `--path /` SHALL select `7`

#### Scenario: Reject ambiguous shorthand

- **WHEN** `.a..b`, `.a.`, `.a/b`, `.a[]`, `.a[0:2]`, `./a~2b`, or `#/a` is supplied as a path
- **THEN** parsing SHALL reject the path and SHALL NOT reinterpret it in another syntax

### Requirement: Concrete copied paths round-trip

The `yp` representation SHALL be accepted unchanged in the `:` prompt and as the value of `--path`; users SHALL NOT need to add a dot, translate array indices, or rewrite quoted keys. For an unambiguous path made of string-key and array-index access, copying a focused node and evaluating that path against its original document SHALL select the same node, including when copied from a filtered view. The matching printed representation SHALL have the same semantics. Original-root selection SHALL be represented by `.`. Array bracket selectors SHALL require arrays; quoted-key selectors SHALL require objects and SHALL NOT coerce key types. Existing duplicate-key, non-string-key, and all-document resolution rules SHALL still apply; copied paths SHALL NOT bypass errors, identify a document by a synthetic index, or select a different occurrence.

The filter compatibility guarantee SHALL apply to `yp`, not the separate `yb` representation. `yb`/`pb` SHALL retain their existing external-use behavior. Syntax shared with `yp` SHALL NOT be rejected merely because `yb` can also emit it. `yq`/`pq` SHALL retain their existing jq-query behavior and output, including `[]` array traversal; this feature SHALL NOT convert those queries to concrete paths or add jq evaluation to filters.

#### Scenario: Copy a nested array member

- **WHEN** `yp` copies `.hits[0].name` from a focused value and that text is pasted into `:` or passed as one `--path` argument against the same input
- **THEN** the filter SHALL select that same value in its original document without rewriting the path
- **AND** `.hits.0.name` SHALL remain an accepted shorthand

#### Scenario: Copy paths with leading brackets and special keys

- **WHEN** `yp` emits `["a.b"][0].name`, `[0].name`, or a path containing JSON-escaped quotes, backslashes, brackets, whitespace, or Unicode in string keys
- **THEN** both filter entry points SHALL accept the emitted text unchanged and resolve its decoded keys and concrete indices
- **AND** brackets inside quoted keys SHALL NOT be parsed as selectors

#### Scenario: Copy from a filtered view

- **WHEN** `.hits` is active and `yp` copies a descendant's path
- **THEN** the path SHALL retain its original `hits` ancestry
- **AND** pasting it SHALL resolve from the original root rather than under the active filter

#### Scenario: Retain jq compatibility

- **WHEN** a user copies or prints a jq query for a value beneath an array using `yq` or `pq`
- **THEN** its existing `[]` traversal representation SHALL remain unchanged
- **AND** a filter containing that empty-bracket traversal SHALL remain unsupported

### Requirement: Resolution against original documents

Paths SHALL resolve against each original parsed document root, independently of current focus or filtering. Object tokens SHALL match decoded string keys exactly. A duplicate matching key SHALL cause an ambiguity error; unrelated duplicate keys SHALL NOT prevent selection. Numeric object keys represented as strings SHALL remain string keys. Non-string YAML keys SHALL NOT be coerced to strings. Array tokens SHALL be zero-based ASCII decimal indices with no leading zero except `0`; negative, signed, out-of-range, overflowing, and `-` indices SHALL fail. Traversal through a scalar SHALL fail. Any value, including null, scalars, empty containers, table rows, and inline elements, SHALL be selectable.

In friendly syntax, `[N]` SHALL require an array with the same checked index rules; `["key"]` SHALL require an object and exact decoded string-key matching. Numeric plain dot tokens SHALL retain container-dependent lookup. Non-string YAML bracket expressions SHALL NOT be treated as string-key selectors.

A nonempty path SHALL resolve in every document before selection is committed. Zero documents with a nonempty path SHALL fail. Empty/root selection SHALL preserve existing zero-document behavior. Multiple selected values SHALL remain separate roots in original document order; no synthetic data array SHALL be introduced.

#### Scenario: Resolve from the original root

- **WHEN** `.hits` is active and the user submits `.aggregations`
- **THEN** resolution SHALL start from each original root and select its `aggregations` member, not `hits.aggregations`

#### Scenario: Array versus object tokens

- **WHEN** `/01` is resolved against an object containing string key `01`, or against an array
- **THEN** the object lookup SHALL succeed and the array lookup SHALL fail
- **AND** `/0` SHALL identify the first array element when one exists

#### Scenario: Ambiguous and typed keys

- **WHEN** a path traverses duplicate decoded string keys
- **THEN** resolution SHALL fail rather than choose the first or last occurrence
- **AND** a YAML numeric key `1` SHALL NOT match token `1`, while a unique string key `"1"` SHALL match it

#### Scenario: All documents participate

- **WHEN** `.a` is applied to the JSON stream `{"a":1} {"a":2}`
- **THEN** selected roots SHALL be `1` and `2`, in that order
- **AND** applying `.a` to `{"a":1} {"b":2}` SHALL fail without selecting only the first document

### Requirement: Atomic filter changes and diagnostics

An interactive path failure SHALL show a diagnostic and preserve the active filter, focus, viewport, search state, and collapse state. Cancelling path input SHALL have no filter side effects. A successful change SHALL focus the first selected root, reset vertical and horizontal scroll to its start, clear active search matches, and preserve per-node collapse and explicit multiline-array state. Selected roots SHALL be exposed expanded initially; remembered descendant states SHALL be retained. Resetting with `.` SHALL use the same transition rules and SHALL restore all original roots, not the previous cursor location.

CLI path syntax errors and a missing option argument SHALL exit with status 2. Resolution failures SHALL exit with status 1. Both SHALL report on stderr, produce no stdout payload, and occur before terminal raw/alternate-screen initialization. Diagnostics SHALL distinguish invalid syntax, missing member, ambiguous member, invalid array index, and scalar traversal, identify the failed token where applicable, and identify the one-based document number for stream resolution failures. Diagnostics SHALL safely escape control characters from path text.

#### Scenario: Failed change preserves the current view

- **WHEN** a user viewing `.hits` with a selected descendant and an active search submits a missing path or cancels the prompt
- **THEN** the current scope, selected node, scrolling, search, and collapse states SHALL remain unchanged

#### Scenario: Successful change and reset

- **WHEN** a user applies a valid filter and later submits `.`
- **THEN** each successful transition SHALL select its first root, clear active search matches, reset scroll offsets, and expose that root
- **AND** descendant collapse and multiline-array choices SHALL survive the transitions

#### Scenario: Startup failure is not terminal output

- **WHEN** CLI path resolution fails in document 2
- **THEN** tless SHALL exit 1 with an actionable stderr diagnostic identifying document 2
- **AND** it SHALL emit neither a data payload nor terminal initialization sequences

### Requirement: Root-relative presentation with original identity

The selected value in each document SHALL render as a standalone root at baseline indentation, without its owning key, ancestors, siblings, or warnings belonging only to excluded data. Root arrays SHALL use root-array presentation rather than their former parent's table or inline layout. A scalar or empty value SHALL remain focusable; an empty object SHALL retain the existing blank-row root convention. Multiple selected roots SHALL use existing sequence presentation with original document order and numbering. Filtering SHALL NOT mutate parsed ancestry, keys, source ranges, or node identity. Status paths and existing path-copy commands SHALL continue to describe the focused node in its original document, including existing sequence and duplicate-occurrence indicators where applicable.

#### Scenario: Nested object becomes the view root

- **WHEN** `.hits` selects `{"hits":{"total":2},"other":3}`
- **THEN** the viewport SHALL show `total: 2` at root indentation without `hits` or `other` headings
- **AND** focusing `total` SHALL retain the full original `hits.total` path

#### Scenario: Scalar and empty roots

- **WHEN** the selected value is null, a scalar, an empty array, or an empty object
- **THEN** it SHALL remain selectable with its original full path and value-copy target
- **AND** excluded ancestors SHALL NOT be inserted to make it selectable

### Requirement: Interactive export scope

All interactive whole-document write commands, including JSON, TOON, feature-gated s-expressions, aliases, and overwrite variants, SHALL serialize the current selected roots as standalone values rather than the original full input or only the focused value. Collapse, wrapping, and focus SHALL NOT change export content. Existing format framing, normalization, limitations, and overwrite safeguards SHALL remain in force. Multiple selected roots SHALL still fail standard TOON whole-document export. Serialization SHALL complete before opening or truncating the destination file. Focused-value and key copy commands SHALL retain their original targets; existing path-copy commands SHALL remain absolute within the original document.

#### Scenario: Export scope differs from focus

- **WHEN** `.hits` is active, focus is on a nested scalar, and `:write result.json` is submitted
- **THEN** the file SHALL contain the entire selected `hits` value without a `hits` wrapper or unrelated data
- **AND** copying the focused value SHALL still copy only that scalar

#### Scenario: Failed filtered export preserves files

- **WHEN** multiple filtered roots are active and `:writetoon! existing.toon` is submitted
- **THEN** standard TOON export SHALL fail and the existing file SHALL remain unchanged

### Requirement: Discoverable path filtering

CLI help and interactive help SHALL explain friendly-dot versus strict-pointer syntax, direct `yp` paste support including numeric/quoted brackets and leading brackets, zero-based array tokens, literal special-key escaping, root reset versus empty-key selection, absolute resolution, all-document atomicity, output scope, and whole-input parsing. Help SHALL distinguish the `yp` round-trip promise from unchanged `yq` jq queries and the separate `yb` representation. Help SHALL state that filtering is not a parser shortcut or a security boundary and that path completion and jq/JSONPath evaluation are unsupported.

#### Scenario: Discovering both forms

- **WHEN** a user reads CLI and interactive help
- **THEN** examples SHALL include `--path '.hits[0].name'`, `--path .hits.hits.0`, `--path /hits/hits/0`, `:["a.b"][0]`, `:./a.b`, and `:.`
- **AND** documentation SHALL warn that filtered writes and pipelines export only selected roots
