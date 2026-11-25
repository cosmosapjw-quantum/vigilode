"""Re-run actual frozen Rust candidates, independently verify exact results."""
import datetime,hashlib,json,os,pathlib,platform,re,subprocess,time
from mhgext.validate import validate_run_manifest
from audit import audit,audit_trace
P=pathlib.Path(__file__).resolve().parent;ROOT=P.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def now():return datetime.datetime.now(datetime.timezone.utc).isoformat()
def rel(p):return str(p.relative_to(ROOT))
def dump(p,x):p.write_text(json.dumps(x,indent=2,sort_keys=True,allow_nan=False)+'\n')
def rows(p):return [json.loads(l.split('CELL ',1)[1]) for l in p.read_text().splitlines() if 'CELL {' in l]
log=(P/'training.log').read_text()
paths=re.findall(r'Running tests/polynomial_event_portfolio.rs \(([^)]+)\)',log)
assert len(paths)==1,paths
binary=pathlib.Path(paths[0]);assert binary.is_file()
bh=sha(binary)
files=subprocess.check_output(['git','ls-files','-z','--','crates','Cargo.toml','Cargo.lock','rust-toolchain.toml'],cwd=ROOT).decode().split('\0')
files=[ROOT/f for f in files if f and (ROOT/f).is_file() and pathlib.Path(f).suffix in ('.rs','.toml','.lock','.json','.h','.c','.f')]
files.extend([ROOT/'crates/rodas5p-integrators/src/retro_event_poly.rs',ROOT/'crates/vigilode/src/polynomial_events.rs',ROOT/'crates/vigilode/tests/polynomial_event_contracts.rs',ROOT/'crates/vigilode/tests/polynomial_event_portfolio.rs',P/'CANDIDATES.json',P/'SELECTION.json',P/'guide.py',P/'audit.py',P/'training.log',P/'holdout.log',P/'replay_campaign.py'])
files=sorted(set(files))
before={f:sha(f) for f in files}
source=P/'REPLAY_SOURCE_LOCK.json'
dump(source,{'parent':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'binary_sha256':bh,'source_hashes':{rel(f):h for f,h in before.items()},'note':'lock captured before replay of compiled campaign; production source unchanged from original training; test-only addition to contract file'})
before[source]=sha(source)
env=dict(os.environ,VIGILODE_POLY_CANDIDATES=str(P/'CANDIDATES.json'),VIGILODE_POLY_SELECTION=str(P/'SELECTION.json'))
start=now();t=time.monotonic();cmds=[];outputs=[];counts={};fixed=0;adaptive=0
for name,num in [('training',72),('holdout',6)]:
    cmd=[str(binary),name,'--ignored','--exact','--nocapture','--test-threads=1']
    r=subprocess.run(cmd,cwd=ROOT,env=env,capture_output=True,timeout=120)
    p=P/f'{name}_replay.log';p.write_bytes(r.stdout+r.stderr);outputs.append(p);cmds.append({'argv':cmd,'exit_code':r.returncode})
    new=rows(p);old=rows(P/f'{name}.log')
    assert r.returncode==0 and len(new)==len(old)==num
    a={(x['name'],x['candidate']):x for x in old}
    count=0
    for x in new:
        y=a[(x['name'],x['candidate'])]
        assert x['ok'] and y['ok']
        if x['kind']=='kernel':
            v=audit(x['coefficients'],x['clock'],x['report']);count+=1
            assert v['roots']==len(y['report']['roots'])
            assert [(q['kind'],q['multiplicity']) for q in x['report']['roots']]==[(q['kind'],q['multiplicity']) for q in y['report']['roots']]
            if x['candidate']!='Adaptive':
                assert x['report']==y['report'];fixed+=1
            else:adaptive+=1
        else:
            v=audit_trace(x['trace']);count+=v['polynomials']
            assert len(x['events'])==len(x['expected'])==len(y['events'])
            assert max(abs(e['t']-t) for e,t in zip(x['events'],x['expected']))<2e-7
            assert x['rhs']==y['rhs'] and x['accepted']==y['accepted']
            if x['candidate']!='Adaptive':
                assert x['events']==y['events'] and x['trace']==y['trace'];fixed+=1
            else:adaptive+=1
    counts[name]={'cells':num,'exact_polynomial_reports':count}
assert sha(binary)==bh and all(sha(f)==h for f,h in before.items())
result=P/'REPLAY_RESULT.json';dump(result,{'counts':counts,'fixed_route_exact_replays':fixed,'adaptive_recertified_cells':adaptive,'binary_sha256':bh,'scope':'same original certificate; adaptive route order may differ'})
outputs.append(result)
rec={'schema':'mhg-run/1.0','run_id':'m1-polynomial-campaign-replay','project_id':'vigilode','objective':'Re-execute frozen MathHarness-guided polynomial event candidate matrix','assertion_tested':'Complete exact represented-root certificates and native analytic event checks','domain':{'training_cells':72,'holdout_cells':6,'scope':'finite declared polynomial fixtures'},'conventions':{'root':'exact represented (0,1]','timing':'diagnostic; not selection retune','adaptive':'fresh exact certificate every query'},'status':'completed','execution':{'argv':cmds[0]['argv'],'cwd':'.','exit_code':0,'started_at':start,'finished_at':now(),'runtime_seconds':time.monotonic()-t,'subcommands':cmds,'binary_sha256':bh},'environment':{'os':platform.platform(),'architecture':platform.machine(),'software':{'python':platform.python_version(),'mathharness':'3.1.0'}},'inputs':[{'artifact_id':f'i{i}','path':rel(f),'sha256_before':h,'sha256_after':sha(f),'immutable':True} for i,(f,h) in enumerate(before.items())],'outputs':[{'artifact_id':f.name,'path':rel(f),'sha256':sha(f),'role':'result' if f==result else 'log'} for f in outputs],'checks':[{'check_id':'frozen-campaign-replay','verdict':'PASS','observed':counts,'expected':{'training':{'cells':72,'exact_polynomial_reports':186},'holdout':{'cells':6,'exact_polynomial_reports':16}}}],'non_claims':['No physical IVP root enclosure','No general callback or reset support','No claim GP-UCB reduced runs']}
assert counts==rec['checks'][0]['expected'],counts
dump(P/'REPLAY_MANIFEST.json',rec)
print('\n'.join(validate_run_manifest(rec,ROOT,check_artifacts=True)))
print(counts,'fixed',fixed,'adaptive',adaptive)
