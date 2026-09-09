#!/usr/bin/env bash
# Repository policy surrounds, but does not replace, native OpenSpec validation.
set -euo pipefail
export OPENSPEC_TELEMETRY=0
base=${OPENSPEC_BASE:-origin/main}
body=${OPENSPEC_PR_BODY:-}
if [[ ${1:-} == --help ]]; then
    echo 'usage: OPENSPEC_BASE=<target-ref> OPENSPEC_PR_BODY="<PR description>" scripts/openspec-check.sh'
    exit 0
fi
[[ $# == 0 ]] || { echo 'Unexpected arguments; use --help.' >&2; exit 2; }
root=$(git rev-parse --show-toplevel)
cd "$root"
merge_base=$(git merge-base "$base" HEAD) || { echo "Fetch the OpenSpec target ref: $base" >&2; exit 1; }
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/head" "$stage/check/openspec/changes/archive"
git archive HEAD | tar -x -C "$stage/head"
# --no-renames includes both deleted and added paths, including rename endpoints.
git diff --name-only --no-renames -z "$merge_base" HEAD -- openspec/changes > "$stage/paths"
: > "$stage/ids"
while IFS= read -r -d '' path; do
    path=${path#openspec/changes/}
    if [[ $path == archive/* ]]; then
        path=${path#archive/}
        path=${path%%/*}
        if [[ $path =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}-(.+)$ ]]; then
            printf '%s\n' "${BASH_REMATCH[1]}" >> "$stage/ids"
        else
            echo "Invalid archive directory: $path" >&2; exit 1
        fi
    else
        printf '%s\n' "${path%%/*}" >> "$stage/ids"
    fi
done < "$stage/paths"
while IFS= read -r line; do
    if [[ $line == 'OpenSpec-Change: '* ]]; then
        printf '%s\n' "${line#OpenSpec-Change: }" >> "$stage/ids"
    fi
done <<< "$body"
sort -u "$stage/ids" > "$stage/selected"
if [[ ! -s $stage/selected ]]; then
    echo 'OpenSpec completion: not applicable (no associated changes).'
    exit 0
fi
version=$(openspec --version)
[[ $version == 1.11.0 ]] || { echo "OpenSpec 1.11.0 required; found $version" >&2; exit 1; }
while IFS= read -r id; do
    [[ $id =~ ^[a-z0-9]+(-[a-z0-9]+)*$ ]] || { echo "Invalid OpenSpec change ID: $id" >&2; exit 1; }
    [[ ! -d $stage/head/openspec/changes/$id ]] || { echo "$id: still active; complete, synchronize and archive it." >&2; exit 1; }
    archives=("$stage/head/openspec/changes/archive/"????-??-??-"$id")
    [[ ${#archives[@]} == 1 && -d ${archives[0]} ]] || { echo "$id: require exactly one preserved archive." >&2; exit 1; }
    archive=${archives[0]}
    # Preserve every tracked artifact from the base, while allowing its final edits.
    git ls-tree -r --name-only "$merge_base" -- openspec/changes > "$stage/artifacts"
    while IFS= read -r path; do
        case $path in
            openspec/changes/"$id"/*) relative=${path#openspec/changes/"$id"/} ;;
            openspec/changes/archive/????-??-??-"$id"/*)
                relative=${path#openspec/changes/archive/}
                relative=${relative#*/}
                ;;
            *) continue ;;
        esac
        [[ -f $archive/$relative ]] || { echo "$id: archive lost $relative" >&2; exit 1; }
    done < "$stage/artifacts"
    [[ -f $archive/proposal.md && -f $archive/tasks.md ]] || { echo "$id: archive needs proposal.md and tasks.md." >&2; exit 1; }
    reviewed=false
    while IFS= read -r line; do
        [[ $line != "OpenSpec-Sync-Reviewed: $id" ]] || reviewed=true
    done <<< "$body"
    "$reviewed" || { echo "$id: review delta-to-main correspondence and add OpenSpec-Sync-Reviewed: $id to the PR description." >&2; exit 1; }
    cp -R "$archive" "$stage/check/openspec/changes/$id"
    cp -R "$archive" "$stage/check/openspec/changes/archive/"
    if [[ -d $archive/specs ]]; then
        for delta in "$archive"/specs/*/spec.md; do
            [[ -f $delta ]] || continue
            spec=${delta%/spec.md}; spec=${spec##*/}
            source_spec=$stage/head/openspec/specs/$spec/spec.md
            [[ -f $source_spec ]] || { echo "$id: missing main spec $spec" >&2; exit 1; }
            mkdir -p "$stage/check/openspec/specs/$spec"
            cp "$source_spec" "$stage/check/openspec/specs/$spec/spec.md"
        done
    else
        # Native change validation owns no-delta validity; reviewers explain it.
        echo "$id: no delta directory; the correspondence review must explain why."
    fi
    # Validate archived delta syntax through the native active-change validator.
    (cd "$stage/check" && openspec validate "$id" --type change --strict --no-interactive)
    rm -rf "$stage/check/openspec/changes/$id"
    echo "$id: archive preserved; synchronization review recorded."
done < "$stage/selected"
(cd "$stage/check" && openspec validate --archived --strict --no-interactive)
if [[ -d $stage/check/openspec/specs ]]; then
    (cd "$stage/check" && openspec validate --specs --strict --no-interactive)
fi
