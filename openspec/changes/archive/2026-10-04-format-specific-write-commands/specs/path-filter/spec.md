## MODIFIED Requirements

### Requirement: Interactive export scope

All interactive whole-document write commands, including default and explicit TOON, JSON, YAML, NDJSON/JSONL, feature-gated s-expressions, aliases, and overwrite variants, SHALL serialize the current selected roots as standalone values rather than the original full input or only the focused value. Collapse, wrapping, and focus SHALL NOT change export content. Format framing, normalization, limitations, and overwrite safeguards defined by interactive-write SHALL remain in force. Multiple selected roots SHALL still fail standard TOON whole-document export, including default `write` and `w`. NDJSON/JSONL SHALL emit one record per selected root, without implicitly expanding array roots. Serialization SHALL complete before opening or truncating the destination file. Focused-value and key copy commands SHALL retain their original targets; existing path-copy commands SHALL remain absolute within the original document.

#### Scenario: Export scope differs from focus

- **WHEN** `.hits` is active, focus is on a nested scalar, and `:write-json result.json` is submitted
- **THEN** the file SHALL contain the entire selected `hits` value without a `hits` wrapper or unrelated data
- **AND** copying the focused value SHALL still copy only that scalar

#### Scenario: Failed filtered export preserves files

- **WHEN** multiple filtered roots are active and `:write-toon! existing.toon` is submitted
- **THEN** standard TOON export SHALL fail and the existing file SHALL remain unchanged

#### Scenario: YAML export keeps selected document scope

- **WHEN** `.hits` selects one value in each of two original documents and `:write-yaml result.yaml` is submitted
- **THEN** the file SHALL contain two YAML documents in original document order, each containing only its selected value
- **AND** owning `hits` keys and unrelated siblings SHALL NOT be exported

#### Scenario: Filtered array roots stay records

- **WHEN** `.hits` selects an array in each of two original documents and `:write-jsonl result.jsonl` is submitted
- **THEN** the file SHALL contain two compact JSON array records in original document order
- **AND** focus and collapsed descendants SHALL NOT alter either record's content
