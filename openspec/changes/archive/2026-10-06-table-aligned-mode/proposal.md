# Proposal

## Why

TOON tables currently render compact comma-separated values, which makes columns difficult to compare and scrolls only the focused line. [Issue #5](https://github.com/commandzero/tless/issues/5) requests an opt-in aligned presentation with synchronized horizontal scrolling and a visible mode indication.

## What Changes

- Tab toggles alignment for the focused tabular array, including focus on its header, row, or cell; other arrays and prompt input retain existing behavior.
- Pad rendered columns using terminal-cell widths, right-align parsed numbers, and keep headers and non-numeric values left-aligned, including numeric-looking strings. Use spaces instead of generated comma separators and preserve commas within data tokens. Include off-screen rows in column widths without retaining their rendered presentation.
- While aligned, use one horizontal offset for the table header and all rows. Keep existing counted `,` and `.` movement and make end/start scrolling and search reveal use that same offset.
- Keep an aligned table unwrapped even if session wrapping is on; leave other content's wrapping unchanged. Turning alignment off restores normal presentation under the current wrapping setting.
- Show a persistent alignment indicator for the focused table and document Tab in interactive help.
- Preserve parsed values, logical selection, collapse choices, source mappings, copy/export behavior, and piped output. Alignment is interactive presentation, not a new TOON encoding.

## Capabilities

### New Capabilities

None. Existing rendering and navigation capabilities own this behavior.

### Modified Capabilities

- `toon-rendering`: Add opt-in table alignment and its lifecycle; clarify the native-text and wrapping contracts for aligned tables.
- `toon-navigation`: Add table-wide horizontal scrolling and source-identity behavior for padded content; clarify search reveal for unwrapped aligned tables.

## Impact

- Document-view key dispatch and status rendering in `src/app.rs` and `src/screenwriter.rs`.
- Viewer presentation state and wrap eligibility in `src/viewer.rs`; table analysis, geometry, formatting, and source spans in `src/toon_display/`.
- Existing unit and PTY integration coverage, particularly `tests/toon_cli.rs`.
- Interactive help, `docs/toon-view.md`, `docs/toon-acceptance.md`, and the Unreleased changelog during implementation.
- No new dependencies, parser or codec changes, CLI flags, persisted settings, or changes to machine output. Rust 1.87 compatibility remains required.
