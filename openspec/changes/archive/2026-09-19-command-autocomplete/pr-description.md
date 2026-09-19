The `:` prompt now suggests long-form command names inline and completes them with Tab or Shift-Tab. Shift-Tab starts at the last match; Escape restores the original input, and Enter executes only the input buffer. Short aliases remain accepted, and searches and arguments do not receive command suggestions.

Validation covers the five feature profiles on Rust 1.97.1 and 1.87.0, terminal interactions including resize and theme changes, and strict OpenSpec validation.

Closes #9

OpenSpec-Change: command-autocomplete
OpenSpec-Sync-Reviewed: command-autocomplete
