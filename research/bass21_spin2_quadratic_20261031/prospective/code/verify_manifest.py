from pathlib import Path
import hashlib,json,zipfile
root=Path(__file__).resolve().parents[1]
manifest=json.loads((root/'FILE_MANIFEST.json').read_text())
assert manifest['schema']=='vigilode.bass21.alignment_prospective_manifest_v1'
assert len(manifest['files'])>=14
for rel,expected in manifest['files'].items():
    p=root/rel
    assert p.exists(),rel
    with p.open('rb') as f:found=hashlib.file_digest(f,'sha256').hexdigest()
    assert found==expected,rel
print('SOURCE_MANIFEST=%d/%d_PASS'%(len(manifest['files']),len(manifest['files'])))
