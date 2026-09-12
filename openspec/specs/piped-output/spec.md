# piped-output Specification

## Purpose

Let shell pipelines select JSON, YAML, or standard TOON output independently of the input format, with predictable framing and errors.

## Requirements

### Requirement: Independent output selection

The CLI SHALL accept `-o <format>` and `--output-format <format>`, with lowercase values `json`, `yaml`, and `toon`, defaulting to `toon`. Input selection SHALL accept `-i <format>` and `--input-format <format>`. Output selection SHALL govern non-terminal stdout, including pipes and redirected files. Extension detection, stdin defaults, and `-` input SHALL retain their meanings. An output selector SHALL NOT imply an input format or force noninteractive operation.

#### Scenario: Default pipeline

- **WHEN** a build reads `{"a":1}` as JSON with non-terminal stdout and no output selector
- **THEN** stdout SHALL contain exactly `a: 1`, without a final newline
- **AND** exit status SHALL be 0 and stderr SHALL be empty

#### Scenario: Explicit conversion

- **WHEN** supported JSON, YAML, or TOON input describing `{"a":1}` is read with `-o json`, `--output-format yaml`, or `--output-format=toon`
- **THEN** stdout SHALL use the requested output format regardless of the input format
- **AND** explicit input selectors SHALL continue to override filename extensions

#### Scenario: Invalid selector

- **WHEN** an output value is missing or is not one of the accepted lowercase values
- **THEN** argument parsing SHALL exit with status 2, a diagnostic on stderr, and no stdout payload

#### Scenario: Terminal stdout

- **WHEN** stdout is a terminal and the user supplies a valid output selector
- **THEN** tless SHALL open the existing TOON viewer
- **AND** its rendering, copy commands, and interactive write commands SHALL retain their existing behavior

### Requirement: Parsed standard TOON output

Non-terminal TOON output SHALL parse the input and use the existing standard whole-document TOON export contract. It SHALL use two-space indentation, comma delimiters, no key folding, and no final newline. It SHALL NOT include generated display warnings, gutters, styles, previews, or status text. Existing export normalization and documented codec limitations SHALL apply, including last-value-wins duplicate keys, numeric conversion, possible table-field reordering, and the known empty-object-array encoding limitation. Success SHALL NOT imply an exact round trip.

#### Scenario: Equivalent inputs

- **WHEN** supported JSON, YAML, and TOON inputs produce equivalent exportable parsed values
- **THEN** default output and explicit `-o toon` SHALL produce the same standard export bytes
- **AND** same-format TOON input SHALL be decoded and re-encoded rather than passed through verbatim

#### Scenario: Empty root object

- **WHEN** the parsed document is one empty object, including empty TOON input
- **THEN** TOON output SHALL succeed with an empty payload

#### Scenario: Unsupported whole document

- **WHEN** TOON output encounters zero parsed roots, multiple roots, non-string mapping keys, non-finite values, unsupported numeric conversion, or more than 256 nested containers
- **THEN** it SHALL fail with status 1 and an actionable stderr diagnostic before writing stdout
- **AND** it SHALL NOT silently fall back to JSON, wrap roots in an array, or substitute display extensions

#### Scenario: Duplicate keys

- **WHEN** JSON `{"a":1,"a":2}` is converted to TOON
- **THEN** stdout SHALL contain `a: 2` without a final newline
- **AND** no generated duplicate-key warning SHALL appear in the payload

### Requirement: Explicit JSON serialization

JSON output SHALL pretty-print each parsed root with two-space indentation and a final LF per root. Multiple roots SHALL remain a sequence of pretty-printed JSON values in input order, without a synthetic array. JSON input SHALL retain its current number spellings, duplicate entries, and entry order. Conversions from other input formats SHALL serialize decoded strings with JSON escaping and reject non-string keys or non-finite numbers rather than emitting invalid JSON. Existing input-parser normalization SHALL still apply.

#### Scenario: JSON compatibility override

