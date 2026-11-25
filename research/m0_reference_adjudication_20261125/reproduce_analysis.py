from pathlib import Path
import datetime,hashlib,json,platform,subprocess,sys,time
from mhgext.validate import validate_run_manifest
D=Path(__file__).resolve().parent;root=D.parent
old=root/'20261125-adversarial-rebaseline/run'
inputs=[D/'analyze.py',D/'finalize.py',D/'receipts/WOLFRAM_STOP.json',old/'cross_solver_v1_rust.json',old/'spatial_battery_v3_rust.json']
inputs+=sorted((D/'independent').glob('*.jsonl'))+sorted((D/'independent').glob('*.wl.*.json'))
inputs+=sorted((D/'intake').glob('*.corpus.json'))+sorted((D/'replay').glob('*.json'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
before={p:sha(p) for p in inputs}
now=lambda:datetime.datetime.now(datetime.timezone.utc).isoformat()
t=now();start=time.monotonic()
commands=[[sys.executable,'analyze.py'],[sys.executable,'finalize.py']]
log=[]
for cmd in commands:
 r=subprocess.run(cmd,cwd=D,timeout=60,capture_output=True)
 log.append({'argv':cmd,'exit_code':r.returncode,'stdout':r.stdout.decode(),'stderr':r.stderr.decode()})
 assert r.returncode==0,r.stderr
assert all(sha(p)==v for p,v in before.items())
out=D/'receipts/analysis_replay.json'
out.write_text(json.dumps(log,indent=2)+'\n')
s=json.loads((D/'REFERENCE_ADJUDICATION.json').read_bytes())
assert s['new_qualified']==14 and s['new_unresolved']==0 and s['historical_disputed']==12
outputs=[D/'SUMMARY.json',D/'ADJUDICATED_REFERENCES.json',D/'REFERENCE_ADJUDICATION.json',out]
m={'schema':'mhg-run/1.0','run_id':'m0-reference-adjudication-reduction','project_id':'vigilode','objective':'Replay fixed evidence reduction, not solve IVPs','assertion_tested':'All 14 selected registered protocols pass all required grid discrepancies; all original records preserved','domain':{'cases':14,'grid_points':101,'threshold':1e-10,'scope':'empirical finite-grid agreement'},'conventions':{'metric':'both directional max-over-grid WRMS','precision':'85-digit mpmath for decimal canonical outputs, binary64 for shared-source spatial paths'},'status':'completed','execution':{'argv':commands[0],'cwd':str(D.relative_to(root)),'exit_code':0,'started_at':t,'finished_at':now(),'runtime_seconds':time.monotonic()-start,'subcommands':commands},'environment':{'os':platform.platform(),'architecture':platform.machine(),'software':{'python':platform.python_version(),'mathharness':'3.1.0'}},'inputs':[{'artifact_id':f'input-{i}','path':str(p.relative_to(root)),'sha256_before':v,'sha256_after':sha(p),'immutable':True} for i,(p,v) in enumerate(before.items())],'outputs':[{'artifact_id':f'output-{i}','path':str(p.relative_to(root)),'sha256':sha(p),'role':'result' if p.suffix=='.json' else 'log'} for i,p in enumerate(outputs)],'checks':[{'check_id':'reference-disposition','verdict':'PASS','observed':{'new_qualified':14,'historical_disputed':12},'expected':{'new_qualified':14,'historical_disputed':12}}],'non_claims':['No rigorous reference enclosure or production solver promotion','No new matched-error rankings','Original Wolfram discretionary incomplete/failed results remain']}
(D/'receipts/REDUCTION_MANIFEST.json').write_text(json.dumps(m,indent=2)+'\n')
print('\n'.join(validate_run_manifest(m,root,check_artifacts=True)))
