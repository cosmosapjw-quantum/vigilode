"""Final binary replay, independent oracle and MathHarness artifact checks."""
from pathlib import Path
import hashlib,json,os,platform,subprocess,time,datetime,re,sys,shutil
from mhgext.validate import validate_run_manifest
P=Path(__file__).resolve().parent;ROOT=P.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def now():return datetime.datetime.now(datetime.timezone.utc).isoformat()
def dump(p,v):p.write_text(json.dumps(v,indent=2,sort_keys=True,allow_nan=False)+'\n')
def read(p,tag):return [json.loads(l.split(tag,1)[1]) for l in p.read_text().splitlines() if tag+'{' in l]
def key(r):return (r['case'],r['candidate'],r['time_tol'])
paths=re.findall(r'Running tests/hybrid_reset_contracts.rs \(([^)]+)\)',(P/'final_facade.log').read_text())
assert len(paths)==1,paths
binary=Path(paths[0]);assert binary.is_file();bh=sha(binary)
catalog={r['id'] for r in json.loads((P/'CANDIDATES.json').read_text())['routes']}
rows=read(P/'training.log','CELL ')
for case in {r['case'] for r in rows}:
 for tol in [1e-6,1e-9]:assert {r['candidate'] for r in rows if r['case']==case and r['time_tol']==tol}==catalog
files=subprocess.check_output(['git','ls-files','-z','--','crates','Cargo.toml','Cargo.lock','rust-toolchain.toml'],cwd=ROOT).decode().split('\0')
files=[ROOT/f for f in files if f and (ROOT/f).is_file() and Path(f).suffix in ('.rs','.toml','.lock','.json','.h','.c','.f')]
files.extend([ROOT/'crates/rodas5p-integrators/src/retro_hybrid_reset.rs',ROOT/'crates/vigilode/tests/hybrid_reset_contracts.rs'])
for n in ['audit.py','guide.py','replay_campaign.py','CANDIDATES.json','SELECTION.json','PLAN.md','HARNESS_ROUTES.json','training.log','holdout.log','TRAINING_AUDIT.json','HOLDOUT_AUDIT.json','audit_sync_gap.py','SYNC_GAP_AUDIT.json']:
 files.append(P/n)
before={f:sha(f) for f in sorted(set(files))}
lock=P/'REPLAY_SOURCE_LOCK.json'
dump(lock,{'parent':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'binary_sha256':bh,'files':{str(f.relative_to(ROOT)):h for f,h in before.items()}})
before[lock]=sha(lock)
out=P/'replay';out.mkdir(exist_ok=False)
for n in ['audit.py','audit_sync_gap.py','SELECTION.json']:shutil.copyfile(P/n,out/n)
env=dict(os.environ,VIGILODE_HYBRID_SELECTION=str(P/'SELECTION.json'))
start=now();clock=time.monotonic();cmds=[];outputs=[];counts={}
for name,num in [('training',112),('holdout',8)]:
 cmd=[str(binary),name,'--ignored','--exact','--nocapture','--test-threads=1']
 r=subprocess.run(cmd,cwd=ROOT,env=env,capture_output=True,timeout=120)
 dest=out/(name+'.log');dest.write_bytes(r.stdout+r.stderr);assert r.returncode==0
 outputs.append(dest);cmds.append({'argv':cmd,'exit_code':r.returncode})
 assert read(dest,'SOURCE ')==read(P/(name+'.log'),'SOURCE ')
 new=read(dest,'CELL ');old={key(r):r for r in read(P/(name+'.log'),'CELL ')}
 assert len(new)==len(old)==num
 fixed=adaptive=0
 for r in new:
  prev=old[key(r)]
  if r['candidate']!='Adaptive':
   assert r['result']==prev['result'] and r.get('reason')==prev.get('reason'),key(r)
   fixed+=1
  else:adaptive+=1
 cmd=[sys.executable,str(out/'audit.py'),name];a=subprocess.run(cmd,cwd=ROOT,env=env,capture_output=True,timeout=120)
 log=out/(name+'_audit.log');log.write_bytes(a.stdout+a.stderr);assert a.returncode==0,a.stderr
 cmds.append({'argv':cmd,'exit_code':a.returncode})
 q=out/(name.upper()+'_AUDIT.json');outputs.extend([log,q])
 old_a={key(r):r for r in json.loads((P/(name.upper()+'_AUDIT.json')).read_text())['rows']}
 after=json.loads(q.read_text())
 assert all(r['qualified']==old_a[key(r)]['qualified'] for r in after['rows'])
 counts[name]={'cells':num,'fixed_exact':fixed,'adaptive_reaudited':adaptive,'physical_qualified':after['physical_qualified'],'endpoint_checks':after['endpoint_checks_including_adaptive_attempts'],'root_checks':after['root_checks_including_adaptive_attempts']}
