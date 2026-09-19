## Context

See [the proposal](proposal.md) for motivation and [the spec](specs/command-autocomplete/spec.md) for the interaction contract. `ScreenWriter` owns one Rustyline editor shared by command and search prompts. Its helper currently implements styling with empty completion and hint traits. Both the helper module and its installation are gated by `colorscheme`; minimal builds use `()`.

`App::parse_command` recognizes explicit aliases and feature-gated commands. It does not accept arbitrary abbreviations. Rustyline 18.0.1 is already locked in the project, with default features disabled. Its local source exposes `Completer`, `Hinter`, circular completion, and Right-arrow hint acceptance. Enable its `custom-bindings` feature for direction-aware completion handling. This adds `radix_trie`, `nibble_vec`, and `endian-type` to the committed lockfile without upgrading Rustyline.

## Goals / Non-goals

- Keep candidate calculation deterministic and independent of terminal I/O so prefix and feature behavior can be tested directly.
- Reuse the existing editor and preserve its input editing, terminal ownership, and command submission path.
- Avoid a custom popup, new editor, command-language redesign, filesystem queries, or dependency upgrade.

## Decisions

### Use inline hints and circular completion

The proposed interaction is an inline suffix while typing, Tab and Shift-Tab cycling, and Right-arrow hint acceptance at end of input. Tab starts at the first match and Shift-Tab starts at the last match. Escape restores the pre-cycle buffer and cursor while keeping the prompt open. Ctrl-C cancels the prompt. Tab-only completion would be simpler but would not expose suggestions as users type, as requested in issue #9. A popup would require additional layout and dismissal behavior on a prompt placed at the bottom of the terminal.

Use native circular completion for replacements, cursor positioning, Escape, and the original-input restore position. The locked Rustyline version starts its loop at index zero and does not initiate completion on Shift-Tab. A conditional key handler starts either direction with the native completion command. For backward initiation, the completer reverses the candidates and the handler translates subsequent forward/backward keys. Other keys clear the direction state and retain their default editor behavior. Share prompt mode and direction state between the helper and handler; reset both for each prompt. Enter continues through the existing submission path and never accepts a hint implicitly. Suppress hints for exact long-form candidates, so typing `write` does not suggest upgrading it to `write!`. The user can still select overwrite forms explicitly with Tab.

### Make the helper available in every build

Replace the styling-only helper with a command-line helper compiled in every feature profile. Keep optional theme behavior within its styling component instead of gating the entire helper. Set its prompt mode before each `readline` call and clear session completion context when the prompt ends. Only command mode supplies candidates or hints.

Theme switching must update style without removing completion support. Search mode supplies no command candidates or hints. This avoids maintaining separate command and search editors and changing their existing lifecycle.

### Keep command names aligned with parsing

Extract a small internal catalog of names, aliases, canonical suggestion names, feature availability, and command kind that both command-name recognition and completion can use. Preserve the current parser's argument handling, including colorscheme whitespace handling, exact aliases, and write overwrite semantics. Do not broaden accepted syntax during the extraction.

A separate hard-coded completion list would be smaller initially but could advertise commands the parser no longer accepts. Add behavioral coverage that every advertised long-form name reaches its existing parser operation with suitable arguments and that feature-disabled names are absent. Keep this within the application crate.

### Complete only a bounded command-token range

Compute the first token after leading ASCII spaces using the existing command grammar. Offer candidates only when the cursor is at that token's end. Return its start offset and replacement name so arguments and leading spaces survive selection. Do not complete from the middle of a token, where replacing only the prefix could duplicate a suffix. Match case-sensitively and sort only long-form candidates lexicographically. Keep `exit` and `quit` as readable names, exclude short aliases and function-style spellings, and retain long-form overwrite variants. A typed alias such as `h` can suggest `help`; Enter still submits `h` unless completion is accepted.

Hints use the same candidates, but only for a nonempty, incomplete token at the end of the entire buffer. Treat byte offsets as UTF-8 boundaries and use terminal cell widths for presentation. Do not trim or normalize user text.

### Preserve the command row

Style the inline hint distinctly using existing terminal attributes, while preserving the command background and restoring typed-text styling after the hint. No new theme configuration is needed. Use the editor's terminal-width handling where it meets the spec; verify clipping, resizing, and cleanup through a pseudoterminal. Hint acceptance must insert the full suffix even if its visual presentation is clipped.

## Risks / Trade-offs

- Rustyline hint refresh and circular-selection details could differ from assumptions. Verify the locked version with focused terminal tests before expanding integration; keep any adaptation in the helper/editor boundary.
- Refactoring name recognition could alter parsing. Preserve exact aliases and argument rules with behavior tests, especially `!`, colorscheme whitespace, and invalid input.
- Bottom-row redraws can scroll or leave stale hint text. Exercise narrow widths, resize, cancellation, search transitions, and theme changes using the existing terminal test infrastructure.
- The parser accepts more spellings than completion advertises. Verify short aliases still execute while the candidate list contains only the long-form names specified in the spec.

## Migration plan

This adds interactive assistance with no configuration or stored-data migration. Document the keys in interactive help and add an Unreleased changelog entry when implementing. Retain Rust 1.87 compatibility and the committed lockfile. Rollback removes helper completion and hints while retaining the existing command parser behavior.

Before the implementation PR merges, complete and verify the tasks, synchronize the delta into the main specs, archive the change, and run the repository's PR-scoped OpenSpec completion gate with the required synchronization review statement.
