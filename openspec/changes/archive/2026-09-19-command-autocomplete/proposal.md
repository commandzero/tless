## Why

The `:` prompt requires users to remember command names and type them without assistance. [Issue #9](https://github.com/commandzero/tless/issues/9) asks for matching command suggestions while typing and selection through the existing input controls.

## What changes

- Suggest long-form command names inline while typing at the `:` prompt.
- Use Tab to start at the first match and Shift-Tab to start at the last match, then cycle matching names, and Right at the end of input to accept the visible suggestion. Enter submits only the current input buffer. Escape restores the input from before cycling and keeps the prompt open; Ctrl-C cancels the prompt.
- Match case-sensitive prefixes of the first command token, with a stable order and candidates filtered by compiled features. Short aliases remain accepted when typed but are not suggested.
- Support completion in every build, including builds without colorschemes, while preserving search input and command-row styling.
- Keep argument completion, shell completion, fuzzy matching, and command syntax changes outside this change.

## Capabilities

### New capabilities

- `command-autocomplete`: Command-name suggestions, selection, prompt isolation, and feature-aware candidates in the interactive command line.

### Modified capabilities

None. Existing command execution, theme switching, and search requirements remain unchanged.

## Impact

The implementation will touch the Rustyline helper in `src/commandline.rs`, editor setup and prompt context in `src/screenwriter.rs`, module gating in `src/main.rs`, and command-name definitions used by `src/app.rs`. It will add completion and terminal behavior tests, help text, and an Unreleased changelog entry. Enable the existing Rustyline dependency's `custom-bindings` feature and record its transitive dependencies in the lockfile. No dependency version upgrade, CLI flag, configuration field, or minimum Rust version change is needed.
