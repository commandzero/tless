---
type: Guide
title: Published TOON codec behavior
description: Dependency policy, numeric and duplicate-key behavior, and known codec limitations.
status: stable
sources:
  - id: codec
    resource: https://crates.io/crates/toon-format/0.7.0
  - id: upgrade
    resource: https://github.com/toon-format/toon-rust/pull/88
  - id: specification
    resource: https://github.com/toon-format/spec/blob/d09c3d33e37389345a74bd96981eb96dd9be6c5e/SPEC.md
  - id: adapter-tests
    resource: ../src/toon.rs
  - id: fixtures
    resource: ../src/toon_fixtures.rs
generated: { by: openai-codex/gpt-6.1-sol, at: 2026-10-14T05:08:07Z }
---

# Published TOON codec behavior

Tless uses the published toon-format 0.7.0 crate from crates.io, pinned by the
manifest and Cargo.lock. All codec default features, including its CLI, are disabled.
There is no vendored codec or Cargo patch. Rust 1.87 is the minimum compiler.

The former local patch supplied Rust 1.67 backports, stricter numeric and duplicate-key
checks, different depth accounting, scanner instrumentation, and order-preserving
list output. Those codec changes are no longer part of tless.

## Input and output contract

TOON input uses the published TOON 4.4 strict decoder with 2-space indentation.
Tless has no non-strict input mode. Canonical output uses the published default
encoder, with commas and 2 spaces. TOON 4.4 has no key folding or path expansion.
The published decoder handles empty documents, CRLF, and an initial BOM.
Input is passed through unchanged so a second carriage return before CRLF remains data.
Redirected stdout defaults to the same standard encoder. `-o json` and `-o yaml`
select other serializers independently of input format. TOON input is decoded
and re-encoded even when the selected output is TOON; malformed input fails
before any stdout payload.

The interactive [document view](toon-view.md) builds its layout directly from
parsed nodes. It preserves their duplicate occurrences, decimal tokens, types,
and order, with explicit display warnings where standard TOON cannot do so.
The conversion behavior below applies to the codec's input and export paths.
It also limits what data is available to the viewer after TOON input decoding.

| Case | Published behavior used by tless |
| --- | --- |
| Duplicate object keys, such as `a: 1` followed by `a: 2` | Strict input decoding rejects duplicate sibling keys. Exporting duplicate JSON keys to TOON keeps the last value. |
| Duplicate table fields, such as `[1]{a,a}:` | Strict decoding rejects the header. |
| `0.123456789012345678901` | Decoding rounds to `0.12345678901234568`. Export also uses serde_json and the codec's numeric conversion. |
| `18446744073709551616` | Decoding returns a string because the integer does not fit u64. |
| `1e1025` | Strict TOON decoding returns the string `"1e1025"` because the token is outside finite f64 range. Exporting the same numeric token from JSON to TOON still fails with `UnsupportedNumber`; no infinite numeric value is exported. |
| `1e20` encoded then decoded | Encoding produces `100000000000000000000`; decoding returns the floating-point number `1e20`. Other out-of-u64 integer tokens can still become strings. |
| Object rows with different key orders | Table encoding follows the first row's field order. Later rows can lose their original order. |
| Arrays of empty objects | Encoding uses list rows (`[2]:` followed by two `-` rows), which strict decoding accepts. Empty arrays encode as `[]`. Uniform nested objects can encode as nested table field groups. |

Standard TOON conversion does not promise exact decimal preservation or preservation
of duplicate entries. Use `:write-json`/`:wj` (or `-o json` for redirected
stdout) when those distinctions matter. Display warnings and previews never
become exported data.
These examples describe the pinned version; review and test changes before upgrading it.

### Upgrade from toon-format 0.6.1 / TOON 4.1

