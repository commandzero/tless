---
type: Guide
title: Contributing
description: Repository standards, local checks, pull requests, and compatibility commitments.
status: stable
generated: { by: codex/gpt-6, at: 2026-09-11T00:29:19Z }
---

# Contributing

CommandZero maintains and releases tless as a fork of jless. We can depart from
upstream conventions, but must preserve the upstream license and attribution.

## Standards and scope

Follow the applicable Rust, Bash, CLI, preflight, updates, versioning, and release
guidance in the shared [repo-man bundle](../../repo-man/index.md), including its
draft recommendations. If the link is unavailable, clone CommandZero/repo-man
beside this checkout or use `~/.agents/memory/repo-man/`.
The TUI and release-target guidance is incomplete. Use the platform and terminal
contract documented below to determine what tless supports.

Keep one application crate and put codec integration in the TOON module.
Use nix wrappers for Unix terminal calls and borrowed or owned file descriptors.
Keep any unsafe code in test setup small and document its safety requirements.
Test terminal behavior. Passing a type check does not replace behavior tests.

## Local checks

Install rustup and Rust 1.97.1 with rustfmt and Clippy. You also need ShellCheck,
actionlint 1.7.12, okf 0.2.7, Node.js 20.19 or newer, and OpenSpec 1.11.0.
Linux clipboard builds use arboard's Rust X11 backend and need no libxcb
development packages. Scripts target Bash 3.2 and use the language's own tools.
They do not require RTK.

```sh
rustup toolchain install 1.97.1 --profile minimal --component clippy --component rustfmt
npm install --global @fission-ai/openspec@1.11.0
scripts/preflight.sh
rustup toolchain install 1.87.0 --profile minimal
TLESS_TOOLCHAIN=1.87.0 scripts/preflight.sh test
```

Preflight checks formatting, runs all-target Clippy for minimal and combined
features, and validates shell scripts and workflows. It also runs release and
OpenSpec gate regression tests, the OpenSpec completion check for the PR, and
the feature-profile tests.

CI uses the same script. It runs lint once, tests the minimum compiler on Linux,
and runs terminal tests on macOS. Release checks cover every supported platform.
The root crate has no library target, so it has no library doctests.

Tests use disposable files and isolated pseudoterminals. If a test fails because
it lacks terminal access, rerun it with access. Do not skip it.
Run the manual checks in [TOON acceptance](toon-acceptance.md) before publishing.

For documentation-only edits, run `scripts/validate-docs.sh` locally. It validates
the complete docs bundle with okf 0.2.7. CI runs this check through preflight,
including for documentation-only and workflow changes.

## Pull requests

Use Conventional Commit PR titles, such as `fix(cli): report input errors on stderr`.
Allowed types are feat, fix, docs, refactor, perf, test, build, ci, chore, and revert.
Add a scope when it helps and `!` for a breaking change. Temporary branch commits
need not follow this convention.

Describe behavior changes and how you checked them. Add notable user-facing
changes to the Unreleased changelog, with upgrade instructions for breaking
changes. Pure maintenance needs no changelog entry. Preserve upstream commit and
release history.

Resolve review conversations, pass the `required` CI check, and get maintainer
review before squash merging into main. Workflow changes run the same checks.
Dependabot maintains action pins and dependencies.

## OpenSpec completion gate

Keep OpenSpec planning artifacts in `openspec/`, outside the `docs/` OKF bundle.
Run OpenSpec commands with `OPENSPEC_TELEMETRY=0`. Planning workflows create
artifacts but do not implement their tasks.

Before merging an implementation PR associated with an OpenSpec change:

1. Complete and verify its implementation tasks.
2. Apply its delta requirements to the main specs and archive its artifacts.
   Deleting the active change directory does not count as archiving.
3. Validate the affected changes, specs, and archived tasks with the pinned
   OpenSpec CLI used during implementation.
4. Compare every delta with the main specs. Check additions, modifications,
   removals, renames, and scenarios. Syntax validation and an archive directory
   do not establish that the specs match the change or the implementation.
5. Run the PR's completion check. It selects changes from added, edited, deleted,
   and renamed Git paths, plus associations declared in the PR description.
   Unrelated active changes must not block the PR.

Commit your changes and fetch the target branch before running the check:

```sh
OPENSPEC_BASE=origin/main OPENSPEC_PR_BODY="$(cat /tmp/pr-description.md)" scripts/openspec-check.sh
```

Preflight calls this script too. CI supplies the PR's target commit and body.
Locally, the target defaults to `origin/main`. The script compares committed
`HEAD` with the merge base of the target branch. It checks both paths of a rename
and changes to active or archived artifacts. It excludes unrelated active
changes and historical archives. If no changes are associated with the PR, it
reports that the check does not apply.

Declare changes whose artifacts are absent from the diff in the PR description.
Every selected change also needs a synchronization review statement:

```text
OpenSpec-Change: toon-only-rendering
OpenSpec-Sync-Reviewed: toon-only-rendering
```

Repeat these fields for each change. Add `OpenSpec-Sync-Reviewed` only after
comparing its archived deltas with the current main specs. A maintainer must
confirm that review before merging. CI checks for the statement but cannot judge
whether the specs match. Missing review statements fail the check, including
when archiving skipped synchronization. Specs that already match need no edit.

For a change without deltas, explain why in the PR description and set
`skip_specs: true` in its metadata. Implementation without artifacts still needs
an `OpenSpec-Change` declaration so the gate can find its associated change.

The gate reads committed `HEAD` and stages only the associated archives and
affected main specs. It validates archived deltas with OpenSpec's change
validator, checks task completion with `validate --archived`, and validates the
main specs. All validation uses OpenSpec 1.11.0 in strict mode.

The repository's checks require proposal and task files and preserve each
artifact from the base change at its relative path in the archive. They enforce
change selection, preservation, archival, and review requirements. OpenSpec
handles syntax and task validation. The gate does not synchronize or archive
anything for you. Run `scripts/test-openspec.sh` to test it against fixtures with
the pinned CLI.

## Compatibility and release ownership

CommandZero maintainers own releases and the public CLI/TUI contract.
During 0.x, use the next minor version for incompatible changes and a patch
version for compatible fixes. Raising the minimum Rust version also requires a
minor release and dependency and feature validation.

The manifest defines the version. Release checks compare it with the lockfile,
tag, changelog, and extracted binary. Tags use `vX.Y.Z` or `vX.Y.Z-rc.N`.

The tless release history starts at 0.1.0, which introduced the executable name,
TOON support, input limit, and tested platform minimums. The upstream jless
package, tags, and download URLs remain unchanged. Recheck consumers before
publication and follow [the release checklist](release-checklist.md).

## Platform contract

| Target | Release test host and support floor |
| --- | --- |
| aarch64-apple-darwin | Native macOS 15 arm64 |
| x86_64-apple-darwin | Native macOS 15 Intel |
| x86_64-unknown-linux-gnu | Native Ubuntu 24.04, glibc 2.39 |
| aarch64-unknown-linux-gnu | Native Ubuntu 24.04 arm64, glibc 2.39 |

Before publishing a release, run native tests and smoke-test the extracted binary
on all four hosts. Record the results for that release.
Other Linux distributions and older operating systems are unverified.
Windows and musl are unsupported. Linux clipboard access requires a working X11
or XWayland display session.
