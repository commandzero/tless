---
type: Guide
title: Release checklist
description: Release preparation, packaging, publication, and recovery.
status: draft
generated: { by: codex/gpt-6, at: 2026-09-09T19:48:12Z }
---

# Release checklist

## Prepare a reviewed release proposal

1. Open a Conventional Commit PR against main. Update Cargo.toml and Cargo.lock
   together. Keep `publish = false` until a separate registry release plan is reviewed.
2. Move notable Unreleased entries into `## [X.Y.Z] - YYYY-MM-DD` with the real
   release date. Leave an Unreleased section and update comparison links.
   Release notes come from that curated section, not generated commit summaries.
3. Review compatibility and upgrade instructions. Version 0.1.0 migrates jless
   to tless and makes TOON support mandatory. Its input cap is 512 MiB and can be removed
   with `--max-input-bytes 0`. Its binary support floors are macOS 15 and Ubuntu 24.04.
4. Recheck CommandZero/homebrew-tools and any installer consumers. Coordinate their
   URL construction before advertising a new distribution channel. No existing
   upstream jless formula, tag, release, or registry package is replaced.
5. Run local preflight and the Rust 1.87 feature profiles. Complete the manual
   clipboard, help, malformed-input restoration, and Linux full-device checks in
   docs/toon-acceptance.md. Record actual host/results in the release PR.
6. Merge after CI and review. Tag that main-branch commit as `vX.Y.Z` or
   `vX.Y.Z-rc.N`. Push the new tag only when the release proposal is accepted.

## Registry publication plan

The CommandZero maintainers own the `tless` package on crates.io. Before enabling
publication in a release PR, review package ownership and the package contents,
run `cargo package --locked` and `cargo publish --dry-run --locked`, and confirm
that the version is not already published. Set `publish = true` only in the
reviewed release commit. After the tagged GitHub release is validated, publish
that exact version with `cargo publish --locked`. Crates.io versions are
immutable; a correction requires a new version and a new reviewed release.
Record the successful package check, publish dry run, ownership review, and
version-availability result in the release PR description.

## Build and verify

The tag workflow validates tag, manifest, lockfile, curated notes, clean source,
and membership in origin/main. It runs preflight and minimum-compiler tests before packaging.

Each target runs native feature tests, builds with locked dependencies and Rust 1.97.1,
extracts its archive, and checks version, JSON, TOON, and input-limit behavior.
The supported targets and OS floors are in the [platform contract](contributing.md#platform-contract).

Archives use `tless-vX.Y.Z-<rust-target-triple>.tar.gz`.
Each has a SHA-256 sidecar containing the hash and archive basename.
The archive root contains tless, LICENSE.md,
NOTICES.md, and BUILD-INFO.txt.
Build metadata records tag, commit, compiler, features, target, host, and support floor.
The release feature set is the manifest default, with TOON always enabled and
S-expression disabled.

For a local native packaging check, run:

```sh
scripts/release.sh package aarch64-apple-darwin
```

This writes local artifacts only. It does not tag or publish.
The packaging script refuses to overwrite existing artifacts.

## Publish and recover

1. The final job requires all 4 archives and checksum sidecars, verifies hashes,
   archive layout, and source metadata, then creates a GitHub draft release.
   Release-candidate tags produce prerelease drafts.
2. A maintainer checks the complete draft and recorded manual results, then publishes it.
   Successful local checks alone do not authorize publication.
3. If packaging fails, fix the cause before retrying. Existing local artifacts need
   deliberate removal or preservation in another directory before rebuilding.
4. If draft creation or upload fails partway, inspect the existing draft. Download
   its assets and compare hashes before adding missing files. Never use upload
   clobber to replace differing bytes. The workflow refuses an existing release.
   Delete an incomplete unpublished draft only after preserving its diagnostics.
5. Never move a published tag or overwrite published assets. A correction needs a
   new version. Keep old release notes, tags, and URLs usable.
6. After publication, prepare any Homebrew update as a separate reviewed PR with
   verified checksums and supported-host install tests. If it fails, keep the
   previous formula working while fixing the update. Do not publish to crates.io
   until a registry-distribution plan and package ownership have been reviewed.
