# TOON sequences verification

## Implementation coverage

Reviewed the four added and nine modified requirements against the implementation and their scenarios. Existing scenarios remain covered by the retained rendering, navigation, extension, and terminal tests. The review found and resolved array-body control, body-anchor restoration, deep/shallow expansion, scalar-body bulk collapse, and collapsed-header mouse-selection defects.

| Area | Evidence |
| --- | --- |
| Document rows and input equivalence | `Layout::build` retains parsed roots and creates ordered headers; layout tests compare JSON and YAML sequences and preserve single-root rendering. |
| Position, preview, styling, width | `document_preview`, `project_with_documents`, and `fit_annotations` preserve counts and bound previews. Layout, lineprinter, and PTY tests verify count-only expanded headers, collapsed content, Unicode fitting, and unchanged indentation. Existing theme tests cover subdued annotation styling. |
| Collapse and root shapes | Presentation state in `JsonViewer` handles scalar and empty documents; root-array body state remains separate. Tests cover independent documents, nested collapse restoration, bulk operations from headers and bodies, and deep expansion. |
| Navigation and line identity | Header/body anchors and cached root metadata support parent/sibling motions, numbered jumps, mouse selection, and resize. Viewer and PTY tests retain array-body controls and keep collapsed-header text clicks from expanding documents. |
| Search and value actions | Search still scans parsed source. Tests reveal hidden matches, preserve duplicate occurrence identity, exclude generated positions, print a collapsed root, write every JSON root after collapse, and preserve failed multi-root TOON export behavior. Copy uses the same unchanged parsed-value selection path as print. |
| Warnings and serialization | `MultipleRoots` is removed. Tests preserve scalar warnings and nested duplicate-warning summaries. Parser rows, paths, codecs, and serializers retain their existing data contracts. |
| Wrapping and existing behavior | Document headers remain one physical row. The retained wrapping, viewport, gutter, table, inline-array, output, and terminal regression suites cover the unchanged contracts. |

## Checks

- Final Rust 1.87 run: all five feature profiles passed. Unit counts were 156, 182, 158, 182, and 184. Each profile also passed 9 piped-output tests and 45 CLI/terminal tests.
- Targeted sequence PTY run: 6 passed with isolated terminal access.
- All-target, all-feature Clippy passed with warnings denied.
- Documentation validation passed with 0 errors and 0 warnings. The existing `vim-themes.svg` informational note remains.
- Strict native validation passed for the active change and all 6 main specs after synchronization.
- Full repository preflight passed formatting, Clippy, shell/workflow checks, release/OpenSpec regression checks, documentation validation, the committed OpenSpec completion gate, and all five development-toolchain test profiles. Each profile passed 9 piped-output and 45 CLI/terminal tests.
- One earlier full run failed the existing broken-pipe test with exit 0 instead of 1. Its isolated rerun and the complete preflight retry passed. The broken-pipe test and machine-output implementation are unchanged by this change.
- Native archived validation passed. All 12 tasks are complete; the archive and main specs match.

## Spec correspondence

Merged all four added requirements into the new `toon-sequences` main spec. Merged three modified requirements in `toon-display-extensions`, two in `toon-navigation`, and four in `toon-rendering`. Reviewed every delta block against its main-spec counterpart and preserved unrelated requirements and all existing scenarios. No requirements were removed or renamed. The committed-state gate ran with `OpenSpec-Change: toon-sequences` and `OpenSpec-Sync-Reviewed: toon-sequences`.

## Scope and limitations

This is a local implementation check on macOS. The automated terminal checks use disposable PTYs. The release acceptance checklist and native release-platform matrix remain release-time work; no release or registry publication was performed.
