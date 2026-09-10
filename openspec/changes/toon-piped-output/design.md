## Context

See [proposal.md](proposal.md) for the issue and scope. `main.rs` currently selects the input format before checking stdout. Its machine branch pretty-prints JSON and passes YAML and TOON through unchanged. `DataFormat` represents input selection and conditionally includes TOON. `toon::encode_document` already provides the standard whole-document export contract.

The viewer's `FlatJson` model retains JSON token spellings and duplicate entries, plus decoded YAML strings and typed keys. Its display text and YAML-derived JSON-like backing text are not safe general-purpose serialization formats. There is no current YAML export path.

## Goals / Non-Goals

The design keeps format conversion independent of terminal setup and preserves the existing JSON-input compatibility escape hatch. All output is assembled before writing, matching the current whole-input memory model.

This work does not change the viewer, interactive export commands, input detection, the codec dependency, or its documented normalization. Streaming conversion, byte-preserving pass-through, a new TOON encoder, and exact numeric TOON round trips are outside this change.

## Decisions

### Separate input and output options

Add an `OutputFormat` value enum whose three variants exist in every build, with a default of TOON. Keep `DataFormat` and input selection separate. Validate output feature availability only in the non-terminal path so minimal-build interactive use still works. Reusing the feature-gated input enum would hide the requested default and blur input selection with export availability.

The feature decision is a proposal assumption that preserves the existing documented build contract. Making the codec mandatory would change dependency and minimal-build policy beyond issue #7.

### Parse, encode, then write

Replace the pass-through helper with a small machine-output boundary accepting input text, input format, and output format and returning either a complete payload or a diagnostic. Reuse existing parsers. Keep terminal initialization after this path. Retain the locked stdout write and flush error handling.

Rendering a hidden viewer or stripping its ANSI output is unsuitable. Display extensions, width-dependent layout, and collapse state are presentation behavior and cannot define a serialization contract.

### Reuse TOON export exactly

Delegate TOON encoding through `src/toon.rs`. The pinned published encoder remains authoritative, including known invalid empty-object-array output. Do not silently repair it, introduce a second encoder, or claim that successful encoding proves validity or fidelity for every value. CLI regressions should cover both ordinary valid output and the documented exception. Fixing the upstream codec limitation is separate work.

### Serialize JSON and YAML from parsed values

Keep the existing pretty-printer for JSON input where it preserves exact tokens and duplicate entries. For conversion paths, use decoded string and typed-key data rather than copying YAML-derived backing ranges. JSON output rejects values outside its data model before writing. It does not route through the TOON codec or floating-point conversion.

Add a YAML serializer over ordered parsed entries. Reuse the existing YAML dependency where it preserves the contract, but do not reduce entries through a map that discards duplicates. Scalar quoting must preserve strings that resemble numbers, booleans, nulls, or non-finite values. Emit typed complex keys using YAML key syntax, preserve numeric spellings where valid, and frame every root with `---` and a final LF. This avoids adding another parser dependency or making JSON/YAML output depend on `toon`.

Byte-preserving YAML pass-through would leave same-format input unvalidated and would not define conversions from JSON or TOON. A shared serde_json map for all output would erase duplicate entries and reject YAML keys before the selected output format can handle them.

## Risks / Trade-offs

- Default conversion can normalize duplicate keys and numbers, and exposes known codec limitations to more users. Document them beside pipeline examples and retain explicit JSON output for original JSON distinctions.
- YAML conversion adds quoting and typed-key cases. Verify decoded value equivalence for supported cases and byte-level preservation tests for duplicate entries and exact JSON numeric tokens.
- Minimal builds will fail for default redirected output. Provide actionable diagnostics and document explicit JSON/YAML overrides. Do not silently select a different format by build profile.
- Buffering the encoded output increases peak memory. Retain the input limit and document that it is not a process-memory limit. Streaming is separate work.
- Existing CLI tests assume pass-through or JSON defaults. Update those expectations deliberately while retaining tests for input selection, limits, broken pipes, and terminal behavior.

## Migration Plan

Update help, README, TOON codec/view guides where they discuss machine output, acceptance instructions, and Unreleased release notes with the implementation. Show `cat file.json | tless -o json | consumer` for JSON consumers and `tless --yaml -o yaml` for YAML serialization. Explain that YAML and TOON input are now validated and reserialized.

Schedule the incompatible default in the next minor release according to repository policy; versioning and publication remain maintainer-owned release actions. Before merge, complete implementation verification, synchronize the new capability, archive the change, and pass the committed-head OpenSpec gate. If the behavior must be withdrawn before release, revert the implementation and its documentation together; consumers can use explicit output selectors in the meantime.
