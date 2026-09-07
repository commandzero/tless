# Pinned TOON 3.0 fixture profile

Source: https://github.com/toon-format/spec/tree/c09f73b267323190f61de5b91563fa579b3b7c5e

Revision: `c09f73b267323190f61de5b91563fa579b3b7c5e`, tag `v3.0.0`.
`spec.md`, `LICENSE`, and `tests/fixtures` are copied from that revision.
Some fixture files retain older internal version labels; the pinned commit
defines the compatibility target.

The adapter runs 180 decode cases and 114 encode cases.
Decoding is strict, with two-space indentation and path expansion off.
Encoding uses two spaces, commas, and no folding. Object comparison preserves
encounter order, and numeric comparison uses normalized decimal values.

## Published codec profile

The source fixtures are unchanged. All 114 encode cases now use their upstream
expected output, including the table case with different object key orders.
Semantic comparison ignores object order. The large-number encode fixture has
an explicit known limitation: decoding its output returns a string, not a number.
All 180 selected decode cases retain their upstream expectations.

See [published codec behavior](../../../docs/toon_codec.md) for numeric conversion,
duplicate-key handling, empty-object array output, and nesting boundaries.
Tests no longer require private codec instrumentation or the removed patch's guarantees.

## Excluded option profiles

| Fixture | Case | Reason |
| --- | --- | --- |
| decode/blank-lines.json | ignores blank lines inside list array when strict=false | non-strict decoding |
| decode/blank-lines.json | ignores blank lines inside tabular array when strict=false | non-strict decoding |
| decode/blank-lines.json | ignores multiple blank lines in arrays when strict=false | non-strict decoding |
| decode/indentation-errors.json | throws on non-multiple indentation with custom indent=4 (3 spaces) | custom indentation |
| decode/indentation-errors.json | accepts correct indentation with custom indent size (4 spaces with indent=4) | custom indentation |
| decode/indentation-errors.json | accepts non-multiple indentation when strict=false | non-strict decoding |
| decode/indentation-errors.json | accepts deeply nested non-multiples when strict=false | non-strict decoding |
| decode/path-expansion.json | expands dotted key to nested object in safe mode | path expansion |
| decode/path-expansion.json | expands dotted key with inline array | path expansion |
| decode/path-expansion.json | expands dotted key with tabular array | path expansion |
| decode/path-expansion.json | expands and deep-merges preserving document-order insertion | path expansion |
| decode/path-expansion.json | throws on expansion conflict (object vs primitive) when strict=true | path expansion |
| decode/path-expansion.json | throws on expansion conflict (object vs array) when strict=true | path expansion |
| decode/path-expansion.json | applies LWW when strict=false (primitive overwrites expanded object) | non-strict decoding, path expansion |
| decode/path-expansion.json | applies LWW when strict=false (expanded object overwrites primitive) | non-strict decoding, path expansion |
| decode/path-expansion.json | preserves quoted dotted key as literal when expandPaths=safe | path expansion |
| decode/path-expansion.json | preserves non-IdentifierSegment keys as literals | path expansion |
| decode/path-expansion.json | expands keys creating empty nested objects | path expansion |
| encode/delimiters.json | encodes primitive arrays with tab delimiter | non-comma encoding |
| encode/delimiters.json | encodes primitive arrays with pipe delimiter | non-comma encoding |
| encode/delimiters.json | encodes tabular arrays with tab delimiter | non-comma encoding |
| encode/delimiters.json | encodes tabular arrays with pipe delimiter | non-comma encoding |
| encode/delimiters.json | encodes nested arrays with tab delimiter | non-comma encoding |
| encode/delimiters.json | encodes nested arrays with pipe delimiter | non-comma encoding |
| encode/delimiters.json | encodes root-level array with tab delimiter | non-comma encoding |
| encode/delimiters.json | encodes root-level array with pipe delimiter | non-comma encoding |
| encode/delimiters.json | encodes root-level array of objects with tab delimiter | non-comma encoding |
| encode/delimiters.json | encodes root-level array of objects with pipe delimiter | non-comma encoding |
| encode/delimiters.json | quotes strings containing tab delimiter | non-comma encoding |
| encode/delimiters.json | quotes strings containing pipe delimiter | non-comma encoding |
| encode/delimiters.json | does not quote commas with tab delimiter | non-comma encoding |
| encode/delimiters.json | does not quote commas with pipe delimiter | non-comma encoding |
| encode/delimiters.json | does not quote commas in tabular values with tab delimiter | non-comma encoding |
| encode/delimiters.json | does not quote commas in object values with pipe delimiter | non-comma encoding |
| encode/delimiters.json | does not quote commas in object values with tab delimiter | non-comma encoding |
| encode/delimiters.json | quotes nested array values containing pipe delimiter | non-comma encoding |
| encode/delimiters.json | quotes nested array values containing tab delimiter | non-comma encoding |
| encode/delimiters.json | preserves ambiguity quoting regardless of delimiter | non-comma encoding |
| encode/key-folding.json | encodes folded chain to primitive (safe mode) | key folding |
| encode/key-folding.json | encodes folded chain with inline array | key folding |
| encode/key-folding.json | encodes folded chain with tabular array | key folding |
| encode/key-folding.json | skips folding when segment requires quotes (safe mode) | key folding |
| encode/key-folding.json | skips folding on sibling literal-key collision (safe mode) | key folding |
| encode/key-folding.json | encodes partial folding with flattenDepth=2 | key folding |
| encode/key-folding.json | encodes full chain with flattenDepth=Infinity (default) | key folding |
| encode/key-folding.json | encodes standard nesting with flattenDepth=0 (no folding) | key folding |
| encode/key-folding.json | encodes standard nesting with flattenDepth=1 (no practical effect) | key folding |
| encode/key-folding.json | encodes folded chain ending with empty object | key folding |
| encode/key-folding.json | stops folding at array boundary (not single-key object) | key folding |
| encode/key-folding.json | encodes folded chains preserving sibling field order | key folding |
| encode/whitespace.json | respects custom indent size option | custom indentation |
