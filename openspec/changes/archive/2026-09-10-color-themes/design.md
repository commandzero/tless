## Context

This is a retrospective migration of the implemented theme system. See proposal.md for scope. Theme owns semantic style resolution; rendering owns span splitting, clipping, cursor placement, and row filling. Config resolves names and overrides. Terminal adapters emit colors and attributes. The status-path renderer deliberately forces black and clears reverse video.

## Goals / Non-Goals

Capture observable contracts and retain source attribution without promoting obsolete proposals to requirements. Do not change runtime behavior, navigation, codecs, palette source files, or licenses.

## Decisions

Use a new color-themes capability for palette selection and styling contracts. Modify only overlapping toon-rendering requirements to qualify default behavior and describe existing theme exceptions. Existing toon-navigation and toon-display-extensions specs already cover logical focus, hidden search matches, collapse, warnings, and data/export separation; duplicating them would create conflicting authorities.

Preserve the original documents under sources/ in this archive with relative links repaired. The original color-themes document contains illustrative old names and an `underline` field spelling; current code uses `underlined`. The upstream rendering and search notes include proposals such as soft wrapping and incremental search, not implementation requirements. The TOON review concerns a removed vendored codec and remains historical evidence, not a new codec specification.

Keep the Borealis source mapping and hashes with the preserved reference. The new spec records observable palette colors. Keep docs/vim-themes.md as the companion audit/reference because it is outside the requested migration scope.

## Risks / Trade-offs

Historical notes can be mistaken for current contracts. The sources index explicitly separates history from normative specs. Existing TOON requirements are retained with all scenarios; changes only reconcile theme-specific styling already present in code.

## Migration Plan

Create deltas, preserve source documents, repair incoming links, verify the implementation with existing tests, sync every delta, and archive. Reverting these documentation edits restores the prior locations without runtime changes.
