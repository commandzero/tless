## 1. Command catalog and matching

- [x] 1.1 Share command-name recognition and feature availability between parsing and completion without changing argument parsing. Verify all existing aliases, overwrite forms, colorscheme whitespace handling, invalid inputs, and feature-disabled names with parser behavior tests.
- [x] 1.2 Implement sorted, unique prefix candidates and bounded first-token replacement. Verify only long-form names are proposed while short aliases still execute; cover empty input, leading spaces, existing arguments, middle-of-token cursors, exact names, case sensitivity, no matches, and UTF-8 preservation against the spec.

## 2. Editor integration

- [x] 2.1 Make the command-line helper available in every build and set command or search context for each prompt. Verify base completion with no default features and absence of command hints and candidates in both search directions.
- [x] 2.2 Add inline suffix hints, exact-name suppression, Right-arrow acceptance, and explicit circular Tab/Shift-Tab selection using the locked Rustyline version. Verify Shift-Tab starts at the last match, Escape restores the pre-cycle buffer and cursor while keeping the prompt open, original-input restoration during cycling, editing after selection, Enter submitting only buffered text, and Ctrl-C cancellation in pseudoterminal tests.
- [x] 2.3 Preserve theme styling and hint cleanup across prompt closure, reopening, and theme changes. Verify narrow-width and resize behavior without document/status-row corruption, including full suffix acceptance when a hint is clipped.

## 3. User guidance and integrated verification

- [x] 3.1 Document completion keys, command-name-only scope, and Enter behavior in interactive help; add an Unreleased changelog entry. Verify the rendered help matches the implemented controls and run `scripts/validate-docs.sh` for documentation changes.
- [x] 3.2 Run `scripts/preflight.sh` and `TLESS_TOOLCHAIN=1.87.0 scripts/preflight.sh test`; verify the feature matrix and terminal tests pass, including actual execution of completed commands, preservation of arguments, and no write side effects before Enter.
- [x] 3.3 Verify each delta requirement against the implementation and record the results in `verification.md`, with test evidence for candidate availability, selection, prompt isolation, and terminal presentation.

## 4. OpenSpec completion before implementation merge

- [x] 4.1 Synchronize this delta into the main specs and archive the completed change. Verify the archived requirements and scenarios match the main spec and preserve every planning artifact.
- [x] 4.2 Run strict change, affected-spec, and archived-task validation with OpenSpec 1.11.0 and telemetry disabled. Add `OpenSpec-Change: command-autocomplete` and, after comparison, `OpenSpec-Sync-Reviewed: command-autocomplete` to the implementation PR description. Verify the PR-scoped completion check passes against committed HEAD and the fetched target branch, following `docs/contributing.md`.
