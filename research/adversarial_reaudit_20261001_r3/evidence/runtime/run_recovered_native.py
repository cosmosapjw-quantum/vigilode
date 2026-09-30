#!/usr/bin/env python3
"""Bounded native execution after incomplete Cargo stream; never certifies full build."""
from pathlib import Path
import datetime, hashlib, json, os, re, signal, subprocess, time
root=Path(__file__).resolve().parents[3];repo=root/'vigilode';ev=Path(__file__).resolve().parent;out=ev/'recovered_native';out.mkdir(exist_ok=True);target=root/'runtime_r3/target-shared/debug'
metadata=json.loads((ev/'cargo_metadata.json').read_text());members=set(metadata['workspace_members'])
changed={'work_unit_contracts','paired_timing_contracts','r2_timing_authority_contracts','r2_work_coverage_consumer_contracts','r2_output_budget_contracts','r2_output_time_identity_contracts','r2_phi_range_contracts'}
slow={'a1_two_by_six_spread_contracts','g4_s5b0_regime_atlas_contracts','givens_production_differential_contracts','homotopy_policy_contracts','inner_forcing_fixed_step_ladder_contracts','unified_candidate_contracts'}
started=time.monotonic();budget_total=300
result={'schema':'vigilode.r3.recovered_native_campaign.v1','started_utc':datetime.datetime.now(datetime.UTC).isoformat(),'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip(),'full_build_status':'UNRESOLVED_INCOMPLETE_STREAM_AND_CACHED_RECOVERY_TIMEOUT','native_campaign_status':'RUNNING','scope':'source-bound current-turn native harness files, mapped through Cargo metadata and test fingerprints; no more Cargo','profile':{'opt_level':0,'debug_info':True,'codegen_units':1,'incremental':False,'features':'all'},'total_budget_seconds':budget_total,'doctests':'NOT_RUN','binaries':[]}
def save():
 (out/'summary.json').write_text(json.dumps(result,indent=2)+'\n')
def run(argv,cwd,stem,suffix,budget):
 t=time.monotonic();p=subprocess.Popen(argv,cwd=cwd,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True);timeout=False
 try:a,b=p.communicate(timeout=max(.01,budget));code=p.returncode
 except subprocess.TimeoutExpired:
  timeout=True;os.killpg(p.pid,signal.SIGTERM)
  try:a,b=p.communicate(timeout=2)
  except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);a,b=p.communicate()
  code=124
 so=out/f'{stem}.{suffix}.log';se=out/f'{stem}.{suffix}.stderr';so.write_bytes(a);se.write_bytes(b)
 return {'argv':argv,'cwd':str(cwd),'exit_code':code,'timed_out':timeout,'budget_seconds':budget,'elapsed_seconds':time.monotonic()-t,'stdout':str(so),'stderr':str(se)},a.decode(errors='replace')
for package in metadata['packages']:
 if package['id'] not in members:continue
 for t in package['targets']:
  if t['kind']==['custom-build']:continue
  folder=target/('examples' if t['kind']==['example'] else 'deps');name=t['name'].replace('-','_');paths=[p for p in folder.glob(name+'-*') if p.suffix=='' and os.access(p,os.X_OK)]
  selected=[]
  for p in paths:
   h=p.name.rsplit('-',1)[1];fingerprints=list((target/'.fingerprint').glob('*-'+h));testjson=[q for fp in fingerprints for q in fp.glob('test-*.json')]
   if testjson:selected.append((p,testjson))
  item={'target':t['name'],'kind':t['kind'],'package_id':package['id'],'manifest_path':package['manifest_path'],'tests':[]}
  if len(selected)!=1:item['status']='UNRESOLVED_ARTIFACT_MAPPING';item['candidate_paths']=[str(p) for p in paths]
  else:
   p,fp=selected[0];item.update(executable=str(p),bytes=p.stat().st_size,mtime_utc=datetime.datetime.fromtimestamp(p.stat().st_mtime,datetime.UTC).isoformat(),fingerprints=[{'path':str(q),'sha256':hashlib.sha256(q.read_bytes()).hexdigest(),'content':json.loads(q.read_text())} for q in fp],status='DISCOVERED')
  result['binaries'].append(item)
