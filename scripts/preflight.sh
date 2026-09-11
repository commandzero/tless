#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

# Select the actual toolchain binaries even when Homebrew Cargo precedes rustup.
toolchain=${TLESS_TOOLCHAIN:-$(awk -F '"' '/^channel =/ { print $2 }' rust-toolchain.toml)}
cargo_path=$(rustup which --toolchain "$toolchain" cargo)
PATH="$(dirname "$cargo_path"):$PATH"
export PATH
export CARGO_TERM_COLOR=never

lint() {
    cargo fmt --all -- --check
    cargo clippy --locked --all-targets --no-default-features -- -D warnings
    cargo clippy --locked --all-targets --all-features -- -D warnings
    for script in scripts/*.sh; do bash -n "$script"; done
    shellcheck scripts/*.sh
    actionlint
    scripts/test-release.sh
    scripts/test-openspec.sh
    scripts/openspec-check.sh
    scripts/validate-docs.sh
}

tests() {
    cargo test --locked --no-default-features
    cargo test --locked
    cargo test --locked --no-default-features --features sexp
    cargo test --locked --no-default-features --features colorscheme
    cargo test --locked --all-features
}

case "${1:-all}" in
    all) lint; tests ;;
    lint) lint ;;
    test) tests ;;
    *) echo 'usage: scripts/preflight.sh [all|lint|test]' >&2; exit 2 ;;
esac
