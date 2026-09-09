## Context

See [proposal.md](proposal.md) for motivation and capability scope.

`FlatJson` stores parsed nodes, source-like scalar ranges, parent/sibling links, and paired container rows. The viewer currently uses those rows as both logical values and display lines. `Mode::Line` and `Mode::Data` affect traversal, line printing, help, and CLI options. This cannot directly represent several independently selectable TOON values on one line.

The optional TOON adapter converts through `serde_json::Value` and the published codec. This path applies last-value-wins to duplicate JSON entries, converts numbers, and can reorder table fields. The pinned codec also emits a zero-field table for arrays of empty objects that its strict decoder rejects. These behaviors remain documented export behavior; they are unsuitable as the viewer's data model.

The selected standard is the repository's retained TOON 3.0 specification and fixtures. This version excludes comments. The proposed `# WARN` annotations therefore belong explicitly to the tless display extension, not to a claimed standard TOON comment syntax.

## Goals / Non-Goals

### Goals

1. Separate parsed value identity, logical layout, visible lines, and terminal styling.
2. Keep collapse, search, paths, and selection coherent across inline and tabular representations.
3. Preserve information available in the parsed model, with explicit display extensions when needed.
4. Keep layout and numeric rendering bounded by input size and documented display limits.

### Non-Goals

1. Changing input parser normalization or recovering original whitespace and comments.
2. Adding an extended-TOON parser, file format, clipboard format, or export command.
3. Upgrading TOON versions or changing current standard export conversion rules.
4. Replacing `FlatJson` throughout the application or adding a separate library crate.
5. Implementing application or CI code during this planning workflow.

## Decisions

### 1. Build a TOON layout directly over parsed nodes

Add an application-owned TOON display module that consumes read-only parsed nodes. Keep the published codec integration behind the existing TOON adapter. The display module implements layout and annotated spans; it does not copy or vendor codec source.

Use stable node indices as value identities, normalizing closing rows to their opening node. Maintain explicit identities for duplicate occurrences. A layout line owns an ordered set of token spans, each tied to a node and token role. Container headers and inline children can share a line while retaining distinct focus targets.

Separate these structures:

1. A logical layout describing TOON lines, container extents, and node-to-span mappings.
2. A visibility projection applying collapse state and retaining absolute expanded line addresses.
3. Annotation spans for arrows, previews, counts, and warnings.
4. A viewport that measures terminal cells, clips, scrolls, and applies styling.

Cache layout decisions and warning counts when loading the document. Recompute the visible projection after collapse; resize changes clipping, not the chosen TOON array form. Avoid serializing the whole document on each keypress. Reference source ranges where possible and materialize only needed token text.

Alternative considered: encode the document with the current adapter and parse the resulting text into display rows. This loses duplicate identity and numeric fidelity before navigation begins. It also duplicates parsing work and does not provide a reliable source mapping.

### 2. Keep one deterministic standard profile

Apply comma delimiters, 2 spaces, and no key folding. Choose table form only for nonempty uniform rows of unique string keys and primitive values with identical field order. Choose list form when table construction would change entry order or require duplicate fields. Empty-object arrays use counted lists containing bare hyphens.

Use the retained standard fixtures as the grammar reference. Compare with codec output where its behavior satisfies the rendering contract. For documented differences, test the explicit layout and source-node mapping separately instead of weakening the contract to match a codec defect.

Finite JSON numbers are formatted from their decimal token components without an f64 round trip. Normalize decimal placement and redundant zeros exactly. Before expanding an exponent, calculate the resulting length with checked arithmetic. Above 4096 rendered characters, retain the original parsed number token with a non-canonical-number warning. Treat YAML numeric spellings outside the profile the same way, except for the named non-finite forms.

Alternative considered: leave every numeric lexeme unchanged. This avoids arithmetic conversion but produces noncanonical TOON unnecessarily for ordinary exponents. Exact bounded normalization provides standard text for ordinary values without a huge-number allocation hazard.

### 3. Define a display extension with explicit warnings

The normative rules are in [toon-display-extensions](specs/toon-display-extensions/spec.md). The display does not pass extended text back through the strict codec.

Examples, before terminal colors and collapse gutters:

```text
status: queued  # WARN Duplicate key
status: done  # WARN Duplicate key
limit: .inf  # WARN Non-finite number
literal: ".inf"
precise: 0.123456789012345678901
huge: 1e1000000  # WARN Non-canonical number
? 1: numeric-key-value  # WARN Non-string key
"1": string-key-value
```

Use decoded keys to detect duplicates within each object, but never insert entries into a map that replaces their nodes. Mark every occurrence, including the first. Assign occurrence ordinals for status and selection. Object counts include all immediate entries.

Non-string keys use an explicit `? ` prefix. The compact key notation uses JSON-style quoted strings and recursive arrays/ordered object pairs, preserving types and allowing the non-finite numeric tokens. It is a display of the parsed key, not YAML source reconstruction. Quote actual string keys that resemble the prefix. Multiple roots use an explicit warning separator before each root, not a synthetic parent value.

TOON 3.0 permits escaped LF, CR, TAB, quote, and backslash, but no Unicode escape syntax. Other parsed control characters use terminal-safe JSON-style `\uXXXX` spelling with `Non-standard string escape` warnings, including strings inside typed keys. This is an explicit display extension; input and export handling stay unchanged.

