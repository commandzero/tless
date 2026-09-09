---
type: Guide
title: Contributing
description: Repository standards, local checks, pull requests, and compatibility commitments.
status: stable
generated: { by: codex/gpt-6, at: 2026-09-09T19:48:12Z }
---

# Contributing

CommandZero maintains tless as an independently released fork of jless.
Changes can diverge from upstream conventions while preserving its license and attribution.

## Standards and scope

Use the shared [repo-man bundle](../../repo-man/index.md).
Clone CommandZero/repo-man beside this checkout if that link is unavailable.
The user-level copy at `~/.agents/memory/repo-man/` is also an accepted source
for this workspace when the sibling checkout is unavailable.
This repository adopts its applicable Rust, Bash, CLI, preflight, updates,
versioning, and release guidance, including draft recommendations.
The pending TUI and release-target documents do not provide complete requirements.
The README's platform and terminal contract supplies this repository's current scope.

One application crate is sufficient. Keep codec integration behind the TOON module.
Existing Unix terminal FFI uses unsafe code; retain small, reviewed boundaries and
terminal tests instead of imposing an incompatible blanket unsafe-code prohibition.
Do not remove behavior tests because a type check passes.

## Local checks

Install rustup, Rust 1.97.1 with rustfmt and Clippy, ShellCheck, actionlint 1.7.12, okf 0.2.7, Node.js 20.19 or newer, and OpenSpec 1.11.0.
Linux builds need libxcb-shape0-dev and libxcb-xfixes0-dev.
Scripts target Bash 3.2 and use language-native tools without requiring RTK.

```sh
rustup toolchain install 1.97.1 --profile minimal --component clippy --component rustfmt
npm install --global @fission-ai/openspec@1.11.0
scripts/preflight.sh
rustup toolchain install 1.87.0 --profile minimal
TLESS_TOOLCHAIN=1.87.0 scripts/preflight.sh test
```

The entry point runs formatting, all-target Clippy for minimal and combined features,
ShellCheck, workflow validation, release and OpenSpec gate regression tests, the PR-scoped OpenSpec gate, and all 4 feature profiles.
CI calls this same entry point. It runs inexpensive lint once, Linux minimum-compiler
coverage, and macOS terminal tests. Release preparation adds the full native platform matrix.
There is no root library target, so root library doctests do not apply.

Tests use disposable files and isolated pseudoterminals.
A terminal-permission failure must be rerun with terminal access; do not skip it.
Run the manual checks in [TOON acceptance](toon-acceptance.md) before publishing a release.
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

## OpenSpec completion gate

OpenSpec planning artifacts live under `openspec/`, outside the `docs/` OKF bundle.
Run all OpenSpec CLI commands with `OPENSPEC_TELEMETRY=0`.
Planning workflows create artifacts without implementing their tasks.

Before an associated implementation PR merges:

1. Complete and verify the change's implementation tasks.
2. Synchronize its delta requirements into main specs and archive its artifacts.
   Deleting an active change directory alone does not satisfy this gate.
3. Pass native OpenSpec validation for the affected changes and specs, including
   archived-task validation, using the pinned CLI selected during implementation.
4. Review delta-to-main-spec correspondence explicitly. Archive presence and syntax
   validation alone do not prove synchronization or implementation correctness.
5. Run a PR-scoped completion check through one local entry point shared with CI.
   Select associated changes from added, edited, deleted, and renamed Git paths,
   plus explicit associations when artifacts are absent from the PR diff.
   Unrelated active changes must not block the PR.

Run the committed-head check locally with the target branch fetched:

```sh
OPENSPEC_BASE=origin/main OPENSPEC_PR_BODY="$(cat /tmp/pr-description.md)" scripts/openspec-check.sh
```

Preflight calls this same entry point, and CI supplies the PR target commit and body.
The default local target is `origin/main`; selection compares its merge base with
committed `HEAD`. Commit changes before running the gate. Both rename endpoints
and added, edited, or deleted active/archive paths select changes. Unrelated active
changes and historical archives are excluded. No associated changes reports a
successful not-applicable result.

For implementation whose artifacts are absent from the diff, include one line per
associated change in the PR description:

```text
OpenSpec-Change: toon-only-rendering
OpenSpec-Sync-Reviewed: toon-only-rendering
```

The second field is required for every selected change. Add it only after reviewing
all archived additions, modifications, removals, renames, and scenarios against
current main specs. Explain any change without deltas in the PR description and use the native
`skip_specs: true` metadata marker.
Maintainer review must confirm this statement before merging; CI records the
statement, rather than proving semantic synchronization. Already-synchronized main
specs need no artificial edit. Missing statements fail, including archives produced
with skipped synchronization. Repeat both fields for multiple changes. Artifact-free
implementation requires the first field; the gate cannot infer undisclosed associations.

OpenSpec 1.11.0 supplies native strict change/spec validation and `validate --archived`
for task completion. The gate stages only associated archives and affected main specs
from committed `HEAD`. It validates archived deltas through the native change command,
then invokes the native archived-task and main-spec validators. It preserves each
base active artifact by path and requires proposal and task files. The gate performs
no automatic synchronization or archival. Native validators own syntax/task semantics;
the repository adds selection, preservation, archival, and explicit review policy.
Run `scripts/test-openspec.sh` for fixture coverage with the real pinned CLI.

## Compatibility and release ownership

The CommandZero maintainers own releases and the public CLI/TUI contract.
During 0.x, incompatible changes use the next minor version; compatible fixes use patches.
Compiler minimum increases also use a minor version and need dependency/feature validation.
The manifest is the version source; release checks compare it with the lockfile, tag,
changelog, and extracted binary. Tags use `vX.Y.Z` or `vX.Y.Z-rc.N`.

The independent tless release history starts at 0.1.0. It changes the executable name,
default TOON feature, input limit, and tested binary platform floors together.
No tless releases or tless Homebrew formula were found during migration preparation.
The upstream jless package, tags, and download URLs remain unchanged.
Recheck consumers before publication, then follow [the release checklist](release-checklist.md).
