#!/usr/bin/env python3
"""One native build, then one bounded invocation per Cargo test harness.
The timeout is an audit execution budget, never a test PASS or assertion failure.
Run after sourcing runtime_r3/env.sh; --target-dir may reuse an idle lane's target.
"""
from pathlib import Path
import argparse, subprocess, json, time, datetime, os, signal, re, hashlib
ap=argparse.ArgumentParser();ap.add_argument('--repo',required=True);ap.add_argument('--target-dir',required=True);ap.add_argument('--evidence-dir',required=True);ap.add_argument('--build-lock',required=True);ap.add_argument('--build-budget',type=float,default=600);ap.add_argument('--total-test-budget',type=float,default=750);args=ap.parse_args()
os.environ['CARGO_INCREMENTAL']='0';os.environ['CARGO_PROFILE_DEV_CODEGEN_UNITS']='1';os.environ['CARGO_PROFILE_TEST_CODEGEN_UNITS']='1'
repo=Path(args.repo).resolve();out=Path(args.evidence_dir).resolve();out.mkdir(parents=True,exist_ok=True)
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def run(argv,cwd,stdout,stderr,budget):
 start=time.monotonic()
 p=subprocess.Popen(argv,cwd=cwd,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True)
 timeout=False
 try:so,se=p.communicate(timeout=budget);code=p.returncode
 except subprocess.TimeoutExpired:
  timeout=True;os.killpg(p.pid,signal.SIGTERM)
  try:so,se=p.communicate(timeout=3)
  except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);so,se=p.communicate()
  code=124
 # Persist complete streams only after process termination; partial live redirection
 # was observed in the first R3 build receipt despite native artifacts existing.
 stdout.write_bytes(so);stderr.write_bytes(se)
 return {'argv':list(map(str,argv)),'cwd':str(cwd),'exit_code':code,'timed_out':timeout,'budget_seconds':budget,'elapsed_seconds':time.monotonic()-start,'stdout':str(stdout),'stderr':str(stderr)}
