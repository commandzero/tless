# toon-rendering Specification

## Purpose

Provide one syntax-colored TOON document view with predictable collapse annotations, optional gutters, and stable layout across terminal sizes.

## Requirements

### Requirement: One rendering contract

The viewer SHALL render every supported input format with the TOON 3.0 profile: 2-space indentation, comma delimiters, and no key folding. The documented display extensions SHALL apply where that profile cannot faithfully represent the parsed data. Every build profile SHALL provide this view. Input format selection SHALL remain independent of rendering, and TOON input and export SHALL be available in every build profile. Optional `colorscheme` and `sexp` features SHALL retain their respective behavior without gating TOON support.

#### Scenario: Equivalent inputs

- **WHEN** JSON, YAML, and TOON inputs produce equivalent parsed data
- **THEN** their fully expanded document text SHALL match
- **AND** input-format selectors SHALL NOT select a different rendering mode

#### Scenario: Obsolete mode controls

- **WHEN** a user passes `--mode` or `-m`
- **THEN** argument parsing SHALL report an unsupported option
- **AND** interactive `m` SHALL have no mode-switch action
- **AND** current help SHALL NOT advertise alternate modes or closing-delimiter navigation

#### Scenario: Minimal build

- **WHEN** tless is built without default features and opens JSON
- **THEN** the viewer SHALL still render TOON
- **AND** TOON input, export, and interactive commands SHALL remain available

### Requirement: Native TOON layout

Fully expanded standard-compatible data SHALL use TOON object fields, inline or multiline primitive arrays, uniform primitive-only object tables, and list arrays for other structures. Tables SHALL require nonempty, unique string field sets shared by every row. Array order and object entry order SHALL be preserved. When a table would reorder a row's fields, the array SHALL use list form. When table alignment is disabled and the viewport can represent every grapheme and no content is clipped, stripping presentation styling and gutters from fully expanded standard-compatible single-root content, and rejoining soft-wrapped continuations without inserting characters, SHALL leave valid TOON text.

#### Scenario: Primitive array default layout

- **WHEN** a nonempty primitive array has at most five elements and its inline line fits the terminal
- **THEN** it SHALL render inline, counting indentation, gutters, annotations, and Unicode terminal cells toward the width
- **AND** an exact fit SHALL remain inline
- **AND** an array with more than five elements or an overflowing inline line SHALL render a counted header followed by one list line per element

#### Scenario: Table and inline array

- **WHEN** the document contains `{"tags":["rust","cli"],"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Lin"}]}`
- **THEN** its document text with alignment disabled SHALL be `tags[2]: rust,cli` followed by `users[2]{id,name}:` and indented rows `1,Ada` and `2,Lin`
- **AND** the viewer SHALL NOT insert JSON closing delimiters or array-index labels

#### Scenario: Empty containers

- **WHEN** the document contains an empty object field, an empty array, or an array of empty objects
- **THEN** it SHALL use respectively `key:`, `key[0]:`, or a counted list with one bare `-` per empty object
- **AND** it SHALL NOT emit a zero-field table with blank rows
- **AND** empty containers SHALL have no collapse arrow, except their sequence document rows support document collapse

#### Scenario: Empty and nonempty root objects

- **WHEN** a single-root input is an object
- **THEN** its fields SHALL have no synthetic root header or enclosing braces
- **AND** the root SHALL stay expanded while its descendants support collapse
- **AND** an empty root object SHALL display an empty document with its type available in application status

#### Scenario: Aligned text is presentation only

- **WHEN** table alignment is enabled
- **THEN** generated padding SHALL be an interactive presentation exception to the native-text contract
- **AND** disabling alignment SHALL restore ordinary TOON text without changing data or field order

### Requirement: Syntax styling