cmd=[sys.executable,str(out/'audit_sync_gap.py')];a=subprocess.run(cmd,cwd=ROOT,env=env,capture_output=True,timeout=120)
log=out/'sync_gap_audit.log';log.write_bytes(a.stdout+a.stderr);assert a.returncode==0,a.stderr
cmds.append({'argv':cmd,'exit_code':a.returncode})
outputs.extend([log,out/'SYNC_GAP_AUDIT.json'])
assert sha(binary)==bh and all(sha(f)==h for f,h in before.items())
assert sum(c['cells'] for c in counts.values())==120
assert sum(c['fixed_exact'] for c in counts.values())==106
assert sum(c['adaptive_reaudited'] for c in counts.values())==14
result=P/'REPLAY_RESULT.json';dump(result,{'counts':counts,'sync_audit':json.loads((out/'SYNC_GAP_AUDIT.json').read_text()),'binary_sha256':bh,'scope':'same exact-source physical certificate; timing-dependent adaptive ordering may differ'})
outputs.append(result)
manifest={'schema':'mhg-run/1.0','run_id':'m1-hybrid-reset-campaign-replay','project_id':'vigilode','objective':'Re-execute generated hybrid reset combinations and frozen physical selection','assertion_tested':'All fixed non-timing receipts match; every adaptive attempt audited independently; all sync boxes contain sampled exact-source flow','domain':{'training':112,'holdout':8,'model':'scalar affine threshold hybrid','time_tol':[1e-6,1e-9]},'conventions':{'root':'physical bracket versus represented control','oracle':'85-digit analytic scalar flow and reset, finite gap samples','cost':'diagnostic whole invocation including receipts'},'status':'completed','execution':{'argv':cmds[0]['argv'],'cwd':'.','exit_code':0,'started_at':start,'finished_at':now(),'runtime_seconds':time.monotonic()-clock,'subcommands':cmds,'binary_sha256':bh},'environment':{'os':platform.platform(),'architecture':platform.machine(),'software':{'python':platform.python_version(),'mathharness':'3.1.0'}},'inputs':[{'artifact_id':f'i{i}','path':str(f.relative_to(ROOT)),'sha256_before':h,'sha256_after':sha(f),'immutable':True} for i,(f,h) in enumerate(before.items())],'outputs':[{'artifact_id':str(f.relative_to(P)),'path':str(f.relative_to(ROOT)),'sha256':sha(f),'role':'result' if f.suffix=='.json' else 'log'} for f in outputs],'checks':[{'check_id':'same-contract-replay','verdict':'PASS','observed':{'cells':120,'fixed':106,'adaptive':14},'expected':{'cells':120,'fixed':106,'adaptive':14}}],'non_claims':['No generic hybrid ODE/DAE or all-root completeness','No whole-trajectory certification in uncertain reset gap','No matched-error external ranking']}
dump(P/'REPLAY_MANIFEST.json',manifest)
print('\n'.join(validate_run_manifest(manifest,ROOT,check_artifacts=True)))
print(json.dumps(counts))
