---
type: Reference
title: Borealis theme
description: Borealis RGB palette, source attribution, and tless role mappings.
generated: { by: codex, at: 2026-09-10T20:59:37Z }
sources:
  - id: borealis
    resource: ../examples/themes/borealis.tmTheme
  - id: borealis-css
    resource: ../examples/themes/borealis.css
---

# Borealis

Select `tless --theme borealis data.json`, or enter `:colorscheme borealis`
during a session. Set `colorscheme: borealis` in the tless configuration to
use it at startup. Theme support requires the `colorscheme` feature, enabled
by default.

Borealis follows the `html:has(#dark-mode:checked)` palette in the
[Borealis CSS source](../examples/themes/borealis.css), supplied from
`~/Development/starlift/src/server/assets/theme/borealis.css`.
The [original TextMate source](../examples/themes/borealis.tmTheme) supplies
the teal string color, which the CSS does not define. Both source files are
preserved unchanged.

This built-in requires a true-color terminal. It emits 24-bit foreground and
background sequences without detecting terminal capability or approximating
colors to the 256-color palette. Existing named and indexed configuration
overrides still apply; configuration does not accept RGB strings.

| tless role | Source setting | RGB |
| --- | --- | --- |
| Document background | `--pre-background-color` | `#050F21` |
| Document text, colons, commas, counts, field definitions, command text | `--text-color` | `#CAD3E2` |
| Keys | `--color-primary` | `#61A2FF` |
| Strings | TextMate JSON String Values | `#02BCB7` |
| Numbers and null | `--text-color-code` | `#B386F9` |
| Booleans | `--color-accent` | `#EE72A6` |
| Previews, line numbers, collapse/expand indicators, empty markers | `--text-subdued-color` | `#98A8C3` |
| Focus and status background | `--background-color-primary` | `#0A2342` |
| Warnings | `--text-color-warning` | `#FCD883` |
| Errors, normal / focused | `--color-severity-danger` / `--text-color-danger` | `#EE4C48` / `#F6726A` |
| Search match / current match | `--background-color-warning-filled` / `--text-color-warning` | `#FACB3D` / `#FCD883` |

All CSS values come from the dark-mode block. The deeper and brighter accents
are variants within that palette, not light-mode colors. Pink has one dark-mode
value, so focus preserves it and uses the selection background. No Borealis role
uses bold. Search replaces focus with a yellow underline on the document background.
Collapsed previews use the ordinary search style for the current match, as the
other companion themes do. Search and status mappings are tless adaptations;
CSS does not define terminal roles. Field definitions retain their source
mapping for search and navigation while using standard text color.

CSS source SHA-256: `b100efe037033aea600701f9658b2cf92c1885b7e458673d4a849107030b375d`.

TextMate Source SHA-256: `d43f3c1dbd38f1b9c0069a7b7039148f3573b93d5ed0bdd2d44858728d19549e`.
