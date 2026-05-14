#!/usr/bin/env bash
# Regression test: VERSION_TAG selection must use head -1 (newest), not tail -1 (oldest).
# The nightly check-changes job compares HEAD against the most recent version tag to decide
# whether a build is needed. If it picks the oldest tag the check is always true and every
# nightly triggers a pointless build.
set -euo pipefail

PASS=0
FAIL=0

WORKDIR=$(mktemp -d)
trap 'rm -rf "$WORKDIR"' EXIT

# ── build a repo with three version tags, oldest→newest ──────────────────────
cd "$WORKDIR"
git init -q
git config user.email "ci@test"
git config user.name  "CI"

GIT_AUTHOR_DATE="2024-01-01T00:00:00Z" \
GIT_COMMITTER_DATE="2024-01-01T00:00:00Z" \
  git commit -q --allow-empty -m "v1.0.0 commit"
git tag app/v1.0.0

GIT_AUTHOR_DATE="2024-02-01T00:00:00Z" \
GIT_COMMITTER_DATE="2024-02-01T00:00:00Z" \
  git commit -q --allow-empty -m "v1.1.0 commit"
git tag app/v1.1.0

GIT_AUTHOR_DATE="2024-03-01T00:00:00Z" \
GIT_COMMITTER_DATE="2024-03-01T00:00:00Z" \
  git commit -q --allow-empty -m "v2.0.0 commit"
git tag app/v2.0.0

# ── test: head -1 after descending sort → newest tag ─────────────────────────
SELECTED=$(git tag -l 'app/v*' --sort=-creatordate 2>/dev/null | head -1)
if [ "$SELECTED" = "app/v2.0.0" ]; then
  echo "PASS: head -1 with --sort=-creatordate selects newest tag (app/v2.0.0)"
  PASS=$((PASS + 1))
else
  echo "FAIL: expected app/v2.0.0, got '$SELECTED'"
  FAIL=$((FAIL + 1))
fi

# ── confirm the original bug: tail -1 would have picked the oldest ────────────
WRONG=$(git tag -l 'app/v*' --sort=-creatordate 2>/dev/null | tail -1)
if [ "$WRONG" = "app/v1.0.0" ]; then
  echo "PASS: confirmed tail -1 would incorrectly select oldest tag (app/v1.0.0)"
  PASS=$((PASS + 1))
else
  echo "FAIL: expected tail -1 to give app/v1.0.0, got '$WRONG'"
  FAIL=$((FAIL + 1))
fi

# ── summary ───────────────────────────────────────────────────────────────────
echo ""
echo "Results: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
