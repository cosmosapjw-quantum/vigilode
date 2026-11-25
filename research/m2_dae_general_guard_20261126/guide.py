from pathlib import Path
import hashlib,json,zipfile,sys,random,itertools
P=Path(__file__).parent
H=Path('/home/cosmosapjw/vigilode-preserved/20261125-final-preflight/harness/mathharness-general-3.1.0')
Z=Path('/home/cosmosapjw/Dropbox/mathharness-general-3.1.0.zip')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def dump(name,obj):(P/name).write_text(json.dumps(obj,indent=2,sort_keys=True,allow_nan=False)+'\n')
assert sha(Z)=='937ddfd13e860162e6bf454d0b3178c64d34a59de5c0d1df98dc513020f678ee'
with zipfile.ZipFile(Z) as z:
    manifest=json.loads(z.read('mathharness-general-3.1.0/SHA256SUMS.json'))
    for name,d in manifest['files'].items():
        f=(H/name).resolve();assert H in f.parents and sha(f)==d,name
sys.path.insert(0,str(H))
from mathharness_general.exploration import explore
from mathharness_general.search.primitives import UCT,MapElites
from mathharness_general.ledger import Ledger,fresh_verify
from mathharness_general.poly_exact import parse_polynomial
axes=[
 {'Mass':['native mass Rosenbrock','native dense D'],'Reduced5':['Schur reduction','RETRO dictionary Lagrange5'],'Prepared5':['PreparedRhs','dictionary Lagrange5'],'Scalar5':['ScalarRoot','DictionaryCompanion','dictionary Lagrange5'],'WarpFolded':['TensorPencil Folded','conditioned polynomial','dictionary Lagrange5']},
 {'IntervalNewton':['directed interval physical Newton'],'DictionaryHint':['retro_event_poly DictionaryCompanion','physical interval Newton']},
 {'Tangent':['original tangent residual and inverse defect'],'Schur':['slab C inverse defect','reduced covector'],'PreparedSchur':['PreparedRhs PreparedBatch','recomputed interval inverse defect','reduced covector']},
 {'Box':['local preconditioned constraint residual','independent interval state box'],'Transport':['composed guard polynomial','original constraint residual','reduced guard covector']}
]
fields=['solver','root_route','derivative','value']
if sys.argv[1]=='generate':
    r=explore('physical index-one DAE polynomial guard constraint-manifold Schur cancellation, tangent enclosure, prepared RHS preconditioner, residual-corrected guard transport, RETRO dictionary hints, causal Green tube, WARP pencil, verified interval Newton candidate composition',domain='numerical_analysis',max_routes=64)
    dump('HARNESS_ROUTES.json',r)
    candidates=[dict(zip(fields,ks),modules=sum((d[k] for d,k in zip(axes,ks)),[])+['retro_dae_evolution','retro_causal','retro_green','directed::Interval']) for ks in itertools.product(*axes)]
    dump('CANDIDATES.json',{'routes':candidates,'manifest_files':len(manifest['files']),'harness_catalog_count':r['route_count']})
    l=Ledger(P/'exact_ledger.json')
    for cid,lhs,rhs in [
        ('SCHUR','gz*(ht+hx*f)-hz*(gt+gx*f)','gz*ht-hz*gt+(gz*hx-hz*gx)*f'),
        ('CANCEL','x+(t-x)-q','t-q'),
        ('PENCIL','t+t*t/2+(t/4-5/8)*(1+t/8)','17*t*t/32+75*t/64-5/8')
    ]:
        if cid=='PENCIL': lhs,rhs='512*t+256*t*t+(16*t-40)*(8+t)','272*t*t+600*t-320'
        l.add_claim(cid,lhs+' = '+rhs,source={'scope':'finite algebraic subcontracts; not general interval proof'},formal_statement=f'POLY_IDENTITY: {parse_polynomial(lhs)} = {parse_polynomial(rhs)}')
        assert l.record_certificate(cid,'identity',{'lhs':lhs,'rhs':rhs})['status']=='PROVED_EXACT_CERTIFICATE'
    v=fresh_verify(P/'exact_ledger.json',strict=True,timeout=60);assert v['ok'];dump('EXACT_CHECKS.json',v)
    print('manifest',len(manifest['files']),'catalog',r['route_count'],'candidates',len(candidates),'exact',v)
elif sys.argv[1]=='select':
    data=json.loads((P/'AUDIT.json').read_text())
    out=[]
    for case in sorted({v['case'] for v in data['rows']}):
        rows=[r for r in data['rows'] if r['case']==case]
        lookup={tuple(v[f] for f in fields):v for v in rows}
        assert len(lookup)==60
        def key(v):return (v['rhs_proxy'],v['guard_evals'],v['hint_calls'],v['root_width'],tuple(v[f] for f in fields))
        def choices(path):return list(axes[len(path)]) if len(path)<len(axes) else []
        good=[r for r in rows if r['qualified']]
        best=min(good,key=key,default=None)
        tree=UCT(choices);archive=MapElites();rng=random.Random(1729);visited=set()
        for _ in range(256):
            path=tree.propose(rng);v=lookup[path];visited.add(path)
            reward=1./(1.+v['rhs_proxy']) if v['qualified'] else -1.
            tree.backup(path,reward)
            if v['qualified']:archive.insert(v['solver']+'/'+v['derivative']+'/'+v['value'],reward,v)
        best_uct=min((lookup[p] for p in visited if lookup[p]['qualified']),key=key,default=None)
        def obj(v):return (v['rhs_proxy'],v['guard_evals'],v['hint_calls'],v['root_width'])
        pareto=[v for v in good if not any(all(a<=b for a,b in zip(obj(w),obj(v))) and obj(w)!=obj(v) for w in good)]
        out.append({'case':case,'selected':best,'qualified':len(good),'uct_best':best_uct,'uct_unique':len(visited),'uct_agrees':best_uct==best,'map_elites':{k:v[1] for k,v in archive.cells.items()},'pareto':pareto})
    dump('SELECTION.json',{'groups':out,'method':'hard physical contract then RHS/guard/hint/width proxy; measured categorical UCT and MAP-Elites; no fewer solver runs claim'})
    for g in out:print(g['case'],g['qualified'],g['uct_unique'],g['uct_agrees'],g['selected'])
