#!/usr/bin/env python3
"""Exact-byte archive verifier, not scientific proof or CI authority."""
import sys,json,hashlib
from pathlib import Path
r=Path(sys.argv[1]).resolve() if len(sys.argv)>1 else Path(__file__).resolve().parent
m=json.loads((r/'MANIFEST.json').read_text())
visited=set()
for name,expected in m['files'].items():
    p=(r/name).resolve()
    assert p.is_relative_to(r) and p.is_file() and not p.is_symlink(),f'unsafe/missing {name}'
    assert name not in visited;visited.add(name)
    digest=hashlib.sha256(p.read_bytes()).hexdigest()
    if digest!=expected:raise ValueError(f'file hash mismatch {name} {digest} != {expected}')
p=r/'rust/src/n3h_source_bounded_copy.rs'
assert hashlib.sha256(p.read_bytes()[:m['parent_n4a_source_prefix_bytes']]).hexdigest()==m['parent_n4a_source_prefix_sha256'],'wrong parent prefix'
assert hashlib.sha256((r/'rodas5p_coefficients_snapshot.json').read_bytes()).hexdigest()==m['rodas5p_tableau_sha256'],'original RODAS tableau wrong'
print('PASS N4B manifest',len(visited),'prefix bytes',m['parent_n4a_source_prefix_bytes'])
