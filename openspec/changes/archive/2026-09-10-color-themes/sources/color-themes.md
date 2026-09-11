---
type: Design
title: Color themes
description: Original default and Cyan theme design and implementation checklist.
generated: { by: codex, at: 2026-09-10T21:13:08Z }
---

# Color themes

This document records the original default/Cyan design and the later
configuration additions. The [Vim companion palettes](../../../../../docs/vim-themes.md) add
256-color built-ins and document backgrounds.

Status: implemented. Theme configuration is enabled by the `colorscheme`
feature, which is included in the default build. A `--no-default-features`
build keeps the default appearance without configuration or theme options.

## Problem

Previously, rendering code chose terminal colors and attributes directly in
`highlighting`, `lineprinter`, and `screenwriter`. A palette change required
editing several rendering paths and their tests. The theme module now owns
styling policy and supports selectable built-in palettes and user color overrides.

Underline support is implemented in all terminal adapters. `AnsiTerminal`
emits SGR 4 to enable underline and SGR 24 to disable it, preserving bold and
dimmed state. Tests cover these transitions.

Selected rows fill the complete terminal width with the theme's selection
background, including indentation, spaces, and clipping markers. Themes without
a distinct selection background keep their existing appearance. Search matches
retain their theme styles over the row background.

## Goals

