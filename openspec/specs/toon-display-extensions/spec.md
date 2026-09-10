# toon-display-extensions Specification

## Purpose

Preserve parsed data that standard TOON cannot represent faithfully and make each display extension visible through subdued warning comments.

## Requirements

### Requirement: Explicit display extension boundary

The viewer SHALL preserve the values, types, entry occurrences, and order available in its parsed model. It SHALL NOT coerce, discard, or reject such data solely to fit standard TOON. Extended content SHALL be labeled with generated subdued comments beginning `# WARN `. This extension SHALL be a display contract, not an alternate view mode or an additional input/export format. Existing parser normalization, parse errors, resource limits, and export behavior SHALL remain unchanged.

#### Scenario: Standard data

- **WHEN** parsed data requires no extension
- **THEN** the viewer SHALL emit no warning comments
- **AND** fully expanded document content SHALL conform to the standard rendering profile

#### Scenario: Information already normalized by a parser

- **WHEN** an existing input parser has already discarded a duplicate or rounded a number
- **THEN** rendering SHALL preserve the resulting parsed value
- **AND** it SHALL NOT claim to recover the original input or invent a warning without evidence in the parsed model

### Requirement: Duplicate object entries

The viewer SHALL render every duplicate object entry in encounter order and attach `# WARN Duplicate key` to every occurrence of a repeated decoded key. Escaped spellings of the same key SHALL compare equal. Arrays containing duplicate-key objects SHALL use list form, not tables. Duplicate occurrences SHALL remain distinct navigation and selection targets.

#### Scenario: Two statuses

- **WHEN** parsed JSON contains `{"status":"queued","status":"done"}`
- **THEN** it SHALL display `status: queued  # WARN Duplicate key` and `status: done  # WARN Duplicate key` on separate lines
- **AND** a collapsed containing object SHALL count 2 entries

#### Scenario: Equivalent key spellings

- **WHEN** parsed JSON contains keys `"a"` and `"\u0061"` in the same object
- **THEN** both occurrences SHALL receive the duplicate-key warning

### Requirement: Non-finite and non-canonical numbers

Non-finite numeric values SHALL remain numeric and display as `.inf`, `-.inf`, or `.nan`, followed by `# WARN Non-finite number`. Standard finite numeric values SHALL render without rounding or conversion to strings. When canonical decimal rendering would require more than 4096 characters, or a parsed numeric token cannot be expressed by the standard profile without changing its value, the viewer SHALL retain its parsed numeric spelling and append `# WARN Non-canonical number`. Literal strings that resemble these tokens SHALL be quoted.

#### Scenario: Infinity and a similar string

- **WHEN** parsed YAML contains numeric infinity and the string `.inf`
- **THEN** the number SHALL display as `.inf  # WARN Non-finite number`
- **AND** the string SHALL display as `".inf"` without a generated warning
- **AND** neither value SHALL become null

#### Scenario: Exact decimal and large exponent

- **WHEN** parsed JSON contains `0.123456789012345678901` and `1e1000000`
- **THEN** the decimal SHALL retain all significant digits
- **AND** the exponent SHALL display as `1e1000000  # WARN Non-canonical number`
- **AND** rendering SHALL NOT allocate a million-character decimal expansion

### Requirement: Non-string keys and multiple roots

A non-string YAML key SHALL use the extension spelling `? <compact-key>: <value>` with `# WARN Non-string key`. Compact keys SHALL retain their parsed type using JSON-style strings, literals, arrays, and ordered object entries, with the numeric extensions where needed. String keys that could resemble extension syntax SHALL be quoted. Multiple parsed roots SHALL retain their order, each preceded by `---  # WARN Multiple document roots`. The viewer SHALL NOT wrap roots in a synthetic array or rename non-string keys into strings.

#### Scenario: Numeric and string keys

- **WHEN** a YAML object contains numeric key `1` and string key `"1"`
- **THEN** the numeric key SHALL display as `? 1: value  # WARN Non-string key`
- **AND** the string key SHALL display as `"1": value`
- **AND** both entries SHALL retain distinct identities

#### Scenario: Several documents

- **WHEN** parsing yields an object root and a primitive root
- **THEN** each SHALL retain its root shape after its warning separator
- **AND** the separator SHALL NOT be a selectable data value

### Requirement: Unsupported control-character escapes

String values and string keys containing control characters other than LF, CR, and TAB SHALL use terminal-safe JSON-style `\uXXXX` spellings and receive `# WARN Non-standard string escape`. These spellings SHALL be labeled as display extensions because TOON 3.0 does not support Unicode escape sequences. The warning SHALL also apply to such strings inside compact typed keys. Literal backslash-u text SHALL remain string data without this warning. Input parsing and export behavior SHALL remain unchanged.

#### Scenario: Unsafe control and literal escape text

- **WHEN** parsed JSON contains string values `"\u0001"` and `"\\u0001"`
- **THEN** the control character SHALL display as `"\u0001"  # WARN Non-standard string escape`
- **AND** the literal backslash-u value SHALL retain its characters without a generated warning
- **AND** neither SHALL write a raw control character to the terminal

### Requirement: Warning placement and collapse

Warnings SHALL be generated annotation spans, separated from preceding content by 2 spaces and rendered subdued. Multiple warnings on a line SHALL use one `# WARN ` prefix and semicolon-separated messages ordered first by parsed-node encounter order. Within each node, messages SHALL follow this kind order: duplicate key, non-finite number, non-canonical number, non-string key, multiple roots, non-standard string escape. Any `Contains N hidden warnings` summary SHALL follow the container's own messages and appear last. Inline-array and table warnings SHALL identify the affected zero-based element or field. A collapsed container SHALL retain warnings about itself and append `Contains N hidden warnings` for warnings on hidden descendants. A warning SHALL never appear as ordinary source string content or as an extra search match.

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

### Requirement: Serialization remains separate

Copying a selected value or invoking an existing export SHALL operate on the selected parsed value and SHALL NOT include warning comments, arrows, count annotations, or previews. Existing TOON exports SHALL retain their documented normalization and failure behavior. Help SHALL explicitly state that extended display text is not standard TOON and that standard exports can normalize data differently from the viewer.

#### Scenario: Duplicate export

- **WHEN** a user invokes the existing TOON export on a displayed duplicate-key object
- **THEN** export SHALL retain its documented last-value-wins behavior
- **AND** the display SHALL continue to show both entries with warnings
- **AND** the export SHALL NOT contain generated warning text
