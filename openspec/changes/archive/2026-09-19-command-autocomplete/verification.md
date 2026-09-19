# Command autocomplete verification

## Requirement review

| Requirement | Implementation and evidence |
| --- | --- |
| Available command names | `src/command.rs` shares names and aliases with parsing. Catalog tests check sorted long names, excluded aliases, feature boundaries, and execution with valid arguments. Parser tests retain aliases, overwrite forms, invalid-input handling, and colorscheme whitespace. |
| Completion scope and text preservation | `candidates` restricts matching to the end of the first token. Unit tests cover empty input, spaces, arguments, cursor positions, case, and Unicode. The terminal write test completes before an existing filename and verifies the resulting file. |
| Suggestions while typing | Helper tests check exact-name suppression, mode isolation, and clipping. Terminal tests distinguish dim hints from input, accept hints with Right, and show that Enter submits only typed text. |
| Completion selection and execution | Terminal tests check forward/backward initiation, direction changes, restoration, Escape, editing and cursor movement after selection, and cancellation without overwriting an existing file. Rustyline owns replacement ranges and cursor restoration. |
| Prompt isolation and terminal presentation | Terminal tests cover both search directions, prompt reopening, idle marker removal before input and after submission or cancellation, six-column clipping, accepting a clipped hint, resize, cursor movement, cancellation cleanup, and foreground/background preservation after theme switches. Interactive help documents the controls and scope. |

## Validation

- `OPENSPEC_BASE=HEAD scripts/preflight.sh` passed with Rust 1.97.1. This ran formatting, both Clippy profiles, shell/workflow checks, release/OpenSpec regression checks, docs validation, and all five feature-profile test suites. The committed merge-base completion gate is recorded separately below.
- `TLESS_TOOLCHAIN=1.87.0 scripts/preflight.sh test` passed for all five feature profiles.
- Strict OpenSpec change validation and all seven main-spec validations passed with OpenSpec 1.11.0.
- Documentation validation passed with zero errors and warnings. The existing informational `vim-themes.svg` link notice remains.
- Tests ran on macOS using disposable pseudoterminals. Linux execution remains for CI.

## Implementation decisions

The locked Rustyline version requires its optional custom-binding API to initiate backward completion. The implementation adds `radix_trie`, `nibble_vec`, and `endian-type` through that feature and retains the Rustyline version. A conditional handler reverses candidate order and translates subsequent directions; native completion preserves cursor and undo behavior.

The editor keeps styling enabled to distinguish hints even when the inherited environment contains `NO_COLOR`, consistent with the viewer's existing terminal styling. Hints reserve the terminal's final cell to avoid bottom-row wrapping. When no hint cells remain, no hint is offered.

Early terminal tests caught replacement/cursor assumptions and a test readiness check that relied on a clipped filename. The final tests use the native completion loop and detect the initial draw at every terminal width.

## Synchronization review

The new main spec copies the delta Purpose and all five added requirement blocks, including every scenario. The only structural differences are the main-spec title and the `Requirements` heading. There are no modifications, removals, or renames to reconcile. A direct content comparison passed.

The archive preserves every planning artifact and includes this verification record and a prepared PR description. The PR-scoped completion gate passed against committed HEAD and the fetched `origin/main`, using that description. It validated the archived delta, completed tasks, and main spec in strict mode.
