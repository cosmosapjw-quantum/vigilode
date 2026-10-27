#!/usr/bin/env python3
"""Deterministically package large raw evidence without deleting the originals."""
from pathlib import Path
import hashlib, json, tarfile

HERE=Path(__file__).resolve().parent
NODE=HERE.parent
REPO=HERE.parents[2]
fixed={
 'causal/WORK_RESULTS.json','causal/TIMING_RESULTS.json',
 'homotopy/RESULTS.json','homotopy/initial_zero_term_RESULTS.json',
 'homotopy/EXACT_CONSTANTS_RESULTS.json',
 'adapter/RESULTS.json','adapter/REPAIRED_RESULTS.json','adapter/PARITY_RESULTS.json','adapter/EXACT_WORK_RESULTS.json',
 'evidence/structural_n64_t1.json','evidence/structural_n2048_t1.json','evidence/structural_n2048_t2.json',
 'evidence/EXACT_STRUCTURAL_DIAGNOSTIC.json','evidence/report_text.txt',
}
files=[]
for p in NODE.rglob('*'):
 if not p.is_file():continue
 rel=p.relative_to(NODE).as_posix()
 if rel.startswith('report/build/') or '__pycache__' in p.parts:continue
 if p.suffix=='.log' or rel in fixed or rel.startswith(('evidence/measured_sources/','homotopy/control_source/','report/tex_support/')) or (rel.startswith('adapter/') and rel.endswith('_executed.rs')):
  files.append(p)
files.sort(key=lambda p:p.as_posix())
archive=HERE/'raw_measurements.tar.xz'
entries=[]
with tarfile.open(archive,'w:xz',preset=6) as tar:
 for p in files:
  rel=p.relative_to(REPO).as_posix()
  info=tar.gettarinfo(str(p),arcname=rel)
  info.uid=info.gid=0;info.uname=info.gname='';info.mtime=0
  with p.open('rb')as f:tar.addfile(info,f)
  entries.append({'path':rel,'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
with tarfile.open(archive,'r:xz')as tar:
 for entry in entries:
  content=tar.extractfile(entry['path']).read()
  assert len(content)==entry['bytes'] and hashlib.sha256(content).hexdigest()==entry['sha256']
manifest={'archive':str(archive.relative_to(REPO)),'sha256':hashlib.sha256(archive.read_bytes()).hexdigest(),
 'bytes':archive.stat().st_size,'members':entries,'member_count':len(entries),'all_member_hashes_verified':True}
(HERE/'RAW_MANIFEST.json').write_text(json.dumps(manifest,indent=2)+'\n')
patterns=['# Raw rows/snapshots are preserved in evidence/raw_measurements.tar.xz.',
          '# Originals remain locally available; extract into repo root to restore.','report/build/','*.log']
patterns+=sorted(fixed)
patterns+=['evidence/measured_sources/','homotopy/control_source/','adapter/*_executed.rs','report/tex_support/']
(NODE/'.gitignore').write_text('\n'.join(patterns)+'\n')
print(json.dumps({k:v for k,v in manifest.items()if k!='members'}))
