from pathlib import Path
import hashlib, zipfile, json
ROOT=Path(__file__).resolve().parents[1]
STAGE=ROOT.parent
files=sorted(p for p in ROOT.rglob('*') if p.is_file() and not (p.name in ('MANIFEST.json','RESTORE_LOG.txt','ARCHIVE_RECEIPT.json','LATEST_DELIVERY_ACK.json') or '__pycache__' in p.parts))
manifest={'schema':'vigilode.bass21.actual-physical-atlas.manifest.v1','original_source_owner':'BASS@cc602e209e7deeb511ab2a1f3b2268d55206c43c','note':'All source/test/results/plots/checkpoints under new_physics. Original PR269 extracted source is sibling under rei_pr269 in same ZIP. GitHub original 127-endpoint CSV is referenced separately, not embedded.','files':[{'path':str(p.relative_to(ROOT)),'size':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in files]}
(ROOT/'MANIFEST.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n')
archive=Path('/mnt/data/BASS21_ACTUAL_ENDPOINT_PHYSICS_20261029_SOURCE_FULL.zip')
with zipfile.ZipFile(archive,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=7) as z:
 for p in sorted(files+[ROOT/'MANIFEST.json']):
  q='new_physics/'+str(p.relative_to(ROOT));info=zipfile.ZipInfo(q,(1980,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;info.external_attr=0o644 << 16;z.writestr(info,p.read_bytes(),compress_type=zipfile.ZIP_DEFLATED,compresslevel=7)
 for p in sorted((STAGE/'rei_pr269').rglob('*')):
  if p.is_file() and '__pycache__' not in p.parts:
   q='rei_pr269/'+str(p.relative_to(STAGE/'rei_pr269'));info=zipfile.ZipInfo(q,(1980,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;info.external_attr=0o644 << 16;z.writestr(info,p.read_bytes(),compress_type=zipfile.ZIP_DEFLATED,compresslevel=7)
with zipfile.ZipFile(archive) as z:
 bad=z.testzip();assert bad is None,bad; n=len(z.infolist())
receipt={'file':archive.name,'size':archive.stat().st_size,'sha256':hashlib.sha256(archive.read_bytes()).hexdigest(),'zip_members':n,'manifest_file_count':len(manifest['files']),'zip_crc':'PASS','uploaded':'NOT_YET','original_dag':'7/23','new_unit':'1/1 scoped','rc':'18/20 scoped, R6/C9 NOT_RUN','production':'HOLD'}
(ROOT/'ARCHIVE_RECEIPT.json').write_text(json.dumps(receipt,indent=2)+'\n');print(receipt)
