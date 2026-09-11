# Verification: toggle-line-wrapping

## Implementation correspondence

Reviewed all 8 changed or added requirements and their 26 scenarios against the implementation and regression coverage.

| Requirement | Scenario coverage and implementation evidence |
| --- | --- |
| Native TOON layout | Primitive-array defaults, tables, empty containers, and root objects retain the existing logical layout. `wrapped_view` projects its output without changing parsed values or layout decisions. Layout regression tests and wrapped export/print tests pass. |
| Gutters and terminal width | Shared-line numbering remains logical; the screenwriter paints numbers and arrows only on first physical rows. Narrow widths, horizontal fallback, bounded painting, original-span focus/search styling, and blank continuation gutters have unit and terminal coverage. |
| Optional line wrapping | Terminal tests cover default-off, toggle/count handling, gutters, collapsed previews, horizontal controls, and unchanged export. Wrapper and painter tests cover exact fits, combining and emoji graphemes, escaped controls, overwide placeholders, zero width, and large narrow values. Prompt editing retains its separate input path; interactive help documents the toggle. |
| Vertical and structural motion | Existing parent/sibling, table-field, inline-expansion, inline-sibling, and root-focus behavior remains in the logical viewer model. Wrapped logical-motion tests confirm continuations do not become navigation entries. |
| Search maps data to rendered spans | Existing hidden-table search and new scalar/shared-header terminal tests verify owner identity and matching-span reveal. Reflow restores the source anchor before translating the match into physical rows. |
| Viewport and selection stability | Existing collapse-ancestor and mouse/resize tests retain identity. New reflow tests cover stored display ranges and cache invalidation; continuation mouse tests verify the original copy target. |
| Theme selection background fills the row | Theme row painting fills every selected physical row, including its blank continuation gutter. The wrapped continuation regression verifies selection backgrounds and cross-row search styling under Borealis and Vim themes. |
| Physical viewport scrolling for wrapped lines | Viewer tests cover logical versus physical motion, tall values at document end, match-based repositioning, reflow, and collapse invalidation. Terminal tests cover tall-value scrolling, continuation clicks, scalar/header search across resize, and logical motion past wrapped values. Painter tests cover cross-row search styles and hit boundaries. |

Follow-up verification found half-page scrolling could change focus while the selected wrapped line remained visible. The fix applies the same intersection rule to half-page movement and viewport boundaries. Regression tests cover selected table cells, surrounding entries, and both boundaries. No outstanding implementation findings remain. Physical row records contain ranges into existing display text rather than copied row strings. Reflow preserves logical ownership, and explicit viewport scrolling can traverse a value taller than the screen without snapping back to its start.

## Validation

The final merged implementation was validated on Rust 1.97.1 and Rust 1.87.0. The compiler matrices ran sequentially using `scripts/preflight.sh test` and `TLESS_TOOLCHAIN=1.87.0 scripts/preflight.sh test`. These results supersede the pre-merge four-profile counts.

| Feature profile | Unit tests | Piped-output tests | Terminal tests | Total per compiler |
| --- | ---: | ---: | ---: | ---: |
| No default features | 118 | 9 | 30 | 157 |
| Default features | 164 | 9 | 38 | 211 |
| No default features, sexp | 120 | 9 | 30 | 159 |
| No default features, colorscheme | 145 | 9 | 30 | 184 |
| All features | 166 | 9 | 38 | 213 |
| **Total** | **713** | **45** | **166** | **924** |

No failed or ignored tests. Terminal tests ran with pseudoterminal access. Full Rust 1.97.1 preflight passed formatting, strict minimal/all-feature Clippy, shell and workflow checks, release/OpenSpec gate regression tests, documentation validation, and the five test profiles. The existing actionlint 1.7.12 installation was added to PATH.

Strict native change and main-spec validation passed. All 8 delta requirement blocks and 26 scenarios match the synchronized main specs, including the theme-background extension introduced during the main merge. The committed-head gate passed with the association and sync-review fields; archived tasks and artifacts are preserved. `git diff --check` passed.

The merged dependency update removed the old `block` dependency and its future-incompatibility notice. Release-only manual platform acceptance remains part of the release checklist.

## Integration with main

Merged the published 0.1.0 release, Rust 2024 dependency updates, and configurable themes from main. Wrapping remains unreleased. Wrapped painting shares the theme-aware token style resolver with unwrapped painting; each selected physical row receives the theme selection background, including blank continuation gutters. The rendering delta now scopes its original palette assertions to the default theme, consistent with the merged theme contract. A regression covers continuation backgrounds and cross-row search styling under Borealis and Vim themes.

Merge validation passed: full preflight, strict OpenSpec gate, documentation validation, and all five feature profiles on Rust 1.97.1 and Rust 1.87.0. The final compiler matrices ran sequentially, each with 924 passing test executions and no failures or ignored tests.
