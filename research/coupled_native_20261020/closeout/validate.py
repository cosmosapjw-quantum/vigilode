"""Freeze archive integrity, tested-source identities and acceptance semantics."""
import csv,gzip,hashlib,json,re,subprocess,sys
from pathlib import Path
node=Path(__file__).resolve().parents[1];run=node/'runs/39349031422';out=Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=True)
def save(n,x):(out/n).write_text(json.dumps(x,indent=2,sort_keys=True,allow_nan=False)+'\n')
reconstruction=json.loads((run/'RECONSTRUCTION.json').read_text())
for record in reconstruction:
 p=run/record['original']
 if not p.exists():
  b=b''.join((run/q).read_bytes()for q in record['parts_in_order']);assert hashlib.sha256(b).hexdigest()==record['sha256'];p.write_bytes(b)
checked=0
for line in (run/'SHA256SUMS').read_text().splitlines():
 want,path=line.split('  ',1);assert hashlib.sha256((run/path).read_bytes()).hexdigest()==want,path;checked+=1
source=(run/'EXEC_SOURCE_COMMIT.txt').read_text().strip()
assert source==json.loads((run/'MANIFEST.json').read_text())['source_commit']
changed=subprocess.check_output(['git','diff','--name-only','c03ca90f19a9e64ba7751c069325a48b54b39c2e','HEAD','--','crates/rodas5p-core','crates/rodas5p-integrators','crates/rodas5p-krylov','Cargo.toml','Cargo.lock'],text=True).splitlines()
assert not changed,changed
summaries=[]
for line in (run/'TESTS.log').read_text().splitlines():
 m=re.search(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored',line)
 if m:summaries.append({'status':m[1],'passed':int(m[2]),'failed':int(m[3]),'ignored':int(m[4])})
assert sum(s['passed']for s in summaries)==19 and not any(s['failed']for s in summaries)
extra=[];state_cache={}
with gzip.open(run/'accuracy.jsonl.gz','rt')as f:
 for line in f:
  r=json.loads(line);row=r['row'];h=row['state_object_sha256']
  with gzip.open(run/'states'/(h+'.json.gz'),'rb')as g:b=g.read()
  assert hashlib.sha256(b).hexdigest()==h
  result=json.loads(b).get('result')or{};attempt=result.get('attempt')or(result.get('result')or{}).get('candidate_attempt')
  candidate=(attempt or{}).get('candidate')
  physical=(candidate or{}).get('embedded_test_passes')
  embedded=(candidate or{}).get('embedded_wrms')
  if result.get('kind')=='sequential-native-k':physical=result.get('embedded_test_passes');embedded=result.get('error_norm')
  transaction=result.get('result')or{}
  if result.get('kind')=='transactional'and candidate is None:
   physical=(transaction.get('fallback')or{}).get('accepted_by_existing_driver');embedded=(transaction.get('fallback')or{}).get('embedded_wrms')
  w=(attempt or{}).get('work')or{}
  extra.append({'cell':row['cell_id'],'arm':row.get('arm')or row['command'][6],'root_completed':row.get('root_completed',False),
    'raw_reference_gate':r['metrics']['valid'],'embedded_test_passes':physical,'embedded_wrms':embedded,
    'candidate_basis':(candidate or{}).get('basis_dimension'),'lambda_rejections':w.get('lambda_rejections'),'basis_expansions':w.get('basis_expansions'),
    'fallback_used':transaction.get('fallback_attempted',False)})
with(out/'ROOT_VS_TIME_ERROR.csv').open('w',newline='')as f:w=csv.DictWriter(f,fieldnames=list(extra[0]));w.writeheader();w.writerows(extra)
save('VALIDATION.json',{'status':'PASS_SCOPED_ARCHIVE_AND_SOURCE','measured_source':source,'run_id':39349031422,
 'verified_archive_files':checked,'reconstructed_large_files':len(reconstruction),'tests':summaries,'total_tests_passed':19,
 'existing_numerical_crates_changed':False,'whole_repository_tests_run':False,
 'root_reference_admitted':sum(r['raw_reference_gate']for r in extra),
 'root_and_embedded_test_passes':sum(r['raw_reference_gate']and r['embedded_test_passes']is True for r in extra),
 'global_accuracy_certified':False,'new_solver_runs':0,'new_timing_samples':0})
print((out/'VALIDATION.json').read_text())
