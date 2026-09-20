---
type: Guide
title: TOON document view
description: Path filtering, document rows, layout, wrapping, logical selection, collapse, and display extensions.
status: draft
generated: { by: openai-codex/gpt-6-astra, at: 2026-09-20T00:24:26Z }
---

# TOON document view

The interactive viewer renders JSON, YAML, and TOON through the same TOON 3.0
profile, using 2-space indentation, commas, and no key folding. Rendering works
in every build. TOON input and standard export are included in every build.

Object fields keep their parsed order. Primitive arrays with at most five
elements share a line when the entire line fits the terminal, including its
indentation, gutters, and annotations. Otherwise each element gets its own
list line. Arrays
of objects use a table only when all rows have the same nonempty, unique string
fields in the same order and all cells are primitive. Other arrays use lists.

```text
tags[2]: rust,cli
users[2]{id,name}:
  1,Ada
  2,Lin
```

Empty object fields use `key:` and empty arrays use `key[0]:`. Arrays of empty
objects use a counted list with a bare `-` for each object. Root objects have
no synthetic header. An empty root has a blank selectable row and its type in
status. In a single-root input, root objects remain expanded.

When parsing yields multiple roots, the viewer adds one selectable document row
for each root, in encounter order. JSONL, NDJSON, concatenated JSON values, and
YAML streams use the same rows regardless of filename extension. An expanded
row contains only its header and position:

```text
▾ --- (1 of 2)
  name: Ada
  active: true
▾ --- (2 of 2)
  name: Lin
  active: false
```

The arrow is in the collapse gutter. Document content starts at the same
content column as `---`, so a document row adds no TOON indentation. A collapsed
row hides its body and adds a subdued preview after the position:

```text
▸ --- (1 of 2) name: Ada; active: true
▾ --- (2 of 2)
  name: Lin
  active: false
```

The position stays visible in both states. Empty roots use `{}` or `[]` in a
collapsed preview, and scalar roots use their existing terminal-safe spelling.
The preview is bounded and remains on one physical row. A single root keeps
the existing presentation without a document row or position annotation.

Line numbers and collapse arrows use dark gray, changing to light gray on the
selected line. The default theme uses xterm color 235 as its selected-line
background, with dark gray as the fallback when 256-color support is unavailable;
themes with their own selection backgrounds retain them. Focus uses bright syntax
colors instead of bold text, and stays on the selected display line when a container
has visible descendants. Inline elements and table cells keep individual focus.

Default syntax colors use terminal palette indexes: keys cyan (6), quoted-key
delimiters and other punctuation use the terminal default, strings green (2),
numbers magenta (5), booleans blue (4, brightening to 12 when selected), and nulls gray (7).
Previews and collapsed object counts use plain dark gray (8), without the
terminal dim attribute, matching line numbers. Warnings use yellow (3). The status bar uses a
dark gray (8) background with black (0) text and a light gray (7) filename.

## Path filtering

`--path <path>` selects the roots used by both the terminal viewer and redirected
output. At the `:` command prompt, submit a dot-led or concrete bracket-led path.
The same parser and resolver serve both entry points:

| Syntax | Example | Meaning |
| --- | --- | --- |
| Friendly dot | `.hits.0.name` | Literal string keys, with decimal indices when traversing arrays |
| Concrete `yp` selectors | `.hits[0].name` | An array index followed by a string key |
| Leading brackets | `["a.b"][0].name`, `[0].name` | A special root key or root-array element |
| Strict JSON pointer | `./hits/0/name` | RFC 6901 tokens after the initial dot |

The CLI also accepts bare `/hits/0/name`. Interactive `/` and `?` remain search
prompts. Path input has no command-name completion; normal-mode brackets retain
their navigation meaning.

Friendly segments do not decode pointer escapes. Bracketed string selectors
decode JSON strings and require object string keys; numeric brackets require
arrays. For example, `["0"]` addresses a string key, not element zero. Use
`["a.b"]`, `["a/b"]`, `[""]`, or JSON escapes for keys that cannot be expressed as
simple dot segments. Empty brackets, wildcards, slices, and expressions are not
supported. Array indices must be canonical nonnegative decimal integers:
leading zeros, signs, and overflow are rejected.

Strict pointers decode `~1` as `/` and `~0` as `~`, once and in RFC order:
`./a~1b` addresses key `a/b`, `./a~0b` addresses `a~b`, and `./~01` addresses
`~1`. Other tilde escapes are errors. URI fragments and percent decoding are not
supported. Pointer whitespace is literal, including trailing spaces.

