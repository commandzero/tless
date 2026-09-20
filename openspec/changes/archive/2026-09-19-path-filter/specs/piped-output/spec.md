## MODIFIED Requirements

### Requirement: Machine output failures and resource bounds

The complete input SHALL be loaded and parsed before path resolution; selected roots SHALL be serialized completely before stdout writing begins. The existing configurable input-byte limit SHALL apply to files and stdin. Input, parse, serialization, and write failures SHALL exit with status 1 and diagnostics only on stderr. A write or flush error, including a broken pipe, SHALL be reported without a panic; it can leave a partial payload. Machine operation SHALL NOT initialize terminal controls or require a controlling terminal.

With `--path`, all output formats SHALL serialize the selected roots defined by the path-filter capability. Existing whole-document/root-count validation and per-format framing SHALL apply to the ordered selected roots; there SHALL be no synthetic wrapper and no source owning key. Serialization restrictions SHALL be evaluated on selected data, but invalid input anywhere SHALL still fail parsing. The default output format SHALL remain TOON, including its rejection of multiple selected roots. No filter, `--path .`, and `--path ''` SHALL retain existing whole-input output behavior. Path syntax errors SHALL exit 2 and resolution failures SHALL exit 1, with stderr diagnostics and no stdout payload.

#### Scenario: Invalid same-format input

- **WHEN** malformed YAML or TOON is selected as input, even with matching output selection
- **THEN** tless SHALL report a parse failure and write no stdout payload

#### Scenario: Resource or downstream failure

- **WHEN** input exceeds `--max-input-bytes` or stdout writing or flushing fails
- **THEN** tless SHALL exit with status 1 and a stderr diagnostic
- **AND** an input-limit failure SHALL leave stdout empty

#### Scenario: Filtered JSON and YAML framing

- **WHEN** input is `{"a":1,"other":9} {"a":2}` with `--path .a`
- **THEN** JSON output SHALL be exactly `1\n2\n` and YAML output SHALL be exactly `---\n1\n---\n2\n`
- **AND** default TOON output SHALL fail with status 1 and no stdout payload rather than wrap or discard roots

#### Scenario: Excluded data still must parse

- **WHEN** `--path .a` selects an early valid value but the remaining input is malformed
- **THEN** parsing SHALL fail with status 1 and empty stdout

#### Scenario: Excluded serialization restrictions

- **WHEN** valid YAML input contains `a: 1` and a non-string mapping key outside `a`, and `--path .a -o json` is supplied
- **THEN** stdout SHALL be exactly `1\n` because the selected scalar is JSON-exportable

#### Scenario: Atomic selection failure

- **WHEN** `--path .a -o json` is applied to `{"a":1} {"b":2}`
- **THEN** resolution SHALL fail with status 1 and empty stdout before any selected root is written
