## MODIFIED Requirements

### Requirement: Atomic filter changes and diagnostics

An interactive path failure SHALL show a diagnostic and preserve the active filter, focus, viewport, search state, and collapse state. Cancelling path input SHALL have no filter side effects. A successful change SHALL focus the first selected root, reset vertical and horizontal scroll to its start, clear active search matches, and preserve per-node collapse and explicit multiline-array state. Selected roots SHALL be exposed expanded initially; remembered descendant states SHALL be retained. Resetting with `.` SHALL use the same transition rules and SHALL restore all original roots, not the previous cursor location.

Interactive resolution and preparation SHALL keep the document UI responsive. Until a filter is ready to commit, the prior scope SHALL remain authoritative for navigation, search, copy, and export, and the application SHALL indicate that the new filter is pending. Escape SHALL cancel a pending filter without changing the committed state. A newer filter request SHALL supersede an older pending filter. Publication SHALL atomically install the selected roots and their first usable viewport using current geometry; obsolete results SHALL NOT commit or overwrite view-state changes made while preparation was pending.

CLI path syntax errors and a missing option argument SHALL exit with status 2 before terminal raw/alternate-screen initialization. Resolution failures SHALL exit with status 1. Both SHALL report on stderr and produce no data payload. With terminal stdout, startup resolution SHALL be allowed to run after a responsive loading screen is initialized; on failure the application SHALL restore terminal state before writing the final stderr diagnostic and exiting. With non-terminal stdout, failures SHALL emit no terminal controls and SHALL NOT require a controlling terminal. Diagnostics SHALL distinguish invalid syntax, missing member, ambiguous member, invalid array index, and scalar traversal, identify the failed token where applicable, and identify the one-based document number for stream resolution failures. Diagnostics SHALL safely escape control characters from path text.

#### Scenario: Failed change preserves the current view

- **WHEN** a user viewing `.hits` with a selected descendant and an active search submits a missing path or cancels the prompt
- **THEN** the current scope, selected node, scrolling, search, and collapse states SHALL remain unchanged

#### Scenario: Successful change and reset

- **WHEN** a user applies a valid filter and later submits `.`
- **THEN** each successful transition SHALL select its first root, clear active search matches, reset scroll offsets, and expose that root
- **AND** descendant collapse and multiline-array choices SHALL survive the transitions

#### Scenario: Interactive startup resolution failure

- **WHEN** CLI path resolution fails in document 2 while terminal stdout is showing a loading screen
- **THEN** tless SHALL restore terminal mode, cursor, mouse reporting, and the original screen before exiting 1 with an actionable stderr diagnostic identifying document 2
- **AND** no document data SHALL be published as a successful view

#### Scenario: Startup failure is not terminal output

- **WHEN** CLI path resolution fails in document 2 with redirected stdout
- **THEN** tless SHALL exit 1 with an actionable stderr diagnostic identifying document 2
- **AND** it SHALL emit neither a data payload nor terminal initialization sequences

#### Scenario: Syntax still fails before terminal setup

- **WHEN** CLI path syntax is invalid or the path option lacks an argument
- **THEN** tless SHALL exit 2 before initializing raw mode or the alternate screen
- **AND** it SHALL emit a diagnostic on stderr without a data payload

#### Scenario: Cancel or supersede prepared filtering

- **WHEN** a submitted interactive filter is still being resolved or prepared and the user presses Escape or submits a newer filter
- **THEN** the obsolete filter SHALL NOT change the committed scope or reset focus, scrolling, or search
- **AND** a replacement filter SHALL commit only after all-document resolution and current-geometry viewport preparation succeed

#### Scenario: Navigation while a filter is pending

- **WHEN** a user navigates or changes descendant collapse state in the committed scope while another filter is being prepared
- **THEN** those interactions SHALL operate on the committed scope
- **AND** a failed or cancelled filter SHALL NOT undo those interactions
- **AND** a successful filter SHALL apply the normal focus/scroll/search reset while preserving the latest per-node descendant state
