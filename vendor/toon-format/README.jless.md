# jless codec backport

Source: [`toon-format` 0.5.0](https://crates.io/crates/toon-format/0.5.0),
published by [toon-format/toon-rust](https://github.com/toon-format/toon-rust).
The library sources and MIT license are retained. CLI/TUI sources are omitted.
The local manifest disables optional upstream integrations and pins compatible
versions. The root Cargo manifest applies this copy through `[patch.crates-io]`.

Local changes:

- Replace `is_some_and` and `is_multiple_of` with Rust 1.67 equivalents.
- Check original numeric lexemes using exact decimal coefficient/exponent
  comparisons before accepting a JSON number. Do not reinterpret overflowing
  integer tokens as strings. Remove saturating float-to-integer conversions
  from decoding.
- Reject duplicate object names and table fields before insertion. With path
  expansion disabled, quoted dotted names remain literal without marker prefixes.
- Carry local fidelity/resource and missing-table-row diagnostics as typed errors, so key text cannot
  change the reported category.
- Use list form for arrays of empty objects instead of an invalid zero-field table.
- Require matching key order for table rows. Differing row orders use list form
  to preserve each object's order, a documented jless canonical-profile exception.
- Set the zero-based codec depth limit to 255 and track actual containers separately
  from indentation, including named arrays and implicit list-item objects. The adapter limits export depth
  before recursive conversion and gives codec work a fixed 16 MiB stack.
- Count scanner character visits in debug builds for the linear-work regression.

All application code accesses this dependency through `src/toon.rs`.
The upstream source files remain otherwise unchanged. Conformance fixtures are
under `tests/fixtures/toon-v3`, outside this crate.

Cargo registry publication removes local patches. TOON-enabled distribution
must use this source checkout or its built binaries until a compatible patched
codec is published under its own version. Do not advertise registry installation
with `--features toon` as equivalent to this checkout.
