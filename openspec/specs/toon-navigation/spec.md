# toon-navigation Specification

## Purpose

Keep every parsed value reachable and identifiable when TOON places multiple logical values on one display line or hides them through collapse.

## Requirements

### Requirement: Logical focus independent of lines

The viewer SHALL distinguish container focus from child-value focus even when they share a display line. It SHALL support focus on individual inline-array elements, table row objects, and table cells without changing the TOON layout. A cell's path SHALL include its array index and field key. A duplicate occurrence SHALL retain a distinct selection identity even if its textual path equals another occurrence's path.

#### Scenario: Table cell identity

- **WHEN** a user focuses `Lin` in the second row of `users[2]{id,name}:`
- **THEN** its path SHALL identify `users[1].name`
- **AND** copying the selected value SHALL select the string `Lin`, not the row or column header

#### Scenario: Duplicate occurrence identity

- **WHEN** a user moves between duplicate `status` entries
- **THEN** each entry SHALL remain independently selectable
- **AND** status SHALL show `occurrence 1 of 2` or `occurrence 2 of 2` alongside its path

### Requirement: Vertical and structural motion

Up/down and counted vertical motions SHALL move over visible display lines, excluding root-separator annotation lines. Table-cell vertical motion SHALL retain the selected field where the destination is a cell-bearing table row. On other lines it SHALL focus that line's owning node. Child, parent, and sibling motions SHALL traverse logical structure, including values sharing a line. Moving into an inline array with `l` or Right Arrow SHALL first switch it to multiline presentation and keep the array selected. Moving into another expanded container SHALL focus its first child; moving into a collapsed container SHALL first expand it. Closing-delimiter matching SHALL have no action or help entry.

#### Scenario: Parent and next entry at the parent level

- **WHEN** a node is selected and the user presses `[` or `]`
- **THEN** `[` SHALL select the current parent, and `]` SHALL select that parent's next sibling
- **AND** the destination itself SHALL be selected without descending into its children or changing collapse state
- **AND** `[` SHALL fall back to the current node's previous sibling when there is no parent
- **AND** `]` SHALL fall back to the current node's next sibling only when there is no parent
- **AND** `]` SHALL leave focus unchanged when a parent exists but has no next sibling
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
- **AND** focus and horizontal scrolling SHALL reveal the matching cell

#### Scenario: Header key match

- **WHEN** search selects a row's `name` key whose spelling is displayed only in the table header
- **THEN** the selected row field SHALL remain the match owner
- **AND** the header spelling SHALL be highlighted and brought into view

### Requirement: Viewport and selection stability

Resize, collapse, expansion, and horizontal scrolling SHALL preserve logical focus where that value remains visible. Hiding the focused value SHALL move focus to the nearest visible collapsed ancestor. Mouse selection SHALL resolve data spans to their values; clicking a collapse arrow SHALL affect its owning container. Clicking a generated warning SHALL select its owning node without treating warning text as data.

#### Scenario: Collapse focused ancestor

- **WHEN** a command collapses an ancestor of the focused cell
- **THEN** focus SHALL move to that ancestor
- **AND** later expansion SHALL preserve the node's data identity and descendant collapse states

#### Scenario: Mouse selection and resize

- **WHEN** a user clicks an inline-array element and resizes the terminal
- **THEN** that element SHALL remain selected
- **AND** the viewer SHALL scroll as needed to keep its data span visible

### Requirement: Numbered jumps follow TOON lines

Absolute line jumps SHALL address the fully expanded TOON layout used by absolute gutters. A target hidden by collapse SHALL select the visible collapsed ancestor unless the command explicitly requests revealing the target. A jump to a separator SHALL select the following root. All values sharing a TOON line SHALL share its jump address; a line jump SHALL initially focus the line's owning node.

#### Scenario: Jump into collapsed content

- **WHEN** an absolute jump targets a line inside a collapsed object
- **THEN** the normal jump SHALL focus the collapsed object
- **AND** a reveal-target jump SHALL expand ancestors and focus the target line's owning node
