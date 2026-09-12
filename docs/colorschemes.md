---
type: Reference
title: Color schemes
description: Color scheme selection, YAML configuration, accepted colors, and display token mappings.
status: stable
sources:
  - id: color-themes-spec
    resource: ../openspec/specs/color-themes/spec.md
    title: Color themes specification
  - id: theme-implementation
    resource: ../src/theme.rs
    title: Theme token and precedence implementation
  - id: config-implementation
    resource: ../src/config.rs
    title: Theme configuration parser
  - id: config-example
    resource: ../examples/config.yaml
    title: Theme configuration example
generated: { by: codex/gpt-6, at: 2026-09-12T00:50:56Z }
---

# Color schemes

Tless color schemes control the terminal colors used for document values,
structure, focus, search, status rows, and messages. Theme support is enabled
by the default `colorscheme` feature. A build made with
`--no-default-features` keeps the default appearance but does not provide theme
selection or configuration.

## Select a scheme

Choose a scheme when starting tless:

```sh
tless --theme borealis data.json
tless --theme navy data.json
```

You can also switch during a session:

```text
:colorscheme borealis
```

Startup selection follows this order:

1. An explicit `--theme <name>` option.
2. The top-level `colorscheme` value in the user configuration.
3. The built-in `default` scheme.

The built-in names are `default`, `cyan`, `borealis`, and the 28 Vim companion
palettes. `classic` is an alias for `default`. `vim` selects Vim's default
companion. See [Vim companion palettes](vim-themes.md) for the complete
companion list and their role mappings.

If a configured theme has the same name as a built-in scheme, its entries
override that built-in palette. A new configured name starts with the default
palette and overrides the entries it supplies.

## Configuration file

Tless reads one YAML document from:

1. `$XDG_CONFIG_HOME/tless/config.yaml`, when `XDG_CONFIG_HOME` is set and non-empty.
2. `$HOME/.config/tless/config.yaml`, otherwise.

A missing file, an empty YAML document, or an explicit YAML `null` document
keeps the built-in defaults. Otherwise, the root must be a mapping. The only accepted top-level keys are `colorscheme` and
`themes`.

`colorscheme` is a non-empty string naming the startup scheme. `themes` maps
non-empty theme names to color mappings. Every color mapping key must be a
supported theme token, and every value must be a supported color name or an
integer in the range 0 through 255.

This is a complete small example:

```yaml
colorscheme: navy
themes:
  navy:
    object-key: blue
    object-key-focused: light-blue
    string: cyan
    status-bar-background: 18
    status-bar-foreground: cyan
  paper:
    document-foreground: black
    document-background: white
    string: blue
    number: magenta
    search-match-current: red
```

Unlisted tokens inherit the selected base style. For a new theme such as
`paper`, that base is `default`. For a name matching a built-in scheme, such as
`borealis`, the base is that built-in scheme.

The parser rejects unknown top-level keys, unknown theme tokens, empty theme
names, invalid color values, non-mapping `themes` values, and YAML documents
with more than one document. RGB strings such as `#112233` are not accepted in
user configuration. The built-in Borealis scheme uses RGB colors internally,
but custom overrides use the color forms described below.

## Color values

Named colors use the terminal's 16-color palette:

| Name | ANSI index |
| --- | ---: |
| `black` | 0 |
| `red` | 1 |
| `green` | 2 |
| `yellow` | 3 |
| `blue` | 4 |
| `magenta` | 5 |
| `cyan` | 6 |
| `white` | 7 |
| `light-black` | 8 |
| `light-red` | 9 |
| `light-green` | 10 |
| `light-yellow` | 11 |
| `light-blue` | 12 |
| `light-magenta` | 13 |
| `light-cyan` | 14 |
| `light-white` | 15 |
| `default` | Terminal default |

An integer selects an entry from the terminal's ANSI 256-color palette. Values
must be between `0` and `255`, inclusive. The first 16 indexes correspond to
the named colors above. Indexes 16 through 255 depend on the standard xterm
256-color palette, although a terminal may customize its first 16 entries.

