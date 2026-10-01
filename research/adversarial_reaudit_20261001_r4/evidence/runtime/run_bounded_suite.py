#!/usr/bin/env python3
"""Source-bound bounded native regression with complete-stream evidence and named omissions."""
from pathlib import Path
import argparse,subprocess,json,time,datetime,os,signal,re,hashlib,collections
ap=argparse.ArgumentParser();ap.add_argument('--repo',required=True);ap.add_argument('--target-dir',required=True);ap.add_argument('--evidence-dir',required=True);ap.add_argument('--build-lock',required=True);ap.add_argument('--plan',required=True);ap.add_argument('--prereg-commit',required=True);a=ap.parse_args()
repo=Path(a.repo).resolve();out=Path(a.evidence_dir).resolve();out.mkdir(parents=True,exist_ok=True);plan=json.loads(Path(a.plan).read_text())
# A caller must provide the actual committed preregistration identity; no dates inferred.
subprocess.run(['git','cat-file','-e',a.prereg_commit+'^{commit}'],cwd=repo,check=True)
os.environ.update(CARGO_INCREMENTAL='0',CARGO_PROFILE_DEV_CODEGEN_UNITS='1',CARGO_PROFILE_TEST_CODEGEN_UNITS='1',CARGO_BUILD_JOBS='1')
def sha(p):
 with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def identity():
 paths=subprocess.check_output(['git','ls-files','crates','Cargo.toml','Cargo.lock','rust-toolchain.toml'],cwd=repo,text=True).splitlines()
 return {p:sha(repo/p) for p in paths}
def run(argv,cwd,stem,budget):
 t=time.monotonic();p=subprocess.Popen(list(map(str,argv)),cwd=cwd,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True);timeout=False
 try:so,se=p.communicate(timeout=max(.01,budget));rc=p.returncode
 except subprocess.TimeoutExpired:
  timeout=True;os.killpg(p.pid,signal.SIGTERM)
  try:so,se=p.communicate(timeout=2)
  except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);so,se=p.communicate()
  rc=124
 (out/(stem+'.stdout')).write_bytes(so);(out/(stem+'.stderr')).write_bytes(se)
 return {'argv':list(map(str,argv)),'cwd':str(cwd),'exit_code':rc,'timed_out':timeout,'budget_seconds':budget,'elapsed_seconds':time.monotonic()-t,'stdout':stem+'.stdout','stderr':stem+'.stderr','stdout_bytes':len(so),'stderr_bytes':len(se)},so.decode(errors='replace')