- Let users select a built-in or configured color theme with `--theme <name>`.
- Let users switch themes during a session with `:colorscheme <name>`.
- Let users define named themes and 256-color values in
  `$XDG_CONFIG_HOME/tless/config.yaml`, falling back to
  `$HOME/.config/tless/config.yaml`. See the [configuration reference](../../../../../README.md#color-themes).
- Keep all mappings from semantic display roles to terminal styles in one
  module.
- Keep rendering logic independent of named colors.
- Preserve the current `main` appearance as the default.
- Retain the palette started on this branch and the bundled Vim companions.
- Make focus and search-match precedence explicit and testable.
- Support underline as a normal terminal style attribute without leaking it
  into following text.

## Non-goals

- Separate named theme files and user-defined style attributes. Color
  overrides in `config.yaml` are supported.
- Automatic terminal background detection.
- User-defined true-color palette definitions. The built-in [Borealis](borealis.md)
  palette uses exact RGB colors.
- Redesigning the existing `Terminal` interface beyond correct underline
  support.

The theme interface keeps rendering callers independent of configuration
format and terminal color representation.

## User interface

Add the `colorscheme` feature and a theme name option:

```rust
#[cfg(feature = "colorscheme")]
pub enum ThemeName {
    Default,
    Cyan,
}

pub struct Opt {
    #[arg(long)]
    pub theme: Option<String>,
}
```

`default` reproduces the appearance on `main`. `cyan` contains the intended
palette changes from the current branch. `Config` resolves built-in and named
themes and reports unknown names with the available choices.

## Theme module

Add `src/theme.rs`. It owns semantic styling policy and returns the existing
`terminal::Style` value.

```rust
pub struct Theme {
    // Resolved built-in palette. Fields are private.
}

pub enum StyleRole {
    Document,
    JsonValue(JsonValueKind),
    ObjectKey,
    ArrayIndex,
    Punctuation,
    PunctuationCommaTrailing,
    ContainerDelimiter,
    PreviewText,
    PreviewCount,
    LineNumber,
    RowMarkerEmpty,
    IndicatorTruncation,
    StatusBar,
    StatusPathBase,
    StatusText,
    Message(MessageSeverity),
}

pub enum JsonValueKind {
    Null,
    Boolean,
    Number,
    String,
    EmptyObject,
    EmptyArray,
}

pub enum DisplayContext {
    Main,
    Preview,
}

pub enum FocusState {
    None,
    Row,
    PairedContainer,
}

pub enum SearchState {
    None,
    Match,
    CurrentMatch,
}

pub struct StyleState {
    pub context: DisplayContext,
    pub focus: FocusState,
    pub search: SearchState,
}

impl Theme {
    pub fn built_in(name: ThemeName) -> Self;
    pub fn style(&self, role: StyleRole, state: StyleState) -> Style;
}
```

`Theme` is a concrete value, not a trait. Built-in themes are data variants of
one implementation, not terminal adapters. Tests use the real theme directly.

### Invariants

- `style` is total, deterministic, allocation-free, and performs no I/O.
- Every built-in theme defines every semantic role.
- Styling starts with the base role and applies focus. A search match then
  replaces the entire style with the context-specific match style; a current
  match replaces it with the current-match style. Search styles do not retain
  attributes from the base role or focus. Configured color overrides apply
  after this selection.
- `CurrentMatch` selects a complete style. Callers do not also apply `Match`.
- Preview rendering reduces `CurrentMatch` to `Match`, preserving the current
  behavior that a collapsed preview does not identify which match navigation
  will visit.
- Callers supply search state only for searchable text. `Theme::style` applies
  a supplied search state regardless of role, including synthetic array indices.
- Rendering code never handles missing theme entries or theme-selection
  errors. Configuration resolves a named theme before rendering starts.

## Ownership and seam placement

`App` resolves the startup theme from `Opt` and `Config`. `ScreenWriter` owns
the current `Theme` and passes `&Theme` to `LinePrinter` and highlighting
functions. The `:colorscheme` command replaces that value and refreshes the
command-line highlighter before the next redraw.

The theme seam sits between semantic rendering decisions and
`terminal::Style`. The existing `Terminal` seam continues to own output
mechanics. `AnsiTerminal` remains the production adapter;
`TextOnlyTerminal` and `VisibleEscapesTerminal` remain local test adapters.
Terminal adapters must not know about JSON values, focus, search, or theme
names.

The following policy moves behind `Theme`:

- all constants currently in `highlighting`;
- `LinePrinter::color_for_value_type`;
- direct color and style construction in `lineprinter` and `screenwriter`;
- `MessageSeverity::color`;
- focus, preview, and search-emphasis combinations.

Substring splitting, match-range iteration, truncation, and cursor placement
remain rendering responsibilities.

## Palette requirements

`default` must match `main` exactly. This protects existing users and gives the
migration a stable oracle.

`cyan` applies the branch's intended differences:

| Role or state | Default | Cyan |
| --- | --- | --- |
| Null | white | light blue |
| Boolean | blue | magenta |
| Number | magenta | magenta |
| String | green | green |
| Empty object or array | white | light black |
| Object key | cyan | cyan |
| Focused object key | light cyan | light cyan |
| Primitive trailing comma | default | dimmed |
| Unfocused container delimiter | light black | dimmed |
| Focused or paired container delimiter | white | yellow |
| Warning annotation | yellow, dimmed | yellow, dimmed |
| Collapsed count or preview text | light black | light black |
| Line number | light black; white when focused | light black; yellow when focused |
| Status/path bar | black on light black | black on light black |
| Filename | white on light black | white on light black |
| Search match in preview text | yellow, underlined | light black |
| Current search match | light yellow, underlined | light yellow, underlined |

All roles not listed in the table initially inherit their `default` mapping.
Any further difference must be added to this table before implementation.

## Terminal underline behavior

- `Style` includes `underline: bool` with a default of `false`.
- Enabling underline emits SGR 4.
- Disabling underline emits SGR 24. It must not emit SGR 22 or alter bold or
  dimmed state.
- `set_style` updates underline alongside the existing attributes.
- Every `Terminal` adapter implements underline behavior. The visible-escape
  adapter exposes underline transitions so rendering assertions can observe
  them.

## Acceptance criteria

- `tless --theme default` renders the same style transitions as `main` for a
  representative document.
- `tless --theme cyan` renders every difference in the palette table.
- Running without `--theme` selects the configured `colorscheme`, or `default`.
- An unknown theme name reports the available built-in and configured names.
- No rendering module refers directly to terminal color constants. Direct
  colors are limited to terminal color definitions and built-in theme data.
- Focused and ordinary search matches remain distinguishable in both themes.
- Underline ends immediately when the next non-underlined style is applied.
- `cargo test` and `cargo check` pass without warnings introduced by this
  change.

## Verification

- Add table-driven theme tests covering every `StyleRole` under relevant
  focus, search, and preview states for both themes.
- Add `AnsiTerminal` transition tests for enabling and disabling underline,
  including transitions to bold and dimmed text.
- Extend `VisibleEscapesTerminal` with observable underline state.
- Run existing line-printer tests once with an explicitly selected `default`
  theme. Add focused tests for the `cyan` differences rather than duplicating
  the entire suite.
- Add an option-parsing test for each valid theme and one invalid name.

## Implementation order

1. Repair underline emission and all terminal test adapters.
2. Add the theme types and complete built-in mappings with table tests.
3. Add theme selection to options and construct it in `App`.
4. Store the theme in `ScreenWriter` and pass it to line rendering and
   highlighting.
5. Replace direct styles with semantic lookups, one rendering area at a time.
6. Remove obsolete constants, mappings, and stale styling documentation.
7. Run the full test suite and compare `default` output with `main`.