- **WHEN** JSON input is redirected with `-o json`
- **THEN** its output SHALL match the previous JSON pretty-printing behavior, including multiple roots and duplicate entries

#### Scenario: YAML string conversion

- **WHEN** YAML contains strings or string keys with quotes, backslashes, tabs, or newlines and output is JSON
- **THEN** the output SHALL be valid JSON preserving the parsed string values

#### Scenario: YAML values outside JSON

- **WHEN** parsed YAML contains a numeric mapping key or infinity and output is JSON
- **THEN** conversion SHALL fail with status 1, a stderr diagnostic, and no stdout payload

### Requirement: Explicit YAML serialization

YAML output SHALL serialize parsed roots as a YAML document stream, with `---` on its own line before each root and a final LF per document. It SHALL preserve parsed scalar types, array order, and mapping entries and their order, including duplicate entries still present in the parsed model. It SHALL support parsed non-string YAML keys and non-finite numbers. It SHALL NOT promise preservation of source comments, anchors, formatting, or information already normalized by an input parser. JSON and YAML output of zero parsed roots SHALL succeed with an empty payload.

#### Scenario: YAML document stream

- **WHEN** input contains multiple parsed roots and output is YAML
- **THEN** each root SHALL become a separate YAML document in input order
- **AND** the roots SHALL NOT be wrapped in an array

#### Scenario: Typed values and keys

- **WHEN** parsed data contains the strings `true` and `.inf`, a numeric infinity, and both numeric key `1` and string key `"1"`
- **THEN** YAML output SHALL preserve these type distinctions through appropriate quoting and key syntax

#### Scenario: Long and escape-heavy string keys

- **WHEN** a parsed string mapping key exceeds YAML's implicit-key length limit after quoting and escaping
- **THEN** YAML output SHALL use explicit key syntax and preserve the entire decoded key and its value
- **AND** this SHALL apply to both JSON and YAML input

### Requirement: Feature availability

JSON, YAML, and standard TOON output SHALL be available in every build profile. All builds SHALL recognize the three output selector values, and TOON input SHALL be available in every build profile. Optional `colorscheme` and `sexp` features SHALL NOT gate TOON parsing, rendering, export, or interactive commands.

#### Scenario: Every build profile

- **WHEN** a build with no default features reads valid JSON with non-terminal stdout
- **THEN** default output and explicit `-o toon` SHALL succeed
- **AND** `-o json` and `-o yaml` SHALL succeed
- **AND** terminal stdout SHALL still open the viewer with TOON rendering and commands

### Requirement: Machine output failures and resource bounds

The complete input SHALL be loaded, parsed, and serialized before stdout writing begins. The existing configurable input-byte limit SHALL apply to files and stdin. Input, parse, serialization, and write failures SHALL exit with status 1 and diagnostics only on stderr. A write or flush error, including a broken pipe, SHALL be reported without a panic; it can leave a partial payload. Machine operation SHALL NOT initialize terminal controls or require a controlling terminal.

#### Scenario: Invalid same-format input

- **WHEN** malformed YAML or TOON is selected as input, even with matching output selection
- **THEN** tless SHALL report a parse failure and write no stdout payload

#### Scenario: Resource or downstream failure

- **WHEN** input exceeds `--max-input-bytes` or stdout writing or flushing fails
- **THEN** tless SHALL exit with status 1 and a stderr diagnostic
- **AND** an input-limit failure SHALL leave stdout empty

### Requirement: Help and migration guidance

Help and user documentation SHALL describe the output selector, TOON default, feature availability, per-format framing, validation, and normalization. Release notes SHALL mark the default change as breaking and show `tless -o json` as the migration for JSON pipelines. Documentation SHALL state that explicit YAML output reserializes input and does not restore byte-for-byte pass-through.

#### Scenario: Discovering the override

- **WHEN** a user reads `tless --help`
- **THEN** it SHALL list `-o` / `--output-format`, the accepted formats, the TOON default, and the non-terminal scope
