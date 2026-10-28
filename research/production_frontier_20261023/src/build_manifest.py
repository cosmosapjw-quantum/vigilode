#!/usr/bin/env python3
"""Regenerate byte inventory after an intentional package change, not solver evidence."""
from pathlib import Path
import hashlib
import json
import subprocess

BASE = Path(__file__).resolve().parents[1]
REPO = BASE.parents[1]
SOURCE_BASE = '9b210c76dffb8909902f733e80d6cfa32405006e'
SOURCE_FINAL = '50d3052843d7bbce75bba55a104725c4cfcf6292'
# Remote checkouts need not contain local metadata-only equivalent commits.
SOURCE_FINAL_REMOTE = '37817308c12384f33ecb9b8ff64509f971eb3b2b'

def main():
    final = SOURCE_FINAL
    if subprocess.run(['git','cat-file','-e',final+'^{commit}'],cwd=REPO,
                      stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode:
        final = SOURCE_FINAL_REMOTE
    changed = subprocess.check_output(['git','diff','--name-only',SOURCE_BASE,final],cwd=REPO,text=True).splitlines()
    native = {REPO/p for p in changed if p.startswith('crates/')}
    package = {p for p in BASE.rglob('*') if p.is_file() and '__pycache__' not in p.parts
               and p.suffix != '.pyc' and p.name != 'ARTIFACT_MANIFEST.json'}
    records = []
    for path in sorted(native | package):
        assert not path.is_symlink()
        data = path.read_bytes()
        records.append({'path':str(path.relative_to(REPO)),'bytes':len(data),
                        'sha256':hashlib.sha256(data).hexdigest(),
                        'role':'changed_native_source' if path in native else 'research_package'})
    manifest = {'schema':'vigilode.frontier.artifact-manifest.v1','path_root':'repository',
                'coverage':'all research package files except this manifest, Python caches; plus native source files changed since source_base',
                'source_base':SOURCE_BASE,'source_mapping':'PUBLISH_MAP.json',
                'file_count':len(records),'files':records}
    (BASE/'ARTIFACT_MANIFEST.json').write_text(json.dumps(manifest,indent=2)+'\n')
    print(json.dumps({'manifest_files':len(records),'package_files':len(package),'native_files':len(native)}))

if __name__ == '__main__':
    main()
