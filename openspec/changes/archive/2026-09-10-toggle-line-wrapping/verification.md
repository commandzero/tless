# Verification: toggle-line-wrapping

## Implementation correspondence

Reviewed all 7 changed or added requirements and their 25 scenarios against the implementation and regression coverage.

| Requirement | Scenario coverage and implementation evidence |
| --- | --- |
| Native TOON layout | Primitive-array defaults, tables, empty containers, and root objects retain the existing logical layout. `wrapped_view` projects its output without changing parsed values or layout decisions. Layout regression tests and wrapped export/print tests pass. |
| Gutters and terminal width | Shared-line numbering remains logical; the screenwriter paints numbers and arrows only on first physical rows. Narrow widths, horizontal fallback, bounded painting, original-span focus/search styling, and blank continuation gutters have unit and terminal coverage. |
| Optional line wrapping | Terminal tests cover default-off, toggle/count handling, gutters, collapsed previews, horizontal controls, and unchanged export. Wrapper and painter tests cover exact fits, combining and emoji graphemes, escaped controls, overwide placeholders, zero width, and large narrow values. Prompt editing retains its separate input path; interactive help documents the toggle. |
| Vertical and structural motion | Existing parent/sibling, table-field, inline-expansion, inline-sibling, and root-focus behavior remains in the logical viewer model. Wrapped logical-motion tests confirm continuations do not become navigation entries. |
| Search maps data to rendered spans | Existing hidden-table search and new scalar/shared-header terminal tests verify owner identity and matching-span reveal. Reflow restores the source anchor before translating the match into physical rows. |
| Viewport and selection stability | Existing collapse-ancestor and mouse/resize tests retain identity. New reflow tests cover stored display ranges and cache invalidation; continuation mouse tests verify the original copy target. |
| Physical viewport scrolling for wrapped lines | Viewer tests cover logical versus physical motion, tall values at document end, match-based repositioning, reflow, and collapse invalidation. Terminal tests cover tall-value scrolling, continuation clicks, scalar/header search across resize, and logical motion past wrapped values. Painter tests cover cross-row search styles and hit boundaries. |

Follow-up verification found half-page scrolling could change focus while the selected wrapped line remained visible. The fix applies the same intersection rule to half-page movement and viewport boundaries. Regression tests cover selected table cells, surrounding entries, and both boundaries. No outstanding implementation findings remain. Physical row records contain ranges into existing display text rather than copied row strings. Reflow preserves logical ownership, and explicit viewport scrolling can traverse a value taller than the screen without snapping back to its start.

## Validation

- Rust 1.87.0: `TLESS_TOOLCHAIN=1.87.0 scripts/preflight.sh test` passed all 4 feature profiles: 630 test executions.
- Rust 1.97.1: `scripts/preflight.sh test` passed the same 4 profiles: 630 test executions.
- Per compiler: minimal 143, default 170, sexp 145, all features 172; no failed or ignored tests.
- Strict native change validation passed before archival.
- Delta-to-main review: all 7 requirement blocks and all 25 scenarios match exactly across `toon-rendering` and `toon-navigation`; unrelated requirements remain unchanged.
- Full `scripts/preflight.sh` passed on Rust 1.97.1: formatting, strict minimal/all-feature Clippy, shell and workflow checks, release/OpenSpec gate regression tests, documentation validation, and all 4 test profiles. The existing actionlint 1.7.12 installation was added to PATH for this run.
- Committed-head OpenSpec gate passed against fetched `origin/main`, with both association and sync-review fields: preserved archive, strict archived delta validation, completed archived tasks, and both affected main specs.
- `git diff --check` passed. The existing `block v0.1.6` dependency emits Cargo’s future-incompatibility notice; no dependency changes were made.

Terminal integration tests ran with pseudoterminal access. Release-only manual platform acceptance remains part of the release checklist.
