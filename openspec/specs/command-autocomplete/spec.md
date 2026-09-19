# Command autocomplete

## Purpose

Help users discover and enter supported command names in the interactive `:` prompt without changing command execution or search input.

## Requirements

### Requirement: Available command names

The `:` prompt SHALL offer case-sensitive prefix matches from the long-form command names supported by the running build. Short aliases and the function-style `quit()` and `exit()` spellings SHALL NOT be suggested, but SHALL remain accepted by the command parser. Candidates SHALL be unique and sorted lexicographically. The following names SHALL be available:

| Build | Names |
| --- | --- |
| Every build | `exit`, `help`, `quit`, `set`, `write`, `write!`, `writetoon`, `writetoon!` |
| With `colorscheme` | `colorscheme` |
| With `sexp` | `writesexp`, `writesexp!` |

Completion SHALL NOT introduce additional accepted command syntax or change the meaning of existing aliases or overwrite forms.

#### Scenario: Prefix has several matches

- **WHEN** the user requests completion for `wri` in a build without `sexp`
- **THEN** candidates SHALL be `write`, `write!`, `writetoon`, and `writetoon!` in that order

#### Scenario: Short aliases remain executable but are not suggested

- **WHEN** the user requests completion for `h` or `q`
- **THEN** candidates SHALL be `help` or `quit`, respectively
- **AND** submitting the uncompleted `h` or `q` SHALL retain its existing behavior

#### Scenario: Feature-specific commands

- **WHEN** the user requests completion in a build without `colorscheme` or `sexp`
- **THEN** completion SHALL work for the base commands and SHALL exclude `colorscheme`, `writesexp`, `writesexp!`, `ws`, and `ws!`
- **AND** enabling either feature SHALL add only its corresponding names

### Requirement: Completion scope and text preservation

Completion SHALL operate only at the end of the first command token, including when arguments follow the cursor. It SHALL preserve leading ASCII spaces and every character after that token. At an empty or ASCII-space-only prompt, explicit completion SHALL offer all available names. It SHALL NOT complete arguments, paths, theme names, or settings. At any other cursor position, or when no prefix matches, completion SHALL leave input unchanged. Non-ASCII text SHALL remain intact.

#### Scenario: Complete a command before an existing argument

- **WHEN** the buffer is `  wri report.toon` with the cursor immediately after `wri` and the user selects `writetoon`
- **THEN** the buffer SHALL become `  writetoon report.toon`
- **AND** the cursor SHALL be immediately after `writetoon`

#### Scenario: Arguments and middle-of-token edits

- **WHEN** the cursor is in `report.toon` in `write report.toon`, after `set `, or inside the token `write`
- **THEN** command-name completion SHALL leave the buffer unchanged and SHALL show no command-name hint

#### Scenario: No match

- **WHEN** the user types `WR`, `unknown`, or `é` as the command token
- **THEN** no command-name hint SHALL appear and Tab SHALL leave input unchanged

### Requirement: Suggestions while typing

At the end of a nonempty command-only input, the prompt SHALL display the remaining suffix of the first matching candidate as a visually distinct inline hint. The hint SHALL NOT be part of the input buffer. No hint SHALL appear for an exact long-form candidate, an empty prefix, arguments, or an unmatched prefix. Typing, deleting, or moving the cursor SHALL update or remove the hint to match the current input. Right at the end of input SHALL accept the displayed suffix into the buffer without executing it.

#### Scenario: Suggest and accept a unique match

- **WHEN** the user types `:se`
- **THEN** the prompt SHALL display `t` as a hint after `se`
- **AND** Right SHALL change the buffer to `set` without executing a command

#### Scenario: Exact commands do not suggest stronger variants

- **WHEN** the command token is exactly `write`
- **THEN** the prompt SHALL show no suffix hint, including no `!` hint
- **AND** explicit Tab cycling SHALL remain available

#### Scenario: Enter does not accept a hint

- **WHEN** the buffer is `se`, the hint is `t`, and the user presses Enter
- **THEN** the application SHALL submit `se` through its existing command parser
- **AND** it SHALL NOT silently submit `set`

### Requirement: Completion selection and execution

Tab SHALL insert the first match and successive Tab presses SHALL cycle forward through the matches for the original prefix. Shift-Tab as the first completion key SHALL insert the last match; successive Shift-Tab presses SHALL cycle backward. Cycling SHALL include the original uncompleted input as a restore position before repeating. Completion SHALL NOT append spaces or execute commands. Editing the buffer or moving the cursor SHALL end that cycle so the next completion uses the new buffer and cursor. Enter SHALL submit the current buffer using existing execution and validation behavior. Escape during a completion cycle SHALL restore the buffer and cursor from before that cycle, end the cycle, and leave the prompt open. Ctrl-C SHALL cancel the prompt without executing the selected command.

#### Scenario: Cycle and restore

- **WHEN** the buffer is `wri` in a build without `sexp` and the user presses Tab repeatedly
- **THEN** the buffer SHALL visit `write`, `write!`, `writetoon`, `writetoon!`, and `wri`, then repeat
- **AND** Shift-Tab SHALL traverse those positions in reverse

#### Scenario: Start with backward completion

- **WHEN** the buffer is `wri` in a build without `sexp` and the first completion key is Shift-Tab
- **THEN** the buffer SHALL become `writetoon!`
- **AND** successive Shift-Tab presses SHALL visit `writetoon`, `write!`, `write`, and `wri`, then repeat

#### Scenario: Escape restores the original input

- **WHEN** the user starts cycling with `wri report.json` and presses Escape after selecting a match
- **THEN** the buffer SHALL return to `wri report.json` with its original cursor position
- **AND** the prompt SHALL remain open without executing a command

#### Scenario: Selection has no side effects

- **WHEN** the user completes `wri output.json` to `write! output.json` and then presses Ctrl-C
- **THEN** the prompt SHALL close without creating or overwriting a file

#### Scenario: Edit after selecting

- **WHEN** the user completes `wri` to `write`, deletes the final `e`, and presses Tab
- **THEN** completion SHALL use the current prefix `writ` instead of advancing the previous cycle

### Requirement: Prompt isolation and terminal presentation

Command completion and hints SHALL be enabled only for `:` input. The `/` and `?` prompts SHALL retain their existing search editing and submission behavior. Closing or reopening a prompt SHALL clear any completion cycle. Hints SHALL fit in the available command-row cells without wrapping into document or status rows; a hint that does not fit SHALL be clipped or omitted. Completion redraws SHALL preserve command foreground and background styling, distinguish hint text, and remove stale hints after edits, cancellation, submission, resizing, and theme changes. Interactive help SHALL describe selection, hint acceptance, submission, and the command-name-only scope.

#### Scenario: Switch to search

- **WHEN** the user cancels `:se` and opens `/` or `?` with `se` as input
- **THEN** no command-name hint or completion SHALL appear
- **AND** submitting the input SHALL search for `se`

#### Scenario: Narrow terminal and themed redraw

- **WHEN** a hint would extend beyond the command row or the terminal is resized while a hint is visible
- **THEN** the hint SHALL remain within the command row and SHALL NOT leave stale text after redraw
- **AND** themed builds SHALL retain the active command-row colors, including after a session theme switch

#### Scenario: Reopen the command prompt

- **WHEN** the user closes a prompt during completion and opens `:` again
- **THEN** completion SHALL start from the new input with no prior selection or hint
