## ADDED Requirements

### Requirement: Viewport-proportional presentation

Interactive viewing SHALL prepare rendered text, styling spans, previews, and display-to-source mappings on demand for the current viewport and explicitly requested navigation targets, rather than materializing the entire expanded document before drawing. Retained presentation data SHALL NOT grow with previously visited rows or unrelated off-screen content. Parsing, semantic validation, structural indexes, warning summaries, and logical line counts can remain document-wide; this requirement SHALL NOT imply streaming input or constant total memory. Explicit whole-document export and search SHALL retain their existing scope.

#### Scenario: Large expanded document

- **WHEN** a document has many more logical lines than the terminal can display
- **THEN** its first useful frame SHALL contain the actual initial TOON rows and application status without first formatting all off-screen rows
- **AND** the viewer SHALL NOT substitute a loading screen, placeholder values, or incomplete semantic decisions for the initial content

#### Scenario: Traversing a long document

- **WHEN** a user visits successive distant regions and then returns to the start
- **THEN** rendered text and source mappings retained solely for previously visited regions SHALL be released rather than accumulated as a second document representation
- **AND** returning to an evicted region SHALL reproduce the same content and logical selection

#### Scenario: Wrap and collapse without eager presentation

- **WHEN** wrapping, collapse, expansion, terminal geometry, or the active path filter changes
- **THEN** the viewer SHALL prepare presentation for the resulting viewport without materializing text and physical rows for every unrelated off-screen line
- **AND** necessary structural reindexing SHALL preserve existing numbering, selection, collapse, and explicit multiline-array semantics

### Requirement: Complete semantics independent of materialization

Whether a line has previously been displayed SHALL NOT affect its TOON layout, warnings, line address, source identity, or navigation behavior. On-demand presentation SHALL preserve the existing rendering, display-extension, navigation, sequence, path-filter, styling, and copy/export contracts. Structural decisions that depend on off-screen data SHALL use the complete relevant data before the affected content is displayed.

#### Scenario: Late table disqualifier

- **WHEN** the last object in an otherwise uniform array has different field order, duplicate fields, or a non-scalar field
- **THEN** the initial viewport SHALL use the existing list-form decision for that entire array
- **AND** scrolling to the final object SHALL NOT retroactively change the array from table to list form

#### Scenario: Off-screen warning and line-number effects

- **WHEN** off-screen descendants contain duplicate-key warnings or width-dependent primitive arrays
- **THEN** collapsed warning totals and absolute line addresses SHALL be correct before those descendants are displayed
- **AND** gutter growth, exact-fit arrays, Unicode widths, warning widths, and reflow SHALL retain the existing fixed-point layout result

#### Scenario: Reveal an unpainted shared-line value

- **WHEN** search or a navigation command targets a table cell, table-field key occurrence, or inline-array element that has never been painted
- **THEN** the correct parsed occurrence SHALL remain selected and its rendered location SHALL be revealed
- **AND** a table-field key match SHALL highlight the shared header spelling while retaining the originating row field's identity
- **AND** filtered roots and hidden ancestors SHALL obey their existing reveal boundaries

#### Scenario: Wrapped distant target

- **WHEN** search selects a value on an unpainted wrapped continuation far from the current viewport
- **THEN** the viewer SHALL reveal the continuation containing that value without first creating presentation for intervening document lines
- **AND** counted logical motion, viewport scrolling, horizontal access after disabling wrapping, and mouse selection SHALL retain their existing meanings

### Requirement: Measured first-useful-frame performance

The reference startup measurement SHALL use a synthetic JSON object with a `records` array containing 150,000 records in ascending order of `id`. Each record SHALL have the ordered shape `{"id":0,"nested":{"ok":true,"label":"record"},"values":[1,2,3]}`, with only `id` varying from 0 through 149999. Compact serialization without a final newline SHALL produce 10,238,903 bytes.

The candidate, tless at commit `9bb9568`, and jless 0.9.0 in data view SHALL run on the same native host with a 140-column by 40-row PTY, default absolute numbers, wrapping off, and no path filter. Measurements SHALL use release executables, record compiler/build provenance and binary identity, exclude one warm-up per executable, and include 20 measured launches per executable without concurrent build or profiling workloads. Time SHALL begin before process launch and end only after initial document content and application status have been emitted. An alternate-screen escape alone SHALL NOT count as a useful frame. P95 SHALL use nearest-rank selection, the nineteenth sorted observation out of twenty.

The candidate's median and p95 first-useful-frame times SHALL each be no more than three times the corresponding jless result. Its median SHALL also be no more than one fifth of the frozen tless baseline's median. These are release-comparison acceptance criteria, not timing assertions for shared CI runners or a guarantee for every input and host. Peak resident memory through the first useful frame SHALL be reported for all three executables, separately from later interaction measurements.

#### Scenario: Reference startup acceptance

- **WHEN** the three executables are measured under the reference protocol
- **THEN** both jless-relative limits and the frozen-tless improvement limit SHALL pass before the refactor is considered complete
- **AND** the report SHALL retain the individual timings, excluded warm-ups, percentile calculation, host/build provenance, input size, frame-detection rule, and peak-memory sampling method
- **AND** previously recorded timings SHALL NOT substitute for a same-host comparison of the candidate and reference executables
