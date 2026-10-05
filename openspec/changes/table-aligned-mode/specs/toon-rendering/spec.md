# Spec Delta

## MODIFIED Requirements

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


## ADDED Requirements

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
