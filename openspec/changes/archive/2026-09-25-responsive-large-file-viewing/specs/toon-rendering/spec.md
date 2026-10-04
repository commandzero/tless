## ADDED Requirements

### Requirement: Avoid redundant startup layout construction

The viewer SHALL construct its initial layout using actual terminal content dimensions and effective absolute or relative line-number settings. Increasing the number gutter SHALL reuse rendered content when all inline arrays still fit the resulting content width. The reuse decision SHALL include Unicode terminal-cell width and annotations, and collapsed previews SHALL use the resulting width. If an inline fit changes, the viewer SHALL recompute the layout and gutter until consistent.

This optimization SHALL preserve existing TOON text, source identities, warnings, explicit multiline choices, navigation, wrapping, search, filtering, and copy/export semantics. It SHALL NOT introduce loading/pending states or change startup failure or machine-output contracts.

#### Scenario: Nondefault initial geometry

- **WHEN** startup uses nondefault terminal dimensions or disables line numbers
- **THEN** the first constructed layout SHALL use that geometry and the effective gutter
- **AND** exact-fit Unicode arrays SHALL retain their existing inline or multiline presentation

#### Scenario: Gutter grows without changing an inline fit

- **WHEN** the line count requires a wider number gutter but every inline array still fits
- **THEN** the viewer SHALL reuse the rendered layout instead of rendering the whole document again
- **AND** collapsed inline previews SHALL honor the narrower available width

#### Scenario: Gutter growth changes an inline fit

- **WHEN** an additional line-number digit makes an annotated or Unicode array too wide
- **THEN** the array SHALL become multiline under the existing layout rules
- **AND** the gutter SHALL be rechecked against the resulting line count
