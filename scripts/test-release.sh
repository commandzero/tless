#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

expect_failure() {
    if "$@" > /dev/null 2>&1; then
        echo "unexpected success: $*" >&2
        exit 1
    fi
}
expect_failure scripts/release.sh check v999.0.0
fixture=$(mktemp -d "${TMPDIR:-/tmp}/tless-release-test.XXXXXX")
trap 'rm -rf "$fixture"' EXIT
mkdir "$fixture/assets" "$fixture/stage" "$fixture/repo" "$fixture/repo/scripts"
expect_failure scripts/release.sh verify-assets "$fixture/assets"
version=$(awk -F '"' '/^version =/ { print $2; exit }' Cargo.toml)
commit=$(git rev-parse HEAD)
for target in aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
    # Fake binaries validate assembly, not runtime behavior. Real smoke tests run during packaging.
    printf 'fixture\n' > "$fixture/stage/tless"
    chmod +x "$fixture/stage/tless"
    cp LICENSE.md "$fixture/stage/LICENSE.md"
    cp NOTICES.md "$fixture/stage/NOTICES.md"
    printf 'tag=v%s\ntarget=%s\ncommit=%s\nfeatures=toon,colorscheme\nsource_dirty=false\n' "$version" "$target" "$commit" > "$fixture/stage/BUILD-INFO.txt"
    archive="tless-v$version-$target.tar.gz"
    tar -czf "$fixture/assets/$archive" -C "$fixture/stage" tless LICENSE.md NOTICES.md BUILD-INFO.txt
    (cd "$fixture/assets"; shasum -a 256 "$archive" > "$archive.sha256")
done
scripts/release.sh verify-assets "$fixture/assets"
printf 'corruption' >> "$fixture/assets/$archive"
expect_failure scripts/release.sh verify-assets "$fixture/assets"

# The tag gate uses a disposable repository and never mutates the real repository.
cp scripts/release.sh "$fixture/repo/scripts/"
cp Cargo.toml Cargo.lock rust-toolchain.toml "$fixture/repo/"
printf '## [%s] - 2026-09-06\n\n### Fixed\n\n- Fixture behavior.\n' "$version" > "$fixture/repo/CHANGELOG.md"
(
    cd "$fixture/repo"
    git init -q
    git add .
    git -c user.name=Test -c user.email=test@example.invalid commit -qm 'test: release fixture'
    git update-ref refs/remotes/origin/main HEAD
    git tag "v$version"
    scripts/release.sh check "v$version"
    printf '\nDirty fixture\n' >> CHANGELOG.md
    expect_failure scripts/release.sh check "v$version"
    git checkout -- CHANGELOG.md
    printf '## [Unreleased]\n' > CHANGELOG.md
    expect_failure scripts/release.sh check "v$version"
)
