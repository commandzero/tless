## MODIFIED Requirements

### Requirement: Syntax styling

Keys and table field names SHALL retain their source identities for search and navigation. Their colors SHALL follow the selected theme; the default SHALL give them the same syntax color, while Borealis SHALL use standard text color for field definitions. Strings, numbers, booleans, nulls, and structural syntax SHALL have distinguishable styles. Default terminal palette indexes SHALL be cyan 6 for keys, green 2 for strings, magenta 5 for numbers, blue 4 for booleans (brightening to 12 when selected), gray 7 for nulls, dark gray 8 for previews, and yellow 3 for warnings. The default status bar SHALL use a dark gray 8 background with black 0 text and a light gray 7 filename. In the default theme, all search matches, including the active match, SHALL be underlined. Default search matches SHALL use yellow 3 foreground and the active match SHALL use bright yellow 11 foreground, overriding value selection colors. The default theme SHALL NOT use reverse video for any element. TOON array counts SHALL use structural styling. Expanded data and complete inline primitive arrays of at most five elements that fit the terminal SHALL retain their data styling, including when collapsed. Only collapsed previews, object count annotations, and extension warning comments SHALL use subdued annotation styling. In the default theme these annotations SHALL remain subdued without focus highlighting or bold when their owning container is selected. Default previews and object counts SHALL use plain terminal color 8 without the dim attribute, matching the default line-number color.

#### Scenario: Expanded primitive array

- **WHEN** `values[3]: 1,true,hello` is expanded
- **THEN** its values SHALL have number, boolean, and string styles
- **AND** these values SHALL NOT be styled as a collapsed preview


### Requirement: Gutters and terminal width

Optional absolute and relative line numbers SHALL remain available outside the document text with existing visibility defaults. Absolute numbers SHALL identify lines in the fully expanded TOON layout, beginning at 1; collapsed descendants SHALL produce gaps. Relative numbers SHALL count visible display-line motions. Automatic primitive-array layouts SHALL be recalculated on terminal-width or gutter-visibility changes while preserving logical selection and collapse states. Explicit multiline choices SHALL survive resize and collapse/reopen. Absolute addresses SHALL follow the current fully expanded layout. Long individual values and table rows SHALL use horizontal scrolling without soft wrapping. `,` and `.` SHALL scroll left and right by ten terminal cells per press, multiplied by any numeric prefix and clamped at the line boundaries. Rendering SHALL escape control characters and respect terminal cell widths.

#### Scenario: Shared-line numbering

- **WHEN** focus moves between cells on a table row or elements in an inline array
- **THEN** the absolute line number SHALL remain unchanged
- **AND** relative vertical distance between those values SHALL be 0

#### Scenario: Narrow viewport

- **WHEN** the terminal cannot fit the full content or preview
- **THEN** expanded content SHALL remain reachable by horizontal scrolling
- **AND** collapsed previews SHALL truncate with `…` at a valid character boundary
- **AND** preview space SHALL be removed before object-count or warning space
- **AND** warnings that still do not fit SHALL remain reachable by horizontal scrolling

#### Scenario: Application controls

- **WHEN** focus, search, status, or command entry is active
- **THEN** status and commands SHALL remain outside document text
- **AND** focus and search SHALL highlight existing spans without inserting text
- **AND** default-theme focus SHALL use bright foreground colors instead of bold and SHALL apply only on the selected display line
- **AND** default-theme line numbers and collapse arrows SHALL use dark gray ordinarily and light gray on the selected display line

## ADDED Requirements

### Requirement: Theme selection background fills the row

When a theme defines a distinct selection background, the selected display line SHALL fill the terminal width with it, including indentation, spaces, gutters, and clipping markers. Themes without a distinct selection background SHALL retain their appearance. Search matches SHALL retain their theme search style over the row background.

#### Scenario: Borealis selected row

- **WHEN** a short document line is selected under Borealis
- **THEN** the selection background SHALL extend through unused columns to the terminal edge
- **AND** search spans SHALL retain their search background
