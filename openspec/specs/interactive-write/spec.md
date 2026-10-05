# interactive-write Specification

## Purpose

Allow users to export active document roots to an explicitly selected native format through a consistent interactive write command family.

## Requirements

### Requirement: Format-specific commands and TOON default

The interactive `:` prompt SHALL accept the following case-sensitive commands with exactly one destination filename argument. Every listed command SHALL also accept a trailing `!` on its command token to permit replacement. Format selection SHALL be independent of the input format, filename extension, and CLI output selector.

| Availability | Command | Encoding |
| --- | --- | --- |
| Every build | `write`, `w`, `write-toon`, `wt` | Standard TOON |
| Every build | `write-json`, `wj` | Pretty-printed JSON |
| Every build | `write-yaml`, `wy` | YAML document stream |
| Every build | `write-ndjson`, `write-jsonl`, `wn` | Compact line-delimited JSON |
| With `sexp` | `write-sexp`, `ws` | Existing s-expression export |

The old long names `writetoon` and `writesexp`, including their `!` variants, SHALL NOT be accepted. Builds without `sexp` SHALL NOT accept `write-sexp`, `ws`, or their overwrite variants. Missing or extra filename arguments and unsupported format names SHALL be rejected without creating or changing a file. Existing filename-token parsing SHALL remain unchanged.

#### Scenario: Default and explicit TOON agree

- **WHEN** one active root is written to separate new destinations with `:write report.toon`, `:w report.toon`, `:write-toon report.toon`, or `:wt report.toon`
- **THEN** each successful command SHALL produce the same standard TOON bytes

#### Scenario: Explicit format overrides context

- **WHEN** the input is TOON, the CLI output selector is YAML, and `:write-json report.toon` is submitted
- **THEN** the literal destination `report.toon` SHALL contain JSON, not TOON or YAML

#### Scenario: Format shortcuts

- **WHEN** `wj`, `wy`, or `wn` is submitted with a destination, with or without `!`
- **THEN** it SHALL have the same encoding and overwrite behavior as `write-json`, `write-yaml`, or `write-ndjson`, respectively

#### Scenario: Optional s-expression output

- **WHEN** `:write-sexp report.sexp` is submitted in a build with `sexp`
- **THEN** it SHALL use the existing s-expression encoding of all active roots
- **AND** the same command in a build without `sexp` SHALL be rejected without file changes

#### Scenario: Unsupported or obsolete command

- **WHEN** `:write-csv report.csv`, `:writetoon report.toon`, or `:writesexp report.sexp` is submitted
- **THEN** the command SHALL be rejected without file changes

### Requirement: Literal destination filenames

A write command SHALL require an explicit destination and use that filename literally. It SHALL NOT append an extension, derive a filename from the input, infer the encoder from the suffix, or reuse a previous destination. `.toon`, `.json`, `.yaml`, `.ndjson`, `.jsonl`, and `.sexp` SHALL be documented filename conventions, not enforced suffixes.

#### Scenario: No suffix insertion

- **WHEN** `:write report` is submitted for one encodable active root
- **THEN** it SHALL create `report` containing TOON
- **AND** it SHALL NOT create `report.toon`

#### Scenario: Missing destination

- **WHEN** `:write` or `:write-json` is submitted without a filename
- **THEN** it SHALL be rejected without creating or modifying a file

### Requirement: Native document framing

`write-toon` and the default commands SHALL retain standard whole-document TOON encoding: exactly one active root, two-space indentation, comma delimiters, no key folding, no final newline, and existing codec normalization and limitations. `write-json` SHALL retain the native JSON output contract: two-space pretty printing and a final LF per active root, multiple roots in order without a synthetic array, preservation of JSON number spellings, duplicate entries and entry order, JSON escaping, and rejection of non-string mapping keys or non-finite numbers. `write-yaml` SHALL retain the native YAML output contract: `---` on its own line before every root and a final LF per document, preservation of parsed scalar types, entry order and duplicates, and support for parsed non-string keys and non-finite numbers. Parser normalization remains in effect; no command SHALL promise source comments, anchors, formatting, or byte-for-byte restoration. Export SHALL NOT contain terminal styles, display warnings, gutters, previews, or status text.

#### Scenario: Multiple-root native output

