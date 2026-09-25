## ADDED Requirements

### Requirement: Nonblocking destination preparation

Top/end jumps, absolute line jumps, counted motions, structural motions, mouse selection, and search reveals SHALL preserve their existing logical targets without requiring previously visited display content. A destination whose geometry is not ready SHALL be shown as pending without blocking input or guessing a line address. The prior committed selection SHALL remain authoritative for copy and export until the destination is committed. Escape SHALL cancel a pending destination and preserve the prior committed view. A later absolute destination request SHALL supersede an earlier pending destination; ordered relative motions SHALL retain their counts and ordering rather than being dropped by display-request coalescing. Geometry-dependent clicks SHALL be ignored with a pending indication while the displayed geometry is stale.

#### Scenario: End jump before off-screen preparation

- **WHEN** the first viewport is ready and the user requests the end before the intervening display text is prepared
- **THEN** the viewer SHALL locate the final target using the current scoped logical structure and prepare its viewport
- **AND** the jump SHALL NOT require rendering all intervening display lines
- **AND** ordinary input and quit SHALL remain available

#### Scenario: Exact line address waits for geometry

- **WHEN** a numbered jump is requested while the current-width line index is incomplete
- **THEN** the request SHALL remain explicitly pending until its exact target can be resolved
- **AND** completion SHALL use the fully expanded current scoped layout and existing collapsed-target rules
- **AND** a newer absolute request or Escape SHALL prevent the obsolete result from moving focus

#### Scenario: Counted motion during preparation

- **WHEN** a user requests ten downward logical motions while viewport preparation is active
- **THEN** the eventual selection SHALL equal ten ordered motions from the applicable logical selection, clamped by existing boundaries
- **AND** coalescing intermediate frames SHALL NOT reduce the motion count

### Requirement: Responsive source-based search

Submitting a search SHALL keep the document UI responsive while searching the same parsed-content ranges and regular-expression semantics as before. Until the search is complete, the viewer SHALL indicate a pending search and SHALL NOT report a final count or no-match result. Escape SHALL cancel the pending search and retain the prior committed search and view. A new search SHALL supersede older search work; a successful path-filter change SHALL invalidate pending search results from the old scope. No search SHALL match generated warnings, previews, gutters, or counts as extra data. Search completion SHALL map selected matches to stable node identities and reveal uncached spans as needed.

#### Scenario: Match outside prepared content

- **WHEN** a matching value or table-field key lies in an uncached or collapsed region of the active scope
- **THEN** a completed search SHALL include that match and reveal the original logical occurrence when selected
- **AND** the shared-header, hidden-ancestor, and wrapped-match behavior SHALL match the existing search contract

#### Scenario: Search superseded by a filter

- **WHEN** a search is pending and a new path filter commits successfully
- **THEN** a result from the old scope SHALL NOT move focus, restore matches, or change the new scope's counts
- **AND** search and repeat-search SHALL remain confined to the committed selected roots

#### Scenario: Regex boundaries and no-match completion

- **WHEN** a pattern crosses an internal preparation boundary or scans a large range without a match
- **THEN** its completed result SHALL equal searching the original full permitted source range with the existing regex semantics
- **AND** internal chunk boundaries SHALL NOT create or remove matches
- **AND** navigation and quit SHALL remain responsive until completion or cancellation