before={str(p.relative_to(repo)):sha(p) for p in repo.rglob('Cargo.toml') if 'target' not in p.parts};before['Cargo.lock']=sha(repo/'Cargo.lock')
result={'schema':'vigilode.reaudit.native_campaign.v1','started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip(),'profile':'debug','codegen_units':1,'incremental':False,'features':'all','targets':'all','dependencies':'locked+offline','test_threads':1,'doctests':'NOT_RUN: --all-targets no-run artifacts only','randomization':'none, changed R2 regression binaries first then name order','execution_scope':'bounded native test campaign; no release benchmark or scientific performance claim','source_identity_before':before,'full_suite_claim':'NOT_COMPLETE','binaries':[]}
cmd=['cargo','test','--workspace','--all-targets','--all-features','--locked','--offline','--no-run','--message-format=json','--target-dir',args.target_dir,'--jobs','1']
result['build']=run(['flock',args.build_lock]+cmd,repo,out/'all_test_artifacts.jsonl',out/'all_test_build.stderr',args.build_budget)
(out/'suite_summary.json').write_text(json.dumps(result,indent=2)+'\n')
if result['build']['exit_code']!=0:
 result['full_suite_claim']='BUILD_TIMEOUT' if result['build']['timed_out'] else 'BUILD_FAILED';(out/'suite_summary.json').write_text(json.dumps(result,indent=2)+'\n');print(result['full_suite_claim'],flush=True);raise SystemExit(0)
artifacts=[];seen=set();build_finished=[]
for ln in (out/'all_test_artifacts.jsonl').read_text().splitlines():
 try:x=json.loads(ln)
 except json.JSONDecodeError:continue
 if x.get('reason')=='build-finished':build_finished.append(x)
 if x.get('reason')!='compiler-artifact' or not x.get('executable') or not x.get('profile',{}).get('test'):continue
 if x['executable'] in seen:continue
 seen.add(x['executable']);artifacts.append(x)
if not artifacts or not build_finished or not build_finished[-1].get('success'):
 result['full_suite_claim']='BUILD_EVIDENCE_INCOMPLETE';result['discovered_native_test_binaries']=len(artifacts);result['build_finished_records']=build_finished;(out/'suite_summary.json').write_text(json.dumps(result,indent=2)+'\n');print(result['full_suite_claim'],flush=True);raise SystemExit(0)
result['discovered_native_test_binaries']=len(artifacts);result['build_finished_records']=build_finished
slow={'a1_two_by_six_spread_contracts','g4_s5b0_regime_atlas_contracts','givens_production_differential_contracts','homotopy_policy_contracts','inner_forcing_fixed_step_ladder_contracts','unified_candidate_contracts'}
# Run costly known research campaigns last so their bounded runtime cannot hide fast regression suites.
changed_targets={'work_unit_contracts','paired_timing_contracts','r2_timing_authority_contracts','r2_work_coverage_consumer_contracts','r2_output_budget_contracts','r2_output_time_identity_contracts','r2_phi_range_contracts'}
artifacts.sort(key=lambda x:(0 if x['target']['name'] in changed_targets else (2 if x['target']['name'] in slow or x['target']['name'].startswith('a1_') else 1),x['target']['name']))
start=time.monotonic()
for i,x in enumerate(artifacts):
 name=x['target']['name'];exe=x['executable'];kind=x['target']['kind'];stem=f'{i:03d}_{name}';manifest=Path(x.get('manifest_path',repo/'Cargo.toml'));cwd=manifest.parent
 remaining=args.total_test_budget-(time.monotonic()-start)
 item={'target':name,'kind':kind,'executable':exe,'package_id':x['package_id'],'manifest_path':str(manifest),'tests':[]}
 if remaining<=0:
  item['status']='NOT_RUN_TOTAL_BUDGET';result['binaries'].append(item);continue
 listing=run([exe,'--list'],cwd,out/(stem+'.list.log'),out/(stem+'.list.stderr'),min(15,remaining))
 item['listing']=listing
 if listing['exit_code']!=0:item['status']='LISTING_FAILED';result['binaries'].append(item);continue
 test_names=[ln[:-6] for ln in Path(listing['stdout']).read_text().splitlines() if ln.endswith(': test')]
 item['listed_tests']=test_names
 # Examples compiled with non-test harness may not respond to libtest; profile.test filters these.
 budget=min(120 if name in slow or name.startswith('a1_') else 30, max(0.1,args.total_test_budget-(time.monotonic()-start)))
 invocation=run([exe,'--test-threads=1'],cwd,out/(stem+'.run.log'),out/(stem+'.run.stderr'),budget)
 item['invocation']=invocation;txt=Path(invocation['stdout']).read_text(errors='replace')
 matches=re.findall(r'^test (.*?) \.\.\. (ok|FAILED|ignored)\b',txt,re.MULTILINE)
 status_by_name={n:s for n,s in matches if n in test_names}
 for n in test_names:
  s=status_by_name.get(n)
  item['tests'].append({'name':n,'status':{'ok':'PASS','FAILED':'FAIL','ignored':'IGNORED_NOT_RUN'}.get(s,'UNRESOLVED_BOUNDED_TIMEOUT' if invocation['timed_out'] else 'UNRESOLVED_MISSING_RESULT')})
 item['status']='BOUNDED_TIMEOUT' if invocation['timed_out'] else ('PASS' if invocation['exit_code']==0 else 'FAIL')
 # Summary regex corroborates full-suite counts and catches interleaved nocapture output.
 sm=re.search(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out',txt)
 if sm:item['rust_summary']={'status':sm[1],'passed':int(sm[2]),'failed':int(sm[3]),'ignored':int(sm[4]),'measured':int(sm[5]),'filtered_out':int(sm[6])}
 result['binaries'].append(item)
 (out/'suite_summary.json').write_text(json.dumps(result,indent=2)+'\n')
 print(json.dumps({'i':i+1,'targets':len(artifacts),'target':name,'status':item['status'],'seconds':round(invocation['elapsed_seconds'],2),'rust_summary':item.get('rust_summary')}),flush=True)
result['source_identity_after']={k:sha(repo/k) for k in before}
result['source_manifests_and_lock_unchanged']=result['source_identity_after']==before
alltests=[t for b in result['binaries'] for t in b['tests']]
result['test_counts']={s:sum(t['status']==s for t in alltests) for s in sorted(set(t['status'] for t in alltests))}
result['binary_counts']={s:sum(b['status']==s for b in result['binaries']) for s in sorted(set(b['status'] for b in result['binaries']))}
result['full_suite_claim']='PASS_WITH_IGNORED_NOT_RUN' if all(b['status']=='PASS' for b in result['binaries']) else 'NOT_COMPLETE_BOUNDED_OR_FAILURE'
result['finished_utc']=datetime.datetime.now(datetime.timezone.utc).isoformat();(out/'suite_summary.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({k:result[k] for k in ['full_suite_claim','binary_counts','test_counts','source_manifests_and_lock_unchanged']}),flush=True)
