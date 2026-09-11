# color-themes Specification

## Purpose

Define selectable terminal color themes, their configuration and styling precedence, and the built-in palette contracts for tless.

## Requirements

### Requirement: Theme selection and feature boundary

Theme support SHALL be enabled by the default `colorscheme` build feature. Startup SHALL select an explicit `--theme` value before configured `colorscheme`, falling back to `default`. `classic` SHALL remain an alias for the tless default; `vim` SHALL select Vim's default companion. The built-ins SHALL include `cyan`, `borealis`, and the 28 audited Vim companions under their original names without a `vim-` prefix, except Vim's default named `vim`. Builds without `colorscheme` SHALL retain the tless default appearance without theme configuration or selection options.

#### Scenario: Startup precedence

- **WHEN** configuration selects `borealis` and the command line selects `default`
- **THEN** tless SHALL use its default terminal palette
- **AND** omitting the command-line selection SHALL use Borealis

#### Scenario: Runtime switching

- **WHEN** a user enters `:colorscheme borealis`
- **THEN** document, status, and command-entry styling SHALL refresh with Borealis
- **AND** navigation position and search state SHALL survive the switch

#### Scenario: Unknown theme

- **WHEN** a requested theme is neither built-in nor configured
- **THEN** tless SHALL report the unknown name and the available names

### Requirement: Named theme configuration

Configuration SHALL read `$XDG_CONFIG_HOME/tless/config.yaml`, falling back to `$HOME/.config/tless/config.yaml`. Missing or empty configuration SHALL retain defaults. `themes` SHALL define named role-color mappings; a built-in name SHALL override that palette, and a new name SHALL inherit the default palette. Colors SHALL accept terminal names, their light variants, `default`, and integer indexes 0 through 255. Invalid keys, values, theme names, and malformed mappings SHALL produce configuration errors. RGB strings and user-defined style attributes SHALL NOT be accepted.

#### Scenario: Partial custom palette

- **WHEN** a configured new theme supplies only a string color of 217
- **THEN** strings SHALL use terminal index 217 and unspecified roles SHALL inherit the default

#### Scenario: Invalid palette value

- **WHEN** a theme specifies index 256 or an RGB hex string
- **THEN** configuration loading SHALL reject that value with an explanatory error

### Requirement: Styling precedence and row colors

Styling SHALL apply a role's base appearance, then focus, then a complete search style. Search SHALL replace base and focus attributes. Configured colors SHALL apply after that selection. Collapsed previews SHALL reduce current-match styling to ordinary matches for companion themes; the default SHALL retain bright-yellow current matches. Explicit status and command foreground/background settings SHALL describe visible colors even for inverted themes. Message severity foreground colors SHALL take precedence over command text colors. Document background overrides SHALL apply to document roles without changing status or command backgrounds.

#### Scenario: Search over focused text

- **WHEN** a focused value also contains a search match
- **THEN** the match SHALL use the theme's search appearance without inheriting the value's focus attributes

#### Scenario: Status configuration

- **WHEN** the user configures a status background and severity foreground
- **THEN** the status row SHALL use that background and messages SHALL retain their severity foreground
- **AND** the rendered status path SHALL remain black with reverse video disabled, independently of filename styling

### Requirement: Default and Cyan palettes

The tless default SHALL retain the terminal colors, annotation styling, and status contract in the toon-rendering specification. Selected keys SHALL brighten from cyan 6 to 14, strings from green 2 to 10, numbers from magenta 5 to 13, booleans from blue 4 to 12, and nulls from gray 7 to 15. Warning annotations SHALL remain yellow and dimmed without focus brightening. Preview text and collapsed object counts SHALL remain plain dark gray 8. Default highlighting SHALL NOT use bold or reverse video. Cyan SHALL retain its alternate null, boolean, delimiter, focus, and search mappings listed below; unspecified roles SHALL inherit the default base mapping.

| Role or state | Cyan |
| --- | --- |
| Null | Light blue |
| Boolean | Magenta |
| Object key, ordinary / focused | Cyan / light cyan |
| Primitive trailing comma | Dimmed |
| Container delimiter, ordinary / focused or paired | Dimmed / yellow |
| Line number, ordinary / focused | Dark gray / yellow |
| Ordinary preview search | Dark gray |
| Current search | Light yellow, underlined |

#### Scenario: Default selected value

- **WHEN** a string is selected with the default theme and no search match
- **THEN** its foreground SHALL be terminal index 10 without bold or reverse video

### Requirement: Terminal color and attribute transitions

Terminal output SHALL support 16-color indexes, 256-color indexes in colorscheme builds, and built-in RGB foreground/background colors. Enabling underline SHALL emit SGR 4 and disabling it SHALL emit SGR 24 without changing bold or dimmed state. Subsequent text SHALL NOT inherit stale underline or background attributes after a style change or reset.

#### Scenario: RGB and underline reset

- **WHEN** RGB underlined text is followed by plain indexed-color text
- **THEN** output SHALL switch colors and disable underline without leaking the prior style

### Requirement: Borealis palette and source fidelity

Borealis SHALL use the dark-mode block of the preserved CSS source for the following RGB mappings, with the string color from the preserved TextMate source. It SHALL emit exact 24-bit colors without terminal-capability detection or 256-color approximation. Named and indexed overrides SHALL continue to apply.

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

Borealis SHALL use no bold. Focus SHALL preserve pink and other syntax foregrounds while using the selection background. Search SHALL use a yellow underline on the document background, replacing focus. Field definitions SHALL use standard text color while retaining their source mapping for search and navigation. The status path SHALL retain the black-text rendering override.

#### Scenario: Borealis structural text

- **WHEN** Borealis renders counts, colons, commas, and table field definitions
- **THEN** they SHALL use `#CAD3E2`
- **AND** normal document background SHALL be `#050F21`

#### Scenario: Borealis current search

- **WHEN** the active match occurs in the main document
- **THEN** it SHALL use `#FCD883` underlined on the document background without bold
- **AND** a collapsed preview SHALL instead use ordinary search color `#FACB3D`
