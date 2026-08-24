#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"

PYTHONDONTWRITEBYTECODE=1 \
  python3 -m unittest discover -s "$SCRIPT_DIR" -p 'test_*.py' -v

python3 - <<'PY' "$ROOT/IDENTITY_POLICY.json"
import json, pathlib, sys
json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
PY

printf '%s\n' 'PASS: Git/content-equivalence PM-4 Task-1 handoff preflight'
