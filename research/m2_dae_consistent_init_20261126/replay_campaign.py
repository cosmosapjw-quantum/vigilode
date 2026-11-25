"""Execute final binary with frozen selections; independently re-audit."""
from pathlib import Path
import hashlib,json,os,platform,subprocess,time,datetime,re,sys,shutil
from mhgext.validate import validate_run_manifest
P=Path(__file__).resolve().parent;ROOT=P.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def now():return datetime.datetime.now(datetime.timezone.utc).isoformat()
def dump(p,v):p.write_text(json.dumps(v,indent=2,sort_keys=True,allow_nan=False)+'\n')
def read(p,tag):return [json.loads(l.split(tag,1)[1]) for l in p.read_text().splitlines() if tag+'{' in l]
def key(r):return (r['case'],r['candidate'],r['tight'])
paths=re.findall(r'Running tests/dae_init_contracts.rs \(([^)]+)\)',(P/'final_facade.log').read_text())
assert len(paths)==1,paths
binary=Path(paths[0]);assert binary.is_file();bh=sha(binary)
catalog={r['id'] for r in json.loads((P/'CANDIDATES.json').read_text())['routes']}
rows=read(P/'training.log','CELL ')
for case in {r['case'] for r in rows}:
 for tight in [False,True]:assert {r['candidate'] for r in rows if r['case']==case and r['tight']==tight}==catalog
files=subprocess.check_output(['git','ls-files','-z','--','crates','Cargo.toml','Cargo.lock','rust-toolchain.toml'],cwd=ROOT).decode().split('\0')
files=[ROOT/f for f in files if f and (ROOT/f).is_file() and Path(f).suffix in ('.rs','.toml','.lock','.json','.h','.c','.f')]
files.extend([ROOT/'crates/rodas5p-integrators/src/retro_dae_init.rs',ROOT/'crates/vigilode/tests/dae_init_contracts.rs'])
for n in ['audit.py','guide.py','replay_campaign.py','CANDIDATES.json','SELECTION.json','PLAN.md','HARNESS_ROUTES.json','training.log','holdout.log','contracts.log','TRAINING_AUDIT.json','HOLDOUT_AUDIT.json','MASS_AUDIT.json']:files.append(P/n)
before={f:sha(f) for f in sorted(set(files))}
lock=P/'REPLAY_SOURCE_LOCK.json'
dump(lock,{'parent':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'binary_sha256':bh,'files':{str(f.relative_to(ROOT)):h for f,h in before.items()}})
before[lock]=sha(lock)
out=P/'replay';out.mkdir(exist_ok=False)
for n in ['audit.py','SELECTION.json']:shutil.copyfile(P/n,out/n)
env=dict(os.environ,VIGILODE_DAE_SELECTION=str(P/'SELECTION.json'),OMP_NUM_THREADS='1',OPENBLAS_NUM_THREADS='1')
start=now();clock=time.monotonic();cmds=[];outputs=[];counts={}
def exec_log(cmd,p):
 r=subprocess.run(cmd,cwd=ROOT,env=env,capture_output=True,timeout=120)
 p.write_bytes(r.stdout+r.stderr);assert r.returncode==0,r.stderr
 cmds.append({'argv':cmd,'exit_code':r.returncode});outputs.append(p)
for name,num in [('training',144),('holdout',8)]:
 cmd=[str(binary),name,'--ignored','--exact','--nocapture','--test-threads=1']
 dest=out/(name+'.log');exec_log(cmd,dest)
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
 old_a={key(r):r for r in json.loads((P/(name.upper()+'_AUDIT.json')).read_text())['rows']}
 after=json.loads(q.read_text())
 assert all(r['qualified']==old_a[key(r)]['qualified'] for r in after['rows'])
 counts[name]={'cells':num,'fixed_exact':fixed,'adaptive_reaudited':adaptive,'qualified':after['qualified'],'refused':after['refused']}
exec_log([str(binary),'mass_client_same_source','--exact','--nocapture','--test-threads=1'],out/'contracts.log')
assert read(out/'contracts.log','MASS ')==read(P/'contracts.log','MASS ')
exec_log([sys.executable,str(out/'audit.py'),'mass'],out/'mass_audit.log')
outputs.append(out/'MASS_AUDIT.json')
assert sha(binary)==bh and all(sha(f)==h for f,h in before.items())
assert sum(c['cells'] for c in counts.values())==152
result=P/'REPLAY_RESULT.json'
dump(result,{'counts':counts,'mass':json.loads((out/'MASS_AUDIT.json').read_text()),'binary_sha256':bh,'scope':'exact static result replay; fresh independent oracle for adaptive selection'})
outputs.append(result)
manifest={'schema':'mhg-run/1.0','run_id':'m2-dae-init-campaign-replay','project_id':'vigilode','objective':'Execute final binary on frozen DAE init portfolio','assertion_tested':'Fixed non-timing receipts and original mass smoke trajectories identical; adaptive independently verified','domain':{'training_cells':144,'holdout_cells':8,'scope':'declared polynomial fixed initial branch'},'conventions':{'oracle':'exact rational symbolic source and 85-digit evaluation','timing':'diagnostic only'},'status':'completed','execution':{'argv':cmds[0]['argv'],'cwd':'.','exit_code':0,'started_at':start,'finished_at':now(),'runtime_seconds':time.monotonic()-clock,'subcommands':cmds,'binary_sha256':bh},'environment':{'os':platform.platform(),'architecture':platform.machine(),'software':{'python':platform.python_version(),'mathharness':'3.1.0'}},'inputs':[{'artifact_id':f'i{i}','path':str(f.relative_to(ROOT)),'sha256_before':h,'sha256_after':sha(f),'immutable':True} for i,(f,h) in enumerate(before.items())],'outputs':[{'artifact_id':str(f.relative_to(P)),'path':str(f.relative_to(ROOT)),'sha256':sha(f),'role':'result' if f.suffix=='.json' else 'log'} for f in outputs],'checks':[{'check_id':'physical-init-replay','verdict':'PASS','observed':{'train':counts['training']['qualified'],'holdout':counts['holdout']['qualified']},'expected':{'train':118,'holdout':8}}],'non_claims':['No global DAE solution certificate','No arbitrary high-index inference','No automatic mutation of strict initial state']}
assert counts['training']['qualified']==118 and counts['holdout']['qualified']==8
dump(P/'REPLAY_MANIFEST.json',manifest)
print('\n'.join(validate_run_manifest(manifest,ROOT,check_artifacts=True)))
print(json.dumps(counts))