Keys and table field names SHALL retain their source identities for search and navigation. Their colors SHALL follow the selected theme; the default SHALL give them the same syntax color, while Borealis SHALL use standard text color for field definitions. Strings, numbers, booleans, nulls, and structural syntax SHALL have distinguishable styles. Default terminal palette indexes SHALL be cyan 6 for keys, green 2 for strings, magenta 5 for numbers, blue 4 for booleans (brightening to 12 when selected), gray 7 for nulls, dark gray 8 for previews, and yellow 3 for warnings. The default status bar SHALL use a dark gray 8 background with black 0 text and a light gray 7 filename. In the default theme, all search matches, including the active match, SHALL be underlined. Default search matches SHALL use yellow 3 foreground and the active match SHALL use bright yellow 11 foreground, overriding value selection colors. The default theme SHALL NOT use reverse video for any element. TOON array counts SHALL use structural styling. TOON syntax spans, including sequence separators (`---`), array counts (`[N]`), list delimiters (`-`), and punctuation, SHALL use normal or theme-colored styling; focused array counts SHALL use the same lightened syntax style as focused punctuation. Subdued styling SHALL be reserved for tless-generated annotations and preview text, even when previews contain TOON spellings. Expanded data and complete inline primitive arrays of at most five elements that fit the terminal SHALL retain their data styling, including when collapsed, except copies in sequence document previews SHALL remain subdued. Collapsed previews, sequence document previews and position annotations, object count annotations, and extension warning comments SHALL use subdued annotation styling. In the default theme these annotations SHALL remain subdued without focus highlighting or bold when their owning container is selected. Default previews and object counts SHALL use plain terminal color 8 without the dim attribute, matching the default line-number color.

#### Scenario: Expanded primitive array

- **WHEN** `values[3]: 1,true,hello` is expanded
- **THEN** its values SHALL have number, boolean, and string styles
- **AND** these values SHALL NOT be styled as a collapsed preview

### Requirement: Collapse presentation

Nonempty collapsible containers SHALL show `▾` when expanded and `▸` when collapsed in a reserved gutter outside TOON indentation. Inline primitive arrays SHALL show `▸` by default; clicking their arrow or pressing Space SHALL expand them to multiline. Tabular rows SHALL NOT have collapse indicators or support row collapse; only their array parent SHALL be collapsible. Collapsing SHALL retain the container's header and replace multiline contents with a preview in document order. Complete inline primitive arrays with at most five elements that fit the terminal SHALL retain value syntax styling; other previews SHALL be subdued. Collapsed object previews SHALL separate fields with a semicolon followed by a space (`; `). A collapsed object below a document root, or in a single-root input, SHALL show its immediate-entry count as `(N)`, such as `(1)` or `(7)`; duplicate entries SHALL each count. Arrays SHALL retain their TOON count and SHALL NOT receive a second count annotation. Expanded containers SHALL have no contents preview or object-count annotation. Sequence document rows SHALL retain their subdued position in both states and show a contents preview only when collapsed. Sequence document rows SHALL be independently collapsible, including for scalar and empty-container documents. Their collapse controls SHALL use the same gutter arrows. Their contents SHALL retain the indentation they would have as standalone roots. A sequence document preview SHALL remain subdued even for a short primitive array. Document rows SHALL use their sequence position without an additional object-entry count.

#### Scenario: Object and array collapse

- **WHEN** `owner` has 2 immediate fields and `tags` has 3 items, and both are collapsed
- **THEN** their lines SHALL retain `owner:` and `tags[3]:`
- **AND** only `owner` SHALL receive `(2)`
- **AND** object previews SHALL be subdued; a complete fitting inline primitive array SHALL retain value syntax styling

#### Scenario: Tabular row collapse

- **WHEN** a user collapses an object row in a table
- **THEN** that row's values SHALL remain visible and unchanged
- **AND** the row SHALL have no collapse indicator
- **AND** the array parent SHALL retain its collapse control

#### Scenario: Inline array collapse

- **WHEN** a user collapses an inline primitive array
- **THEN** the header SHALL remain and its complete values SHALL keep their syntax colors if there are at most five elements and the inline line fits the terminal
- **AND** this SHALL also apply after explicitly expanding the array to multiline
- **AND** larger or overflowing previews SHALL remain subdued
- **AND** expanding it SHALL restore syntax-colored values in its chosen layout, with `l` or Right Arrow selecting multiline presentation

#### Scenario: Collapse restoration

- **WHEN** a collapsed ancestor is expanded again
- **THEN** descendant collapse states SHALL be restored
- **AND** the underlying data and layout choice SHALL be unchanged

### Requirement: Gutters and terminal width

