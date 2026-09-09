# toon-rendering Specification

## Purpose

Provide one syntax-colored TOON document view with predictable collapse annotations, optional gutters, and stable layout across terminal sizes.

## Requirements

### Requirement: One rendering contract

The viewer SHALL render every supported input format with the TOON 3.0 profile: 2-space indentation, comma delimiters, and no key folding. The documented display extensions SHALL apply where that profile cannot faithfully represent the parsed data. Every build profile SHALL provide this view. Input parser and export feature selection SHALL retain their existing meaning.

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
- **AND** disabled input or export features SHALL remain disabled

### Requirement: Native TOON layout

Fully expanded standard-compatible data SHALL use TOON object fields, inline primitive arrays, uniform primitive-only object tables, and list arrays for other structures. Tables SHALL require nonempty, unique string field sets shared by every row. Array order and object entry order SHALL be preserved. When a table would reorder a row's fields, the array SHALL use list form. Stripping presentation styling and gutters from fully expanded standard-compatible content SHALL leave valid TOON text.

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

### Requirement: Syntax styling

Keys and table field names SHALL share a syntax category. Strings, numbers, booleans, nulls, and structural syntax SHALL have distinguishable styles. TOON array counts SHALL use structural styling. Expanded data SHALL retain its data styling. Only collapsed previews, object count annotations, and extension warning comments SHALL use subdued annotation styling. These annotations SHALL remain subdued without focus highlighting or bold when their owning container is selected.

#### Scenario: Expanded primitive array

- **WHEN** `values[3]: 1,true,hello` is expanded
- **THEN** its values SHALL have number, boolean, and string styles
- **AND** these values SHALL NOT be styled as a collapsed preview

### Requirement: Collapse presentation

Nonempty collapsible containers SHALL show `▾` when expanded and `▸` when collapsed in a reserved gutter outside TOON indentation. Collapsing SHALL retain the container's header, hide its contents, and append a subdued preview in document order. A collapsed object SHALL show its immediate-entry count as `{N}`, such as `{1}` or `{7}`; duplicate entries SHALL each count. Arrays SHALL retain their TOON count and SHALL NOT receive a second count annotation. Expanded containers SHALL have no preview or object-count annotation.

#### Scenario: Object and array collapse

- **WHEN** `owner` has 2 immediate fields and `tags` has 3 items, and both are collapsed
- **THEN** their lines SHALL retain `owner:` and `tags[3]:`
- **AND** only `owner` SHALL receive `{2}`
- **AND** both previews SHALL use subdued TOON-style spelling and escaping

#### Scenario: Tabular row collapse

- **WHEN** a user collapses an object row in a table
- **THEN** that row's values SHALL be replaced by a subdued entry count and preview at the existing indentation
- **AND** expanding it SHALL restore the original table row
- **AND** collapsing a row SHALL NOT change the surrounding array to list form

#### Scenario: Inline array collapse

- **WHEN** a user collapses an inline primitive array
- **THEN** the header SHALL remain and its values SHALL become a subdued preview
- **AND** expanding it SHALL restore syntax-colored inline values

#### Scenario: Collapse restoration

- **WHEN** a collapsed ancestor is expanded again
- **THEN** descendant collapse states SHALL be restored
- **AND** the underlying data and layout choice SHALL be unchanged

### Requirement: Gutters and terminal width

Optional absolute and relative line numbers SHALL remain available outside the document text with existing visibility defaults. Absolute numbers SHALL identify lines in the fully expanded TOON layout, beginning at 1; collapsed descendants SHALL produce gaps. Relative numbers SHALL count visible display-line motions. Long expanded lines SHALL use horizontal scrolling without soft wrapping or width-dependent changes between array forms. Rendering SHALL escape control characters and respect terminal cell widths.

#### Scenario: Shared-line numbering

- **WHEN** focus moves between cells on a table row or elements in an inline array
- **THEN** the absolute line number SHALL remain unchanged
- **AND** relative vertical distance between those values SHALL be 0

#### Scenario: Narrow viewport

- **WHEN** the terminal cannot fit the full content or preview
- **THEN** expanded content SHALL remain reachable by horizontal scrolling
- **AND** collapsed previews SHALL truncate with `…` at a valid character boundary
- **AND** preview space SHALL be removed before object-count or warning space
- **AND** warnings that still do not fit SHALL remain reachable by horizontal scrolling

#### Scenario: Application controls

- **WHEN** focus, search, status, or command entry is active
- **THEN** status and commands SHALL remain outside document text
- **AND** focus and search SHALL highlight existing spans without inserting text
- **AND** focus SHALL use bright foreground colors instead of bold and SHALL apply only on the selected display line
- **AND** the current line-number gutter SHALL use a lighter shade than other line numbers
