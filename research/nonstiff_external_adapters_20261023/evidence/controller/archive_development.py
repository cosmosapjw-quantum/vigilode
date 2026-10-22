from pathlib import Path
import tarfile,hashlib,json
r=Path(__file__).resolve().parent
files=[]
for d,skip in [('diffsol-development','target'),('julia-development','julia-depot'),('diffsol-metadata-preparation',None),('independent-review',None)]:
 for p in (r/d).rglob('*'):
  if p.is_file() and (skip is None or skip not in p.relative_to(r/d).parts):files.append(p)
for p in r.iterdir():
 if p.is_file() and (p.name.startswith(('reviewer-repair','runner-validator','staged-','SOURCE_WHITESPACE','BASELINE_BUNDLE','BASE_IDENTITY','ADAPTER_SOURCE')) or p.name in ['test_reviewer_repairs.py','execute_frozen_adapters.py']):files.append(p)
files=sorted(set(files));manifest=[]
with tarfile.open(r/'DEVELOPMENT_EVIDENCE.tar.gz','w:gz') as tar:
 for p in files:
  tar.add(p,arcname=str(p.relative_to(r)),recursive=False);manifest.append({'path':str(p.relative_to(r)),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
(r/'DEVELOPMENT_MANIFEST.json').write_text(json.dumps({'files':manifest,'archive_sha256':hashlib.sha256((r/'DEVELOPMENT_EVIDENCE.tar.gz').read_bytes()).hexdigest(),'note':'Development/install/first-failure evidence, not timed campaign. No target/depot caches or runtime executables.'},indent=2)+'\n')
print(len(files),(r/'DEVELOPMENT_EVIDENCE.tar.gz').stat().st_size)