Optional absolute and relative line numbers SHALL remain available outside the document text with existing visibility defaults. Absolute numbers SHALL identify lines in the fully expanded TOON layout, beginning at 1; collapsed descendants SHALL produce gaps. Relative numbers SHALL count visible display-line motions. Automatic primitive-array layouts SHALL be recalculated on terminal-width or gutter-visibility changes while preserving logical selection and collapse states. Explicit multiline choices SHALL survive resize and collapse/reopen. Absolute addresses SHALL follow the current fully expanded layout. With wrapping disabled, long individual values and table rows SHALL use horizontal scrolling without soft wrapping. With wrapping enabled, expanded lines SHALL follow the optional line wrapping requirement. Continuation rows SHALL NOT add absolute addresses or relative motion distance. On lines using horizontal scrolling, `,` and `.` SHALL scroll left and right by ten terminal cells per press, multiplied by any numeric prefix and clamped at the line boundaries, or at the shared table boundaries when alignment is enabled. Rendering SHALL escape control characters and respect terminal cell widths.

#### Scenario: Shared-line numbering

- **WHEN** focus moves between cells on a table row or elements in an inline array
- **THEN** the absolute line number SHALL remain unchanged
- **AND** relative vertical distance between those values SHALL be 0

#### Scenario: Narrow viewport

- **WHEN** the terminal cannot fit the full content or preview
- **THEN** expanded content SHALL remain reachable by horizontal scrolling when wrapping is disabled and by vertical viewport scrolling when wrapping is enabled for that line; aligned tables SHALL remain reachable by shared horizontal scrolling
- **AND** collapsed previews SHALL truncate with `…` at a valid character boundary
- **AND** preview space SHALL be removed before object-count or warning space
- **AND** warnings that still do not fit SHALL remain reachable by horizontal scrolling on unwrapped lines and vertical viewport scrolling on wrapped lines

#### Scenario: Application controls

- **WHEN** focus, search, status, or command entry is active
- **THEN** status and commands SHALL remain outside document text
- **AND** focus and search SHALL highlight existing spans without inserting text
- **AND** default-theme focus SHALL use bright foreground colors instead of bold and SHALL apply on all visible physical rows belonging to the selected logical display line
- **AND** default-theme line numbers and collapse arrows SHALL use dark gray ordinarily and light gray on the selected display line

### Requirement: Optional line wrapping

Ctrl+L in the document view SHALL toggle wrapping for all eligible lines for the current session, initially disabled. Each press SHALL toggle once regardless of a numeric prefix. Command and search prompts SHALL retain their input-editor behavior. The application SHALL report whether wrapping is on or off and document the key in interactive help.

Wrapping SHALL operate after normal TOON layout selection and indentation reduction. It SHALL NOT change primitive-array layout decisions, parsed values, collapse states, export contents, or piped output. Expanded lines, including keys, scalar values, unaligned table headers, unaligned table rows, and their warning annotations, SHALL wrap greedily at grapheme boundaries using terminal cell widths, without word-boundary preference. Continuations SHALL start at the document area's left edge, after the reserved number and collapse-arrow gutter. Original indentation SHALL appear only as part of the original line text, without adding repeated indentation or key prefixes.

Aligned table headers and rows, including their warning annotations, SHALL remain unwrapped and use table-wide horizontal scrolling regardless of the session wrapping setting. Ctrl+L SHALL NOT disable table alignment or reset its shared offset. Disabling alignment SHALL restore wrap eligibility under the current session setting.

Only the first physical row SHALL show the logical line's number and collapse arrow. Continuation gutters SHALL remain blank even when the first row is above the viewport. Wrapping SHALL NOT add clipping ellipses at ordinary wrap boundaries. Syntax, focus, and search styling SHALL continue over the original spans.

Collapsed container lines, including their previews, counts, and warnings, SHALL remain one physical row with existing preview fitting and horizontal scrolling. Sequence document rows SHALL remain single rows in both expanded and collapsed states, including their position, any collapsed contents preview, and warning annotations. While wrapping is enabled, horizontal scroll commands SHALL have no effect on eligible expanded lines. Enabling wrapping SHALL clear their previous horizontal offsets; disabling wrapping SHALL start at the left edge and then reveal the selected span or active search match as needed.

#### Scenario: Toggle and exact fit

