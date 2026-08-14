#!/usr/bin/env bash
# Enforce the core-module coverage gate: every agenthub-core source file and
# the package as a whole must reach ≥80% line coverage (see todo.md P1).
# Run from the repository root. Requires cargo-llvm-cov (and llvm-tools).
set -euo pipefail

THRESHOLD="80.0"

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
  echo "error: cargo-llvm-cov is not installed (cargo install cargo-llvm-cov --locked)" >&2
  exit 1
fi

echo "Measuring agenthub-core line coverage (threshold ${THRESHOLD}%)..."
# Clean stale instrumentation data first: cargo-llvm-cov merges with any
# previously collected profraw files, which corrupts repeat local runs.
cargo llvm-cov clean --workspace >/dev/null 2>&1 || true
SUMMARY="$(cargo llvm-cov --package agenthub-core --summary-only 2>/dev/null)"

fail=0

# Per-file line coverage. Summary columns are:
#   name regions missed cover% functions missed executed% lines missed cover%
# so line coverage is column 10, with a trailing '%' to strip.
while read -r name cov; do
  if awk -v c="$cov" -v t="$THRESHOLD" 'BEGIN { exit !(c < t) }'; then
    echo "FAIL: ${name} line coverage ${cov}% < ${THRESHOLD}%" >&2
    fail=1
  else
    echo "ok:   ${name} ${cov}%"
  fi
done < <(echo "$SUMMARY" | awk '{ gsub(/%/, "", $10); if ($1 ~ /\.rs$/) print $1, $10 }')

total_cov="$(echo "$SUMMARY" | awk '$1 == "TOTAL" { gsub(/%/, "", $10); print $10 }')"
if awk -v c="$total_cov" -v t="$THRESHOLD" 'BEGIN { exit !(c < t) }'; then
  echo "FAIL: agenthub-core total line coverage ${total_cov}% < ${THRESHOLD}%" >&2
  fail=1
else
  echo "ok:   agenthub-core total ${total_cov}%"
fi

if [ "$fail" -ne 0 ]; then
  echo "Coverage gate FAILED (threshold ${THRESHOLD}%)." >&2
  exit 1
fi
echo "Coverage gate passed."
