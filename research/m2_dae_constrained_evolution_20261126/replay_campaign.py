"""Re-execute final compiled candidate and verify exact original-source receipts."""
from pathlib import Path
import hashlib,json,os,platform,subprocess,time,datetime,re,sys,shutil
from mhgext.validate import validate_run_manifest
P=Path(__file__).resolve().parent;ROOT=P.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def now():return datetime.datetime.now(datetime.timezone.utc).isoformat()
def dump(p,v):p.write_text(json.dumps(v,indent=2,sort_keys=True,allow_nan=False)+'\n')
def read(p,tag):return [json.loads(l.split(tag,1)[1]) for l in p.read_text().splitlines() if tag+'{' in l]
def key(r):return (r['case'],r['candidate'],r['tight'])
paths=re.findall(r'Running tests/dae_evolution_contracts.rs \(([^)]+)\)',(P/'final_facade.log').read_text())
assert len(paths)==1,paths
binary=Path(paths[0]);assert binary.is_file();bh=sha(binary)
catalog={r['id'] for r in json.loads((P/'CANDIDATES.json').read_text())['routes']}
rows=read(P/'training.log','CELL ')
for case in {r['case'] for r in rows}:
 for tight in [False,True]:assert {r['candidate'] for r in rows if r['case']==case and r['tight']==tight}==catalog
files=subprocess.check_output(['git','ls-files','-z','--','crates','Cargo.toml','Cargo.lock','rust-toolchain.toml'],cwd=ROOT).decode().split('\0')
files=[ROOT/f for f in files if f and (ROOT/f).is_file() and Path(f).suffix in ('.rs','.toml','.lock','.json','.h','.c','.f')]
files.extend([ROOT/'crates/rodas5p-integrators/src/retro_dae_evolution.rs',ROOT/'crates/vigilode/src/dae_evolution.rs',ROOT/'crates/vigilode/tests/dae_evolution_contracts.rs'])
for n in ['audit.py','guide.py','replay_campaign.py','CANDIDATES.json','SELECTION.json','PLAN.md','RECOVERY_PROTOCOL.md','HARNESS_ROUTES.json','training.log','holdout.log','recovery.log','TRAINING_AUDIT.json','HOLDOUT_AUDIT.json','RECOVERY_AUDIT.json','final_contracts.log']:
 files.append(P/n)
before={f:sha(f) for f in sorted(set(files))}
lock=P/'REPLAY_SOURCE_LOCK.json'
dump(lock,{'parent':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'binary_sha256':bh,'files':{str(f.relative_to(ROOT)):h for f,h in before.items()}})
before[lock]=sha(lock)
out=P/'replay';out.mkdir(exist_ok=False)
for n in ['audit.py','SELECTION.json']:shutil.copyfile(P/n,out/n)
env=dict(os.environ,VIGILODE_DAE_EVOLUTION_SELECTION=str(P/'SELECTION.json'),OMP_NUM_THREADS='1',OPENBLAS_NUM_THREADS='1')
start=now();clock=time.monotonic();cmds=[];outputs=[];counts={}
def exec_log(cmd,p):
 r=subprocess.run(cmd,cwd=ROOT,env=env,capture_output=True,timeout=180)
 p.write_bytes(r.stdout+r.stderr);assert r.returncode==0,r.stderr
 cmds.append({'argv':cmd,'exit_code':r.returncode});outputs.append(p)
for name,num in [('training',80),('holdout',10),('recovery',10)]:
 dest=out/(name+'.log');exec_log([str(binary),name,'--ignored','--exact','--nocapture','--test-threads=1'],dest)
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
 exec_log([sys.executable,str(out/'audit.py'),name],out/(name+'_audit.log'))
 q=out/(name.upper()+'_AUDIT.json');outputs.append(q)
 after=json.loads(q.read_text());before_a={key(r):r for r in json.loads((P/(name.upper()+'_AUDIT.json')).read_text())['rows']}
 assert all(r['qualified']==before_a[key(r)]['qualified'] for r in after['rows'])
 # An empirical adaptive selector can choose a different qualified
 # route and its physical certificate status may change.
 counts[name]={'cells':num,'fixed_exact':fixed,'adaptive_reaudited':adaptive,'qualified':after['qualified'],'global_qualified':after['global_qualified'],'slab_checks':after['slab_checks'],'query_checks':after['query_checks'],'ideal_checks':after['ideal_checks']}
assert sha(binary)==bh and all(sha(f)==h for f,h in before.items())
assert sum(c['cells'] for c in counts.values())==100
assert [counts[k]['qualified'] for k in ['training','holdout','recovery']]==[62,9,10]
result=P/'REPLAY_RESULT.json'
dump(result,{'counts':counts,'binary_sha256':bh,'scope':'fixed receipts exact; adaptive freshly independently checked'})
outputs.append(result)
manifest={'schema':'mhg-run/1.0','run_id':'m2-dae-evolution-campaign-replay','project_id':'vigilode','objective':'Execute final binary on frozen constrained-evolution candidates','assertion_tested':'Fixed non-timing receipts exact; adaptive fresh rational/physical audits; frozen holdout failure preserved','domain':{'training':80,'holdout':10,'recovery':10,'scope':'declared polynomial index-one DAE'},'conventions':{'oracle':'exact rational interval arithmetic, 70-digit analytic solutions; two-level SciPy reduced Radau for Robertson','timing':'diagnostic'},'status':'completed','execution':{'argv':cmds[0]['argv'],'cwd':'.','exit_code':0,'started_at':start,'finished_at':now(),'runtime_seconds':time.monotonic()-clock,'subcommands':cmds,'binary_sha256':bh},'environment':{'os':platform.platform(),'architecture':platform.machine(),'software':{'python':platform.python_version(),'mathharness':'3.1.0'}},'inputs':[{'artifact_id':f'i{i}','path':str(f.relative_to(ROOT)),'sha256_before':h,'sha256_after':sha(f),'immutable':True} for i,(f,h) in enumerate(before.items())],'outputs':[{'artifact_id':str(f.relative_to(P)),'path':str(f.relative_to(ROOT)),'sha256':sha(f),'role':'result' if f.suffix=='.json' else 'log'} for f in outputs],'checks':[{'check_id':'same-contract-replay','verdict':'PASS','observed':{'training':62,'holdout':9,'recovery':10},'expected':{'training':62,'holdout':9,'recovery':10}}],'non_claims':['No universal physical DAE certification','No independent new holdout in recovery','No generic nonlinear WARP inverse or matched-error ranking']}
dump(P/'REPLAY_MANIFEST.json',manifest)
print('\n'.join(validate_run_manifest(manifest,ROOT,check_artifacts=True)))
print(json.dumps(counts))
