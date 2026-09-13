## MODIFIED Requirements

### Requirement: Explicit display extension boundary

The viewer SHALL preserve the values, types, entry occurrences, and order available in its parsed model. It SHALL NOT coerce, discard, or reject such data solely to fit standard TOON. Extended content SHALL be labeled with generated subdued comments beginning `# WARN `, except multiple roots SHALL use sequence document rows without a multiple-roots warning. This extension SHALL be a display contract, not an alternate view mode or an additional input/export format. Existing parser normalization, parse errors, resource limits, and export behavior SHALL remain unchanged.

#### Scenario: Standard data

- **WHEN** parsed data requires no extension
- **THEN** the viewer SHALL emit no warning comments
- **AND** fully expanded document content SHALL conform to the standard rendering profile

#### Scenario: Information already normalized by a parser

- **WHEN** an existing input parser has already discarded a duplicate or rounded a number
- **THEN** rendering SHALL preserve the resulting parsed value
- **AND** it SHALL NOT claim to recover the original input or invent a warning without evidence in the parsed model

### Requirement: Non-string keys and multiple roots

A non-string YAML key SHALL use the extension spelling `? <compact-key>: <value>` with `# WARN Non-string key`. Compact keys SHALL retain their parsed type using JSON-style strings, literals, arrays, and ordered object entries, with the numeric extensions where needed. String keys that could resemble extension syntax SHALL be quoted. Multiple parsed roots SHALL retain their order, each represented by a selectable `---` sequence document row with a subdued position in both states and a contents preview only when collapsed. Multiple roots alone SHALL NOT generate a warning. The viewer SHALL NOT wrap roots in a synthetic array or rename non-string keys into strings.

#### Scenario: Numeric and string keys

- **WHEN** a YAML object contains numeric key `1` and string key `"1"`
- **THEN** the numeric key SHALL display as `? 1: value  # WARN Non-string key`
- **AND** the string key SHALL display as `"1": value`
- **AND** both entries SHALL retain distinct identities

#### Scenario: Several documents

- **WHEN** parsing yields an object root and a primitive root
- **THEN** each SHALL retain its root shape under its sequence document row without extra indentation
- **AND** each document row SHALL be selectable and collapsible and SHALL identify its existing parsed root without introducing a serialized value

### Requirement: Warning placement and collapse

Warnings SHALL be generated annotation spans, separated from preceding content by 2 spaces and rendered subdued. Multiple warnings on a line SHALL use one `# WARN ` prefix and semicolon-separated messages ordered first by parsed-node encounter order. Within each node, messages SHALL follow this kind order: duplicate key, non-finite number, non-canonical number, non-string key, non-standard string escape. Any `Contains N hidden warnings` summary SHALL follow the container's own messages and appear last. Inline-array and table warnings SHALL identify the affected zero-based element or field. A collapsed container SHALL retain warnings about itself and append `Contains N hidden warnings` for warnings on hidden descendants. A warning SHALL never appear as ordinary source string content or as an extra search match.

#### Scenario: Hidden duplicate

- **WHEN** a container hides 2 duplicate-entry warnings
- **THEN** its collapsed line SHALL include `# WARN Contains 2 hidden warnings`
- **AND** expanding it SHALL show the individual warnings again

#### Scenario: Warning inside a shared line

- **WHEN** the second inline-array element is infinity
- **THEN** the line SHALL end with `# WARN Non-finite number at [1]`
- **AND** every value on the line SHALL remain visible before the comment

#### Scenario: Mixed warning kinds on different nodes

- **WHEN** an inline array contains an unsupported-control string, infinity, and a non-canonical number in that order
- **THEN** its final comment SHALL list `Non-standard string escape at [0]`, `Non-finite number at [1]`, and `Non-canonical number at [2]` in that parsed-node order
- **AND** kind priority SHALL NOT move another node's warning ahead of an earlier node

#### Scenario: Source text resembles a warning

- **WHEN** a string contains `# WARN Duplicate key`
- **THEN** the renderer SHALL quote and escape that string as data
- **AND** warning counts SHALL exclude it
