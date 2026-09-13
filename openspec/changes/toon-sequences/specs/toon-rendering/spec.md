## MODIFIED Requirements

### Requirement: Native TOON layout

Fully expanded standard-compatible data SHALL use TOON object fields, inline or multiline primitive arrays, uniform primitive-only object tables, and list arrays for other structures. Tables SHALL require nonempty, unique string field sets shared by every row. Array order and object entry order SHALL be preserved. When a table would reorder a row's fields, the array SHALL use list form. When the viewport can represent every grapheme and no content is clipped, stripping presentation styling and gutters from fully expanded standard-compatible single-root content, and rejoining soft-wrapped continuations without inserting characters, SHALL leave valid TOON text.

#### Scenario: Primitive array default layout

- **WHEN** a nonempty primitive array has at most five elements and its inline line fits the terminal
- **THEN** it SHALL render inline, counting indentation, gutters, annotations, and Unicode terminal cells toward the width
- **AND** an exact fit SHALL remain inline
- **AND** an array with more than five elements or an overflowing inline line SHALL render a counted header followed by one list line per element

#### Scenario: Table and inline array

- **WHEN** the document contains `{"tags":["rust","cli"],"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Lin"}]}`
- **THEN** its document text SHALL be `tags[2]: rust,cli` followed by `users[2]{id,name}:` and indented rows `1,Ada` and `2,Lin`
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

### Requirement: Syntax styling

Keys and table field names SHALL retain their source identities for search and navigation. Their colors SHALL follow the selected theme; the default SHALL give them the same syntax color, while Borealis SHALL use standard text color for field definitions. Strings, numbers, booleans, nulls, and structural syntax SHALL have distinguishable styles. Default terminal palette indexes SHALL be cyan 6 for keys, green 2 for strings, magenta 5 for numbers, blue 4 for booleans (brightening to 12 when selected), gray 7 for nulls, dark gray 8 for previews, and yellow 3 for warnings. The default status bar SHALL use a dark gray 8 background with black 0 text and a light gray 7 filename. In the default theme, all search matches, including the active match, SHALL be underlined. Default search matches SHALL use yellow 3 foreground and the active match SHALL use bright yellow 11 foreground, overriding value selection colors. The default theme SHALL NOT use reverse video for any element. TOON array counts SHALL use structural styling. Expanded data and complete inline primitive arrays of at most five elements that fit the terminal SHALL retain their data styling, including when collapsed, except copies in sequence document previews SHALL remain subdued. Collapsed previews, sequence document previews and position annotations, object count annotations, and extension warning comments SHALL use subdued annotation styling. In the default theme these annotations SHALL remain subdued without focus highlighting or bold when their owning container is selected. Default previews and object counts SHALL use plain terminal color 8 without the dim attribute, matching the default line-number color.

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

### Requirement: Optional line wrapping

Ctrl+L in the document view SHALL toggle wrapping for all eligible lines for the current session, initially disabled. Each press SHALL toggle once regardless of a numeric prefix. Command and search prompts SHALL retain their input-editor behavior. The application SHALL report whether wrapping is on or off and document the key in interactive help.

Wrapping SHALL operate after normal TOON layout selection and indentation reduction. It SHALL NOT change primitive-array layout decisions, parsed values, collapse states, export contents, or piped output. Expanded lines, including keys, scalar values, table headers, table rows, and their warning annotations, SHALL wrap greedily at grapheme boundaries using terminal cell widths, without word-boundary preference. Continuations SHALL start at the document area's left edge, after the reserved number and collapse-arrow gutter. Original indentation SHALL appear only as part of the original line text, without adding repeated indentation or key prefixes.

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
