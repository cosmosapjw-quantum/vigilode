from pathlib import Path
import hashlib,json,zipfile,sys,random
P=Path(__file__).parent
H=Path('/home/cosmosapjw/vigilode-preserved/20261125-final-preflight/harness/mathharness-general-3.1.0')
Z=Path('/home/cosmosapjw/Dropbox/mathharness-general-3.1.0.zip')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def dump(name,obj):(P/name).write_text(json.dumps(obj,indent=2,sort_keys=True,allow_nan=False)+'\n')
assert sha(Z)=='937ddfd13e860162e6bf454d0b3178c64d34a59de5c0d1df98dc513020f678ee'
with zipfile.ZipFile(Z) as z:
    manifest=json.loads(z.read('mathharness-general-3.1.0/SHA256SUMS.json'))
    for name,d in manifest['files'].items():
        f=(H/name).resolve()
        assert H in f.parents and sha(f)==d,name
sys.path.insert(0,str(H))
from mathharness_general.exploration import explore
from mathharness_general.search.primitives import UCT,MapElites
from mathharness_general.ledger import Ledger,fresh_verify
from mathharness_general.poly_exact import parse_polynomial
pre={'Mass':['native mass Rosenbrock','native dense D'],'Reduced5':['Schur reduction','dictionary Lagrange5'],'Prepared5':['PreparedRhs','dictionary Lagrange5'],'Scalar5':['ScalarRoot','DictionaryCompanion','dictionary Lagrange5'],'WarpFolded':['TensorPencil Folded','conditioned polynomial','dictionary Lagrange5']}
roots={'Bisection':['directed tube signs'],'IntervalNewton':['directed physical velocity','interval Newton'],'DictionaryHint':['retro_event_poly DictionaryCompanion','physical interval Newton']}
sync={'Rectangle':['RetroTable phi0'],'GreenMoment':['RetroTable phi1/phi2','correlated clock'],'CriticalPoint':['RetroTable phi0','derivative root extrema']}
post={'Mass':['native mass restart','retro_dae_init Reuse'],'Reduced5':['Schur restart','dictionary Lagrange5']}
fields=['pre_route','root_route','sync_route','post_route']
if sys.argv[1]=='generate':
    r=explore('accepted step physical scalar DAE threshold, branch contraction, transverse interval Newton, RETRO dictionary root hint, causal Green error transport, affine reset synchronization, critical points, Green moment, WARP time pencil, prepared RHS, measured categorical combination guidance',domain='numerical_analysis',max_routes=64)
    dump('HARNESS_ROUTES.json',r)
    from itertools import product
    candidates=[dict(zip(fields,ks),modules=sum((d[k] for d,k in zip([pre,roots,sync,post],ks)),[])+['retro_dae_evolution','retro_causal','directed::Interval']) for ks in product(pre,roots,sync,post)]
    dump('CANDIDATES.json',{'routes':candidates,'manifest_files':len(manifest['files']),'harness_catalog_count':r['route_count']})
    l=Ledger(P/'exact_ledger.json')
    for cid,lhs,rhs in [('CLOCK-RESET','A+B*(s-u)','A+B*s-B*u'),('DERIVATIVE','B+a*(A+B*t-e)','B+a*(A-e)+a*B*t'),('PENCIL-FORCING','2*x+2*(t-s)+t*t-s*s','2*x+(t-s)*(2+t+s)')]:
        l.add_claim(cid,lhs+' = '+rhs,source={'scope':'finite algebraic subcontracts'},formal_statement=f'POLY_IDENTITY: {parse_polynomial(lhs)} = {parse_polynomial(rhs)}')
        assert l.record_certificate(cid,'identity',{'lhs':lhs,'rhs':rhs})['status']=='PROVED_EXACT_CERTIFICATE'
    v=fresh_verify(P/'exact_ledger.json',strict=True,timeout=60);assert v['ok'];dump('EXACT_CHECKS.json',v)
    print('manifest',len(manifest['files']),'catalog',r['route_count'],'candidate combinations',len(candidates),'exact',v)
elif sys.argv[1]=='select':
    data=json.loads((P/'AUDIT.json').read_text())
    out=[]
    for kind in sorted({v['kind'] for v in data['rows']}):
        rows=[r for r in data['rows'] if r['kind']==kind]
        lookup={tuple(v[f] for f in fields):v for v in rows}
        assert len(lookup)==90
        def key(v):return (v['rhs_proxy'],v['guard_evals'],v['hint_calls'],v['sync_width'],v['bound'],tuple(v[f] for f in fields))
        def choices(path):return list([pre,roots,sync,post][len(path)]) if len(path)<4 else []
        best=min((r for r in rows if r['qualified']),key=key,default=None)
        tree=UCT(choices);archive=MapElites();rng=random.Random(1729);visited=set()
        for _ in range(384):
            path=tree.propose(rng);v=lookup[path];visited.add(path)
            reward=1./(1.+v['rhs_proxy']) if v['qualified'] else -1.
            tree.backup(path,reward)
            if v['qualified']:archive.insert(v['pre_route']+'/'+v['sync_route'],reward,v)
        best_uct=min((lookup[p] for p in visited if lookup[p]['qualified']),key=key,default=None)
        good=[v for v in rows if v['qualified']]
        def obj(v):return (v['rhs_proxy'],v['guard_evals'],v['hint_calls'],v['sync_width'],v['bound'])
        pareto=[v for v in good if not any(all(a<=b for a,b in zip(obj(w),obj(v))) and obj(w)!=obj(v) for w in good)]
        out.append({'kind':kind,'selected':best,'qualified':len(good),'uct_best':best_uct,'uct_unique':len(visited),'uct_agrees':best_uct==best,'map_elites':{k:v[1] for k,v in archive.cells.items()},'pareto':pareto})
    dump('SELECTION.json',{'groups':out,'method':'exact hard contract then RHS/guard/hint/width proxy; UCT measured lookup; MAP-Elites retained; no timing or fewer solver runs claim'})
    for g in out:
        print(g['kind'],g['qualified'],g['uct_unique'],g['uct_agrees'],g['selected'])
