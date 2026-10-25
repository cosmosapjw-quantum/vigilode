#!/usr/bin/env python3
"""One immutable scoped checkpoint, with independent payload/hash/CRC readback."""
import argparse,hashlib,json,zipfile
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('phase',choices=['S0','S1','S2','S3','FINAL']);p.add_argument('--revision',default='');a=p.parse_args()
root=Path(__file__).resolve().parent
if a.phase=='S0':
 names=['CONTRACT.md','PREREGISTRATION.md','SOURCE_SURVEY_KO.md','RELATED_REPOS_KO.md','RELATED_ADDITIONAL_KO.md','BACKUP_SURVEY_KO.md','LITERATURE_KO.md','STAGE_LEDGER.json','DAG.json','BUDGET.json','SOURCE_IDENTITY.json','evidence/NATIVE_SOURCE_PINS.json','evidence/AGENTS_READBACK.json','evidence/LITERATURE_INTAKE.json','evidence/D4_RESTORE_RECEIPT.json','evidence/REUSED_CR_PATTERN_PINS.json','delivery/S0_PUBLICATION.json']
 files=[root/n for n in names]
else:
 files=sorted(f for f in root.rglob('*') if f.is_file() and not any(x in f.relative_to(root).parts for x in ('bin','toolchain','__pycache__','archives','private')) and f.name!='MANIFEST.json' and not f.name.endswith(('.zip','.tar.xz')))
manifest={'phase':a.phase,'files':[{'path':str(f.relative_to(root)),'bytes':f.stat().st_size,'sha256':hashlib.sha256(f.read_bytes()).hexdigest()} for f in files]}
dest=root.parent/'archives';dest.mkdir(exist_ok=True)
suffix=('_'+a.revision) if a.revision else ''
out=dest/f'VIGILODE_D5_CALLBACK_{a.phase}_20261025{suffix}.zip'
with zipfile.ZipFile(out,'x',compression=zipfile.ZIP_DEFLATED,compresslevel=6) as z:
 for f in files:z.write(f,str(f.relative_to(root)))
 z.writestr('MANIFEST.json',json.dumps(manifest,ensure_ascii=False,indent=2)+'\n')
with zipfile.ZipFile(out) as z:
 assert z.testzip() is None
 for f in manifest['files']:
  b=z.read(f['path']);assert len(b)==f['bytes'] and hashlib.sha256(b).hexdigest()==f['sha256']
receipt={'phase':a.phase,'path':str(out),'filename':out.name,'bytes':out.stat().st_size,'sha256':hashlib.sha256(out.read_bytes()).hexdigest(),'payloads':len(files),'local_crc_all_payload_hash':'PASS','remote_whole_byte_restore':'NOT_RUN'}
(root/'delivery'/f'{a.phase}_ARCHIVE.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps(receipt))
