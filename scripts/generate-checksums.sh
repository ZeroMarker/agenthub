#!/usr/bin/env bash
# generate-checksums.sh - Generate SHA-256SUMS.txt for release artifacts
#
# Usage:
#   bash scripts/generate-checksums.sh <artifact-dir>
#
# Hashes every file in <artifact-dir> and writes SHA-256SUMS.txt
# inside that directory with lines: "<hash>  <filename>"

set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 <artifact-dir>" >&2
    exit 1
fi

DIR="$1"
if [ ! -d "$DIR" ]; then
    echo "Directory not found: $DIR" >&2
    exit 1
fi

cd "$DIR"
if command -v sha256sum >/dev/null 2>&1; then
    find . -maxdepth 1 -type f ! -name 'SHA-256SUMS.txt' -exec sha256sum {} + | sort -k2 > SHA-256SUMS.txt
else
    # macOS / BSD fallback
    find . -maxdepth 1 -type f ! -name 'SHA-256SUMS.txt' -exec shasum -a 256 {} + | sort -k2 > SHA-256SUMS.txt
fi

echo "[OK] Wrote $DIR/SHA-256SUMS.txt"