- **WHEN** a document is opened and an expanded line exceeds the document width
- **THEN** it SHALL initially use horizontal clipping and scrolling
- **AND** Ctrl+L SHALL wrap it and show the enabled state
- **AND** a line that fits exactly SHALL occupy one physical row
- **AND** another Ctrl+L SHALL restore unwrapped presentation and show the disabled state

#### Scenario: Blank continuation gutters

- **WHEN** a logical line numbered 12 occupies 3 physical rows with wrapping enabled
- **THEN** only its first row SHALL show 12 and any collapse arrow
- **AND** both continuation rows SHALL reserve the same gutter width with blank cells
- **AND** the next logical line SHALL retain its original absolute number
- **AND** the same gutter isolation SHALL apply with absolute numbers, relative numbers, both, or neither enabled

#### Scenario: Unicode and tiny widths

- **WHEN** a wrap boundary encounters a wide character, combining sequence, or emoji grapheme
- **THEN** the viewer SHALL keep the grapheme intact and move it to the next row if it fits there
- **AND** when a grapheme is wider than the entire positive document width it SHALL consume one bounded placeholder row showing an ellipsis and advance past that grapheme
- **AND** a zero-cell document area SHALL paint no document text and retain a finite one-row representation per logical line
- **AND** no document text SHALL overwrite the gutter, status, or command rows

#### Scenario: Collapsed previews remain unwrapped

- **WHEN** wrapping is enabled and an expanded container is collapsed
- **THEN** its resulting preview line SHALL occupy one physical row
- **AND** existing preview truncation, count and warning priority, and horizontal access SHALL remain available
- **AND** expanding it SHALL restore wrapping for eligible lines

#### Scenario: Layout and output remain stable

- **WHEN** wrapping is toggled at a fixed width and gutter configuration
- **THEN** primitive-array inline versus multiline choices and table structure SHALL remain unchanged
- **AND** source escape sequences SHALL remain displayed escapes rather than interpreted control characters
- **AND** copying, exporting, and redirected stdout SHALL contain no soft-wrap newlines or placeholders

#### Scenario: Alignment overrides wrapping locally

- **WHEN** session wrapping is on and the user enables alignment on one table
- **THEN** that table header and its rows SHALL remain single physical rows with horizontal scrolling
- **AND** other eligible content SHALL remain wrapped
- **AND** toggling Ctrl+L SHALL preserve that table's alignment and shared offset
- **AND** disabling alignment SHALL restore normal presentation under the current wrapping setting

### Requirement: Theme selection background fills the row

When a theme defines a distinct selection background, each visible physical row of the selected logical display line SHALL fill the terminal width with it, including indentation, spaces, gutters, and clipping markers. Themes without a distinct selection background SHALL retain their appearance. Search matches SHALL retain their theme search style over the row background.

#### Scenario: Borealis selected row

- **WHEN** a short document line is selected under Borealis
- **THEN** the selection background SHALL extend through unused columns to the terminal edge
- **AND** search spans SHALL retain their search background

### Requirement: Avoid redundant startup layout construction

The viewer SHALL construct its initial layout using actual terminal content dimensions and effective absolute or relative line-number settings. Increasing the number gutter SHALL reuse rendered content when all inline arrays still fit the resulting content width. The reuse decision SHALL include Unicode terminal-cell width and annotations, and collapsed previews SHALL use the resulting width. If an inline fit changes, the viewer SHALL recompute the layout and gutter until consistent.

This optimization SHALL preserve existing TOON text, source identities, warnings, explicit multiline choices, navigation, wrapping, search, filtering, and copy/export semantics. It SHALL NOT introduce loading/pending states or change startup failure or machine-output contracts.

#### Scenario: Nondefault initial geometry

- **WHEN** startup uses nondefault terminal dimensions or disables line numbers
- **THEN** the first constructed layout SHALL use that geometry and the effective gutter
- **AND** exact-fit Unicode arrays SHALL retain their existing inline or multiline presentation

#### Scenario: Gutter grows without changing an inline fit

- **WHEN** the line count requires a wider number gutter but every inline array still fits
- **THEN** the viewer SHALL reuse the rendered layout instead of rendering the whole document again
- **AND** collapsed inline previews SHALL honor the narrower available width

#### Scenario: Gutter growth changes an inline fit

