#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
[[ "$(okf --version)" == "okf 0.2.7 "* ]] || { echo 'Install okf 0.2.7 for documentation validation.' >&2; exit 1; }
okf validate docs/
