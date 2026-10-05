# Spec Delta

## MODIFIED Requirements

### Requirement: Search maps data to rendered spans

Search SHALL retain the existing key/value matching behavior over parsed content and map each match to its logical node and displayed span. It SHALL NOT search generated alignment padding, warnings, previews, gutters, or counts as extra data. Selecting a hidden match SHALL expand the necessary ancestors and reveal the matching element or cell. Matches in table field keys SHALL select the corresponding logical field occurrence and highlight the shared header spelling.

With a path filter active, search and repeat-search SHALL enumerate matches only within selected values, including descendants hidden by collapse. The selected root's omitted owning key SHALL NOT be searched. Excluded nodes SHALL NOT contribute matches or match counts. Revealing a match SHALL expand ancestors only as far as its selected root.

#### Scenario: Hidden table match

- **WHEN** search selects `Lin` inside a collapsed table
- **THEN** the array and any collapsed owning row SHALL expand
- **AND** focus and viewport scrolling SHALL reveal the matching cell, using vertical scrolling to its continuation row when wrapping applies to that line and horizontal scrolling otherwise, using the shared table offset for an aligned table

#### Scenario: Header key match

- **WHEN** search selects a row's `name` key whose spelling is displayed only in the table header
- **THEN** the selected row field SHALL remain the match owner
- **AND** the header spelling SHALL be highlighted and brought into view

#### Scenario: Search cannot escape the filter

- **WHEN** a search term exists only outside the selected subtrees
- **THEN** search SHALL report no match and SHALL leave the active filter unchanged

#### Scenario: Search reveal preserves aligned columns

- **WHEN** search selects a clipped value or field key in an aligned table
- **THEN** the shared table offset SHALL reveal the match and move the header and all rows together
- **AND** padding SHALL add no search matches or source characters
- **AND** the original row field SHALL remain the match and copy owner


## ADDED Requirements

### Requirement: Shared horizontal viewport for aligned tables

An expanded aligned table SHALL use one horizontal offset for its header and all rows. From its header, row, or cell, `,` and `.` SHALL move that offset by ten terminal cells times the numeric prefix. `;` SHALL toggle the shared end/start position. Bounds SHALL use the complete aligned table, including syntax and annotations, not the focused row. Other lines and collapsed previews SHALL retain ordinary scrolling.

#### Scenario: Counted shared scrolling

- **WHEN** the user presses `2.` then `,` while focused on any row of an aligned table
- **THEN** its header and all rows SHALL move right by twenty cells then left by ten cells, subject to shared bounds
- **AND** newly exposed rows SHALL use the same offset
- **AND** focus and unrelated lines' offsets SHALL remain unchanged

#### Scenario: Table-wide bounds and end toggle

- **WHEN** the focused row is shorter than another row or annotation and the user scrolls right or presses `;`
- **THEN** scrolling SHALL allow access to the longest complete table line without clamping to the focused row
- **AND** `;` SHALL use that table's shared end position and then return to the shared left edge
- **AND** repeated left/right movement and oversized counts SHALL stop safely at table boundaries

#### Scenario: Offset lifecycle

- **WHEN** alignment is enabled or disabled
- **THEN** the table SHALL start at the left edge and reveal the selected span or active match as needed, without importing stale per-line offsets
- **AND** while alignment stays enabled its shared offset SHALL survive vertical focus motion, collapse/reopen, and leaving and returning to the table
- **AND** reflow or filtering SHALL preserve that offset where valid, clamp to changed bounds, and reveal the selection or active match as needed

### Requirement: Data identity in padded table presentation

Alignment SHALL preserve each row and cell's parsed identity, path, navigation, copy target, and line address. Search highlights and mouse selection SHALL resolve actual header/value tokens through the padded presentation. Generated padding SHALL NOT belong to a value's source span. Clicking padding SHALL select the logical line owner, not an adjacent cell.

#### Scenario: Select a padded cell

- **WHEN** a user clicks a value after alignment padding and shared horizontal scrolling
- **THEN** its original row and field SHALL be selected with the same path and copy target as in unaligned presentation
- **AND** vertical cell motion SHALL retain the selected field in the next table row
- **AND** toggling alignment SHALL keep that cell selected

#### Scenario: Padding and shared header identity

- **WHEN** a user clicks generated padding on an aligned row or header
- **THEN** focus SHALL select the row object or table array respectively without selecting an adjacent field
- **AND** clicking a field spelling or selecting a field-key search match SHALL retain the existing shared-header identity rules
- **AND** alignment SHALL NOT add logical lines or change absolute jump addresses
