## MODIFIED Requirements

### Requirement: Available command names

The `:` prompt SHALL offer case-sensitive prefix matches from the long-form command names supported by the running build. Short aliases and the function-style `quit()` and `exit()` spellings SHALL NOT be suggested, but SHALL remain accepted by the command parser. Candidates SHALL be unique and sorted lexicographically. The following names SHALL be available:

| Build | Names |
| --- | --- |
| Every build | `exit`, `help`, `quit`, `set`, `write`, `write!`, `write-json`, `write-json!`, `write-jsonl`, `write-jsonl!`, `write-ndjson`, `write-ndjson!`, `write-toon`, `write-toon!`, `write-yaml`, `write-yaml!` |
| With `colorscheme` | `colorscheme` |
| With `sexp` | `write-sexp`, `write-sexp!` |

Completion SHALL use the command syntax and meanings defined by interactive-write, including the TOON default and retained short aliases. It SHALL NOT independently change command semantics. Obsolete long names `writetoon`, `writetoon!`, `writesexp`, and `writesexp!` SHALL NOT be offered or accepted.

#### Scenario: Prefix has several matches

- **WHEN** the user requests completion for `write-j`
- **THEN** candidates SHALL be `write-json`, `write-json!`, `write-jsonl`, and `write-jsonl!` in that order

#### Scenario: Short aliases remain executable but are not suggested

- **WHEN** the user requests completion for `h` or `q`
- **THEN** candidates SHALL be `help` or `quit`, respectively
- **AND** submitting the uncompleted `h` or `q` SHALL retain its existing behavior
- **AND** `w`, `wt`, and their overwrite forms SHALL remain accepted with TOON encoding but SHALL NOT appear as candidates
- **AND** `wj`, `wy`, `wn`, and their overwrite forms SHALL remain accepted for JSON, YAML, and NDJSON/JSONL respectively but SHALL NOT appear as candidates

#### Scenario: Feature-specific commands

- **WHEN** the user requests completion in a build without `colorscheme` or `sexp`
- **THEN** completion SHALL work for the base commands and SHALL exclude `colorscheme`, `write-sexp`, `write-sexp!`, `ws`, and `ws!`
- **AND** enabling either feature SHALL add only its corresponding long names to completion
- **AND** `ws` and `ws!` SHALL be accepted only with `sexp`

### Requirement: Completion scope and text preservation

Completion SHALL operate only at the end of the first command token, including when arguments follow the cursor. It SHALL preserve leading ASCII spaces and every character after that token. At an empty or ASCII-space-only prompt, explicit completion SHALL offer all available names. It SHALL NOT complete arguments, paths, theme names, or settings. At any other cursor position, or when no prefix matches, completion SHALL leave input unchanged. Non-ASCII text SHALL remain intact.

#### Scenario: Complete a command before an existing argument

- **WHEN** the buffer is `  write-t report.toon` with the cursor immediately after `write-t` and the user selects `write-toon`
- **THEN** the buffer SHALL become `  write-toon report.toon`
- **AND** the cursor SHALL be immediately after `write-toon`

#### Scenario: Arguments and middle-of-token edits

- **WHEN** the cursor is in `report.toon` in `write report.toon`, after `set `, or inside the token `write`
- **THEN** command-name completion SHALL leave the buffer unchanged and SHALL show no command-name hint

#### Scenario: No match

- **WHEN** the user types `WR`, `unknown`, or `é` as the command token
- **THEN** no command-name hint SHALL appear and Tab SHALL leave input unchanged

### Requirement: Completion selection and execution

Tab SHALL insert the first match and successive Tab presses SHALL cycle forward through the matches for the original prefix. Shift-Tab as the first completion key SHALL insert the last match; successive Shift-Tab presses SHALL cycle backward. Cycling SHALL include the original uncompleted input as a restore position before repeating. Completion SHALL NOT append spaces or execute commands. Editing the buffer or moving the cursor SHALL end that cycle so the next completion uses the new buffer and cursor. Enter SHALL submit the current buffer using existing execution and validation behavior. Escape during a completion cycle SHALL restore the buffer and cursor from before that cycle, end the cycle, and leave the prompt open. Ctrl-C SHALL cancel the prompt without executing the selected command.

#### Scenario: Cycle and restore

- **WHEN** the buffer is `write-j` and the user presses Tab repeatedly
- **THEN** the buffer SHALL visit `write-json`, `write-json!`, `write-jsonl`, `write-jsonl!`, and `write-j`, then repeat
- **AND** Shift-Tab SHALL traverse those positions in reverse

#### Scenario: Start with backward completion

- **WHEN** the buffer is `write-j` and the first completion key is Shift-Tab
- **THEN** the buffer SHALL become `write-jsonl!`
- **AND** successive Shift-Tab presses SHALL visit `write-jsonl`, `write-json!`, `write-json`, and `write-j`, then repeat

#### Scenario: Escape restores the original input

- **WHEN** the user starts cycling with `write-j report.json` and presses Escape after selecting a match
- **THEN** the buffer SHALL return to `write-j report.json` with its original cursor position
- **AND** the prompt SHALL remain open without executing a command

#### Scenario: Selection has no side effects

- **WHEN** the user completes `write-j output.json` to `write-json! output.json` and then presses Ctrl-C
- **THEN** the prompt SHALL close without creating or overwriting a file

#### Scenario: Edit after selecting

- **WHEN** the user completes `wri` to `write`, deletes the final `e`, and presses Tab
- **THEN** completion SHALL use the current prefix `writ` instead of advancing the previous cycle