- **WHEN** two active roots are exported with `write-json` and `write-yaml`
- **THEN** JSON SHALL contain two pretty-printed values in root order without an array wrapper
- **AND** YAML SHALL contain two documents, each preceded by `---` and terminated by LF

#### Scenario: TOON root-count failure

- **WHEN** multiple active roots are exported with `write` or `write-toon`
- **THEN** encoding SHALL fail with a diagnostic and no destination creation or modification

### Requirement: Equivalent NDJSON and JSONL output

`write-ndjson` and `write-jsonl` SHALL produce byte-identical UTF-8 output for the same active roots. Each active root SHALL be serialized as exactly one compact JSON value followed by LF, in active-root order, with no blank lines, BOM, indentation, or synthetic wrapper. Literal string line breaks SHALL be JSON-escaped so they do not introduce record boundaries. Arrays SHALL remain single array values; their elements SHALL NOT become independent records. Scalar roots SHALL be supported. JSON conversion restrictions and preservation of number spellings, mapping-entry order, and duplicate entries SHALL match `write-json`; invalid JSON conversions SHALL fail before any file is opened.

#### Scenario: Two records with escaped line breaks

- **WHEN** the active roots are `{"message":"a\nb"}` and `{"n":2}` and either line-delimited command is used
- **THEN** the exact file bytes SHALL be `{"message":"a\nb"}\n{"n":2}\n`, where the record separators are LF and the string's line break is the two-character JSON escape `\n`
- **AND** either command SHALL produce the same bytes

#### Scenario: Array and scalar records

- **WHEN** the active roots are `[1,2]` and `null`
- **THEN** line-delimited output SHALL contain exactly the records `[1,2]` and `null`, each followed by LF
- **AND** it SHALL NOT emit separate records for `1` and `2`

#### Scenario: JSON conversion fails

- **WHEN** an active YAML root contains a non-string mapping key or a non-finite number and is exported with `write-ndjson!` or `write-jsonl!`
- **THEN** encoding SHALL fail with a diagnostic
- **AND** any existing destination SHALL retain its bytes

### Requirement: Safe file creation and replacement

All write commands SHALL serialize every active root successfully before opening the destination. Without `!`, an existing destination SHALL cause an error and remain unchanged. With `!`, a successfully encoded payload SHALL create or truncate the literal destination. Serialization failure SHALL leave an existing destination unchanged and SHALL NOT create a missing destination. Open, write, and flush failures SHALL display an error; success SHALL be reported only after writing and flushing succeed. Filesystem failures after opening SHALL NOT be described as atomic replacement and can leave partial output. Commands SHALL leave the viewer's selection, focus, collapse, and wrapping state unchanged.

#### Scenario: Refuse replacement by default

- **WHEN** `:write-json existing.json` targets an existing file
- **THEN** it SHALL report that overwrite requires `!`
- **AND** the existing bytes SHALL remain unchanged

#### Scenario: Explicit replacement

- **WHEN** `:write-yaml! existing.yaml` is submitted and encoding and file operations succeed
- **THEN** the file SHALL contain only the new YAML payload, with no trailing bytes from its old contents
- **AND** the viewer SHALL report success after flushing

#### Scenario: Failure before destination creation

- **WHEN** `:write missing.toon` is submitted with multiple active roots
- **THEN** it SHALL report the encoding error and SHALL NOT create `missing.toon`

### Requirement: Discoverability and migration

Interactive help and user documentation SHALL describe every available command, overwrite forms, literal filenames, active-root scope, array-preserving NDJSON/JSONL framing, standard TOON's single-root restriction, and feature-gated s-expression output. Release notes SHALL mark the default and legacy-long-name changes as breaking and show `write-json` as the replacement for JSON uses of `write`, `write-toon` for `writetoon`, and `write-sexp` for `writesexp`. The CLI input/output format selector values and pipeline serialization SHALL remain unchanged by this change.

#### Scenario: User migrates a JSON write

- **WHEN** a user consults help or migration notes for the new default
- **THEN** they SHALL find `:write report.toon` for TOON and `:write-json report.json` for the former JSON behavior
- **AND** documentation SHALL distinguish new interactive line-delimited output from unchanged CLI format-selector support
