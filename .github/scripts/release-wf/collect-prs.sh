#!/usr/bin/env bash
# Queries GitHub for merged PRs with any of the specified area labels and writes
# formatted release notes to OUTPUT_FILE.
#
# Each label is queried separately (GitHub search ANDs multiple label: qualifiers,
# but a PR only ever carries one area: label). Results are combined and deduplicated
# by PR number before writing.
#
# Required env:
#   SINCE_DATE           - ISO-8601 lower bound for merged: filter
#   AREA_LABELS          - comma-separated label names, e.g. "area: core,area: ci"
#   GITHUB_TOKEN         - GitHub auth token
#   GITHUB_REPOSITORY    - owner/repo
# Optional env:
#   GITHUB_API_BASE_URL  - defaults to https://api.github.com
#   OUTPUT_FILE          - path to write notes to (default: release_notes.md)
set -euo pipefail

SINCE_DATE="${SINCE_DATE:?SINCE_DATE env var is required}"
AREA_LABELS="${AREA_LABELS:?AREA_LABELS env var is required}"
GITHUB_TOKEN="${GITHUB_TOKEN:?GITHUB_TOKEN env var is required}"
GITHUB_REPOSITORY="${GITHUB_REPOSITORY:?GITHUB_REPOSITORY env var is required}"
GITHUB_API_BASE_URL="${GITHUB_API_BASE_URL:-https://api.github.com}"
OUTPUT_FILE="${OUTPUT_FILE:-release_notes.md}"

ALL_PRS="[]"

IFS=',' read -ra LABEL_ARRAY <<< "$AREA_LABELS"
for LABEL in "${LABEL_ARRAY[@]}"; do
  LABEL=$(echo "$LABEL" | xargs)  # trim whitespace

  echo "Querying: label=\"${LABEL}\" merged after ${SINCE_DATE}" >&2

  NEXT_URL="${GITHUB_API_BASE_URL}/search/issues"
  QUERY_ARGS=(-G \
    --data-urlencode "q=is:pr is:merged merged:>${SINCE_DATE} label:\"${LABEL}\" repo:${GITHUB_REPOSITORY}" \
    --data-urlencode "per_page=100")
  PAGE=1

  while [ -n "$NEXT_URL" ]; do
    RESPONSE=$(curl -sf --include \
      -H "Authorization: Bearer ${GITHUB_TOKEN}" \
      -H "Accept: application/vnd.github.v3+json" \
      "${QUERY_ARGS[@]}" \
      "$NEXT_URL")

    # Split headers from body (headers end at the first blank line).
    HEADERS=$(echo "$RESPONSE" | sed '/^[[:space:]]*$/q')
    BODY=$(echo "$RESPONSE" | sed '1,/^[[:space:]]*$/d')

    BATCH=$(echo "$BODY" \
      | jq '[.items[] | {number: .number, title: .title, url: .html_url, author: {login: .user.login}}]')

    ALL_PRS=$(printf '%s\n%s\n' "$ALL_PRS" "$BATCH" \
      | jq -s 'add | unique_by(.number) | sort_by(.number) | reverse')

    # Extract the next-page URL from the Link header, if present.
    NEXT_URL=$(echo "$HEADERS" \
      | grep -i '^link:' \
      | grep -o '<[^>]*>; rel="next"' \
      | sed 's/<\([^>]*\)>; rel="next"/\1/' \
      || true)

    # Only the first request needs the query-string args; subsequent pages use the full URL from Link.
    QUERY_ARGS=()
    PAGE=$((PAGE + 1))
    [ -n "$NEXT_URL" ] && echo "  Fetching page ${PAGE} for label \"${LABEL}\"..." >&2
  done
done

PR_COUNT=$(echo "$ALL_PRS" | jq 'length')
echo "Total unique PRs: ${PR_COUNT}" >&2

{
  echo "## What's Changed"
  echo ""
  if [ "$PR_COUNT" -eq 0 ]; then
    echo "No changes found for the specified areas."
  else
    echo "$ALL_PRS" | jq -r '.[] | "- \(.title) ([#\(.number)](\(.url))) by @\(.author.login)"'
  fi
} > "$OUTPUT_FILE"

echo "Release notes written to ${OUTPUT_FILE}" >&2