`.` restores every original root. CLI `--path ''` is equivalent; `./` is not:
it selects an empty key. Every filter is absolute against the original parsed
documents, even after filtering. Each document must resolve exactly one value.
Missing members, out-of-range indices, scalar traversal, typed-selector
mismatches, and duplicate decoded-key ambiguity reject the entire change.
Non-string YAML keys are never coerced into string matches. An empty input can
retain its empty root set but cannot resolve a nonempty path.

Selected subtrees render as roots, without their owning keys or excluded
ancestor/sibling warnings. Navigation, line addresses, mouse targets, wrapping,
search counts, and repeat-search stay inside them. Hidden descendants remain
searchable, but omitted root keys do not. Status, copy, and printed paths keep
their original ancestry. Multiple selected roots retain encounter order and
document-row navigation.

Successful changes focus the first selected value, reset scroll and search, and
expose the selected roots while preserving descendant collapse and explicit
multiline-array choices. Invalid or cancelled changes leave the current view
and search state intact.

`yp` (or printed `pP`) emits a concrete path that can be pasted unchanged into
`--path` or `:`. Root copy is `.`. Resolution still rejects ambiguity or a path
missing from another document. `yq`/`pq` remain jq queries, including `[]`
traversal; `yb`/`pb` retain their external-language representation. Neither has
the `yp` round-trip guarantee.

Filtering is not streaming, a security boundary, or partial parsing: the full
input remains in memory, and byte limits and parse/depth checks apply to excluded data.
CLI syntax
errors exit 2; input or resolution failures exit 1 before terminal setup or
machine output. Serialization restrictions apply to selected values.

## Selection and navigation

Up/down moves through visible data lines. Parent, child, and sibling motions
move through parsed values, including elements and cells sharing one line.
`J` stops at the last sibling, including with a numeric count, without moving
to the parent or wrapping.
Press `l` or Right Arrow on an inline array to show one element per line while
keeping the array selected. Press it again to select the first element. This
explicit multiline choice survives resizing and collapsing/reopening the array.
`[` selects the current node's parent; `]` selects that parent's next sibling. For `{a: …, b: {x: …}, c: …}`, from `b.x` they select `b` and `c`.
If there is no parent, `[` falls back to the current node's previous sibling.
If the parent has no next sibling, or there is no parent, `]` falls back to the
current node's next sibling. The destination keeps its collapse state. Focus stays put when
neither target exists.
In a sequence, each document row represents its parsed root. From a top-level
field, `[` selects its document row and `]` selects the next document row. Sibling
motions from a document row move between rows. Vertical motion includes each
visible row, while a collapsed document contributes only its own row.
Moving down from a table cell retains its field on the next expanded table row.
The status bar shows paths without an `input` prefix, such as `.users[1].name`,
and shows `.` at the document root. A cell's path includes its row index and key. Its
parent is the row object, whose parent is the array. Duplicate entries remain
separate selections, with an occurrence number beside the path.

Search matches are underlined and use yellow foreground (3), with bright yellow (11) for the active
match. The default theme never uses reverse video.
Search matches parsed keys and values. A hidden result expands its document and
other ancestors. Table-key matches highlight the shared header while retaining
the selected row's field identity. Generated document headers, positions,
warnings, counts, and previews add no matches.

The `▾` and `▸` arrows occupy a separate gutter. Inline primitive arrays show
`▸` by default; clicking it or pressing Space expands the array to multiline.
Tabular array rows have no collapse control and stay visible; only their array
parent can be collapsed. Collapsing retains a container
header and shows a subdued preview. Object fields use `; ` separators, such as
`name: Ada; active: true`. Objects also show their immediate-entry
count as `(N)`, such as `(7)` for seven entries. Counts, previews, and warnings
remain subdued even on the selected line. Arrays keep the count already in their
TOON header. A collapsed primitive array with at most five values keeps its
value colors when the complete inline line fits the terminal, including after
expanding it to multiline. Expanding an ancestor
restores descendant collapse states. Empty containers have no collapse arrow in
single-root content; sequence document rows still use their row arrow.
Document rows use the same arrows and collapse commands. Their positions stay
subdued and visible in both states, and only a collapsed row shows its subdued
contents preview. Collapsing a document hides its body without changing the
collapse state of another document.