The [published 0.7.0 release](https://docs.rs/crate/toon-format/0.7.0/source/CHANGELOG.md)
and [upstream PR #88](https://github.com/toon-format/toon-rust/pull/88)
move decoding to spec 4.4. Documents produced by conforming 4.x encoders
remain valid. The default encoder's table, list, nested-group, and keyed-object
layout is unchanged; the published encoder already met the tightened quoting
and literal-character rules. The document view retains its separate
parsed-value, warning, navigation, and alignment contracts.

Changes relevant to tless's always-strict input include:

- A first unquoted `[` before the first unquoted colon commits the line to
  array-header syntax. Malformed headers such as `a[2:]{x}` now fail rather
  than falling through to an ordinary key-value line.
- A `"` opens a quoted span anywhere in a line. Only SP and HTAB count as
  whitespace; NBSP and other Unicode spacing characters remain content.
- A hyphen is a list marker only at item depth, and multiple spaces may
  follow it. Spaces between a quoted key and its colon are accepted.
- Indented roots, tabs-only lines, empty field entries, and nameless or
  space-preceded nested field groups are rejected. Blank lines outside
  header spans and a trailing newline remain accepted.

The upstream changes to non-strict recovery do not add a tless option:
duplicate sibling keys, indentation errors, length or row-width mismatches,
and trailing content still fail in the strict application profile.

The codec now enables serde_json's `float_roundtrip` feature. When JSON-to-TOON
numeric conversion needs a binary float, it parses decimal text to the correctly
rounded value and emits its shortest round-trip digits. For example, the upstream
reference case encodes `123456789012345678901` as `123456789012345680000`,
rather than the former `123456789012345670000`. This fixes binary-float
round-trip accuracy; it does not preserve arbitrary decimal precision or the
original number spelling. Exact parsed decimal tokens in the document view
and native JSON writes remain separate from this standard TOON conversion.

Out-of-range TOON numbers also follow the published string-fallback policy:
`1e1025` now decodes as the string `"1e1025"` rather than failing. This is
not acceptance of a non-finite numeric value; exporting that token as a JSON
number still fails.

## Bounds and failure behavior

The 512 MiB input-byte limit remains configurable. The complete document remains
in memory. The decoder uses the published codec's nesting accounting, which can
accept 257 actual containers in the tested nested-object and named-array cases.
Tless export retains its own limit of 256 containers relative to the selected root.
The former coefficient/exponent limits and linear scanner-work guarantee are removed.
The codec runs on a 16 MiB worker stack; this is not a total memory bound.

Standard TOON export (`:write`, `:w`, `:write-toon`, or `:wt`)
still rejects multiple active roots, non-string YAML keys, and non-finite
values. Focused copy/print can select a supported value inside an otherwise
unsupported document. Interactive `:write-json`/`:wj`, `:write-yaml`/`:wy`,
and `:write-ndjson`/`:write-jsonl`/`:wn` use their own native encoders over
all active roots. Each command also has a `!` overwrite form; an explicit,
literal filename is required. Encoding completes before an output file is
opened; encoding failure leaves the destination unchanged. File I/O errors
after opening can leave partial output, so overwrite is not atomic.
Successful encoding is not a promise of an exact data round trip.

## Verification

The current pinned TOON 4.4 fixtures under
tests/fixtures/toon-v4/v4.4.0/tests/fixtures are derived and reformatted from
specification revision d09c3d33e37389345a74bd96981eb96dd9be6c5e (tag v4.4.0).
Each of the 564 cases has its own `.toon` file: a TOON metadata header whose lines
start with `# `, a `# ---` separator, and the original TOON payload verbatim.
The header retains upstream group and case metadata; `input-json` for encode cases
and `expected-json` when supplied for decode cases hold escaped, compact JSON model text.
JSON strings are quoted and escaped in metadata, not in the TOON payload.
tq 0.5.0 (TOON 4.1) was used only to encode metadata; the TOON 4.4 golden bodies
were not regenerated, including their original line endings, BOMs, controls,
malformed syntax, and presence or absence of a final newline.
The selected strict two-space decode profile covers 348 cases; the comma,
two-space encode profile covers 139 reference payloads and semantic round trips.
Non-strict, alternative indentation, and alternative encoder delimiters are
outside the application profile: 56 decode cases and 21 encode cases are
excluded. Fixture metadata retains upstream's `version: "4.0"`; case-level
`minSpecVersion` is checked against the supported spec 4.4 bound.
Historical TOON 4.1 source fixtures remain unchanged under
tests/fixtures/toon-v4/tests/fixtures, from revision
62f16b369408180f1faf1cba7da1b46d1f336f12, and TOON 3.0 fixtures remain unchanged under
tests/fixtures/toon-v3. All upstream MIT licenses are preserved.
Object order is not part of the codec's semantic round-trip comparison.
The rendering fixture test parses `input-json` directly and compares the raw
TOON body, retaining ordered-semantic and terminal-safety checks where needed.

Application tests cover published duplicate-key and numeric behavior, empty-object
array output, depth boundaries, and retained file/terminal controls.
The old scanner-instrumentation test is removed because it called a private patch API.
This test suite does not certify every behavior of the upstream crate.