- **WHEN** an additional line-number digit makes an annotated or Unicode array too wide
- **THEN** the array SHALL become multiline under the existing layout rules
- **AND** the gutter SHALL be rechecked against the resulting line count

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

### Requirement: Per-table alignment toggle

Tab in the document view SHALL toggle alignment once for the focused tabular array, including focus on its header, row, or cell, regardless of a numeric prefix. Alignment SHALL start disabled for every table. Tab SHALL NOT align non-tabular values or change prompt-editor behavior. A collapsed table SHALL retain its collapse state when toggled.

#### Scenario: Toggle from header and cells

- **WHEN** the user presses Tab on a table header, row object, or cell
- **THEN** only that table SHALL toggle alignment and preserve logical focus
- **AND** pressing Tab again SHALL restore its unaligned presentation
- **AND** toggling one table SHALL NOT change another table

#### Scenario: Ineligible focus and prompt input

- **WHEN** Tab is pressed on a scalar, primitive array, list-form array, or in a command or search prompt
- **THEN** it SHALL NOT change table alignment or collapse state
- **AND** prompt input SHALL retain its existing Tab behavior

#### Scenario: Collapsed table

- **WHEN** the user enables alignment on a collapsed tabular array
- **THEN** the array SHALL remain collapsed with its ordinary preview
- **AND** expanding it SHALL display aligned headers and rows

### Requirement: Terminal-cell aligned table columns

An expanded aligned table SHALL left-align each rendered cell with its field header using generated whitespace outside the data tokens. Column widths SHALL include the widest rendered header or value across all table rows, measured in terminal cells. Commas and the counted header SHALL remain visible syntax. Gutters, indentation reduction, and scrolling SHALL preserve column alignment.

#### Scenario: Exact header and value positions

- **WHEN** alignment is enabled for `users[2]{id,name}:` with rows `1,Ada` and `200,Lin`
- **THEN** excluding gutters its aligned lines SHALL be:

```text
users[2]{id ,name}:
         1  ,Ada
         200,Lin
```

- **AND** each value SHALL start at the same terminal column as its header
- **AND** generated row-leading spaces SHALL align the first value with the first header without changing table nesting

#### Scenario: Width includes unseen rows and quoted tokens

- **WHEN** the widest cell is off-screen or contains quoted escapes, a combining sequence, a wide character, or an emoji grapheme
- **THEN** every visible row SHALL use the complete-table terminal-cell widths of rendered tokens, including quotes and escapes
- **AND** scrolling to that cell SHALL NOT change existing column widths
- **AND** padding SHALL NOT be inserted within quoted tokens or split graphemes
- **AND** warning annotations SHALL remain after the row data and SHALL NOT determine column widths

#### Scenario: Resize and indentation reduction

- **WHEN** the terminal, gutters, or indentation reduction changes for an aligned table
- **THEN** each cell and its header SHALL remain aligned under the same horizontal transform
- **AND** clipping SHALL respect grapheme boundaries and reserved application rows even at zero document width

### Requirement: Alignment state and visible indication

Alignment SHALL persist per table during the session across focus changes, resize, gutter changes, collapse/reopen, and path filtering that later restores that table. A persistent status indicator SHALL identify alignment while focus belongs to an aligned table, including a collapsed one. Help SHALL document Tab, shared scrolling, and the local wrapping override. Alignment SHALL NOT change parsed data, copy/export, or piped output.

#### Scenario: Persistent focused-table indication

- **WHEN** focus moves between an aligned table's header, rows, and cells
- **THEN** status SHALL continue to indicate table alignment without relying on a transient message or color alone
- **AND** leaving that table SHALL remove its indicator without clearing its alignment setting
- **AND** narrow terminals SHALL use a shortened visible indicator when status space is available

#### Scenario: Hide and restore

- **WHEN** an aligned table is collapsed, hidden by an ancestor, or excluded and later restored by path filtering
- **THEN** its alignment setting SHALL survive
- **AND** its expanded aligned presentation SHALL use widths for the complete restored table
- **AND** selecting a row or cell alone as a filtered root SHALL NOT turn that value into a table

#### Scenario: Copy and output isolation

- **WHEN** a user copies a selected aligned cell or writes an aligned table in a supported output format
- **THEN** the result SHALL serialize the parsed selection with no generated alignment padding or indicator
- **AND** redirected stdout SHALL retain its existing output contract
