#!/usr/bin/env sh
set -eu

REPO_ZIP_URL="${FERRISGRID_SKILLS_ZIP_URL:-https://github.com/BrunoV21/FerrisGrid-CLI/archive/refs/heads/main.zip}"
TARGET_DIR="${1:-$PWD}"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: required command not found: $1" >&2
    exit 1
  fi
}

require_command curl
require_command unzip
require_command find
require_command sed

TMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

ZIP_FILE="$TMP_DIR/ferrisgrid.zip"
EXTRACT_DIR="$TMP_DIR/extract"
mkdir -p "$EXTRACT_DIR" "$TARGET_DIR"

echo "Downloading FerrisGrid skills from $REPO_ZIP_URL"
curl -fsSL "$REPO_ZIP_URL" -o "$ZIP_FILE"

unzip -q "$ZIP_FILE" '*/.agents/skills/*' -d "$EXTRACT_DIR"

SKILLS_DIR="$(find "$EXTRACT_DIR" -type d -path '*/.agents/skills' -print | sed -n '1p')"
if [ -z "$SKILLS_DIR" ]; then
  echo "error: .agents/skills was not found in the downloaded archive" >&2
  exit 1
fi

cp -R "$SKILLS_DIR"/. "$TARGET_DIR"/

echo "Installed FerrisGrid skills into $TARGET_DIR"
