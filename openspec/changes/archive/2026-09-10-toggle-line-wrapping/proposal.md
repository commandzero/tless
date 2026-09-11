## Why

Long values currently require horizontal scrolling, which makes them difficult to read in a narrow terminal.
[Issue #6](https://github.com/commandzero/tless/issues/6) requests optional wrapping that preserves logical navigation and keeps continuation text out of the gutter.

## What Changes

- Add Ctrl+L to toggle wrapping for the document view for the current session, initially off.
- Wrap expanded TOON lines at terminal-cell boundaries within the available document width, including long values and table rows.
- Keep each wrapped line as one navigation and numbering entry. Continuations have blank gutters.
- Keep collapsed previews on one physical row with existing truncation and horizontal access.
- Make scrolling, search reveal, mouse selection, and resize use the same mapping between logical lines and physical rows.
- Document the toggle and its interaction with horizontal scrolling.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `toon-rendering`: Optional wrapping, gutter isolation, Unicode boundaries, and collapsed-preview exclusion.
- `toon-navigation`: Logical motions and addresses across physical continuation rows, viewport scrolling, search reveal, and mouse selection.

## Impact

The change affects interactive key dispatch in `src/app.rs`, viewport state in `src/viewer.rs`, painting and hit testing in `src/screenwriter.rs` and `src/lineprinter.rs`, and their use of `src/toon_display.rs` projections.
It also requires help, acceptance documentation, changelog, and rendering/navigation tests.

This is an opt-in interactive feature with no input-format, serialization, CLI flag, dependency, or minimum-Rust-version change.
Piped output and exports retain their existing byte-level behavior.
