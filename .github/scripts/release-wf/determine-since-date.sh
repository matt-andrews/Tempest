#!/usr/bin/env bash
# Outputs the ISO-8601 date to use as the lower bound for PR queries.
#
# For versioned tags (e.g. app/v1.2.3): walks back to the previous tag in the
# same family and uses its committer date.
# For non-versioned tags (e.g. app/nightly): fetches the existing GitHub Release's
# created_at and falls back to 25 hours ago when none exists.
#
# Required env:
#   TAG                  - the release tag being published
#   GITHUB_TOKEN         - GitHub auth token
#   GITHUB_REPOSITORY    - owner/repo
# Optional env:
#   GITHUB_API_BASE_URL  - defaults to https://api.github.com
set -euo pipefail

TAG="${TAG:?TAG env var is required}"
GITHUB_TOKEN="${GITHUB_TOKEN:?GITHUB_TOKEN env var is required}"
GITHUB_REPOSITORY="${GITHUB_REPOSITORY:?GITHUB_REPOSITORY env var is required}"
GITHUB_API_BASE_URL="${GITHUB_API_BASE_URL:-https://api.github.com}"

# Strip trailing version numbers to get the tag family prefix.
# e.g. "app/v1.2.3" → "app/v",  "app/nightly" → "app/nightly"
TAG_PREFIX=$(echo "$TAG" | sed 's/[0-9][0-9.]*$//')
ENCODED_TAG=$(jq -rn --arg t "$TAG" '$t|@uri')

if [ "$TAG_PREFIX" != "$TAG" ]; then
  # Versioned tag — find the previous tag in the same family.
  PREV_TAG=$(git tag --list "${TAG_PREFIX}*" --sort=-version:refname \
    | grep -vFx "${TAG}" \
    | head -1 || true)

  if [ -n "$PREV_TAG" ]; then
    SINCE_DATE=$(git log -1 --format=%cI "$PREV_TAG")
    echo "Since date from previous tag ${PREV_TAG}: ${SINCE_DATE}" >&2
  else
    SINCE_DATE="1970-01-01T00:00:00Z"
    echo "No previous tag found — including full history" >&2
  fi
else
  # Non-versioned tag — anchor to the last release's creation date.
  RESPONSE=$(curl -sf \
    -H "Authorization: Bearer ${GITHUB_TOKEN}" \
    -H "Accept: application/vnd.github.v3+json" \
    "${GITHUB_API_BASE_URL}/repos/${GITHUB_REPOSITORY}/releases/tags/${ENCODED_TAG}" \
    2>/dev/null || echo "")

  SINCE_DATE=$(echo "$RESPONSE" | jq -r '.created_at // empty' 2>/dev/null || echo "")

  if [ -z "$SINCE_DATE" ]; then
    SINCE_DATE=$(date -u -d '25 hours ago' +%Y-%m-%dT%H:%M:%SZ)
    echo "No existing release found — falling back to 25 hours ago" >&2
  else
    echo "Since date from existing release: ${SINCE_DATE}" >&2
  fi
fi

echo "$SINCE_DATE"
