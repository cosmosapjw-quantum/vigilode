from pathlib import Path
import hashlib,zipfile,json,sys
R=Path(__file__).resolve().parent
tag=sys.argv[1]
p=R/'delivery'/f'VIGILODE_D3_DENSE_{tag}_20261024_RESUME.zip'
assert not p.exists(), 'create-only stage archive'
files=[]
for f in sorted(R.iterdir()):
 if f.is_file() and f.suffix in ('.md','.json','.py','.wl'):files.append(f)
for sub in ['source','evidence','figures']:
 if (R/sub).exists():files.extend(f for f in sorted((R/sub).rglob('*')) if f.is_file() and f.suffix not in ('.rmeta','.pyc'))
for name in ['REUSED_SOURCE_IDENTITY.json','S0_CONTRACT.md','S0_PLAN.md','D2_BUDGET.json','D2_REVIEW.json','D2_RESULT.json']:
 f=R/'intake'/name
 if f.exists():files.append(f)
# Prior immutable stage receipts are payloads of later checkpoints.
# The current archive's own acknowledgement is necessarily written afterwards.
for f in sorted((R/'delivery').glob('*.json')):
 if f.name not in (tag+'_LOCAL_ARCHIVE.json',tag+'_PROVIDER_ACK.json'):files.append(f)
items=[]
with zipfile.ZipFile(p,'x',zipfile.ZIP_DEFLATED,compresslevel=6) as z:
 for f in files:
  b=f.read_bytes();n=f.relative_to(R).as_posix();z.writestr(n,b)
  items.append({'path':n,'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest(),'git_blob':hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()})
 z.writestr('ARCHIVE_MANIFEST.json',json.dumps(items,indent=2)+'\n')
with zipfile.ZipFile(p) as z:
 assert z.testzip() is None
 for item in items:assert hashlib.sha256(z.read(item['path'])).hexdigest()==item['sha256']
receipt={'stage':tag,'file':str(p),'bytes':p.stat().st_size,'sha256':hashlib.file_digest(p.open('rb'),'sha256').hexdigest(),'payloads':len(items),'crc_and_payload_hashes':'PASS'}
(R/'delivery'/f'{tag}_LOCAL_ARCHIVE.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps(receipt))
