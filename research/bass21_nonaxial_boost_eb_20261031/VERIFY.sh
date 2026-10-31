#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
export OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 MPLBACKEND=Agg PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$PWD/code"
python3 code/verify_manifest.py
python3 -m pytest -q -p no:cacheprovider tests/test_nonaxial.py
python3 code/check_sourcefree.py
python3 - <<'PY'
import hashlib,json,pathlib
p=pathlib.Path('.')
w=json.loads((p/'OUTPUT_SHA256.json').read_text())
def H(path):return hashlib.sha256(path.read_bytes()).hexdigest()
assert {k:H(p/k) for k in w['sha256']}==w['sha256'],'INPUT_OUTPUT_HASH_MISMATCH'
print('ORIGINAL_OUTPUT_SHA256_BEFORE_PASS',len(w['sha256']))
PY
python3 code/run_science.py > /dev/null
python3 - <<'PY'
import hashlib,json,pathlib
p=pathlib.Path('.')
w=json.loads((p/'OUTPUT_SHA256.json').read_text())
def H(path):return hashlib.sha256(path.read_bytes()).hexdigest()
assert {k:H(p/k) for k in w['sha256']}==w['sha256'],'OUTPUT_REPLAY_HASH_MISMATCH'
print('ORIGINAL_OUTPUT_BYTE_REPLAY_PASS',len(w['sha256']))
PY
