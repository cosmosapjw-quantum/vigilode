#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ARCHIVE="VIGILODE_PM4_TASK1_CWD_SURGICAL_HANDOFF_20260824.tar.gz"
SIDECAR="$ARCHIVE.sha256"

cd "$SCRIPT_DIR"
rm -f "$ARCHIVE" "$ARCHIVE.b64"

for part in $(seq -w 0 22); do
  file="$ARCHIVE.b64.part-$part"
  [[ -f "$file" ]] || {
    printf 'STOP: missing transport part: %s\n' "$file" >&2
    exit 1
  }
  cat "$file" >> "$ARCHIVE.b64"
done

base64 -d "$ARCHIVE.b64" > "$ARCHIVE"
rm -f "$ARCHIVE.b64"
sha256sum -c "$SIDECAR"

tar -tzf "$ARCHIVE" >/dev/null
printf 'PASS: reconstructed %s\n' "$SCRIPT_DIR/$ARCHIVE"
