#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
python - <<'PY'
import hashlib,zipfile,json
from pathlib import Path
root=Path('.')
manifest=json.loads((root/'replay'/'MANIFEST.json').read_text())
for row in manifest['files']:
    p=root/row['path']
    assert p.is_file(),p
    data=p.read_bytes()
    assert len(data)==row['size'] and hashlib.sha256(data).hexdigest()==row['sha256'],str(p)
for p in [root/'original'/'trajectory.npz',root/'original'/'PR118_SOURCE_FULL.zip',root/'original'/'ORIGINAL_BASS_PR234_SOURCE.zip']:
    assert zipfile.ZipFile(p).testzip() is None,p
print('MANIFEST PASS',len(manifest['files']),'ZIP3_CRC PASS')
PY
before=$(sha256sum replay/results/SUMMARY.json replay/results/NATIVE_BASS_193_EPOCH_HISTORY.csv replay/results/ACTUAL_FJ_ALL_193_EPOCHS.npz | sha256sum | cut -d' ' -f1)
export PYTHONDONTWRITEBYTECODE=1
python replay/code/replay.py --write > /dev/null
python -m unittest discover -s replay/tests -p 'test_*.py' -v
python replay/code/plot_history.py
after=$(sha256sum replay/results/SUMMARY.json replay/results/NATIVE_BASS_193_EPOCH_HISTORY.csv replay/results/ACTUAL_FJ_ALL_193_EPOCHS.npz | sha256sum | cut -d' ' -f1)
[[ "$before" == "$after" ]] || { echo 'SCIENTIFIC_REPLAY_BYTE_DRIFT'; exit 1; }
echo 'SCIENTIFIC_REPLAY_BYTE_IDENTITY_PASS'
