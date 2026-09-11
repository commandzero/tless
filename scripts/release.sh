#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

version=$(awk -F '"' '/^version =/ { print $2; exit }' Cargo.toml)
tag="v$version"
toolchain=$(awk -F '"' '/^channel =/ { print $2 }' rust-toolchain.toml)
cargo_path=$(rustup which --toolchain "$toolchain" cargo)
PATH="$(dirname "$cargo_path"):$PATH"
export PATH
targets=(aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu)
release_features=toon,colorscheme

fail() { echo "$*" >&2; exit 1; }

default_features() {
    awk '
        /^\[features\]$/ { in_features = 1; next }
        in_features && /^\[/ { exit }
        in_features && /^default = / {
            value = $0
            sub(/^default = \[/, "", value)
            sub(/\]$/, "", value)
            gsub(/"/, "", value)
            gsub(/[[:space:]]/, "", value)
            print value
            exit
        }
    ' Cargo.toml
}

notes() {
    awk -v heading="## [$version] - " '
        index($0, heading) == 1 { found = 1; next }
        found && /^## / { exit }
        found { print }
    ' CHANGELOG.md
}

check() {
    [[ "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-rc\.(0|[1-9][0-9]*))?$ ]] || fail 'invalid release version'
    [[ "${1:-}" == "$tag" ]] || fail "expected tag $tag"
    awk -v version="$version" '
        /^name = "tless"$/ { package = 1; next }
        package && /^version = / { if ($0 == "version = \"" version "\"") found = 1; exit }
        END { exit !found }
    ' Cargo.lock || fail 'lockfile version mismatch'
    awk -v prefix="## [$version] - " 'index($0, prefix) == 1 && $0 ~ /[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]$/ { found = 1 } END { exit !found }' CHANGELOG.md || fail 'missing dated version heading'
    [[ -n "$(notes)" ]] || fail "missing curated changelog section for $version"
    # A release tag must identify the checked-out, reviewed main-branch source.
    [[ "$(git rev-parse "$tag^{commit}")" == "$(git rev-parse HEAD)" ]] || fail 'tag does not identify HEAD'
    git merge-base --is-ancestor HEAD origin/main || fail 'release commit is not on origin/main'
    [[ -z "$(git status --porcelain --untracked-files=normal)" ]] || fail 'release requires a clean checkout'
}

smoke() {
    local binary=$1
    [[ "$("$binary" --version)" == "tless $version" ]] || fail 'binary version mismatch'
    [[ "$(printf '42' | "$binary" --input json -)" == '42' ]] || fail 'JSON smoke test failed'
    [[ "$(printf 'name: Ada' | "$binary" --input toon -)" == 'name: Ada' ]] || fail 'TOON smoke test failed'
    if printf '123' | "$binary" --max-input-bytes 2 - >/dev/null 2>&1; then
        fail 'input limit smoke test failed'
    fi
}

package() {
    local target=$1 supported=false
    for item in "${targets[@]}"; do [[ "$target" != "$item" ]] || supported=true; done
    "$supported" || fail "unsupported target: $target"
    [[ "$(rustc -vV | awk '/^host:/ {print $2}')" == "$target" ]] || fail 'package and smoke-test on the native target'
    if [[ "$target" == *apple-darwin ]]; then export MACOSX_DEPLOYMENT_TARGET=15.0; fi
    [[ "$(default_features)" == "$release_features" ]] || fail "release default features must be $release_features"
    cargo build --locked --release --target "$target"
    mkdir -p dist
    local stage archive
    stage=$(mktemp -d "${TMPDIR:-/tmp}/tless-package.XXXXXX")
    trap 'rm -rf "$stage"' EXIT
    cp "target/$target/release/tless" "$stage/tless"
    cp LICENSE.md "$stage/LICENSE.md"
    cp NOTICES.md "$stage/NOTICES.md"
    {
        printf 'tag=%s\ncommit=%s\ncompiler=%s\nfeatures=%s\ntarget=%s\n' "$tag" "$(git rev-parse HEAD)" "$(rustc --version)" "$release_features" "$target"
        if [[ "$target" == *apple-darwin ]]; then
            printf 'minimum_os=macOS 15\n'
        else
            printf 'minimum_os=Ubuntu 24.04, glibc 2.39\n'
        fi
        if [[ -z "$(git status --porcelain --untracked-files=normal)" ]]; then
            printf 'source_dirty=false\n'
        else
            printf 'source_dirty=true\n'
        fi
        printf 'build_host=%s\n' "$(uname -sr)"
    } > "$stage/BUILD-INFO.txt"
    archive="tless-$tag-$target.tar.gz"
    [[ ! -e "dist/$archive" && ! -e "dist/$archive.sha256" ]] || fail 'refusing to replace existing release artifacts'
    tar -czf "dist/$archive" -C "$stage" tless LICENSE.md NOTICES.md BUILD-INFO.txt
    mkdir "$stage/extracted"
    tar -xzf "dist/$archive" -C "$stage/extracted"
    smoke "$stage/extracted/tless"
    (cd dist; shasum -a 256 "$archive" > "$archive.sha256")
    rm -rf "$stage"
    trap - EXIT
}

verify_assets() {
    local directory=$1 target archive count
    count=$(find "$directory" -type f | wc -l | tr -d ' ')
    [[ "$count" == 8 ]] || fail 'expected exactly 4 archives and 4 checksum sidecars'
    for target in "${targets[@]}"; do
        archive="tless-$tag-$target.tar.gz"
        [[ -f "$directory/$archive" && -f "$directory/$archive.sha256" ]] || fail "missing $archive or checksum"
        [[ "$(cat "$directory/$archive.sha256")" == "$(cd "$directory"; shasum -a 256 "$archive")" ]] || fail "checksum mismatch: $archive"
        [[ "$(tar -tzf "$directory/$archive" | sort)" == "$(printf '%s\n' tless LICENSE.md NOTICES.md BUILD-INFO.txt | sort)" ]] || fail "unexpected archive layout: $archive"
        tar -xOzf "$directory/$archive" BUILD-INFO.txt | awk -v tag="$tag" -v target="$target" -v commit="$(git rev-parse HEAD)" '
            $0 == "tag=" tag { t = 1 }
            $0 == "target=" target { a = 1 }
            $0 == "commit=" commit { c = 1 }
            $0 == "features=" expected_features { f = 1 }
            $0 == "source_dirty=false" { clean = 1 }
            END { exit !(t && a && c && f && clean) }
        ' expected_features="$release_features" || fail "build metadata mismatch: $archive"
    done
}

case "${1:-}" in
    check) check "${2:-}" ;;
    notes) notes ;;
    package) package "${2:-}" ;;
    verify-assets) verify_assets "${2:-dist}" ;;
    *) echo 'usage: scripts/release.sh check TAG | notes | package TARGET | verify-assets DIR' >&2; exit 2 ;;
esac
