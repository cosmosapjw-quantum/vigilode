#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
export OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 PYTHONDONTWRITEBYTECODE=1
python3 - <<'PY'
from pathlib import Path
import hashlib
root=Path('.')
for row in (root/'SHA256SUMS.txt').read_text().splitlines():
 if not row.strip():continue
 h,n=row.split('  ',1)
 b=Path(n).read_bytes()
 assert hashlib.sha256(b).hexdigest()==h,(n,'byte mismatch')
print('IMMUTABLE_FILE_SHA_CHECK=PASS')
PY
python3 -m unittest discover -s tests -v
mkdir -p _clean_replay/results _clean_replay/figures
python3 code/compute_boundary_memory.py --source source/PR145_CORRECTED_FULL_SOURCE.zip --output _clean_replay/results > _clean_replay/RUN.log
python3 - <<'PY'
import hashlib,pathlib
src=pathlib.Path('results');rep=pathlib.Path('_clean_replay/results')
fig=pathlib.Path('figures');rfig=pathlib.Path('_clean_replay/figures')
files=list(src.iterdir())+list(fig.iterdir())
for p in files:
 other=rep/p.name if p.parent.name=='results' else rfig/p.name
 assert other.exists()
 assert hashlib.sha256(p.read_bytes()).digest()==hashlib.sha256(other.read_bytes()).digest(),p
print('REPLAY_ALL_OUTPUTS=%d/%d_BYTE_EXACT'%(len(files),len(files)))
PY