Configuration sets colors only. It cannot set bold, underline, dim, reverse
video, RGB values, or other terminal attributes. Built-in schemes may use those
attributes as part of their own palette.

## Theme tokens

The following keys are accepted inside each named theme. The aliases in the
last column are accepted for compatibility and map to the same token.

| Token | Applies to | Aliases |
| --- | --- | --- |
| `document-foreground` | The document foreground and roles whose base foreground is otherwise the terminal default. | |
| `document-background` | The background of document rows and document content. | |
| `null` | JSON and YAML `null` values. | |
| `boolean` | Boolean values. | |
| `number` | Numeric values. | |
| `string` | String values. | |
| `empty-container` | Empty objects and empty arrays. | |
| `object-key` | Object keys and TOON table field definitions. | |
| `object-key-focused` | A focused object key or table field definition. | `focused-object-key` |
| `array-index` | Array indexes, including focused array indexes. | |
| `punctuation` | Structural punctuation such as colons and ordinary separators. | |
| `punctuation-comma-trailing` | Commas that follow primitive values in the rendered document. | `primitive-trailing-comma` |
| `container-delimiter` | Brackets, braces, and other container delimiters. | |
| `container-delimiter-focused` | A focused or paired container delimiter. | `focused-container-delimiter` |
| `ellipsis` | The ellipsis shown when horizontal scrolling hides part of a line. | |
| `preview-text` | Text inside a collapsed container preview. | |
| `preview-count` | The item count shown in a collapsed container preview. | |
| `line-number` | Ordinary line numbers. | |
| `line-number-focused` | The line number on the focused row. | `focused-line-number` |
| `row-marker-empty` | The marker shown on unused or empty rows. | `empty-row-marker` |
| `indicator-truncation` | The indicator shown when rendered content is truncated. | `truncation-indicator` |
| `status-bar` | The status row and the path portion of that row. | |
| `status-text` | The prompt and text rendered in the command or search row. | |
| `status-bar-foreground` | The visible foreground of the status bar and path. | |
| `status-bar-background` | The visible background of the status bar and path. | |
| `command-line-foreground` | The visible foreground of the command or search row. | |
| `command-line-background` | The visible background of the command or search row. | |
| `message-info` | Informational messages. | |
| `message-warning` | Warning messages. | |
| `message-error` | Error messages. | |
| `search-match` | An ordinary search match in the main document. | |
| `search-match-preview` | An ordinary search match inside a collapsed preview. | |
| `search-match-current` | The current search match. | |

Quote the `null` token so YAML treats it as a string key:

```yaml
themes:
  navy:
    "null": magenta
```

## Styling order

For the default and configured themes, tless chooses a role's base style first.
It then applies row focus. A search match replaces both the base and focus
styles with the selected search style. Configured colors are applied after that
selection, so a configured token describes the visible color even when a
built-in style would otherwise use reverse video.

The main precedence rules are:

- A focused object key or field definition uses `object-key-focused`. Focused
  delimiters and line numbers use their focused tokens. Focused array indexes
  continue to use `array-index`; there is no separate focused array-index token.
- A search match takes precedence over focus. `search-match-current` takes
  precedence over `search-match` for the active match.
- Search matches in previews use `search-match-preview`.
- `status-bar-foreground` and `status-bar-background` control the status row
  independently of document colors.
- `command-line-foreground` and `command-line-background` control the command
  and search row. Message tokens keep their severity foreground over that row
  background.
- `document-background` applies to document content and rows, while status and
  command rows use their own row colors.

The resolved base palette controls current matches in collapsed previews.
A non-default base (`cyan`, `borealis`, or a Vim companion) uses the ordinary
preview match color, including when configuration overrides that palette.
The `default` base keeps the distinct current-match color. New configured
theme names inherit `default` and therefore keep that distinction too.

## Related references

- [Vim companion palettes](vim-themes.md) lists the 28 bundled palettes and
  their source role mappings.
- [Color themes specification](../openspec/specs/color-themes/spec.md) records
  the selection, configuration, precedence, and built-in palette contracts.
- [Theme configuration example](../examples/config.yaml) shows named and
  indexed color overrides.
