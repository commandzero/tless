## Purpose

Keep the terminal viewer usable while large inputs are loaded and display work is prepared, with bounded presentation resources and verifiable startup and interaction latency.

## ADDED Requirements

### Requirement: Responsive interactive loading

With terminal stdout and a controlling terminal, the application SHALL display a loading state without waiting for input EOF, parsing, path resolution, or whole-document display preparation. The loading state SHALL distinguish reading, parsing, and preparing content without presenting estimates as exact counts. Resize, `q`, and Ctrl+C SHALL remain available. `q` SHALL exit 0; Ctrl+C SHALL exit 130. Document-dependent commands before a validated document is available SHALL report that content is not ready without queuing hidden actions. No parsed content SHALL be exposed as valid until the complete input has passed the existing parsing and input-limit checks and the startup path has resolved successfully.

#### Scenario: Slow pipe with an open writer

- **WHEN** input arrives through stdin and the producer pauses without closing its output
- **THEN** the loading screen SHALL remain responsive to resize and quit
- **AND** keyboard input SHALL come from the controlling terminal rather than consuming or replacing the data stream
- **AND** quitting SHALL NOT wait for the producer to close the pipe

#### Scenario: Invalid tail after a valid prefix

- **WHEN** a selected early subtree is valid but a later part of the input is malformed or exceeds the input-byte limit
- **THEN** no prefix SHALL be presented as a successfully loaded document
- **AND** the terminal SHALL be restored before an actionable stderr diagnostic and exit status 1

### Requirement: Safe background lifecycle

Background preparation SHALL NOT write terminal output, block keyboard processing, or overwrite newer view state. Only results matching the current document, scope, geometry, and request state SHALL be committed. Repeated requests SHALL have bounded outstanding storage; superseded work SHALL yield to current work. A finite current request SHALL eventually complete or report an error without starvation. Quitting SHALL restore raw mode, cursor visibility, mouse reporting, and the alternate screen without waiting for blocked input or obsolete computation. A background failure SHALL be reported rather than leaving an indefinite loading or pending state.

#### Scenario: Rapid resize and navigation

- **WHEN** several widths and destinations are requested before earlier preparation completes
- **THEN** an older completion SHALL NOT revert the latest width, scope, focus, or viewport
- **AND** the current request SHALL receive priority over obsolete and speculative work

#### Scenario: Failure or quit during preparation

- **WHEN** loading fails internally or the user quits while background work is blocked
- **THEN** terminal restoration SHALL occur without waiting for that work
- **AND** an internal loading failure SHALL exit 1 with a diagnostic after restoration
- **AND** background work SHALL NOT emit terminal output after restoration

#### Scenario: Prompt owns terminal input

- **WHEN** background results arrive while a command or search prompt is being edited
- **THEN** prompt text, cursor position, cancellation, and editing SHALL remain intact
- **AND** document painting SHALL NOT overwrite the prompt
- **AND** applicable results SHALL be applied when document-view terminal ownership resumes

### Requirement: Bounded presentation resources

Preparing and retaining display text, styling spans, source-to-display maps, previews, and physical-row fragments SHALL be limited by explicit finite byte budgets for the viewport, overscan, caches, and queued results. These budgets SHALL NOT grow with the number of rendered document lines. Oversized scalar values and shared table headers SHALL remain navigable without requiring an unbounded display allocation for one line. Complete source storage, parsed nodes, compact semantic/layout indexes, and search-result indexes SHALL be accounted for separately; this capability SHALL NOT claim a bound on total document memory or out-of-core parsing.

#### Scenario: Browse beyond the cache

- **WHEN** the user traverses more disjoint regions than fit in the presentation cache
- **THEN** old presentation data SHALL be evicted while parsed identities and navigation remain valid
- **AND** retained and in-flight presentation bytes SHALL stay within their documented budgets

#### Scenario: Giant scalar and table header

- **WHEN** a scalar exceeds the presentation-cache budget or a table has many rows sharing the same field names
- **THEN** visible portions and search targets SHALL remain reachable without constructing a full oversized display string or duplicating every field occurrence into the painted header
- **AND** decoded values, copied values, source-based search, and exports SHALL remain complete

### Requirement: Measured startup and interaction acceptance

Release acceptance SHALL use a reproducible synthetic local-input workload and a recorded native reference environment, with release builds, fixed 140-by-40 terminal geometry, one warm-up, and at least 20 measured runs. The reference workload SHALL include approximately 10 MiB of nested JSON, as defined by the change design, and SHALL NOT depend on confidential input. Measurements SHALL distinguish loading-screen latency, first useful content, command acknowledgement, destination completion, and quit latency. A loading message or echoed key SHALL NOT count as useful content or completed navigation.

On the reference workload, p95 loading-screen latency SHALL be at most 100 ms; first useful content SHALL be at most 500 ms; cached navigation completion SHALL be under 16 ms; and acknowledgement of uncached jumps, resize, wrapping, collapse, filtering, and search SHALL be at most 50 ms. Reference-workload uncached jump and reflow completion SHALL be at most 500 ms. Quit-to-terminal-restoration SHALL be at most 100 ms, including a deliberately stalled pipe. Slow source delivery SHALL NOT be included in the first-content deadline. Cold-start measurements and full search completion SHALL be reported separately. Absolute timing gates SHALL run in the documented reference environment; ordinary CI SHALL enforce deterministic behavior and resource invariants rather than these hardware-sensitive timing thresholds.

#### Scenario: Useful first frame rather than a spinner

- **WHEN** the release benchmark opens its generated nested JSON workload
- **THEN** it SHALL measure process start through a complete frame containing verified fixture data, usable focus, and current geometry separately from the loading frame
- **AND** both latency distributions SHALL meet their respective reference thresholds

#### Scenario: Input during expensive work

- **WHEN** the benchmark issues resize, wrap, an uncached destination request, or quit while preparation is active
- **THEN** it SHALL verify the resulting visible state or terminal restoration rather than treating received input bytes as acknowledgement
- **AND** acknowledgement and completion SHALL be reported separately

### Requirement: Machine output and input compatibility

Non-terminal stdout SHALL retain existing format selection, complete-input validation, serialization, framing, exit statuses, input-byte limits, and failure guarantees. It SHALL NOT initialize a terminal, display progress, or require a controlling terminal. Interactive loading SHALL retain supported JSON, YAML, TOON, sequence, filename, stdin, and `-` input behavior. This change SHALL NOT bypass the published TOON codec's validation or normalize JSON values through that codec for display.

#### Scenario: Redirected malformed input

- **WHEN** malformed input or a failing path selection is used with redirected stdout
- **THEN** stdout SHALL remain free of data and terminal controls, with the existing stderr diagnostic and failure status

#### Scenario: Same data after background loading

- **WHEN** equivalent supported inputs finish loading through files, stdin, or `-`
- **THEN** the committed view SHALL obey the same rendering, navigation, copy, and export contracts regardless of loading progress
