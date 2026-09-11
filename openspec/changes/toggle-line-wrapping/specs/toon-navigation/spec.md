## MODIFIED Requirements

### Requirement: Vertical and structural motion

Up/down and counted vertical motions SHALL move over visible logical display lines, excluding root-separator annotation lines and soft-wrap continuations. A wrapped line SHALL count as one entry for counted motions. Table-cell vertical motion SHALL retain the selected field where the destination is a cell-bearing table row. On other lines it SHALL focus that line's owning node. Child, parent, and sibling motions SHALL traverse logical structure, including values sharing a line. `J` SHALL stop at the final sibling without moving to the parent or wrapping, including when a numeric count exceeds the remaining siblings. Moving into an inline array with `l` or Right Arrow SHALL first switch it to multiline presentation and keep the array selected. Moving into another expanded container SHALL focus its first child; moving into a collapsed container SHALL first expand it. Closing-delimiter matching SHALL have no action or help entry.

#### Scenario: Parent and next entry at the parent level

- **WHEN** a node is selected and the user presses `[` or `]`
- **THEN** `[` SHALL select the current parent, and `]` SHALL select that parent's next sibling
- **AND** the destination itself SHALL be selected without descending into its children or changing collapse state
- **AND** `[` SHALL fall back to the current node's previous sibling when there is no parent
- **AND** `]` SHALL fall back to the current node's next sibling when its parent has no next sibling or there is no parent
- **AND** focus SHALL stay unchanged when neither the preferred target nor its fallback exists
- **AND** this SHALL use parsed parent identity for inline array elements and table cells

#### Scenario: Table motion

- **WHEN** focus is on the name cell of a table row and the user moves down to another expanded row
- **THEN** focus SHALL move to that row's name cell
- **AND** a parent motion SHALL focus the row object, followed by the array on a second parent motion

#### Scenario: Expand an inline array

- **WHEN** an inline array is selected and the user presses `l` or Right Arrow
- **THEN** its values SHALL move onto individual list lines and the array SHALL remain selected
- **AND** the next child motion SHALL focus its first element
- **AND** opening a collapsed inline array with that motion SHALL also select multiline presentation

#### Scenario: Inline siblings

- **WHEN** an inline-array element is focused and the user requests the next sibling
- **THEN** focus SHALL move to the next element on the same line
- **AND** vertical scrolling SHALL NOT occur unless needed to reveal the line

#### Scenario: Root focus

- **WHEN** parent motion reaches a nonempty root object
- **THEN** root selection SHALL be available through its first display line and application status
- **AND** the viewer SHALL NOT insert a synthetic root heading
- **AND** an empty root SHALL remain selectable using one blank viewport row

### Requirement: Search maps data to rendered spans

Search SHALL retain the existing key/value matching behavior over parsed content and map each match to its logical node and displayed span. It SHALL NOT search generated warnings, previews, gutters, or counts as extra data. Selecting a hidden match SHALL expand the necessary ancestors and reveal the matching element or cell. Matches in table field keys SHALL select the corresponding logical field occurrence and highlight the shared header spelling.

#### Scenario: Hidden table match

- **WHEN** search selects `Lin` inside a collapsed table
- **THEN** the array and any collapsed owning row SHALL expand
- **AND** focus and viewport scrolling SHALL reveal the matching cell, using vertical scrolling to its continuation row when wrapping is enabled and horizontal scrolling otherwise

#### Scenario: Header key match

- **WHEN** search selects a row's `name` key whose spelling is displayed only in the table header
- **THEN** the selected row field SHALL remain the match owner
- **AND** the header spelling SHALL be highlighted and brought into view

### Requirement: Viewport and selection stability

Wrap toggles, resize, gutter or indentation changes, collapse, expansion, and viewport scrolling SHALL preserve logical focus where that value remains visible. Hiding the focused value SHALL move focus to the nearest visible collapsed ancestor. Mouse selection SHALL resolve data spans to their values; clicking a collapse arrow SHALL affect its owning container. Clicking a generated warning SHALL select its owning node without treating warning text as data.

#### Scenario: Collapse focused ancestor

- **WHEN** a command collapses an ancestor of the focused cell
- **THEN** focus SHALL move to that ancestor
- **AND** later expansion SHALL preserve the node's data identity and descendant collapse states

#### Scenario: Mouse selection and resize

- **WHEN** a user clicks an inline-array element and resizes the terminal
- **THEN** that element SHALL remain selected
- **AND** the viewer SHALL scroll as needed to keep its data span visible

## ADDED Requirements

### Requirement: Physical viewport scrolling for wrapped lines

With wrapping enabled, viewport scroll commands and the mouse wheel SHALL move in physical screen rows. Full-page and half-page commands SHALL use the document viewport height and their existing overlap and count conventions, measured in physical rows. These commands SHALL be able to reveal every continuation of a line taller than the viewport. They SHALL preserve the focused logical node while any part of its line remains visible, moving focus to a visible logical entry only when the entire focused line leaves the viewport. Scroll padding SHALL yield when a focused line cannot fit in the available height.

Logical focus motions and numbered jumps SHALL reveal the destination's selected span, or the first row for a line-owner jump. Search SHALL reveal the row containing the start of the active match, and show the whole match if it fits in the viewport. Top, middle, and bottom screen selection commands SHALL resolve the physical target row to its owning logical entry. Repositioning the focused line SHALL align the selected span's row to the requested screen position, clamped at document boundaries.

A click on continuation text SHALL select the original value or cell using its original span mapping. A click in a blank continuation gutter SHALL NOT toggle collapse. Reflow SHALL preserve logical focus and the active search match, and reveal that match or selected span after rebuilding the viewport.

#### Scenario: Move past a wrapped value

- **WHEN** focus is on a value occupying 4 physical rows and the user moves down once
- **THEN** focus SHALL move to the next logical display line
- **AND** moving up once SHALL return to the wrapped value
- **AND** counted motions and relative numbering SHALL count the value once
- **AND** absolute jumps SHALL retain the addresses of the unwrapped logical layout

#### Scenario: Read a value taller than the screen

- **WHEN** one expanded value occupies more rows than the document viewport
- **THEN** Ctrl+E, Ctrl+Y, page commands, and the mouse wheel SHALL allow traversal of all its continuations
- **AND** scrolling within that value SHALL retain its selection and path
- **AND** repeated scrolling SHALL stop at document boundaries without skipping an unreachable portion

#### Scenario: Search and click in continuations

- **WHEN** a search match or clicked table cell appears on a continuation row
- **THEN** the viewer SHALL select its original logical node and preserve its path and copy target
- **AND** a search match spanning a wrap boundary SHALL retain highlighting on both sides
- **AND** the active match's start SHALL be visible after search reveal
- **AND** clicking the blank continuation gutter SHALL leave collapse states unchanged

#### Scenario: Reflow while viewing a long value

- **WHEN** wrapping is toggled or the terminal, gutters, or indentation reduction changes while a long value or search match is selected
- **THEN** the same logical node and match SHALL remain selected
- **AND** the viewer SHALL recompute physical rows and reveal the selection or match within the document viewport
- **AND** collapse states and explicit multiline-array choices SHALL survive
