## Context

See proposal.md for motivation. `src/command.rs` currently owns a single command registry used by parsing and completion; `WriteFormat` includes JSON, TOON, and feature-gated s-expressions. `App::write_contents_to_file` already serializes active roots before opening the destination and uses `create_new` without `!` and create/truncate with `!`.

`src/output.rs` exposes JSON, YAML, and TOON serialization through `serialize_roots`. JSON/YAML traversal works directly over flat rows, preserving ordering and duplicate mapping entries without constructing another recursive document. Pipeline selectors in `src/options.rs` intentionally remain JSON/YAML/TOON. The optional s-expression serializer lives in `FlatJson::sexp_string` and is not an input parser.

## Goals / Non-Goals

**Goals:**

- Reuse the existing command registry and serializer boundary rather than creating a second completion list or export model.
- Share JSON conversion and validation between pretty and compact framing, preserving number precision and duplicate keys.
- Preserve the existing complete-encoding-before-open invariant and active-root identity.

**Non-Goals:**

- New input parsers, CLI selector values, CSV/TOML/XML output, aliases beyond the specified write shortcuts, quoted filename syntax, filename inference, array-to-record expansion, or user-configurable defaults.
- Streaming file writes, temporary-file/rename atomic replacement, changes to focused copy/print commands, codec upgrades, or dependency changes.

## Decisions

### Keep command availability in the existing registry

Extend `WriteFormat` with YAML and one canonical line-delimited JSON variant. Register both `write-ndjson` and `write-jsonl` as long names pointing to that same variant so both are discoverable. Map `write`/`w` and `write-toon`/`wt` to TOON, `write-json`/`wj` to JSON, `write-yaml`/`wy` to YAML, `write-ndjson`/`wn` and `write-jsonl` to NDJSON, and feature-gated `write-sexp`/`ws` to the existing s-expression variant. Register corresponding bang forms. Remove the obsolete unhyphenated long names. Keep candidate sorting and first-token completion behavior in the existing completion layer.

A second list of format names would risk parser/completion drift. Dynamically accepting arbitrary `write-` suffixes would hide feature availability and error behavior; the finite registry already models both correctly.

### Keep interactive format selection separate from CLI selection

Do not add NDJSON or s-expressions to CLI `OutputFormat` merely to support interactive commands. Add a compact-JSON roots entry point in `src/output.rs`; route existing formats to `serialize_roots` and optional s-expressions to their existing serializer. This keeps the public CLI contract unchanged while sharing encoding internals.

### Parameterize JSON layout, not JSON semantics

Extend the existing iterative JSON traversal with a small explicit layout choice for pretty versus compact JSON. Keep YAML framing separate. Compact JSON emits structural delimiters and commas without layout whitespace and appends exactly one LF after each active root; existing string escaping prevents embedded record separators. Reuse key, number, string, and scalar validation. Traverse each selected subtree once into the output buffer, borrowing the source model rather than copying roots.

Do not parse pretty output back through `serde_json`, which can lose duplicate keys and change numeric spellings. Do not strip whitespace from encoded text: string whitespace is data, and lexical post-processing duplicates the encoder's responsibilities. Avoid additional document models or per-record buffer copies.

### Keep filenames literal and arrays intact

Continue requiring the existing single filename token. A suffix is documentation, not a selector: `write-json data.toon` still writes JSON, and `write data` writes TOON to `data`. Array expansion would make record count and data shape depend on root type; one active root per line instead matches existing selected-root export semantics. Explicit record-extraction can be considered separately, not inferred here.

### Preserve file-operation semantics

Keep serialization before `File::options().open`, `create_new` refusal without `!`, create/truncate with `!`, and write/flush diagnostics. Encoder failures cannot modify the destination. File-operation failures after opening may leave a partial file; no new atomic-write guarantee is introduced.

## Risks / Trade-offs

- [Default TOON cannot represent multiple active roots and retains codec normalization limits] → Document the breaking default and explicit `write-json`, `write-yaml`, and line-delimited alternatives; never silently wrap or drop roots.
- [Users rely on JSON `write` or old long commands] → Ship as the next minor release with explicit migration examples; retain the existing `w`, `wt`, and feature-gated `ws` short aliases with the specified new/default mappings, not deprecated long-name shims.
- [Compact output could corrupt escaping, duplicates, or numbers] → Share conversion code and cover string controls, nested structures, duplicate mappings, precise numeric spellings, scalar roots, and invalid YAML-to-JSON conversions.
- [Completion deltas accidentally remove unrelated behavior] → Modify complete affected requirement blocks and retain all argument-preservation, cycle, cancellation, and feature-gating guarantees.
- [Bang-form writes can leave partial bytes after filesystem errors] → Preserve and document the existing guarantee boundary; atomic replacement is out of scope.

## Migration Plan

1. Implement registry, serializer layout, dispatch, and behavior coverage together; update help, README, relevant docs, and Unreleased migration notes before release.
2. Demonstrate `write-json` for former JSON default writes, `write-toon` for `writetoon`, and `write-sexp` for feature-gated `writesexp`. Explain that `.toon` is a convention, not automatically appended.
3. Run targeted pseudoterminal exports and verify actual destination contents and unchanged-file failure paths, then preflight and Rust 1.87 compatibility checks.
4. Synchronize all three deltas, archive this change, validate with OpenSpec 1.11.0, and perform the required synchronization review and PR completion check before merging the implementation.
5. If release rollback is necessary, roll back the implementation release as a unit; do not mix restored parser names with the new help or default encoding.
