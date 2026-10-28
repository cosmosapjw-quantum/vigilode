"""Archive/source checks only. No new numerical run or changed scoring."""
import hashlib,json,re,subprocess,sys
from pathlib import Path
node=Path(__file__).resolve().parent;run=node/'runs/39355449893';out=Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=True)
parts=json.loads((run/'RECONSTRUCTION.json').read_text())
for r in parts:
 b=b''.join((run/p).read_bytes()for p in r['parts_in_order']);assert hashlib.sha256(b).hexdigest()==r['sha256']
 p=run/r['original']
 if p.exists():assert hashlib.sha256(p.read_bytes()).hexdigest()==r['sha256']
 else:p.write_bytes(b)
count=0
for line in(run/'SHA256SUMS').read_text().splitlines():
 want,p=line.split('  ',1);assert hashlib.sha256((run/p).read_bytes()).hexdigest()==want,p;count+=1
source=(run/'EXEC_SOURCE_COMMIT.txt').read_text().strip();assert source=='f08d152a35fb2db748692570969cb1723c287c26'
assert json.loads((run/'MANIFEST.json').read_text())['source_commit']==source
status=json.loads((run/'EXIT_STATUS.json').read_text());assert status=={'stage':'complete','exit_code':0}
sums=[]
for line in(run/'TESTS.log').read_text().splitlines():
 m=re.search(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored',line)
 if m:sums.append({'status':m[1],'passed':int(m[2]),'failed':int(m[3]),'ignored':int(m[4])})
assert sum(r['passed']for r in sums)==33 and sum(r['failed']for r in sums)==0
paths=['crates/vigilode-coupled/src','crates/vigilode-coupled/tests','crates/vigilode-coupled/examples/economy_bench.rs','crates/vigilode-coupled/Cargo.lock']
assert not subprocess.check_output(['git','diff','--name-only',source,'HEAD','--',*paths],text=True).strip()
parent='15c43ce76734d0a560fcec5909ac49f1e6f2d347'
assert not subprocess.check_output(['git','diff','--name-only',parent,'HEAD','--','crates/rodas5p-core','crates/rodas5p-integrators','crates/rodas5p-krylov','Cargo.toml','Cargo.lock'],text=True).strip()
audit=json.loads((run/'AUDIT.json').read_text());assert audit['status']=='PASS_SCOPED_READBACK'
summary=json.loads((run/'SUMMARY.json').read_text());assert summary['accuracy_rows']==80 and summary['timing_processes']==640
report={'status':'PASS_FROZEN_ARCHIVE_SOURCE_LINK','measured_source':source,'measurement_run':39355449893,
 'archive_files_verified':count,'split_files_reconstructed':len(parts),'tests':sums,'tests_passed':33,
 'numerical_source_unchanged_since_measurement':True,'upstream_libraries_unchanged':True,
 'saved_numerical_readback':audit,'new_solver_runs':0,'new_timing_samples':0,
 'whole_workspace_tested':False,'optimal_budget_claim':False}
(out/'VALIDATION.json').write_text(json.dumps(report,indent=2,sort_keys=True)+'\n');print(json.dumps(report))
