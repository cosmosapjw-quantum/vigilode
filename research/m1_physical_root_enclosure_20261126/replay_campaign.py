"""Execute the final binary with frozen selections and verify every row anew."""
from pathlib import Path
import hashlib,json,os,platform,subprocess,time,datetime,re,sys,shutil
from mhgext.validate import validate_run_manifest
P=Path(__file__).resolve().parent;ROOT=P.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def now():return datetime.datetime.now(datetime.timezone.utc).isoformat()
def dump(p,v):p.write_text(json.dumps(v,indent=2,sort_keys=True,allow_nan=False)+'\n')
def read(p,tag):return [json.loads(l.split(tag,1)[1]) for l in p.read_text().splitlines() if tag+'{' in l]
def key(r):return (r['case'],r['candidate'],r.get('time_tol',1e-5))
paths=re.findall(r'Running tests/physical_event_contracts.rs \(([^)]+)\)',(P/'final_facade.log').read_text())
assert len(paths)==1,paths
binary=Path(paths[0]);assert binary.is_file();bh=sha(binary)
# The recorded Rust candidate array must equal the generated catalog.
candidates={x['id'] for x in json.loads((P/'CANDIDATES.json').read_text())['routes']}
for name in ['training','precision']:
 rows=read(P/(name+'.log'),'CELL ')
 for case in {r['case'] for r in rows}:
  for tol in {r.get('time_tol',1e-5) for r in rows}:
   assert {r['candidate'] for r in rows if r['case']==case and r.get('time_tol',1e-5)==tol}==candidates
files=subprocess.check_output(['git','ls-files','-z','--','crates','Cargo.toml','Cargo.lock','rust-toolchain.toml'],cwd=ROOT).decode().split('\0')
files=[ROOT/f for f in files if f and (ROOT/f).is_file() and Path(f).suffix in ('.rs','.toml','.lock','.json','.h','.c','.f')]
files.extend([ROOT/'crates/rodas5p-integrators/src/retro_physical_event.rs',ROOT/'crates/vigilode/tests/physical_event_contracts.rs'])
for n in ['audit.py','guide.py','select_precision.py','replay_campaign.py','CANDIDATES.json','SELECTION.json','PRECISION_SELECTION.json','PLAN.md','PRECISION_PROTOCOL.md','HARNESS_ROUTES.json']:
 files.append(P/n)
names=[('training','training'),('holdout','holdout'),('precision','precision_campaign'),('precision_holdout','precision_holdout')]
for n,_ in names:files.extend([P/(n+'.log'),P/(n.upper()+'_AUDIT.json')])
before={f:sha(f) for f in sorted(set(files))}
lock=P/'REPLAY_SOURCE_LOCK.json'
dump(lock,{'parent':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'binary_sha256':bh,'files':{str(f.relative_to(ROOT)):h for f,h in before.items()}})
before[lock]=sha(lock)
out=P/'replay';out.mkdir(exist_ok=False)
shutil.copyfile(P/'audit.py',out/'audit.py')
shutil.copyfile(P/'PRECISION_SELECTION.json',out/'PRECISION_SELECTION.json')
env=dict(os.environ,VIGILODE_PHYSICAL_SELECTION=str(P/'SELECTION.json'),VIGILODE_PHYSICAL_PRECISION_SELECTION=str(P/'PRECISION_SELECTION.json'))
start=now();clock=time.monotonic();cmds=[];outputs=[];counts={}
for name,test in names:
 cmd=[str(binary),test,'--ignored','--exact','--nocapture','--test-threads=1']
 r=subprocess.run(cmd,cwd=ROOT,env=env,capture_output=True,timeout=120)
 dest=out/(name+'.log');dest.write_bytes(r.stdout+r.stderr)
 assert r.returncode==0
 cmds.append({'argv':cmd,'exit_code':r.returncode})
 outputs.append(dest)
 assert read(dest,'SOURCE ')==read(P/(name+'.log'),'SOURCE ')
 new=read(dest,'CELL ');old={key(r):r for r in read(P/(name+'.log'),'CELL ')}
 assert len(new)==len(old)
 fixed=adaptive=0
 for r in new:
  prev=old[key(r)]
  if r['candidate']!='Adaptive':
   assert r['report']==prev['report'] and r.get('reason')==prev.get('reason'),key(r)
   fixed+=1
  else:adaptive+=1
 audit_cmd=[sys.executable,str(out/'audit.py'),name]
 a=subprocess.run(audit_cmd,cwd=ROOT,env=env,capture_output=True,timeout=120)
 auditlog=out/(name+'_audit.log');auditlog.write_bytes(a.stdout+a.stderr);assert a.returncode==0,a.stderr
 cmds.append({'argv':audit_cmd,'exit_code':a.returncode})
 q=out/(name.upper()+'_AUDIT.json');outputs.extend([auditlog,q])
 before_a={key(r):r for r in json.loads((P/(name.upper()+'_AUDIT.json')).read_text())['rows']}
 after=json.loads(q.read_text())
 assert all(r['audit_pass']==before_a[key(r)]['audit_pass'] for r in after['rows'])
 counts[name]={'cells':len(new),'fixed_exact':fixed,'adaptive_reaudited':adaptive,'qualified':after['qualified'],'slab_proofs':after['slab_proofs'],'samples':after['samples_checked']}
assert sha(binary)==bh and all(sha(p)==h for p,h in before.items())
result=P/'REPLAY_RESULT.json';dump(result,{'counts':counts,'binary_sha256':bh,'scope':'static reports exact; adaptive fresh independent physical checks'})
outputs.append(result)
manifest={'schema':'mhg-run/1.0','run_id':'m1-physical-root-campaign-replay','project_id':'vigilode','objective':'Re-execute the frozen MathHarness-guided physical-root candidates','assertion_tested':'Original native packets and all static reports identical; adaptive rows independently recertified','domain':{'model':'constant source-owned dissipative affine','time_tolerances':[1e-5,1e-8,1e-10]},'conventions':{'metric':'absolute physical time bracket; first root','oracle':'Fraction + 85-digit mpmath matrix exponential; finite samples, not a formal proof'},'status':'completed','execution':{'argv':cmds[0]['argv'],'cwd':'.','exit_code':0,'started_at':start,'finished_at':now(),'runtime_seconds':time.monotonic()-clock,'subcommands':cmds,'binary_sha256':bh},'environment':{'os':platform.platform(),'architecture':platform.machine(),'software':{'python':platform.python_version(),'mathharness':'3.1.0'}},'inputs':[{'artifact_id':f'i{i}','path':str(f.relative_to(ROOT)),'sha256_before':h,'sha256_after':sha(f),'immutable':True} for i,(f,h) in enumerate(before.items())],'outputs':[{'artifact_id':str(f.relative_to(P)),'path':str(f.relative_to(ROOT)),'sha256':sha(f),'role':'result' if f.suffix=='.json' else 'log'} for f in outputs],'checks':[{'check_id':'campaign-replay','verdict':'PASS','observed':counts,'expected':counts}],'non_claims':['No generic nonlinear or physical reset certification','No full IVP performance comparison','No bound on work reduction from GP-UCB']}
dump(P/'REPLAY_MANIFEST.json',manifest)
print('\n'.join(validate_run_manifest(manifest,ROOT,check_artifacts=True)))
print(json.dumps(counts))
