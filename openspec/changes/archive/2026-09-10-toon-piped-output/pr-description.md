Redirected stdout now defaults to TOON. `cat file.json | tless | cat` emits TOON, and `-o json` preserves the previous JSON pipeline output. Add independent `-o` / `--output` selection for JSON, YAML, and TOON, parsing and serializing input before writing. TOON retains its existing feature gate and documented codec limitations.

Cover conversions, framing, typed YAML values, malformed input, minimal builds, input limits, broken pipes, and unchanged interactive commands. Update help and migration documentation for the incompatible default.

Validation: Rust 1.87 feature matrix, development feature matrix, repository preflight, strict OpenSpec validation, and documentation validation. See `verification.md` for the recorded results and limitations.

Closes #7

OpenSpec-Change: toon-piped-output
OpenSpec-Sync-Reviewed: toon-piped-output
