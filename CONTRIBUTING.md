# Contributing

CommandZero maintains tless as an independently released fork of jless.
Changes can diverge from upstream conventions while preserving its license and attribution.

## Standards and scope

Use the shared [repo-man bundle](../repo-man/index.md).
Clone CommandZero/repo-man beside this checkout if that link is unavailable.
This repository adopts its applicable Rust, Bash, CLI, preflight, updates,
versioning, and release guidance, including draft recommendations.
The pending TUI and release-target documents do not provide complete requirements.
The README's platform and terminal contract supplies this repository's current scope.

One application crate is sufficient. Keep codec integration behind the TOON module.
Existing Unix terminal FFI uses unsafe code; retain small, reviewed boundaries and
terminal tests instead of imposing an incompatible blanket unsafe-code prohibition.
Do not remove behavior tests because a type check passes.

## Local checks

Install rustup, Rust 1.97.1 with rustfmt and Clippy, ShellCheck, actionlint 1.7.12, and okf 0.2.7.
Linux builds need libxcb-shape0-dev and libxcb-xfixes0-dev.
Scripts target Bash 3.2 and use language-native tools without requiring RTK.

```sh
rustup toolchain install 1.97.1 --profile minimal --component clippy --component rustfmt
scripts/preflight.sh
rustup toolchain install 1.67.0 --profile minimal
TLESS_TOOLCHAIN=1.67.0 scripts/preflight.sh test
```

The entry point runs formatting, all-target Clippy for minimal and combined features,
ShellCheck, workflow validation, release-gate regression tests, and all 4 feature profiles.
CI calls this same entry point. It runs inexpensive lint once, Linux minimum-compiler
coverage, and macOS terminal tests. Release preparation adds the full native platform matrix.
There is no root library target, so root library doctests do not apply.

Tests use disposable files and isolated pseudoterminals.
A terminal-permission failure must be rerun with terminal access; do not skip it.
Run the manual checks in docs/toon-acceptance.md before publishing a release.
Documentation-only edits can use `scripts/validate-docs.sh` locally. It validates
the complete docs bundle with pinned okf 0.2.7. CI calls this same check through
preflight, including on documentation-only and workflow changes.

## Pull requests

Use Conventional Commit PR titles, for example `fix(cli): report input errors on stderr`.
Allowed types are feat, fix, docs, refactor, perf, test, build, ci, chore, and revert.
Use a scope when it helps and `!` for a breaking change.
Squash merge into main after the `required` CI check and maintainer review.
Resolve review conversations. Temporary branch commits need not follow the convention.
Workflow changes trigger the same checks. Dependabot maintains action pins and dependencies.

Describe behavior changes and validation. Update the Unreleased changelog for notable
user effects, including upgrade instructions for breaks. Pure maintenance does not
require a changelog entry. Preserve old upstream commit and release history.

## Compatibility and release ownership

The CommandZero maintainers own releases and the public CLI/TUI contract.
During 0.x, incompatible changes use the next minor version; compatible fixes use patches.
Compiler minimum increases also use a minor version and need dependency/feature validation.
The manifest is the version source; release checks compare it with the lockfile, tag,
changelog, and extracted binary. Tags use `vX.Y.Z` or `vX.Y.Z-rc.N`.

Version 0.10.0 is the first planned tless release. It changes the executable name,
default TOON feature, input limit, and tested binary platform floors together.
No tless releases or tless Homebrew formula were found during migration preparation.
The upstream jless package, tags, and download URLs remain unchanged.
Recheck consumers before publication, then follow docs/release-checklist.md.
