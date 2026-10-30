#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"
export OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 PYTHONDONTWRITEBYTECODE=1
python code/verify_manifest.py
python -m pytest -q tests/test_alignment.py
TD="$(mktemp -d)"
trap 'rm -rf "$TD"' EXIT
python code/compute_alignment.py --source source/PR151_ORIGINAL_SOURCE_FULL.zip --out "$TD/results" > "$TD/RESULT.json"
python - "$TD" <<'PY'
from pathlib import Path
import hashlib,sys
src=Path('.');temp=Path(sys.argv[1]); n=0
for parent in ('results','figures'):
 for f in sorted((src/parent).glob('*')):
  if not f.is_file():continue
  other=temp/f
  assert other.exists(),f'MISSING REPLAY {other}'
  assert hashlib.sha256(f.read_bytes()).digest()==hashlib.sha256(other.read_bytes()).digest(),f'BYTE_DIFF {f}'
  n+=1
assert n==7,(n,7)
print(f'SCIENCE_OUTPUT_REPLAY={n}/7_BYTE_EXACT')
PY
