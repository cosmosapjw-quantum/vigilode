#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
python code/verify_integrity.py
if [[ ! -f work/original/trajectory.npz ]]; then
  mkdir -p work
  python -m zipfile -e source/BASS21_PR121_PARENT_FULL.zip work
fi
python -m unittest discover -s tests -p 'test_paired_replay.py' -v
python code/paired_replay.py --write >/dev/null
python code/plot_data.py >/dev/null
python - <<'PY'
import hashlib,json,pathlib
R=pathlib.Path.cwd()
x=json.loads((R/'RESULT_OUTPUT_HASHES.json').read_text())
for path,expected in x.items():
    actual=hashlib.sha256((R/path).read_bytes()).hexdigest()
    if actual!=expected: raise AssertionError((path,actual,expected))
print('POST_REPLAY_RESULT_BYTE_IDENTITY_PASS',len(x))
PY
printf 'VERIFY PASS: 15/15 source-owned matched-pair tests, original ten NPZ SHA and derived outputs; no IVP\n'
