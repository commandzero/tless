# Contributor instructions

Read [CONTRIBUTING.md](CONTRIBUTING.md) before changing this repository.
The shared standards bundle is [repo-man](../repo-man/index.md).
If it is missing, obtain CommandZero/repo-man as a sibling checkout.

This repository adopts the applicable Rust, Bash, CLI, preflight, updates,
versioning, and release guidance in that bundle, including current draft
recommendations as scoped by CONTRIBUTING.md. This is a repository selection,
not a change to the shared documents' draft status.

Use the repo-man skill when contributing or auditing.
Run `scripts/preflight.sh`; keep Rust 1.67 compatibility and the committed lockfile.
Preserve upstream source licenses and historical release notes.
Do not publish registry packages while the local codec patch is required.

Root Markdown is contributor/product documentation, not an OKF bundle.
There is no `docs/` bundle or active OpenSpec change tree.
If either is introduced, add the applicable complete-bundle or OpenSpec gate.
