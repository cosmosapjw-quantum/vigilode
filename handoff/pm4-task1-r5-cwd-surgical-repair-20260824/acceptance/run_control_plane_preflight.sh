#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
HANDOFF_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"

: "${PM4_R4_ARCHIVE:?set PM4_R4_ARCHIVE to the absolute R4 archive path}"
: "${PM4_R4_SIDECAR:?set PM4_R4_SIDECAR to the absolute R4 sidecar path}"

python3 "$SCRIPT_DIR/test_archive_authority_contract.py" \
  --archive "$PM4_R4_ARCHIVE" \
  --sidecar "$PM4_R4_SIDECAR"

python3 -m unittest discover \
  -s "$SCRIPT_DIR" \
  -p 'test_load_bearing_command_contract.py' \
  -v

printf '%s\n' "PASS: CWD-independent PM-4 control-plane preflight at $HANDOFF_ROOT"
