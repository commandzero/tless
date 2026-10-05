# Design

## Context

See [proposal.md](proposal.md) for motivation and scope. `src/toon_display/format.rs` currently emits table headers and comma-separated row values with source-aware spans. `Layout` separates structural analysis and geometry from row materialization. `ScreenWriter` stores horizontal offsets by absolute line and routes focus/search reveal through those offsets. Document key dispatch multiplies `,` and `.` counts by ten. Wrapping is a session setting; expanded tables currently participate in it.

The existing viewport-proportional presentation contract forbids retaining formatted off-screen rows. Alignment must account for their values without creating a second rendered document. The implementation must remain compatible with Rust 1.87 and the published codec integration.

## Goals / Non-Goals

**Goals:**

- Reuse table eligibility, parsed node identities, scalar spellings, terminal-cell measurement, and viewport projection.
- Keep one source of truth for alignment and scroll ownership; painting, mouse mapping, focus reveal, and search reveal must agree.
- Make off-screen column measurements stable without changing lazy row materialization or normal startup work.

**Non-Goals:**

- New table eligibility rules, sorting, editing, fixed-width truncation, numeric right alignment, or sticky headers.
- New CLI flags, persistence, alternate output encodings, codec changes, or global changes to wrapping.
- Reformatting non-tabular arrays or tables exported to files or redirected stdout.

## Decisions

### 1. Key state by parsed table-array identity

Keep enabled-table identities and their compact column metrics in viewer presentation state, keyed by the existing parsed array node index rather than an absolute line, path string, or focused row. Resolve the owning table from the selected array, row, or cell using existing analysis and parsed ancestry. Reject non-tabular focus without changing any state. Route document-view Tab once regardless of a count; leave prompt handling alone.

Keep settings across collapse and path-filter projection, which change visibility rather than the parsed document. A filtered row or cell presented as its own root is not a table and must not inherit its original parent's grid. Clear state when the document is replaced. This avoids collisions between duplicate paths and movement of line addresses during reflow. A session-global flag was rejected because unrelated tables would change together.

### 2. Measure rendered tokens once per enabled table

On first enable, scan that table's headers and primitive cells using the same quoting/escape spelling as normal display. Retain only a vector of maximum terminal-cell widths and compact extent metadata, not row strings or span maps. Reuse these metrics for redraw, focus movement, collapse/reopen, and terminal resize; recompute geometry-dependent prefixes and extents as needed. Release cached metrics when alignment is disabled; a subsequent enable can measure again.

Include all rows, not just the viewport, so scrolling never changes widths. Measurement must exclude generated warnings from column maxima but include them in complete-line scroll bounds. Prefer measuring scalar spellings without allocation where possible; if quoting requires a temporary buffer, reuse it and discard each token immediately. Do not call the full row formatter for every table row. An eager document-wide column index was rejected because alignment starts off and must not add startup work for every table.

### 3. Preserve the counted header and align to its first field

Keep `name[count]{...}:` syntax. Start the grid at the first field after `{`. Pad each non-final field or value on its right to the column maximum, then emit the comma. Pad row-leading presentation so the first cell begins directly beneath the first header. Do not pad inside quoted strings or append unnecessary trailing spaces after the final value. For example:

```text
users[2]{id ,name}:
         1  ,Ada
         200,Lin
```

Use a common table horizontal coordinate system after applying nesting indentation reduction; generated row-leading grid padding is not structural indentation. Applying the existing per-line indentation reduction independently to each padded line could misalign the grid, so derive the reduction from the owning table for all members. Gutters remain outside this coordinate system.

Rebuild spans as tokens are emitted rather than inserting whitespace into already-mapped strings. Padding has no source range, and mouse clicks on it fall back to the line owner. Shared header field aliases keep the existing first-row mouse target and originating-row search identity. A separate header row was rejected because it would change numbered line addresses and navigation.

### 4. Share offsets and reveal paths at the table boundary

Extend screenwriter offset ownership to distinguish ordinary lines from expanded aligned table members. All consumers select the same table key for an aligned header or row: painting, mouse conversion, `,`, `.`, `;`, selected-span reveal, and search reveal. Keep the shared offset separate from ordinary collapsed-preview offsets. Do not duplicate offset updates across individual rows.

Bounds use the longest complete aligned header or row including annotations. Use saturating counted movement and the existing end/start fitting convention against that extent. Preserve a shared offset while alignment remains enabled, including collapse/reopen and filter exclusion/restoration. Reflow clamps it to current bounds and reveals the selection or match when needed. Enable/disable clears stale table-member line offsets and starts at zero before ordinary selected-span reveal. This avoids restoring incompatible unaligned positions or making short rows prevent access to longer ones.

### 5. Alignment takes precedence over wrapping only for its table

Aligned expanded headers and rows are ineligible for soft wrapping even when the session setting is on. This follows issue #5's requirement that horizontal keys scroll the whole aligned table: wrapping members independently would destroy column comparison and otherwise make those keys no-ops. Ctrl+L continues to control other eligible lines and does not clear shared table offsets. Disabling alignment restores the current wrapping policy rather than a saved global setting.

Turning wrapping off globally was rejected because one table would change unrelated content. Wrapping an aligned table was rejected because header and value columns would no longer stay together. Collapsed previews retain their ordinary single-line behavior.

### 6. Treat padding as presentation, with a persistent status indication

Show a textual `Table aligned` status label while focus belongs to an enabled table, including its collapsed header. Use a compact label when necessary under existing status width constraints. Do not rely on an informational message that disappears on the next action, and do not insert an indicator into document text.

Native TOON text guarantees explicitly exclude enabled alignment. Copy/export still serialize parsed selections; warning/source identities and piped output remain unchanged. This avoids claiming terminal padding is a new standard serialization format.

## Risks / Trade-offs

- [First enable scans a large table] → Scan only the targeted table and retain column metrics, not rendered rows. Exercise a large table with the widest value near the end; do not add a loading screen or incomplete grid.
- [Padding shifts search and mouse coordinates] → Emit mapped tokens at their final positions and test quoted Unicode cells, shared header matches, and clicks after nonzero shared scrolling.
- [Existing layout-generation resets clear table state] → Keep identity-based settings distinct from line-address caches; recompute geometry without discarding enabled identities or valid shared offsets.
- [Row-leading padding is mistaken for nesting indentation] → Use the table coordinate transform for every member and verify nested tables with indentation reduction and gutter changes.
- [Long warnings are unreachable from shorter rows] → Measure complete aligned extent separately from column widths and prove end/start scrolling exposes annotations.
- [Wrapping override surprises users] → State it in help and viewing guidance; keep the session wrapping setting and unrelated content unchanged.

## Migration Plan

No stored-data or CLI migration. Ship as an initially disabled interactive feature. Update help, viewing/acceptance documentation, and the Unreleased changelog with implementation. Verify native PTY behavior and repository preflight, then synchronize the two delta specs and archive this change before merging the associated implementation PR. Rollback removes the new interaction and its state without changing parsers, serialized data, or the existing output contract.
