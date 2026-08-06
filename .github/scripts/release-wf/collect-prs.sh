#!/usr/bin/env bash
# Finds merged PRs with any of the specified area labels and writes formatted
# release notes to OUTPUT_FILE.
#
# Versioned releases enumerate the first-parent commits in the exact
# BASE_TAG..TAG Git range and ask GitHub which PRs are associated with those
# commits. Date-mode releases query each label separately. Results are combined
# and deduplicated by PR number.
#
# Required env:
#   RANGE_MODE           - "git" for an exact tag range, or "date"
#   TAG                  - current release tag
#   AREA_LABELS          - comma-separated label names, e.g. "area: core,area: ci"
#   GITHUB_TOKEN         - GitHub auth token
#   GITHUB_REPOSITORY    - owner/repo
# Optional env:
#   BASE_TAG             - previous tag; empty includes all history through TAG
#   SINCE_DATE           - ISO-8601 lower bound used by date mode
#   GITHUB_API_BASE_URL  - defaults to https://api.github.com
#   OUTPUT_FILE          - path to write notes to (default: release_notes.md)
set -euo pipefail

RANGE_MODE="${RANGE_MODE:?RANGE_MODE env var is required}"
TAG="${TAG:?TAG env var is required}"
AREA_LABELS="${AREA_LABELS:?AREA_LABELS env var is required}"
GITHUB_TOKEN="${GITHUB_TOKEN:?GITHUB_TOKEN env var is required}"
GITHUB_REPOSITORY="${GITHUB_REPOSITORY:?GITHUB_REPOSITORY env var is required}"
BASE_TAG="${BASE_TAG:-}"
SINCE_DATE="${SINCE_DATE:-}"
GITHUB_API_BASE_URL="${GITHUB_API_BASE_URL:-https://api.github.com}"
OUTPUT_FILE="${OUTPUT_FILE:-release_notes.md}"

ALL_PRS="[]"

append_pr_batch() {
  local batch="$1"
  ALL_PRS=$(printf '%s\n%s\n' "$ALL_PRS" "$batch" \
    | jq -s 'add | unique_by(.number) | sort_by(.number) | reverse')
}

collect_prs_from_git_range() {
  local revision_range
  if [ -n "$BASE_TAG" ]; then
    git rev-parse --verify "${BASE_TAG}^{commit}" >/dev/null
    revision_range="${BASE_TAG}..${TAG}"
  else
    revision_range="${TAG}"
  fi
  git rev-parse --verify "${TAG}^{commit}" >/dev/null

  local allowed_labels
  allowed_labels=$(jq -cn --arg labels "$AREA_LABELS" \
    '$labels | split(",") | map(gsub("^[[:space:]]+|[[:space:]]+$"; ""))')

  echo "Collecting PRs associated with commits in ${revision_range}" >&2

  while IFS= read -r commit_sha; do
    [ -z "$commit_sha" ] && continue

    local response batch
    response=$(curl -sf \
      -H "Authorization: Bearer ${GITHUB_TOKEN}" \
      -H "Accept: application/vnd.github+json" \
      -H "X-GitHub-Api-Version: 2022-11-28" \
      "${GITHUB_API_BASE_URL}/repos/${GITHUB_REPOSITORY}/commits/${commit_sha}/pulls?per_page=100")

    batch=$(echo "$response" | jq --argjson allowed "$allowed_labels" '[
      .[]
      | select(.merged_at != null)
      | select(any(.labels[]?; .name as $name | $allowed | index($name)))
      | {number: .number, title: .title, url: .html_url, author: {login: .user.login}}
    ]')

    append_pr_batch "$batch"
  done < <(git rev-list --first-parent --reverse "$revision_range")

  return 0
}

collect_prs_since_date() {
  : "${SINCE_DATE:?SINCE_DATE env var is required for date mode}"

  local label
  IFS=',' read -ra label_array <<< "$AREA_LABELS"
  for label in "${label_array[@]}"; do
    label=$(echo "$label" | xargs)  # trim whitespace

    echo "Querying: label=\"${label}\" merged after ${SINCE_DATE}" >&2

    local next_url="${GITHUB_API_BASE_URL}/search/issues"
    local query_args=(-G \
      --data-urlencode "q=is:pr is:merged merged:>${SINCE_DATE} label:\"${label}\" repo:${GITHUB_REPOSITORY}" \
      --data-urlencode "per_page=100")
    local page=1

    while [ -n "$next_url" ]; do
      local response headers body batch
      response=$(curl -sf --include \
        -H "Authorization: Bearer ${GITHUB_TOKEN}" \
        -H "Accept: application/vnd.github.v3+json" \
        "${query_args[@]}" \
        "$next_url")

      # Split headers from body (headers end at the first blank line).
      headers=$(echo "$response" | sed '/^[[:space:]]*$/q')
      body=$(echo "$response" | sed '1,/^[[:space:]]*$/d')

      batch=$(echo "$body" \
        | jq '[.items[] | {number: .number, title: .title, url: .html_url, author: {login: .user.login}}]')

      append_pr_batch "$batch"

      # Extract the next-page URL from the Link header, if present.
      next_url=$(echo "$headers" \
        | grep -i '^link:' \
        | grep -o '<[^>]*>; rel="next"' \
        | sed 's/<\([^>]*\)>; rel="next"/\1/' \
        || true)

      # Only the first request needs query args; later pages use the Link URL.
      query_args=()
      page=$((page + 1))
      [ -n "$next_url" ] && echo "  Fetching page ${page} for label \"${label}\"..." >&2
    done
  done

  return 0
}

case "$RANGE_MODE" in
  git) collect_prs_from_git_range ;;
  date) collect_prs_since_date ;;
  *) echo "Unsupported RANGE_MODE: ${RANGE_MODE}" >&2; exit 1 ;;
esac

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
