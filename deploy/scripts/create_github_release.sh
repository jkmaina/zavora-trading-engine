#!/bin/bash
set -e

# Script to create GitHub releases from RELEASE_NOTES.md
# Usage: ./create_github_release.sh <tag> <title>
# Example: ./create_github_release.sh v0.1.1 "Cloud Deployment Update"

if [ $# -lt 2 ]; then
  echo "Usage: $0 <tag> <title>"
  echo "Example: $0 v0.1.1 \"Cloud Deployment Update\""
  exit 1
fi

TAG=$1
TITLE=$2
NOTES_FILE="../../RELEASE_NOTES.md"
TEMP_NOTES="/tmp/release_notes_${TAG}.md"

# Current directory should be the script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

if [ ! -f "${NOTES_FILE}" ]; then
  echo "Error: Release notes file not found at ${NOTES_FILE}"
  exit 1
fi

# Extract the section for this version from RELEASE_NOTES.md
SECTION_START=$(grep -n "^## ${TAG}" "${NOTES_FILE}" | cut -d':' -f1)

if [ -z "${SECTION_START}" ]; then
  echo "Error: Section for ${TAG} not found in ${NOTES_FILE}"
  exit 1
fi

NEXT_SECTION=$(tail -n +$((SECTION_START + 1)) "${NOTES_FILE}" | grep -n "^## " | head -1 | cut -d':' -f1)

if [ -z "${NEXT_SECTION}" ]; then
  # If this is the last section, take everything until the end
  tail -n +${SECTION_START} "${NOTES_FILE}" > "${TEMP_NOTES}"
else
  # Otherwise, take lines until the next section
  head -n $((SECTION_START + NEXT_SECTION - 1)) "${NOTES_FILE}" | tail -n +${SECTION_START} > "${TEMP_NOTES}"
fi

# Create the GitHub release
echo "Creating GitHub release for ${TAG}..."

# Use git to create release
git push origin ${TAG}

# Create the release on GitHub with the extracted notes
if command -v gh > /dev/null 2>&1; then
  gh release create "${TAG}" \
    --title "${TITLE}" \
    --notes-file "${TEMP_NOTES}"
else
  echo "GitHub CLI not installed. Please install it or create the release manually."
  echo "Release notes have been saved to ${TEMP_NOTES}"
  exit 1
fi

echo "GitHub release for ${TAG} created successfully!"
rm "${TEMP_NOTES}"