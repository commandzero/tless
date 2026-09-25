## ADDED Requirements

### Requirement: Demand-prepared document presentation

A validated document SHALL become browsable without materializing display text and source mappings for every line. The committed viewport SHALL retain the existing native TOON layout, warnings, counts, ordering, duplicate occurrences, sequence headers, themes, source mapping, and collapse conventions. Off-screen content SHALL be prepared when needed without making unseen content unsearchable or changing copied/exported data. A table decision SHALL account for every row, and a displayed exact count SHALL be complete rather than inferred from the prepared prefix.

#### Scenario: Late table incompatibility

- **WHEN** the first rows of an array share primitive fields but the final row has incompatible fields or ordering
- **THEN** the first committed viewport SHALL use the correct list presentation rather than a provisional table
- **AND** later preparation SHALL NOT silently change earlier field identities or table membership

#### Scenario: Warnings beyond the viewport

- **WHEN** duplicate keys or display warnings occur outside the prepared viewport or inside collapsed content
- **THEN** occurrence identities and displayed warning totals SHALL remain correct
- **AND** navigating to that data SHALL retain its original number tokens, entry order, and source identities

### Requirement: Atomic geometry transitions

The initial committed viewport SHALL use the actual terminal dimensions and active gutter settings. Width, gutter, indentation, and wrap changes SHALL preserve existing layout and focus semantics without blocking document input. While geometry is being prepared, the viewer SHALL show a pending state and either retain a safely clipped prior frame or show a loading surface. It SHALL NOT mix line addresses, content, hit targets, or source maps from different geometries. Unknown addresses SHALL NOT be displayed as exact numbers. A committed frame SHALL use complete current-geometry addresses, including fully expanded logical line numbers and collapse gaps.

#### Scenario: Terminal differs from default geometry

- **WHEN** startup occurs in a terminal whose dimensions differ from the application's defaults
- **THEN** the first useful frame SHALL use the actual dimensions and effective number gutter
- **AND** controls and status SHALL remain outside document content

#### Scenario: Inline fit changes with gutter width

- **WHEN** the number of digits in absolute line addresses changes a primitive array's fit
- **THEN** the committed layout SHALL account for gutters, indentation, Unicode cell widths, and annotations before deciding exact fit
- **AND** transient geometry SHALL NOT become an addressable document layout

#### Scenario: Resize while a cell is selected

- **WHEN** the user resizes repeatedly while a table cell or wrapped search match is selected
- **THEN** the completed frame SHALL retain that logical selection or match under the latest dimensions
- **AND** clicks while geometry is pending SHALL NOT resolve against stale hit targets
- **AND** the committed frame SHALL preserve explicit multiline choices and descendant collapse state

### Requirement: Incrementally prepared wrapped view

Wrapping SHALL retain its existing logical-line and grapheme semantics while preparing only required physical-row regions and bounded overscan. Every continuation SHALL remain reachable, including those in a value larger than the viewport or presentation cache. Counts used for physical positioning SHALL be exact when committed; unfinished counts SHALL not be substituted for exact positions. Unwrapped browsing SHALL NOT require allocating one retained physical-row object per document line.

#### Scenario: Jump to a distant wrapped region

- **WHEN** wrapping is enabled and the user selects a distant node whose continuations have not been prepared
- **THEN** the viewer SHALL acknowledge the pending destination while retaining responsive input
- **AND** completion SHALL reveal the selected span with correct blank continuation gutters and source-based highlighting
- **AND** visiting the region SHALL NOT require retaining wrapped text for every preceding line

#### Scenario: Long scalar across evicted regions

- **WHEN** a user pages through a scalar whose display exceeds the presentation budget and then returns to an evicted region
- **THEN** the same grapheme boundaries, logical number, selection identity, and value-copy target SHALL be reconstructed
- **AND** no source bytes or continuations SHALL be skipped because of cache eviction
