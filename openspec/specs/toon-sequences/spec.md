# toon-sequences Specification

## Purpose

Make each document in a multi-root input selectable and collapsible while preserving its position, parsed data, and standalone TOON indentation.

## Requirements

### Requirement: One row per sequence document

When parsing yields more than one root, the viewer SHALL display one selectable `---` document row per root in encounter order. This SHALL apply to all supported multi-root inputs, including NDJSON, JSONL, concatenated JSON values, and YAML streams, independently of filename extension. Each row SHALL identify that document's root for navigation and value actions. It SHALL act as the parent of the root's immediate fields or array elements without an extra intervening object or array selection. Single-root inputs SHALL retain their existing presentation.

#### Scenario: Equivalent sequences

- **WHEN** NDJSON, JSONL, or a YAML stream produces the same two object roots
- **THEN** each view SHALL show two document rows with the same navigation structure
- **AND** neither view SHALL display `# WARN Multiple document roots`
- **AND** multiplicity alone SHALL contribute no warning count

#### Scenario: Single document

- **WHEN** a JSONL file produces only one parsed root
- **THEN** the viewer SHALL use the existing single-root layout without a sequence row or position annotation

### Requirement: Persistent position and subdued preview

Every document row SHALL show `---` followed by `(i of n)`, where `i` is the one-based document position and `n` is the total parsed root count. The position SHALL appear in both states and remain subdued when selected. Expanded document rows SHALL NOT show a contents preview. Collapsed document rows SHALL append a subdued contents preview after the position. Preview content SHALL follow document order and existing bounded preview conventions, including `; ` between object fields. Collapsed empty roots SHALL have a type-identifying preview. Document rows SHALL remain one physical row with wrapping enabled. Preview text SHALL give up width before the position annotation, with existing horizontal access when annotations cannot fit. Collapse, search, and resizing SHALL NOT renumber documents.

#### Scenario: Expanded and collapsed preview

- **WHEN** the first of three documents contains `{"name":"Ada","active":true}`
- **THEN** its expanded row SHALL show `--- (1 of 3)` without a contents preview
- **AND** its collapsed row SHALL show `--- (1 of 3) name: Ada; active: true` when sufficient width is available
- **AND** expanding it again SHALL remove the contents preview while retaining `(1 of 3)`
- **AND** the collapse gutter SHALL show `▾` when expanded and `▸` when collapsed

#### Scenario: Narrow selected row

- **WHEN** a selected collapsed document row cannot fit its full preview
- **THEN** the preview SHALL truncate at a valid character boundary with `…` before consuming space reserved for `(i of n)`
- **AND** position and preview styling SHALL remain subdued
- **AND** the document row SHALL remain one physical row after a wrap toggle

### Requirement: Document collapse without added indentation

Document rows SHALL start expanded and support the existing shallow and deep collapse/expand controls, including mouse arrows and sibling operations. Collapsing a document SHALL hide its body and retain its row and position and show a subdued contents preview and applicable warning summary. Expanding it SHALL restore descendant collapse states unless a deep operation explicitly changes them. A document row SHALL add no indentation to its body; nested structures SHALL retain their normal relative indentation. Scalar and empty-container documents SHALL remain selectable and support document collapse without inventing child values.

#### Scenario: Independent documents

- **WHEN** the first of two documents is collapsed
- **THEN** its body SHALL disappear while its document row remains visible
- **AND** the second document's presentation and collapse state SHALL remain unchanged
- **AND** reopening the first document SHALL restore its descendant collapse states

#### Scenario: Unindented object fields

- **WHEN** a sequence document contains `{"name":"Ada","details":{"active":true}}`
- **THEN** `name:` and `details:` SHALL start at the same content column as `---`
- **AND** `active:` SHALL keep its normal indentation beneath `details:`

#### Scenario: Mixed root shapes

- **WHEN** a sequence contains an object, an array, a scalar, an empty object, and an empty array
- **THEN** each SHALL have an independently selectable document row with its own position in both states and a contents preview only when collapsed
- **AND** each document SHALL support collapse and expansion without changing its type or adding data

### Requirement: Preserve data identity and reveal hidden matches

Sequence rows and positions SHALL be presentation metadata. Selecting a document row for copy or focused print SHALL target the corresponding parsed root. Paths and duplicate-occurrence identities SHALL retain their existing meanings. Search SHALL match parsed content only, excluding generated positions and previews, and SHALL reveal a hidden match by expanding its document and necessary ancestors. Hiding a selected descendant SHALL move selection to the document row. Whole-document serialization, redirected output, and standard TOON export restrictions SHALL remain unchanged.

#### Scenario: Copy a document

- **WHEN** a collapsed document row is selected and the user copies its value as JSON
- **THEN** the result SHALL be that document's original parsed root value
- **AND** it SHALL contain no `---`, position annotation, preview text, or synthetic wrapper

#### Scenario: Reveal hidden data

- **WHEN** search selects a value hidden inside a collapsed sequence document
- **THEN** the document and required ancestors SHALL expand and reveal that value
- **AND** its existing path and value identity SHALL remain intact
- **AND** generated preview copies of that value SHALL NOT add search matches