Absolute line numbers are on by default; relative numbers are off. Keep using
`-n`/`-N` and `-r`/`-R` to control them. Absolute jumps and gutters address fully
expanded TOON lines, so collapse leaves gaps. Values sharing a line share its
address. Document rows have addresses and participate in vertical motion. A
collapsed document contributes only its row. Relative numbers count visible
vertical motions.

Expanded values and table rows scroll horizontally by default. Press Ctrl+L to
toggle wrapping for the current session, starting off. Wrapped rows use
terminal cells and keep a single logical line for navigation, numbering, and
selection. Continuation rows have blank gutters. Explicit scrolling, wheel
input, paging, and `zz`/`zt`/`zb` positioning use physical rows, so a value
taller than the viewport can be read without moving focus. `,`, `.`, and `;`
scroll collapsed previews and other unwrapped expanded lines; they do nothing
on a wrapped expanded line. Collapsed previews stay on one row and retain
horizontal scrolling. `,` and `.` move 10 terminal cells per press, multiplied
by a numeric prefix. Resizing, gutter changes, and indentation changes rebuild
the physical rows while keeping the selected value. Collapsed previews give up
width before count and warning annotations; their warnings remain reachable
through horizontal scrolling. Warnings on expanded wrapped lines scroll vertically.

Line/Data modes, `--mode`, `-m`, interactive `m`, and matching-closing-delimiter
actions have been removed. Use structural parent/child motions for containers.

## Display extensions

The display preserves what remains in the parsed model. It cannot recover
duplicates or precision an input parser already discarded. Data outside the
standard TOON profile receives subdued `# WARN` comments.

```text
status: queued  # WARN Duplicate key
status: done  # WARN Duplicate key
limit: .inf  # WARN Non-finite number
literal: ".inf"
precise: 0.123456789012345678901
huge: 1e1000000  # WARN Non-canonical number
control: "\u0001"  # WARN Non-standard string escape
? 1: numeric-key-value  # WARN Non-string key
"1": string-key-value
```

Every occurrence of a duplicate decoded key receives a warning. Non-finite
numbers retain numeric type as `.inf`, `-.inf`, or `.nan`. Finite decimal tokens
normalize exactly without floating-point conversion. If expansion would exceed
4096 characters, or a numeric spelling cannot be normalized without changing
its value, the original parsed token stays visible with a warning.

TOON 3.0 supports escapes for LF, CR, TAB, quotes, and backslashes. Other
control characters use terminal-safe JSON-style `\uXXXX` spellings with
`Non-standard string escape` warnings. These spellings are display extensions;
a literal backslash-u string does not receive a warning. Shared-line warnings
follow parsed-node encounter order. Within each node,
warning order is duplicate key, non-finite number, non-canonical number,
non-string key, then non-standard string escape. A collapsed
container's hidden-warning count follows its own messages and appears last.

Non-string keys use `? ` followed by compact typed notation. Strings remain
JSON-style quoted strings; arrays and ordered object pairs preserve key types
recursively. Each parsed root in a multi-root input has a selectable
`--- (i of n)` document row. Multiple roots do not generate a warning, and
document rows, positions, and previews do not create parsed values or matches.

Shared lines collect warnings into one final comment. Inline-array warnings
name the zero-based element, such as `Non-finite number at [1]`; table warnings
name an escaped field. Collapsed containers retain their own warnings and
report `Contains N hidden warnings` for descendant warnings. Source strings
containing `# WARN` are quoted data and do not increase warning counts.

## Copy and export

Extended display text is not standard TOON 3.0. Its warnings, collapse arrows,
counts, and previews are presentation annotations, not a new file format.
Copy and print commands operate on the selected parsed value. Selecting a
document row targets its parsed root. Whole-document write commands (`:w`,
`:wt`, and feature-enabled `:ws`, including long aliases and overwrite forms)
serialize all active roots as standalone values, not the screen or just the
focused value. Reset with `:.` to export the full original input. JSON preserves
selected token spellings, duplicate keys, and order; YAML retains document
framing. Standard TOON still requires exactly one root and applies its normal
conversion restrictions. Encoding finishes before opening an output file, so
failed conversion cannot truncate an existing file.

Interactive JSON commands stay JSON. Redirected stdout defaults to standard TOON;
use `-o json` or `-o yaml` to select a different machine-output format. Standard TOON export through `yt`, `pt`, and
`:wt` retains the [published codec's conversion behavior](toon-codec.md),
including last-value-wins duplicates and possible numeric precision loss.
Redirected stdout retains the [command-line contract](../README.md#command-line-arguments).