result['binaries'].sort(key=lambda b:(0 if b['target'] in changed else (2 if b['target'] in slow or b['target'].startswith('a1_') else 1),b['target']))
# List every recovered harness first so not-run cases have explicit named identities.
for i,item in enumerate(result['binaries']):
 if item['status']!='DISCOVERED':continue
 item['index']=i;stem=f'{i:03d}_{item["target"]}';cwd=Path(item['manifest_path']).parent
 remaining=budget_total-(time.monotonic()-started)
 if remaining<=0:item['status']='NOT_LISTED_TOTAL_BUDGET';continue
 listing,txt=run([item['executable'],'--list'],cwd,stem,'list',min(5,remaining));item['listing']=listing
 if listing['exit_code']!=0:item['status']='LISTING_FAILED';continue
 item['listed_tests']=[l[:-6] for l in txt.splitlines() if l.endswith(': test')];item['tests']=[{'name':n,'status':'NOT_RUN_TOTAL_BUDGET'} for n in item['listed_tests']]
 item['status']='LISTED_NOT_RUN';save()
for item in result['binaries']:
 if item['status']!='LISTED_NOT_RUN':continue
 remaining=budget_total-(time.monotonic()-started)
 if remaining<=0:item['status']='NOT_RUN_TOTAL_BUDGET';continue
 name=item['target'];stem=f'{item["index"]:03d}_{name}';cwd=Path(item['manifest_path']).parent
 limit=min(60 if name in slow or name.startswith('a1_') else 30,remaining)
 invocation,txt=run([item['executable'],'--test-threads=1'],cwd,stem,'run',limit);item['invocation']=invocation
 states={n:s for n,s in re.findall(r'^test (.*?) \.\.\. (ok|FAILED|ignored)\b',txt,re.MULTILINE)}
 for test in item['tests']:
  test['status']={'ok':'PASS','FAILED':'FAIL','ignored':'IGNORED_NOT_RUN'}.get(states.get(test['name']),'UNRESOLVED_BOUNDED_TIMEOUT' if invocation['timed_out'] else 'UNRESOLVED_MISSING_RESULT')
 item['status']='BOUNDED_TIMEOUT' if invocation['timed_out'] else ('PASS' if invocation['exit_code']==0 else 'FAIL')
 sm=re.search(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out',txt)
 if sm:item['rust_summary']={'status':sm[1],'passed':int(sm[2]),'failed':int(sm[3]),'ignored':int(sm[4]),'measured':int(sm[5]),'filtered_out':int(sm[6])}
 save();print(json.dumps({'target':name,'status':item['status'],'rust_summary':item.get('rust_summary')}),flush=True)
alltests=[t for b in result['binaries'] for t in b['tests']]
result['discovered_binary_count']=len(result['binaries']);result['discovered_named_test_count']=len(alltests)
result['test_counts']={s:sum(t['status']==s for t in alltests) for s in sorted(set(t['status'] for t in alltests))};result['binary_counts']={s:sum(b['status']==s for b in result['binaries']) for s in sorted(set(b['status'] for b in result['binaries']))}
result['changed_target_results']=[{k:b.get(k) for k in ['target','status','executable','rust_summary','tests']} for b in result['binaries'] if b['target'] in changed]
result['native_campaign_status']='COMPLETE_OBSERVED_NATIVE_WITH_IGNORED' if all(b['status']=='PASS' for b in result['binaries']) else 'PARTIAL_BOUNDED_NATIVE_CAMPAIGN';result['finished_utc']=datetime.datetime.now(datetime.UTC).isoformat();result['elapsed_seconds']=time.monotonic()-started;result['memory_events_after']=Path('/sys/fs/cgroup/memory.events').read_text();save();print(json.dumps({k:result[k] for k in ['native_campaign_status','discovered_binary_count','discovered_named_test_count','test_counts','binary_counts']}),flush=True)
