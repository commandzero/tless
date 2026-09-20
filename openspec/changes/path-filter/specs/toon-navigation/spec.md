## MODIFIED Requirements

### Requirement: Vertical and structural motion

Up/down and counted vertical motions SHALL move over visible logical display lines, including sequence document rows and excluding soft-wrap continuations. A wrapped line SHALL count as one entry for counted motions. Table-cell vertical motion SHALL retain the selected field where the destination is a cell-bearing table row. On other lines it SHALL focus that line's owning node. Child, parent, and sibling motions SHALL traverse logical structure, including values sharing a line. `J` SHALL stop at the final sibling without moving to the parent or wrapping, including when a numeric count exceeds the remaining siblings. Moving into an inline array from its data line with `l` or Right Arrow SHALL first switch it to multiline presentation and keep the array selected. Moving into another expanded container SHALL focus its first child; moving into a collapsed container SHALL first expand it. From a collapsed sequence document row, `l` or Right Arrow SHALL first expand the document and keep its row selected. From an expanded document row it SHALL focus the first logical child, if any, without an intermediate root selection. Closing-delimiter matching SHALL have no action or help entry.

When a path filter is active, structural and vertical navigation SHALL operate on selected roots and their descendants only. Each selected root SHALL act as parentless for navigation without changing its parsed parent identity. Sibling motions between selected sequence roots SHALL remain available. No motion, count, fallback, mouse action, or reveal operation SHALL select excluded ancestors or siblings. Existing boundary fallbacks SHALL be evaluated within this scope.

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

- **WHEN** an inline array data line is selected and the user presses `l` or Right Arrow
- **THEN** its values SHALL move onto individual list lines and the array SHALL remain selected
- **AND** the next child motion SHALL focus its first element
- **AND** opening a collapsed inline array with that motion SHALL also select multiline presentation

#### Scenario: Inline siblings

- **WHEN** an inline-array element is focused and the user requests the next sibling
- **THEN** focus SHALL move to the next element on the same line
- **AND** vertical scrolling SHALL NOT occur unless needed to reveal the line

#### Scenario: Root focus

- **WHEN** parent motion reaches a nonempty object in a single-root input
- **THEN** root selection SHALL be available through its first display line and application status
- **AND** the viewer SHALL NOT insert a synthetic root heading
- **AND** an empty root SHALL remain selectable using one blank viewport row

#### Scenario: Sequence parent and sibling motion

- **WHEN** a top-level field in document 1 of a sequence is selected
- **THEN** `[` SHALL select document 1's row
- **AND** `]` SHALL select document 2's row if it exists, preserving its collapse state
- **AND** from a document row, sibling motions SHALL move between document rows
- **AND** existing fallback and boundary rules SHALL apply at the first and last documents

#### Scenario: Vertical motion through document rows

- **WHEN** the user moves vertically through a sequence with expanded and collapsed documents
- **THEN** each visible document row SHALL be a selectable step and count toward relative navigation distance
- **AND** a collapsed document SHALL contribute only its document row

#### Scenario: Filtered root boundary

- **WHEN** a selected subtree root is focused and parent or sibling navigation would reach excluded data
- **THEN** focus SHALL remain within the selected-root sequence and SHALL NOT reveal excluded data

### Requirement: Search maps data to rendered spans

Search SHALL retain the existing key/value matching behavior over parsed content and map each match to its logical node and displayed span. It SHALL NOT search generated warnings, previews, gutters, or counts as extra data. Selecting a hidden match SHALL expand the necessary ancestors and reveal the matching element or cell. Matches in table field keys SHALL select the corresponding logical field occurrence and highlight the shared header spelling.

With a path filter active, search and repeat-search SHALL enumerate matches only within selected values, including descendants hidden by collapse. The selected root's omitted owning key SHALL NOT be searched. Excluded nodes SHALL NOT contribute matches or match counts. Revealing a match SHALL expand ancestors only as far as its selected root.

#### Scenario: Hidden table match

- **WHEN** search selects `Lin` inside a collapsed table
- **THEN** the array and any collapsed owning row SHALL expand
- **AND** focus and viewport scrolling SHALL reveal the matching cell, using vertical scrolling to its continuation row when wrapping is enabled and horizontal scrolling otherwise

#### Scenario: Header key match

- **WHEN** search selects a row's `name` key whose spelling is displayed only in the table header
- **THEN** the selected row field SHALL remain the match owner
- **AND** the header spelling SHALL be highlighted and brought into view

#### Scenario: Search cannot escape the filter

- **WHEN** a search term exists only outside the selected subtrees
- **THEN** search SHALL report no match and SHALL leave the active filter unchanged

### Requirement: Numbered jumps follow TOON lines

Absolute line jumps SHALL address the fully expanded TOON layout used by absolute gutters. A target hidden by collapse SHALL select the visible collapsed ancestor unless the command explicitly requests revealing the target. A jump to a sequence document row SHALL select that row and preserve its collapse state. All values sharing a TOON line SHALL share its jump address; a line jump SHALL initially focus the line's owning node.

With a path filter active, absolute gutters and jumps SHALL address the fully expanded filtered layout starting at line 1, including its sequence headers. Relative numbering SHALL count only filtered visible logical lines. Reflow, mouse selection, and scroll bounds SHALL use this same scoped layout.

#### Scenario: Jump into collapsed content

- **WHEN** an absolute jump targets a line inside a collapsed object
- **THEN** the normal jump SHALL focus the collapsed object
- **AND** a reveal-target jump SHALL expand ancestors and focus the target line's owning node

#### Scenario: Filtered line addresses

- **WHEN** a subtree with original-document lines preceding it becomes the selected root
- **THEN** its filtered layout SHALL start at line 1 and line jumps SHALL NOT address excluded original-document lines
