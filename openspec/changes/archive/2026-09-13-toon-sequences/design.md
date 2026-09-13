## Context

See [the proposal](proposal.md) for motivation and scope. `Layout` currently emits a separator line owned by each parsed root, but marks it as an annotation. The viewer and screen writer skip these lines for navigation and collapse controls. Root objects have no header and are deliberately non-collapsible. Parsed roots already carry sibling links, and their immediate children already point to them as parents.

## Goals / Non-Goals

The design should reuse parsed root identity for document selection, keep navigation independent of display indentation, and preserve the existing serializer boundary. It must support scalar and empty roots as well as ordinary object documents.

This change does not add a parser, streaming input, a serialized sequence format, or a synthetic array. It does not alter single-root presentation or standard TOON export support.

## Decisions

### Anchor each root at its document row

Replace the passive separator with a document header in multi-root layouts. Its owner remains the existing parsed root, and the root's primary selection anchor becomes that header. Existing parent and root sibling links then implement `[` and `]` without adding a second root container. Child motion from an expanded document follows the root's immediate fields or array elements. Scalar and empty roots have no logical children.

Keep any body line needed to display a scalar, an empty value, or an array header associated with the same root. Vertical selection must retain the selected body line where a root owns multiple lines, while structural parent and sibling motions resolve to the document header. Review the current one-anchor-per-node assumptions in focus and line mapping during implementation.

A synthetic parsed wrapper would alter paths, export behavior, and parent relationships. A purely decorative row would preserve the current navigation problem. A document header backed by the existing root avoids both costs.

### Keep document visibility state in the view

Track document collapse by root identity in the viewer's presentation state. This supports scalar and empty documents without inventing container values or changing parser rows. Projection hides the root's body when its document is collapsed. Route shallow, deep, sibling, mouse, and search-reveal operations through this state alongside existing descendant collapse state. Expanding a document restores descendant state unless the command explicitly requests deep expansion.

### Separate hierarchy from indentation

Render document bodies at their existing root depth. The document header adds an addressable line but no indentation. Existing nested arrays, tables, and object fields keep their normal layout rules. Do not merge adjacent document roots into a table, even if their shapes match.

### Keep the position visible and preview contents only when collapsed

Render `---` followed by a subdued position annotation in both states. Append a bounded contents preview in document order only when collapsed. For example, the following shows two expanded documents, with arrows in the gutter:

```text
▾ --- (1 of 2)
  name: Ada
  active: true
▾ --- (2 of 2)
  name: Lin
  active: false
```

Collapsing the first document hides its body, changes its arrow, and adds the subdued contents preview:

```text
▸ --- (1 of 2) name: Ada; active: true
▾ --- (2 of 2)
  name: Lin
  active: false
```

Use `{}` and `[]` to identify empty roots in collapsed previews. Scalar previews use the existing terminal-safe scalar spelling. Reuse preview bounds and theme annotation styling, including the subdued selected state. Keep the sequence position separate from preview text so width fitting preserves it first. Document headers remain unwrapped, following the current separator and collapsed-preview width behavior.

### Retain data warnings and serialization boundaries

Stop generating `MultipleRoots` warnings. Retain all other warning kinds and aggregate hidden warnings when a document collapses. Positions, headers, and preview spans are generated content and cannot create search matches. Copy and focused print resolve a header to its parsed root; whole-document export keeps its current rules, including standard TOON rejection of multiple roots. No new dependency or codec change is needed.

## Risks / Trade-offs

- Root identity currently maps to one primary line. Keep header anchors and body-line selection distinct, with navigation, search, mouse, and resize tests for scalar and array documents.
- Document collapse lives beside existing container collapse. Centralize view operations so search, bulk controls, and ancestor reveal cannot disagree about visibility.
- Large documents can make contents previews expensive. Bound preview generation and cache it per root, and exclude generated spans from search and serialization.
- Removing separators from navigation filters affects gutters and line jumps. Test absolute addresses, relative distances, wrapping, and collapsed gaps together.

## Migration Plan

Implement the layout, view state, and navigation changes together, then update regression tests and user help. Run the repository preflight and Rust 1.87 checks before completing implementation. Synchronize the delta specs and archive this change only after implementation verification, following the repository's OpenSpec completion gate.

There is no stored-data migration. Reverting the implementation restores the previous separator presentation without changing user input files.