r={'schema':'vigilode.r4.native_campaign.v1','started_utc':datetime.datetime.now(datetime.UTC).isoformat(),'audited_source_commit':plan['source_commit'],'execution_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip(),'prereg_commit':a.prereg_commit,'profile':{'name':'debug','incremental':False,'codegen_units':1,'jobs':1,'test_threads':1,'features':'all','targets':'all'},'source_identity_before':identity(),'full_suite_claim':'NOT_COMPLETE','doctests':'NOT_RUN','ignored_policy':'NOT_RUN','binaries':[]}
def save(): (out/'suite_summary.json').write_text(json.dumps(r,indent=2)+'\n')
cmd=['flock',a.build_lock]+plan['build_argv']+['--target-dir',a.target_dir]
r['build'],txt=run(cmd,repo,'build',plan['build_budget_seconds']);save()
if r['build']['exit_code']!=0:
 r['full_suite_claim']='BUILD_TIMEOUT' if r['build']['timed_out'] else 'BUILD_FAILED';r['source_identity_after']=identity();r['source_unchanged']=r['source_identity_after']==r['source_identity_before'];save();print(json.dumps({'full_suite_claim':r['full_suite_claim'],'build':r['build']}),flush=True);raise SystemExit(0)
arts=[];seen=set();finished=[]
for ln in txt.splitlines():
 try:x=json.loads(ln)
 except json.JSONDecodeError:continue
 if x.get('reason')=='build-finished':finished.append(x)
 if x.get('reason')=='compiler-artifact' and x.get('executable') and x.get('profile',{}).get('test') and x['executable'] not in seen:
  seen.add(x['executable']);arts.append(x)
r['build_finished_records']=finished
if not arts or not finished or not finished[-1].get('success'):
 r['full_suite_claim']='BUILD_EVIDENCE_INCOMPLETE';r['artifact_count']=len(arts);save();print(r['full_suite_claim'],flush=True);raise SystemExit(0)
changed=set(plan['changed_priority_targets']+plan['additional_priority_targets'])
slow={'a1_two_by_six_spread_contracts','g4_s5b0_regime_atlas_contracts','givens_production_differential_contracts','homotopy_policy_contracts','inner_forcing_fixed_step_ladder_contracts','unified_candidate_contracts'}
arts.sort(key=lambda x:(0 if x['target']['name'] in changed else 2 if x['target']['name'] in slow or x['target']['name'].startswith('a1_') else 1,x['target']['name']))
start=time.monotonic()
for i,x in enumerate(arts):
 name=x['target']['name'];cwd=Path(x['manifest_path']).parent;stem=f'{i:03d}_{name}'
 b={'target':name,'kind':x['target']['kind'],'executable':x['executable'],'manifest_path':x['manifest_path'],'package_id':x['package_id'],'tests':[],'stem':stem}
 b['listing'],listing=run([x['executable'],'--list'],cwd,stem+'.list',10)
 names=[ln[:-6] for ln in listing.splitlines() if ln.endswith(': test')]
 ls=re.search(r'(\d+) tests?, (\d+) benchmarks?',listing)
 b['listed_count']=len(names);b['tests']=[{'name':n,'status':'NOT_RUN_TOTAL_BUDGET'} for n in names]
 if b['listing']['exit_code']!=0 or ls is None or int(ls[1])!=len(names):b['status']='LISTING_INCOMPLETE'
 else:b['status']='LISTED_NOT_RUN'
 r['binaries'].append(b);save()
for b in r['binaries']:
 if b['status']!='LISTED_NOT_RUN':continue
 remaining=plan['native_total_budget_seconds']-(time.monotonic()-start)
 if remaining<=0:b['status']='NOT_RUN_TOTAL_BUDGET';continue
 name=b['target'];budget=min(plan['historical_slow_harness_budget_seconds'] if name in slow or name.startswith('a1_') else plan['ordinary_harness_budget_seconds'],remaining)
 b['invocation'],txt=run([b['executable'],'--test-threads=1'],Path(b['manifest_path']).parent,b['stem']+'.run',budget)
 statuses={n:s for n,s in re.findall(r'^test (.*?) \.\.\. (ok|FAILED|ignored)\b',txt,re.MULTILINE)}
 for t in b['tests']:t['status']={'ok':'PASS','FAILED':'FAIL','ignored':'IGNORED_NOT_RUN'}.get(statuses.get(t['name']),'UNRESOLVED_BOUNDED_TIMEOUT' if b['invocation']['timed_out'] else 'UNRESOLVED_MISSING_RESULT')
 sm=re.search(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out',txt)
 b['summary_consistent']=False
 if sm:
  b['rust_summary']={'status':sm[1],'passed':int(sm[2]),'failed':int(sm[3]),'ignored':int(sm[4]),'measured':int(sm[5]),'filtered_out':int(sm[6])}
  c=collections.Counter(t['status'] for t in b['tests']);b['summary_consistent']=c['PASS']==int(sm[2]) and c['FAIL']==int(sm[3]) and c['IGNORED_NOT_RUN']==int(sm[4]) and len(b['tests'])==sum(map(int,[sm[2],sm[3],sm[4]])) and int(sm[6])==0
 b['status']='BOUNDED_TIMEOUT' if b['invocation']['timed_out'] else 'FAIL' if b['invocation']['exit_code']!=0 else 'PASS' if b['summary_consistent'] else 'EXECUTION_EVIDENCE_INCOMPLETE'
 save();print(json.dumps({'target':name,'status':b['status'],'rust_summary':b.get('rust_summary')}),flush=True)
r['source_identity_after']=identity();r['source_unchanged']=r['source_identity_after']==r['source_identity_before'];r['discovered_binary_count']=len(r['binaries']);r['test_counts']=dict(collections.Counter(t['status'] for b in r['binaries'] for t in b['tests']));r['binary_counts']=dict(collections.Counter(b['status'] for b in r['binaries']));r['changed_target_results']=[{k:b.get(k) for k in ['target','status','rust_summary','tests']} for b in r['binaries'] if b['target'] in changed]
r['full_suite_claim']='PASS_WITH_IGNORED_NOT_RUN_AND_DOCTESTS_NOT_RUN' if all(b['status']=='PASS' for b in r['binaries']) and r['source_unchanged'] else 'NOT_COMPLETE_BOUNDED_OR_FAILURE';r['finished_utc']=datetime.datetime.now(datetime.UTC).isoformat();r['native_elapsed_seconds']=time.monotonic()-start;r['memory_events_after']=Path('/sys/fs/cgroup/memory.events').read_text();save();print(json.dumps({k:r[k] for k in ['full_suite_claim','test_counts','binary_counts','source_unchanged']}),flush=True)
