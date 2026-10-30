#!/usr/bin/env python3
"""Reproducible SHA-256 audit of frozen N4 source/evidence. Not scientific proof."""
from __future__ import annotations
import hashlib, json, sys
from pathlib import Path

ROOT = Path(sys.argv[1]).resolve() if len(sys.argv)>1 else Path(__file__).resolve().parent
manifest = json.loads((ROOT/'MANIFEST.json').read_text(encoding='utf-8'))
count = 0
for rel, facts in manifest['files'].items():
    p = Path(rel)
    if p.is_absolute() or any(part in ('.','..') for part in p.parts):
        raise RuntimeError('unsafe path: '+rel)
    f = ROOT / p
    if not f.is_file() or f.is_symlink():
        raise RuntimeError('missing/symlink: '+rel)
    raw = f.read_bytes()
    if len(raw) != facts['bytes'] or hashlib.sha256(raw).hexdigest() != facts['sha256']:
        raise RuntimeError('source/hash mismatch: '+rel)
    count+=1
parent = ROOT/'rust/src/n3h_source_bounded_copy.rs'
prefix = parent.read_bytes()[:37026]
if hashlib.sha256(prefix).hexdigest() != '823f06f3e94f94dac75accb0d9f22e25469b56ec0422a8cce72ba13fc5eebecb':
    raise RuntimeError('N3H original prefix is not source-identical')
original_tableau = ROOT/'fixtures/rodas5p_coefficients_snapshot.json'
assert hashlib.sha256(original_tableau.read_bytes()).hexdigest() == '211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6'
print(f'VERIFIED_N4_MANIFEST={count}/{len(manifest["files"])} SHA256 PASS')
print('VERIFIED_ORIGINAL_N3H_SOURCE_PREFIX=37026_BYTE_IDENTITY_PASS')
print('VERIFIED_RODAS5P_TABLEAU_SHA256=PASS')
