## MODIFIED Requirements

### Requirement: Native TOON layout

Fully expanded standard-compatible data SHALL use TOON object fields, inline or multiline primitive arrays, uniform primitive-only object tables, and list arrays for other structures. Tables SHALL require nonempty, unique string field sets shared by every row. Array order and object entry order SHALL be preserved. When a table would reorder a row's fields, the array SHALL use list form. When the viewport can represent every grapheme and no content is clipped, stripping presentation styling and gutters from fully expanded standard-compatible content, and rejoining soft-wrapped continuations without inserting characters, SHALL leave valid TOON text.

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
- **AND** empty containers SHALL have no collapse arrow

#### Scenario: Empty and nonempty root objects

- **WHEN** the root is an object
- **THEN** its fields SHALL have no synthetic root header or enclosing braces
- **AND** the root SHALL stay expanded while its descendants support collapse
- **AND** an empty root object SHALL display an empty document with its type available in application status

### Requirement: Gutters and terminal width

Optional absolute and relative line numbers SHALL remain available outside the document text with existing visibility defaults. Absolute numbers SHALL identify lines in the fully expanded TOON layout, beginning at 1; collapsed descendants SHALL produce gaps. Relative numbers SHALL count visible display-line motions. Automatic primitive-array layouts SHALL be recalculated on terminal-width or gutter-visibility changes while preserving logical selection and collapse states. Explicit multiline choices SHALL survive resize and collapse/reopen. Absolute addresses SHALL follow the current fully expanded layout. With wrapping disabled, long individual values and table rows SHALL use horizontal scrolling without soft wrapping. With wrapping enabled, expanded lines SHALL follow the optional line wrapping requirement. Continuation rows SHALL NOT add absolute addresses or relative motion distance. On lines using horizontal scrolling, `,` and `.` SHALL scroll left and right by ten terminal cells per press, multiplied by any numeric prefix and clamped at the line boundaries. Rendering SHALL escape control characters and respect terminal cell widths.

#### Scenario: Shared-line numbering

- **WHEN** focus moves between cells on a table row or elements in an inline array
- **THEN** the absolute line number SHALL remain unchanged
- **AND** relative vertical distance between those values SHALL be 0

#### Scenario: Narrow viewport

- **WHEN** the terminal cannot fit the full content or preview
- **THEN** expanded content SHALL remain reachable by horizontal scrolling when wrapping is disabled and by vertical viewport scrolling when wrapping is enabled
- **AND** collapsed previews SHALL truncate with `…` at a valid character boundary
- **AND** preview space SHALL be removed before object-count or warning space
- **AND** warnings that still do not fit SHALL remain reachable by horizontal scrolling on unwrapped lines and vertical viewport scrolling on wrapped lines

#### Scenario: Application controls

- **WHEN** focus, search, status, or command entry is active
- **THEN** status and commands SHALL remain outside document text
- **AND** focus and search SHALL highlight existing spans without inserting text
- **AND** default-theme focus SHALL use bright foreground colors instead of bold and SHALL apply on all visible physical rows belonging to the selected logical display line
- **AND** default-theme line numbers and collapse arrows SHALL use dark gray ordinarily and light gray on the selected display line

### Requirement: Theme selection background fills the row

When a theme defines a distinct selection background, each visible physical row of the selected logical display line SHALL fill the terminal width with it, including indentation, spaces, gutters, and clipping markers. Themes without a distinct selection background SHALL retain their appearance. Search matches SHALL retain their theme search style over the row background.

#### Scenario: Borealis selected row

- **WHEN** a short document line is selected under Borealis
- **THEN** the selection background SHALL extend through unused columns to the terminal edge
- **AND** search spans SHALL retain their search background

## ADDED Requirements

### Requirement: Optional line wrapping

Ctrl+L in the document view SHALL toggle wrapping for all eligible lines for the current session, initially disabled. Each press SHALL toggle once regardless of a numeric prefix. Command and search prompts SHALL retain their input-editor behavior. The application SHALL report whether wrapping is on or off and document the key in interactive help.

Wrapping SHALL operate after normal TOON layout selection and indentation reduction. It SHALL NOT change primitive-array layout decisions, parsed values, collapse states, export contents, or piped output. Expanded lines, including keys, scalar values, table headers, table rows, and their warning annotations, SHALL wrap greedily at grapheme boundaries using terminal cell widths, without word-boundary preference. Continuations SHALL start at the document area's left edge, after the reserved number and collapse-arrow gutter. Original indentation SHALL appear only as part of the original line text, without adding repeated indentation or key prefixes.

Only the first physical row SHALL show the logical line's number and collapse arrow. Continuation gutters SHALL remain blank even when the first row is above the viewport. Wrapping SHALL NOT add clipping ellipses at ordinary wrap boundaries. Syntax, focus, and search styling SHALL continue over the original spans.

Collapsed container lines, including their previews, counts, and warnings, SHALL remain one physical row with existing preview fitting and horizontal scrolling. Root separators SHALL remain single rows. While wrapping is enabled, horizontal scroll commands SHALL have no effect on eligible expanded lines. Enabling wrapping SHALL clear their previous horizontal offsets; disabling wrapping SHALL start at the left edge and then reveal the selected span or active search match as needed.

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
