#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
export PYTHONDONTWRITEBYTECODE=1
export OPENBLAS_NUM_THREADS=1
export OMP_NUM_THREADS=1
python -m compileall -q code tests
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
python code/lrs_polarization_replay.py --archive source/VIGILODE_BASS21_FULL25_NATIVE193_20261030_SOURCE_FULL.zip --out "$TMP" > "$TMP/EXECUTION.json"
python - "$TMP" <<'PY'
import json,numpy as np,sys,pathlib
old=pathlib.Path('results');new=pathlib.Path(sys.argv[1]);
a=np.load(old/'LRS_50x193x17_POLARIZATION.npz');b=np.load(new/'LRS_50x193x17_POLARIZATION.npz')
for key in a.files:
    assert np.array_equal(a[key],b[key]),('OUTPUT_NUMERICAL_IDENTITY_DRIFT',key)
for csv in ('FINAL_50_DIRECTIONAL_POLARIZATION.csv','MATCHED_25_DIRECTIONAL_POLARIZATION.csv','MATCHED_25_SHAPLEY_3BLOCK.csv','SCIENCE_SUMMARY.json'):
    assert (old/csv).read_bytes()==(new/csv).read_bytes(),('OUTPUT_CONTENT_DRIFT',csv)
print('REGENERATED_PHYSICS_NUMERICAL_AND_TEXT_CONTENT_PASS')
PY
BASS21_ORIGINAL_ARCHIVE="$PWD/source/VIGILODE_BASS21_FULL25_NATIVE193_20261030_SOURCE_FULL.zip" python -m unittest discover -s tests -v
python code/refinement_checks.py source/VIGILODE_BASS21_FULL25_NATIVE193_20261030_SOURCE_FULL.zip "$TMP" > /dev/null
python - "$TMP" <<'PY'
from pathlib import Path
import json,sys
p=Path(sys.argv[1])
a=json.loads(Path('results/REFINEMENT_SUMMARY.json').read_text())
b=json.loads((p/'REFINEMENT_SUMMARY.json').read_text())
assert a==b
print('FROZEN_GRID_REFINEMENT_PASS')
PY
python - <<'PY'
from pathlib import Path
import hashlib,json
m=json.loads(Path('FILE_MANIFEST.json').read_text());root=Path('.')
for k,expected in m['files'].items():
    assert hashlib.sha256((root/k).read_bytes()).hexdigest()==expected,('ARTIFACT_CONTENT_DRIFT',k)
print('FULL_PAYLOAD_SHA256_PASS',len(m['files']))
PY
echo 'ALL_SCOPED_VERIFICATION_PASSED'