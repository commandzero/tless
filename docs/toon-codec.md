---
type: Guide
title: Published TOON codec behavior
description: Dependency policy, numeric and duplicate-key behavior, and known codec limitations.
status: stable
sources:
  - id: codec
    resource: https://crates.io/crates/toon-format/0.6.1
  - id: adapter-tests
    resource: ../src/toon.rs
  - id: fixtures
    resource: ../src/toon_fixtures.rs
generated: { by: openai-codex/gpt-6.1-sol, at: 2026-10-06T19:17:08Z }
---

# Published TOON codec behavior

Tless uses the published toon-format 0.6.1 crate from crates.io, pinned by the
manifest and Cargo.lock. Its optional CLI dependencies are disabled.
There is no vendored codec or Cargo patch. Rust 1.87 is the minimum compiler.

The former local patch supplied Rust 1.67 backports, stricter numeric and duplicate-key
checks, different depth accounting, scanner instrumentation, and order-preserving
list output. Those codec changes are no longer part of tless.

## Input and output contract

TOON input uses the published TOON 4.1 strict decoder with 2-space indentation.
Canonical output uses the published default encoder, with commas and 2 spaces.
TOON 4.1 has no key folding or path expansion. The wrapper handles empty documents
and CRLF; the published decoder strips an initial BOM. Redirected stdout defaults to the same standard encoder. `-o json` and `-o yaml`
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
| `1e1025` | Decoding fails because the floating-point value is infinite. |
| `1e20` encoded then decoded | Encoding produces `100000000000000000000`; decoding returns the floating-point number `1e20`. Other out-of-u64 integer tokens can still become strings. |
| Object rows with different key orders | Table encoding follows the first row's field order. Later rows can lose their original order. |
| Arrays of empty objects | Encoding uses list rows (`[2]:` followed by two `-` rows), which strict decoding accepts. Empty arrays encode as `[]`. Uniform nested objects can encode as nested table field groups. |

Standard TOON conversion does not promise exact decimal preservation or preservation
of duplicate entries. Use `:write-json`/`:wj` (or `-o json` for redirected
stdout) when those distinctions matter. Display warnings and previews never
become exported data.
These examples describe the pinned version; review and test changes before upgrading it.

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

The pinned TOON 4.1 source fixtures remain unchanged under tests/fixtures/toon-v4,
from specification revision 62f16b369408180f1faf1cba7da1b46d1f336f12.
The selected strict two-space decode profile covers 333 cases; the comma,
two-space encode profile covers 156 reference payloads and semantic round trips.
Non-strict, alternative indentation, and alternative encoder delimiters are
outside the application profile. Historical TOON 3.0 source fixtures remain
preserved under tests/fixtures/toon-v3. Object order is not part of semantic
round-trip comparison.

Application tests cover published duplicate-key and numeric behavior, empty-object
array output, depth boundaries, and retained file/terminal controls.
The old scanner-instrumentation test is removed because it called a private patch API.
This test suite does not certify every behavior of the upstream crate.
