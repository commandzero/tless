#!/usr/bin/env bash
# Exercise repository selection/preservation policy with the real pinned validator.
set -euo pipefail
export OPENSPEC_TELEMETRY=0
script=$(cd "$(dirname "$0")" && pwd)/openspec-check.sh
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
cd "$scratch"
git init -q
git config user.email openspec-test@example.invalid
git config user.name 'OpenSpec gate test'
git config commit.gpgsign false
git config core.hooksPath /dev/null
write_change() {
    mkdir -p "openspec/changes/$1/specs/sample"
    cat > "openspec/changes/$1/proposal.md" <<'PROPOSAL'
# Change: Sample behavior

## Why
Exercise native validation in the completion gate.

## What Changes
- Add sample behavior.

## Impact
- Affected specs: sample
PROPOSAL
    printf '%s\n' '## Tasks' '- [x] 1. Verify sample behavior' > "openspec/changes/$1/tasks.md"
    cat > "openspec/changes/$1/specs/sample/spec.md" <<'DELTA'
## ADDED Requirements
### Requirement: Sample behavior
The application SHALL preserve sample behavior.

#### Scenario: Sample input
- **WHEN** sample input is supplied
- **THEN** sample behavior is preserved
DELTA
}
write_main() {
    mkdir -p openspec/specs/sample
    cat > openspec/specs/sample/spec.md <<'SPEC'
# Sample Specification

## Purpose
Define sample behavior for exercising the repository completion gate and its native validation boundary.

## Requirements
### Requirement: Sample behavior
The application SHALL preserve sample behavior.

#### Scenario: Sample input
- **WHEN** sample input is supplied
- **THEN** sample behavior is preserved
SPEC
}
archive_change() {
    mkdir -p openspec/changes/archive
    mv "openspec/changes/$1" "openspec/changes/archive/2026-09-09-$1"
}
commit() { git add -A; git commit -qm fixture; }
expect() {
    local expected=$1 label=$2 body=${3:-} actual=0
    OPENSPEC_BASE=baseline OPENSPEC_PR_BODY="$body" "$script" > "$scratch/result" 2>&1 || actual=$?
    if [[ $expected == pass && $actual != 0 || $expected == fail && $actual == 0 ]]; then
        echo "FAIL: $label (exit $actual)" >&2
        cat "$scratch/result" >&2
        exit 1
    fi
    echo "PASS: $label"
}
reset_case() { git reset --hard -q baseline; git clean -fdq; }
write_change existing
write_change unrelated
write_main
commit
git branch baseline
expect pass 'unrelated active changes'
write_change added; commit
expect fail 'added active change'
reset_case
printf '\nUpdated rationale.\n' >> openspec/changes/existing/proposal.md; commit
expect fail 'edited active change'
reset_case
rm -rf openspec/changes/existing; commit
expect fail 'deleted-only change'
reset_case
mv openspec/changes/existing openspec/changes/renamed; commit
expect fail 'renamed active change (both endpoints)'
reset_case
expect fail 'explicit association absent from diff' 'OpenSpec-Change: existing'
archive_change existing; commit
expect fail 'skipped synchronization review'
expect pass 'already synchronized main spec' 'OpenSpec-Sync-Reviewed: existing'
reset_case
archive_change existing
rm openspec/changes/archive/2026-09-09-existing/proposal.md
commit
expect fail 'lost archive artifact' 'OpenSpec-Sync-Reviewed: existing'
reset_case
archive_change existing
printf '%s\n' '## Tasks' '- [ ] 1. Incomplete implementation' > openspec/changes/archive/2026-09-09-existing/tasks.md
commit
expect fail 'native archived task validation' 'OpenSpec-Sync-Reviewed: existing'
reset_case
archive_change existing
printf '%s\n' 'Invalid main specification' > openspec/specs/sample/spec.md
commit
expect fail 'native main spec validation' 'OpenSpec-Sync-Reviewed: existing'
reset_case
archive_change existing
printf '%s\n' 'Invalid delta specification' > openspec/changes/archive/2026-09-09-existing/specs/sample/spec.md
commit
expect fail 'native archived delta syntax validation' 'OpenSpec-Sync-Reviewed: existing'
reset_case
write_change fresh; archive_change fresh; commit
expect pass 'created and archived in PR' 'OpenSpec-Sync-Reviewed: fresh'
reset_case
write_change maintenance
rm -rf openspec/changes/maintenance/specs
printf '%s\n' 'schema: spec-driven' 'skip_specs: true' > openspec/changes/maintenance/.openspec.yaml
archive_change maintenance; commit
expect pass 'native no-delta marker with review' 'OpenSpec-Sync-Reviewed: maintenance'
reset_case
archive_change existing; archive_change unrelated; commit
expect fail 'multiple changes each require review' 'OpenSpec-Sync-Reviewed: existing'
expect pass 'multiple completed changes' $'OpenSpec-Sync-Reviewed: existing\nOpenSpec-Sync-Reviewed: unrelated'
reset_case
archive_change existing; commit
git branch -f baseline HEAD
expect pass 'explicit archived association without artifact diff' $'OpenSpec-Change: existing\nOpenSpec-Sync-Reviewed: existing'
