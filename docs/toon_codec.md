---
type: Guide
title: Published TOON codec behavior
description: Dependency policy, numeric and duplicate-key behavior, and known codec limitations.
status: stable
sources:
  - id: codec
    resource: https://crates.io/crates/toon-format/0.5.0
  - id: adapter-tests
    resource: ../src/toon.rs
  - id: fixtures
    resource: ../src/toon_fixtures.rs
generated: { by: codex/gpt-6, at: 2026-09-07T05:44:59Z }
---

# Published TOON codec behavior

Tless uses the published toon-format 0.5.0 crate from crates.io, pinned by the
manifest and Cargo.lock. Its optional CLI dependencies are disabled.
There is no vendored codec or Cargo patch. Rust 1.87 is the minimum compiler.

The former local patch supplied Rust 1.67 backports, stricter numeric and duplicate-key
checks, different depth accounting, scanner instrumentation, and order-preserving
list output. Those codec changes are no longer part of tless.

## Input and output contract

Interactive TOON input uses the published strict decoder with 2-space indentation
and path expansion disabled. Canonical output uses the published default encoder,
with commas, 2 spaces, and no key folding. The wrapper still handles empty documents,
CRLF, and BOM rejection. Redirected TOON stdout passes through the input unchanged
without syntax validation.

| Case | Published behavior used by tless |
| --- | --- |
| Duplicate object keys, such as `a: 1` followed by `a: 2` | The last value wins. Exporting duplicate JSON keys to TOON also keeps the last value. |
| Duplicate table fields, such as `[1]{a,a}:` | Strict decoding rejects the header. |
| `0.123456789012345678901` | Decoding rounds to `0.12345678901234568`. Export also uses serde_json and the codec's numeric conversion. |
| `18446744073709551616` | Decoding returns a string because the integer does not fit u64. |
| `1e1025` | Decoding fails because the floating-point value is infinite. |
| `1e20` encoded then decoded | Encoding produces `100000000000000000000`; decoding that token returns a string. Numeric round trips are not guaranteed. |
| Object rows with different key orders | Table encoding follows the first row's field order. Later rows can lose their original order. |
| Arrays of empty objects | Version 0.5.0 emits a zero-field table with blank rows. Its strict decoder rejects that output. This is a known codec limitation, preserved in a regression test. |

The viewer does not promise exact decimal preservation or preservation of duplicate
entries when converting through TOON. Use JSON output when those distinctions matter.
These examples describe the pinned version; review and test changes before upgrading it.

## Bounds and failure behavior

The 512 MiB input-byte limit remains configurable. The complete document remains
in memory. The decoder uses the published codec's nesting accounting, which can
accept 257 actual containers in the tested nested-object and named-array cases.
Tless export retains its own limit of 256 containers relative to the selected root.
The former coefficient/exponent limits and linear scanner-work guarantee are removed.
The codec runs on a 16 MiB worker stack; this is not a total memory bound.

Export still rejects multiple document roots, non-string YAML keys, and non-finite
values. Focused export can select a supported value inside an otherwise unsupported
document. Encoding completes before the output file is opened. Successful encoding
is not a promise of an exact data round trip. File I/O failures can leave partial output.

## Verification

The unchanged TOON 3.0 source fixtures remain under tests/fixtures/toon-v3.
All 180 selected decode cases and 114 selected encode cases are exercised.
The encoder uses each original expected payload, including the upstream table-order
case. Object order is not part of semantic round-trip comparison.
The large-number encode case has an explicit assertion for its known string result
on decoding; it is not silently excluded or reported as a successful numeric round trip.

Application tests cover published duplicate-key and numeric behavior, empty-object
array output, depth boundaries, and retained file/terminal controls.
The old scanner-instrumentation test is removed because it called a private patch API.
This test suite does not certify every behavior of the upstream crate.
