## Why

NDJSON, JSONL, and other multi-root inputs currently show warning separators that navigation skips. Each document needs a selectable, collapsible parent so users can inspect and move between documents without losing their place in the sequence.

## What Changes

- Replace each `---  # WARN Multiple document roots` annotation with a selectable `---` document row for every multi-root input, including YAML streams.
- Show the subdued document position, such as `(1 of 3)`, in both states. Expanded rows show only this count after `---`; collapsed rows also show a subdued contents preview.
- Collapse each document independently while retaining its row and position and adding a contents preview.
- Make document rows participate in vertical, parent, child, sibling, mouse, and numbered navigation. From a top-level object field, `[` selects its document row and `]` selects the next document row.
- Preserve the existing indentation of each document's content. Document rows add no indentation level.
- Preserve parsed values, paths, copy/export behavior, and single-root presentation.

## Capabilities

### New Capabilities

- `toon-sequences`: Document row identity, sequence positions, previews, collapse, and unchanged data semantics across multi-root input formats.

### Modified Capabilities

- `toon-navigation`: Include document rows in navigation and line jumps; use them as root selection targets.
- `toon-rendering`: Allow document headers and root collapse in sequences, with persistent subdued positions, contents previews only when collapsed, and no extra indentation.
- `toon-display-extensions`: Remove the multiple-roots warning and replace passive separators with document rows.

## Impact

The implementation will touch the TOON layout and projection in `src/toon_display.rs`, navigation in `src/viewer.rs`, and rendering, hit testing, and gutters in `src/lineprinter.rs` and `src/screenwriter.rs`. Tests and user help must cover sequence navigation and collapse. Parser and serializer contracts, dependencies, Rust 1.87 compatibility, and the committed lockfile remain unchanged.