Warning text contains fixed messages and escaped field identifiers, not arbitrary unescaped input. Each shared line has one final comment so a warning cannot visually consume later cells. Use `at [N]` for an inline element and `at field "name"` for a table cell. Field labels follow JSON string escaping. Keep warnings in node order, with the message-kind ordering defined in the spec.

Collapsed containers show their own warnings plus a cached count of hidden descendant warnings. Count semantic warning records, not painted comments or repeated header spans. A non-finite complex key can carry both non-string-key and non-finite-number warnings. Ordinary source strings containing `# WARN` are always quoted and never create warning records.

Alternative considered: normalize to the standard data model and warn about the conversion. The selected behavior preserves what the user is inspecting. A warning does not justify dropping an entry or replacing infinity with null.

### 4. Preserve navigation through mapped spans

Keep logical focus as a node plus an optional key/value target. Track a display-line anchor separately from that focus. Vertical motions use visible data lines; structural motions use node relationships. When crossing expanded table rows, retain the chosen field. The header owns the array, a data row owns its object, and an inline-array line owns its array until child motion selects an element.

The first child motion on a collapsed container expands it. A subsequent child motion enters its first child. Parent motion selects the logical parent even when both targets occupy one line. Use existing sibling actions for same-line elements and cells. Mouse hit testing needs both row and column coordinates, unlike the existing row-only click action.

A root object has an implicit selectable anchor at its first line. An empty root has one blank selectable viewport row but no invented document token. It remains expanded because TOON provides no root object header. Root-separator warning rows carry line addresses but are skipped by vertical value motion; absolute jumps onto them select the following root.

Search retains the current matcher and associates source matches with node identities before looking up display spans. Header key matches identify a logical field occurrence even though their painted text is shared. Render warnings and previews through separate annotations so neither becomes a second searchable occurrence.

Alternative considered: navigate only rendered lines. This would make individual table cells and inline elements unavailable for focused copy/export and would break paths.

### 5. Keep UI chrome and serialization separate

Retain the existing line-number options and default visibility. Absolute addresses now refer to the expanded TOON layout; relative values count visible value-line motions. Document this behavioral change. Status, command input, and help stay outside the document area. Syntax focus/search styles remain supported.

Render preview text only for collapsed containers. Give count and warning annotations space before previews. If essential annotations still exceed width, preserve them in the horizontally scrollable line. Escape control characters before measuring width; truncate only at valid display boundaries.

Existing copy/export commands continue to consume parsed selections. Do not serialize the framebuffer. The display's warnings are not a new interchange promise. Existing TOON export can still normalize duplicates and numbers; update help to distinguish this behavior from display fidelity.

### 6. Keep rendering independent of the optional codec feature

Compile the application display module in every build. Retain the `toon` feature for its existing TOON parser and export commands. Retain `sexp` behavior. This avoids breaking `--no-default-features` consumers merely to choose the only display format.

Alternative considered: make all TOON codec dependencies unconditional and remove the feature. The renderer does not need the codec conversion path, and changing feature contracts is unnecessary for this request.

## Risks / Trade-offs

1. Extended display text is not strict TOON 3.0. Mitigation: warn inline, document the extension, and keep input/export behavior separate.
2. Existing parsers can already normalize input. Mitigation: define fidelity at the parsed-model boundary and retain regression tests for those parser limits.
3. Shared lines complicate focus and search. Mitigation: test node identity, cell mapping, duplicate occurrences, collapse, and mouse coordinates independently of rendered colors.
4. The new layout index consumes memory. Mitigation: reference parsed ranges, store compact line metadata, cache once, and verify redraw does not re-encode the entire input.
5. Numeric normalization can allocate excessive output. Mitigation: calculate expansion length before allocating and use the specified warning fallback.
6. Removing mode and delimiter navigation changes established controls. Mitigation: update CLI/help and Unreleased migration notes together, and keep unrelated commands stable.

## Migration Plan

1. Add the PR-scoped OpenSpec completion check selected by contributor guidance. Share its local entry point with CI and use native CLI validation. Keep this work in the implementation phase.
2. Implement and test layout, extensions, and logical focus before replacing the active renderer.
3. Switch interactive rendering, remove mode controls, and update help and acceptance documentation as one behavior change.
4. Run preflight, Rust 1.87 coverage, all existing feature profiles, targeted terminal tests, and documentation validation. Add manual wide/narrow-terminal checks to the existing acceptance guide.
5. Verify all requirements, synchronize the deltas into main specs, and archive this change before its implementation PR merges. Preserve historical documents and upstream fixtures.
6. If rollback is needed before release, revert the implementation as a unit. Do not retain the old renderer as a hidden alternate mode. Any released rollback requires an explicit compatibility decision.

## References

1. [TOON 3.0 reference specification](../../../tests/fixtures/toon-v3/spec.md).
2. [Published codec behavior](../../../docs/toon-codec.md).
3. [Contributor requirements](../../../docs/contributing.md).
4. [Rendering requirements](specs/toon-rendering/spec.md).
5. [Display extension requirements](specs/toon-display-extensions/spec.md).
6. [Navigation requirements](specs/toon-navigation/spec.md).
