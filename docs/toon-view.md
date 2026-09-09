---
type: Guide
title: TOON document view
description: Document layout, logical selection, collapse, and display extensions.
status: draft
generated: { by: codex/gpt-6, at: 2026-09-09T22:18:23Z }
---

# TOON document view

The interactive viewer renders JSON, YAML, and TOON through the same TOON 3.0
profile, using 2-space indentation, commas, and no key folding. Rendering works
in every build. The optional `toon` feature controls input and standard export.

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
status. Root objects remain expanded.

Line numbers and collapse arrows use dark gray, changing to light gray on the
selected line. Focus uses bright syntax colors
instead of bold text, and stays on the selected display line when a container
has visible descendants. Inline elements and table cells keep individual focus.

## Selection and navigation

Up/down moves through visible data lines. Parent, child, and sibling motions
move through parsed values, including elements and cells sharing one line.
Press `l` or Right Arrow on an inline array to show one element per line while
keeping the array selected. Press it again to select the first element. This
explicit multiline choice survives resizing and collapsing/reopening the array.
`[` selects the current node's parent; `]` selects that parent's next sibling. For `{a: …, b: {x: …}, c: …}`, from `b.x` they select `b` and `c`.
The destination stays expanded or collapsed as it was. If there is no parent or
no next sibling for `]`, focus stays put.
Moving down from a table cell retains its field on the next expanded table row.
A cell's path includes the row index and key, such as `users[1].name`. Its
parent is the row object, whose parent is the array. Duplicate entries remain
separate selections, with an occurrence number beside the path.

Search matches parsed keys and values. A hidden result expands its ancestors.
Table-key matches highlight the shared header while retaining the selected
row's field identity. Generated warnings, counts, and previews add no matches.

The `▾` and `▸` arrows occupy a separate gutter. Collapsing retains a container
header and shows a subdued preview. Object fields use `; ` separators, such as
`name: Ada; active: true`. Objects also show their immediate-entry
count as `{N}`, such as `{7}` for seven entries. Counts, previews, and warnings
remain subdued even on the selected line. Arrays keep the count already in their
TOON header. Expanding an ancestor
restores descendant collapse states. Empty containers have no collapse arrow.

Absolute line numbers are on by default; relative numbers are off. Keep using
`-n`/`-N` and `-r`/`-R` to control them. Absolute jumps and gutters address fully
expanded TOON lines, so collapse leaves gaps. Values sharing a line share its
address. Relative numbers count visible vertical motions. Root warning
separators have absolute addresses but vertical navigation skips them.

Long individual values and table rows scroll horizontally without wrapping.
Resizing keeps the selected value while automatic array layouts adapt to the
available width. Line addresses follow the resulting layout. Collapsed previews give up width before count and
warning annotations; warnings remain reachable through horizontal scrolling.

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
non-string key, multiple roots, then non-standard string escape. A collapsed
container's hidden-warning count follows its own messages and appears last.

Non-string keys use `? ` followed by compact typed notation. Strings remain
JSON-style quoted strings; arrays and ordered object pairs preserve key types
recursively. Each of several parsed roots starts with
`---  # WARN Multiple document roots`. These separators do not create values.

Shared lines collect warnings into one final comment. Inline-array warnings
name the zero-based element, such as `Non-finite number at [1]`; table warnings
name an escaped field. Collapsed containers retain their own warnings and
report `Contains N hidden warnings` for descendant warnings. Source strings
containing `# WARN` are quoted data and do not increase warning counts.

## Copy and export

Extended display text is not standard TOON 3.0. Its warnings, collapse arrows,
counts, and previews are presentation annotations, not a new file format.
Copy and print commands operate on the selected parsed value. Whole-document
write commands operate on the parsed document. None serializes the screen.

Existing JSON output stays JSON. Standard TOON export through `yt`, `pt`, and
`:wt` retains the [published codec's conversion behavior](toon-codec.md),
including last-value-wins duplicates and possible numeric precision loss.
Redirected stdout retains the [command-line contract](../README.md#command-line-contract).
