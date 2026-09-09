#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
[[ "$(okf --version)" == "okf 0.2.7 "* ]] || { echo 'Install okf 0.2.7 for documentation validation.' >&2; exit 1; }
while IFS= read -r -d '' path; do
    filename=${path##*/}
    [[ "$filename" =~ ^[a-z0-9]+(-[a-z0-9]+)*(\.[a-z0-9]+)*$ ]] || {
        echo "Use lowercase kebab-case for documentation filenames: $path" >&2
        exit 1
    }
done < <(find docs -type f -print0; find tests -type f -iname '*.md' -print0)
okf validate docs/
