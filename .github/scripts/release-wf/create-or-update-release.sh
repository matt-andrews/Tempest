#!/usr/bin/env bash
# Creates a new GitHub Release or edits the existing one for the given tag.
#
# Checks for an existing release first; if found, PATCHes it. Otherwise POSTs
# a new release. On success the API response is discarded — callers care only
# that the release exists, not about its returned JSON.
#
# Required env:
#   TAG                  - the release tag
#   NOTES_FILE           - path to a file containing the release body (markdown)
#   GITHUB_TOKEN         - GitHub auth token
#   GITHUB_REPOSITORY    - owner/repo
# Optional env:
#   RELEASE_NAME         - release title; defaults to TAG
#   MAKE_LATEST          - "true" | "false" (default: "false")
#   PRERELEASE           - "true" | "false" (default: "false")
#   GITHUB_API_BASE_URL  - defaults to https://api.github.com
set -euo pipefail

TAG="${TAG:?TAG env var is required}"
NOTES_FILE="${NOTES_FILE:?NOTES_FILE env var is required}"
GITHUB_TOKEN="${GITHUB_TOKEN:?GITHUB_TOKEN env var is required}"
GITHUB_REPOSITORY="${GITHUB_REPOSITORY:?GITHUB_REPOSITORY env var is required}"
RELEASE_NAME="${RELEASE_NAME:-$TAG}"
MAKE_LATEST="${MAKE_LATEST:-false}"
PRERELEASE="${PRERELEASE:-false}"
GITHUB_API_BASE_URL="${GITHUB_API_BASE_URL:-https://api.github.com}"

ENCODED_TAG=$(jq -rn --arg t "$TAG" '$t|@uri')

PRERELEASE_BOOL="false"
[ "$PRERELEASE" = "true" ] && PRERELEASE_BOOL="true"

# GitHub API accepts make_latest as a string ("true"/"false"), not a boolean.
MAKE_LATEST_STR="false"
[ "$MAKE_LATEST" = "true" ] && MAKE_LATEST_STR="true"

REQUEST_BODY=$(jq -n \
  --arg tag_name    "$TAG" \
  --arg name        "$RELEASE_NAME" \
  --rawfile body    "$NOTES_FILE" \
  --argjson prerelease "$PRERELEASE_BOOL" \
  --arg make_latest "$MAKE_LATEST_STR" \
  '{tag_name: $tag_name, name: $name, body: $body, prerelease: $prerelease, make_latest: $make_latest}')

# Check whether a release already exists for this tag.
EXISTING_JSON=$(curl -sf \
  -H "Authorization: Bearer ${GITHUB_TOKEN}" \
  -H "Accept: application/vnd.github.v3+json" \
  "${GITHUB_API_BASE_URL}/repos/${GITHUB_REPOSITORY}/releases/tags/${ENCODED_TAG}" \
  2>/dev/null || echo "")

RELEASE_ID=$(echo "$EXISTING_JSON" | jq -r '.id // empty' 2>/dev/null || echo "")

if [ -n "$RELEASE_ID" ]; then
  echo "Editing release ${RELEASE_ID} for tag ${TAG}" >&2
  curl -sf -X PATCH \
    -H "Authorization: Bearer ${GITHUB_TOKEN}" \
    -H "Content-Type: application/json" \
    -H "Accept: application/vnd.github.v3+json" \
    -d "$REQUEST_BODY" \
    "${GITHUB_API_BASE_URL}/repos/${GITHUB_REPOSITORY}/releases/${RELEASE_ID}" \
    > /dev/null || { echo "Failed to edit release ${RELEASE_ID} for tag ${TAG}" >&2; exit 1; }
else
  echo "Creating release for tag ${TAG}" >&2
  curl -sf -X POST \
    -H "Authorization: Bearer ${GITHUB_TOKEN}" \
    -H "Content-Type: application/json" \
    -H "Accept: application/vnd.github.v3+json" \
    -d "$REQUEST_BODY" \
    "${GITHUB_API_BASE_URL}/repos/${GITHUB_REPOSITORY}/releases" \
    > /dev/null || { echo "Failed to create release for tag ${TAG}" >&2; exit 1; }
fi
