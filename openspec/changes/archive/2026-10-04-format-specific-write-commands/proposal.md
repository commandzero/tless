## Why

Interactive export does not expose all native serializers: `write` defaults to JSON despite the TOON viewer and pipeline default, YAML is available only for pipeline output, and explicit line-delimited JSON export is missing. A uniform `write-{format}` command family makes encoding discoverable and independent of input format or destination extension.

## What Changes

- **BREAKING**: `:write <filename>` and `:w <filename>` (including `!` forms) write standard TOON instead of JSON. Use `.toon` filenames by convention; the required filename remains literal, with no extension inference or automatic suffix.
- Add `write-toon`, `write-json`, `write-yaml`, `write-ndjson`, and `write-jsonl`, each with a `!` overwrite form. NDJSON and JSONL are equivalent names for compact JSON, one active root per line; array roots remain arrays, not implicitly expanded records.
- Expose the existing optional s-expression serializer as `write-sexp` and `write-sexp!` only in `sexp` builds.
- **BREAKING**: Replace `writetoon`/`writetoon!` and feature-gated `writesexp`/`writesexp!` with the hyphenated names. Retain `wt`/`wt!` and feature-gated `ws`/`ws!`; add `wj`/`wj!` for JSON, `wy`/`wy!` for YAML, and `wn`/`wn!` for NDJSON/JSONL.
- Preserve active-root scope, format restrictions, serialization-before-open behavior, and explicit overwrite safeguards. Add the new command names to completion and update interactive help and migration guidance.
- Leave CLI input/output selector values and pipeline framing unchanged. NDJSON/JSONL are new interactive output spellings, not new parsers or CLI selector values in this change.

## Capabilities

### New Capabilities

- `interactive-write`: Format-specific file commands, TOON default, compact line-delimited JSON, feature availability, filenames, overwrite safety, and migration guidance.

### Modified Capabilities

- `command-autocomplete`: Replace legacy long write names with the new command family, retain existing short-alias acceptance, and update completion scenarios.
- `path-filter`: Extend export-scope requirements to YAML and line-delimited JSON and migrate examples to explicit format commands.

## Impact

- Command registry/parser in `src/command.rs`, completion consumers in `src/commandline.rs`, and write dispatch in `src/app.rs`.
- Native serialization in `src/output.rs`, with reuse of the existing JSON/YAML/TOON traversal and optional `FlatJson::sexp_string`; no new codec dependency or vendored source.
- Command, serialization, and pseudoterminal integration coverage, plus `src/tless.help`, `src/toon.help`, README, relevant docs, and Unreleased changelog when implemented.
- Incompatible default and long-name changes require the next minor release under the repository's 0.x versioning policy. Planning introduces no executable behavior or release-version changes.
