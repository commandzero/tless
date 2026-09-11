## Context

See [proposal.md](proposal.md) for motivation and scope.
The existing `Layout` and collapse projection map parsed nodes to TOON lines.
`JsonViewer` navigates these lines and stores `top_visible_line`.
`ScreenWriter` currently adds the physical screen row directly to that index for painting and mouse selection.
It also owns gutter sizes, indentation reduction, and horizontal offsets.
`lineprinter` already uses grapheme boundaries, terminal cell widths, and mapped token spans.

These one-row assumptions must change together.
The main rendering spec also explicitly forbids soft wrapping, so this proposal modifies that requirement and qualifies the valid-TOON presentation contract.

## Goals / Non-Goals

1. Keep parsed node identity, canonical TOON lines, search source ranges, and absolute line addresses independent of wrapping.
2. Use one physical-row mapping for painting, viewport movement, hit testing, and reveal operations.
3. Preserve current behavior when wrapping is off, with no dependency or Rust 1.87 compatibility change.

This design does not introduce persistent settings, command-line flags, word wrapping, per-value toggles, or changes to serialization.
It does not change the existing width-based selection of primitive-array layouts.

## Decisions

### Add a presentation projection after logical layout

Keep canonical and visible logical lines intact.
Add a physical-row projection with each row's logical-line index, original text byte range, source cell offset, and first-row flag.
Represent the viewport origin as a logical line plus a continuation offset, or an equivalent physical index resolved through this projection.
Keep node focus and absolute addresses in their existing logical coordinate system.

Do not insert continuation rows into the canonical layout.
That alternative would alter numbered jumps, relative distances, and node ownership.
Do not rely on the terminal's automatic wrap, which cannot reserve blank continuation gutters or supply reliable mouse coordinates.

### Share geometry before painting

Resolve terminal width, enabled gutters, and indentation reduction before calculating physical rows.
Move or expose the necessary geometry so the viewer and screen writer consume the same projection.
Rebuild it after resize, gutter and indentation changes, wrapping toggles, logical-layout changes, and collapse projection changes.
Paint only rows intersecting the document viewport, with explicit cursor positions and clearing.
Keep status and command rows outside its bounds.

Split original text greedily using the existing Unicode libraries.
Retain original byte ranges so token styling and source matching work across boundaries.
Continuation rows add neither repeated indentation nor prefixes.
An overwide grapheme in a positive-width viewport uses a one-row ellipsis placeholder mapped to that grapheme; zero-width content retains one finite row without painting text.
These cases must advance or terminate without loops.

Cache row boundaries by projection generation and geometry rather than rescanning the whole document on every redraw.
Store offsets, not copies of wrapped strings.
Test large values at narrow widths for bounded memory and forward progress.

### Separate logical motion from physical scrolling

Keep up/down, counted moves, sibling and parent moves, and numbered jumps in logical coordinates.
Convert the destination's selected span into a physical row only for reveal.
Use physical rows for explicit viewport scrolling, wheel scrolling, and page distances.
Retain a focused node while its line intersects the viewport so a tall value can be read without changing selection.
If focus leaves the viewport entirely, resolve the nearest permitted visible row to its logical owner using existing table-field retention where applicable.
Scroll padding cannot force a value taller than the viewport back to its first row.

Using only logical viewport offsets was rejected because users could never reveal the middle of a single value taller than the screen.
Screen-position selection and repositioning commands must also resolve through the physical projection.

### Use original spans for search and mouse input

Map search results from source ranges to displayed spans and then to physical rows.
Reveal the match start, including a match in a shared table header, without changing the match owner.
Paint every visible fragment with the existing syntax, focus, and search styles.

Convert clicks through the physical row's original byte/cell offset before existing span hit testing.
Only first-row collapse gutters are actionable.
Ignore continuation-gutter clicks for collapse operations.
After reflow, prefer the active match as the reveal anchor, otherwise the selected span.

### Keep collapsed lines on the current clipping path

Classify collapse state from the projection and node metadata rather than testing only for preview-style tokens.
Some collapsed inline arrays retain syntax-colored values and still need the one-row rule.
Collapsed previews retain annotation fitting and horizontal access, including warnings that exceed the width.
Expanded table rows are eligible for wrapping even though their owning values are objects.

For eligible expanded lines, enabling wrapping discards old horizontal offsets and horizontal scroll commands do nothing until wrapping is disabled.
Disabling starts at the left edge, then uses normal selection or match reveal.
Keeping hidden offsets was rejected because toggling off could unexpectedly hide the start of a value.

### Keep toggle state in the interactive session

Dispatch Ctrl+L only in document input handling and clear any numeric prefix after one toggle.
Use the existing message area to report `Line wrapping on` or `Line wrapping off`.
Returning from help preserves the document's setting.
Prompt editors keep their own Ctrl+L handling.

The default is off to preserve current startup behavior.
The proposal treats the issue's toggle as session-wide, with continuation text starting after the gutter; both choices are explicit reviewable defaults.

## Risks / Trade-offs

1. Coordinate mismatches can mis-select cells or paint gutters. Use one projection and integration tests covering clicks, shared headers, and continuation rows.
2. Narrow terminals create many rows. Cache boundaries, avoid text copies, and verify tall-line traversal and tiny-width progress.
3. Reflow can leave stale viewport offsets. Anchor to logical identity and rebuild before reveal or input hit testing.
4. Wrapping produces visual line breaks that are not TOON data. Keep copy/export paths independent and document that wrapping affects only presentation.
5. Existing pagination tests assume one row per line. Preserve those tests with wrapping off and add wrapped cases rather than replacing their expectations.

## Migration Plan

1. Implement the projection and input integration with wrapping initially off.
2. Update help, acceptance checks, and the Unreleased changelog.
3. Run rendering/navigation tests, isolated terminal checks, full preflight, and Rust 1.87 compatibility checks across the required feature profiles.
4. Verify the implementation against both delta specs, sync them into main specs, and archive this change before merging its implementation PR.

No data or configuration migration is required.
Users can return to existing presentation with Ctrl+L; reverting the feature requires no stored-state cleanup.
