#!/usr/bin/env python3
"""Validate package manifest including original nested ZIP; no cryptographic invention."""
import pathlib,json,hashlib,zipfile
root=pathlib.Path(__file__).resolve().parents[1]
m=json.loads((root/'FILE_MANIFEST.json').read_text())
for name,h in m['sha256'].items():
 p=root/name
 assert p.is_file() and hashlib.sha256(p.read_bytes()).hexdigest()==h,('MANIFEST_MEMBER_FAIL',name)
assert zipfile.is_zipfile(root/'source'/'PR153_FULL_SOURCE.zip')
assert (root/'source'/'PR153_FULL_SOURCE.zip').stat().st_size==77035898
print('MANIFEST_PASS',len(m['sha256']))
