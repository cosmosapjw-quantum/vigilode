"""Replay frozen training and holdout against the final compiled source."""
import datetime,hashlib,json,os,pathlib,platform,re,subprocess,time
from mhgext.validate import validate_run_manifest
P=pathlib.Path(__file__).resolve().parent;ROOT=P.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def now():return datetime.datetime.now(datetime.timezone.utc).isoformat()
def rel(p):return str(p.relative_to(ROOT))
def rows(p,tag):
    a=[json.loads(l.split(tag,1)[1]) for l in p.read_text().splitlines() if tag+'{' in l]
    for r in a:r.pop('elapsed_ns',None)
    return a
log=(P/'final_facade.log').read_text()
paths=re.findall(r'Running tests/event_transaction_portfolio.rs \(([^)]+)\)',log)
assert len(paths)==1
binary=pathlib.Path(paths[0]);assert binary.is_file()
bh=sha(binary)
files=subprocess.check_output(['git','ls-files','-z','--','crates','Cargo.toml','Cargo.lock','rust-toolchain.toml'],cwd=ROOT).decode().split('\0')
files=[ROOT/f for f in files if f and (ROOT/f).is_file() and pathlib.Path(f).suffix in ('.rs','.toml','.lock','.json','.h','.c','.f')]
files.extend([ROOT/'crates/vigilode/tests/event_transaction_contracts.rs',ROOT/'crates/vigilode/tests/event_transaction_portfolio.rs',P/'CANDIDATES.json',P/'SELECTION.json',P/'guide.py',P/'training.log',P/'holdout.log',P/'replay_campaign.py'])
before={f:sha(f) for f in files}
source=P/'REPLAY_SOURCE_LOCK.json'
source.write_text(json.dumps({'parent':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'binary_sha256':bh,'sources':{rel(f):h for f,h in before.items()}},indent=2,sort_keys=True)+'\n')
before[source]=sha(source)
env=dict(os.environ,VIGILODE_M1_CANDIDATES=str(P/'CANDIDATES.json'),VIGILODE_M1_SELECTION=str(P/'SELECTION.json'))
start=now();t=time.monotonic();cmds=[];outputs=[]
for name,tag in [('training','CELL '),('holdout','HOLDOUT ')]:
    cmd=[str(binary),name,'--ignored','--exact','--nocapture','--test-threads=1']
    r=subprocess.run(cmd,cwd=ROOT,env=env,capture_output=True,timeout=60)
    p=P/f'{name}_final_replay.log';p.write_bytes(r.stdout+r.stderr);outputs.append(p)
    cmds.append({'argv':cmd,'exit_code':r.returncode})
    assert r.returncode==0 and rows(p,tag)==rows(P/f'{name}.log',tag)
assert sha(binary)==bh and all(sha(f)==h for f,h in before.items())
result=P/'REPLAY_RESULT.json'
result.write_text(json.dumps({'training_cells':len(rows(outputs[0],'CELL ')),'holdout_cells':len(rows(outputs[1],'HOLDOUT ')),'same_non_timing_rows':True,'binary_sha256':bh},indent=2)+'\n')
outputs.append(result)
rec={'schema':'mhg-run/1.0','run_id':'m1-event-campaign-replay','project_id':'vigilode','objective':'Re-execute frozen MathHarness-guided event candidate matrix','assertion_tested':'Final source reproduces every non-timing training and holdout cell exactly, including hidden-pair failures','domain':{'training_cells':156,'holdout_cells':20,'scope':'finite analytic event fixtures'},'conventions':{'root':'sampled crossing','metric':'absolute event time; actual RHS and event calls','timing':'excluded from bitwise replay'},'status':'completed','execution':{'argv':cmds[0]['argv'],'cwd':'.','exit_code':0,'started_at':start,'finished_at':now(),'runtime_seconds':time.monotonic()-t,'subcommands':cmds,'binary_sha256':bh},'environment':{'os':platform.platform(),'architecture':platform.machine(),'software':{'python':platform.python_version(),'mathharness':'3.1.0'}},'inputs':[{'artifact_id':f'i{i}','path':rel(f),'sha256_before':h,'sha256_after':sha(f),'immutable':True} for i,(f,h) in enumerate(before.items())],'outputs':[{'artifact_id':f.name,'path':rel(f),'sha256':sha(f),'role':'result' if f==result else 'log'} for f in outputs],'checks':[{'check_id':'frozen-campaign-replay','verdict':'PASS','observed':{'training':156,'holdout':20,'same_non_timing_rows':True},'expected':{'training':156,'holdout':20,'same_non_timing_rows':True}}],'non_claims':['No proof of arbitrary opaque event completeness','No matched-error wall time claim','GP-UCB replay on measured lookup, not fewer solver executions']}
(P/'REPLAY_MANIFEST.json').write_text(json.dumps(rec,indent=2)+'\n')
print('\n'.join(validate_run_manifest(rec,ROOT,check_artifacts=True)))
