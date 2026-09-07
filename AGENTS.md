# Contributor instructions

Read [docs/contributing.md](docs/contributing.md) before changing this repository.
The shared standards bundle is [repo-man](../repo-man/index.md).
If it is missing, obtain CommandZero/repo-man as a sibling checkout.

This repository adopts the applicable Rust, Bash, CLI, preflight, updates,
versioning, and release guidance in that bundle, including current draft
recommendations as scoped by docs/contributing.md. This is a repository selection,
not a change to the shared documents' draft status.

Use the repo-man skill when contributing or auditing.
Run `scripts/preflight.sh`; keep Rust 1.87 compatibility and the committed lockfile.
Preserve upstream source licenses and historical release notes.
Use the published toon-format crate with default features disabled. Do not vendor
codec source or add a Cargo patch. Registry publication is a separate release decision.

Keep only repository entry points, identity, and legal documents at the root.
Use lowercase snake_case filenames throughout `docs/` and for Markdown files in `tests/`.
The complete `docs/` directory is an OKF bundle. Run `scripts/validate-docs.sh`
for documentation changes; CI calls the same check. There is no active OpenSpec
change tree. If one is introduced, add the applicable OpenSpec completion gate.
