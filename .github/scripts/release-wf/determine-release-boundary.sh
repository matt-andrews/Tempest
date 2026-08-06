#!/usr/bin/env bash
# Writes a GitHub Actions output-compatible release boundary to stdout.
#
# Versioned tags use an exact Git range so release notes contain only commits
# after the previous tag and through the current tag. Non-versioned tags retain
# the existing date-based behavior because a moving tag has no previous Git ref.
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
# The suffix is a variable-length regular expression, so parameter replacement
# is not equivalent here.
# shellcheck disable=SC2001
TAG_PREFIX=$(sed 's/[0-9][0-9.]*$//' <<< "$TAG")

if [ "$TAG_PREFIX" != "$TAG" ]; then
  PREV_TAG=$(git tag --list "${TAG_PREFIX}*" --sort=version:refname \
    | awk -v current="$TAG" '$0 == current { print previous; exit } { previous = $0 }')

  echo "range_mode=git"
  echo "base_tag=${PREV_TAG}"
  echo "since_date="

  if [ -n "$PREV_TAG" ]; then
    echo "Using exact Git range ${PREV_TAG}..${TAG}" >&2
  else
    echo "No previous tag found — including history reachable from ${TAG}" >&2
  fi
else
  # A non-versioned tag may be moved between releases, so retain the prior
  # release-created-at boundary for this mode.
  ENCODED_TAG=$(jq -rn --arg t "$TAG" '$t|@uri')
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

  echo "range_mode=date"
  echo "base_tag="
  echo "since_date=${SINCE_DATE}"
fi
