#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
python - <<'PY'
import pathlib,hashlib,json
p=pathlib.Path('.')
m=json.loads((p/'MANIFEST.json').read_text())
for item in m['files']:
 raw=(p/item['path']).read_bytes()
 if len(raw)!=item['bytes'] or hashlib.sha256(raw).hexdigest()!=item['sha256']:
  raise SystemExit('INPUT_MANIFEST_SHA_FAILURE:'+item['path'])
print('MANIFEST_PASS',len(m['files']))
PY
python code/endpoint_atlas.py --write > results/CLEAN_REPLAY_STDOUT.json
python -m unittest discover -s tests -p 'test_*.py' -v
python code/plot_physics.py
python - <<'PY'
import pathlib,json,hashlib
root=pathlib.Path('.')
expected=json.loads((root/'RESULT_OUTPUT_HASHES.json').read_text())
for rel,sha in expected.items():
 if hashlib.sha256((root/rel).read_bytes()).hexdigest()!=sha:
  raise SystemExit('RESULT_REPRO_HASH_FAIL:'+rel)
print('GENERATED_SCIENCE_BYTES_PASS',len(expected))
PY
