#!/usr/bin/env bash
# Regression tests for collect-prs.sh:
#   1. A curl failure must exit non-zero (not silently produce empty notes).
#   2. A jq parse failure must exit non-zero.
#   3. Happy path: valid response produces correct release notes.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
COLLECT_PRS="$SCRIPT_DIR/../collect-prs.sh"

PASS=0
FAIL=0

WORKDIR=$(mktemp -d)
trap 'rm -rf "$WORKDIR"' EXIT

# ── helper ────────────────────────────────────────────────────────────────────
# run_script BIN_DIR OUTPUT_FILE → exit status stored in LAST_EXIT
run_script() {
  local bin_dir="$1"
  local out_file="$2"
  PATH="$bin_dir:$PATH" \
    SINCE_DATE="2024-01-01T00:00:00Z" \
    AREA_LABELS="area: core" \
    GITHUB_TOKEN="fake-token" \
    GITHUB_REPOSITORY="owner/repo" \
    OUTPUT_FILE="$out_file" \
    bash "$COLLECT_PRS" >/dev/null 2>&1
}

# ── test 1: curl failure → non-zero exit ──────────────────────────────────────
BIN1="$WORKDIR/bin1"
mkdir -p "$BIN1"
printf '#!/bin/sh\nexit 6\n' > "$BIN1/curl"
chmod +x "$BIN1/curl"

OUT1="$WORKDIR/notes1.md"
if run_script "$BIN1" "$OUT1"; then
  echo "FAIL: curl failure should cause non-zero exit (got 0)"
  FAIL=$((FAIL + 1))
else
  echo "PASS: curl failure causes non-zero exit"
  PASS=$((PASS + 1))
fi

# no partial output file should be written
if [ -f "$OUT1" ]; then
  echo "FAIL: output file should not exist after curl failure"
  FAIL=$((FAIL + 1))
else
  echo "PASS: no output file written after curl failure"
  PASS=$((PASS + 1))
fi

# ── test 2: invalid JSON body → non-zero exit ─────────────────────────────────
BIN2="$WORKDIR/bin2"
mkdir -p "$BIN2"
# Returns HTTP 200 with a blank-line separator but garbage JSON body
printf '#!/bin/sh\nprintf "HTTP/2 200\\r\\n\\r\\nNOT_JSON\\n"\n' > "$BIN2/curl"
chmod +x "$BIN2/curl"

OUT2="$WORKDIR/notes2.md"
if run_script "$BIN2" "$OUT2"; then
  echo "FAIL: invalid JSON body should cause non-zero exit (got 0)"
  FAIL=$((FAIL + 1))
else
  echo "PASS: invalid JSON body causes non-zero exit"
  PASS=$((PASS + 1))
fi

# ── test 3: valid response → exit 0 + correct notes ──────────────────────────
BIN3="$WORKDIR/bin3"
mkdir -p "$BIN3"
# Minimal GitHub search response with one PR; no Link header so pagination stops.
cat > "$BIN3/curl" << 'EOF'
#!/bin/sh
printf 'HTTP/2 200\r\ncontent-type: application/json\r\n\r\n'
printf '{"total_count":1,"incomplete_results":false,"items":[{"number":42,"title":"Fix the thing","html_url":"https://github.com/owner/repo/pull/42","user":{"login":"alice"}}]}\n'
EOF
chmod +x "$BIN3/curl"

OUT3="$WORKDIR/notes3.md"
if run_script "$BIN3" "$OUT3"; then
  if grep -q "Fix the thing" "$OUT3" 2>/dev/null; then
    echo "PASS: valid response produces correct release notes"
    PASS=$((PASS + 1))
  else
    echo "FAIL: output file missing expected PR title"
    FAIL=$((FAIL + 1))
  fi
else
  echo "FAIL: happy path should exit 0"
  FAIL=$((FAIL + 1))
fi

# ── summary ───────────────────────────────────────────────────────────────────
echo ""
echo "Results: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
