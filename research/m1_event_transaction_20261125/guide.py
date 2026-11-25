"""MathHarness proposal and measured-cost guidance for a finite event portfolio.
The search routines are used without modifying the protected harness."""
import itertools,json,pathlib,random,sys,statistics,math
from mathharness_general.exploration import explore
from mathharness_general.search.primitives import bo_maximize
P=pathlib.Path(__file__).resolve().parent
def dump(name,v): (P/name).write_text(json.dumps(v,indent=2,sort_keys=True,allow_nan=False)+'\n')
def generate():
    routes=json.loads((P/'HARNESS_ROUTES.json').read_bytes())
    byid={x['candidate_id']:x for x in routes['routes']}
    # Translate catalog ideas to project-compatible executable controls.
    # Proposal status is retained: no mathematical route is a solver proof.
    mapping={
      'R001':{'axis':'subdivisions','values':[2,4,8],'translation':'sampled subinterval coverage; not a certified interval enclosure'},
      'R004':{'axis':'root_method','values':['Illinois','Bisection'],'translation':'sign-changing bracket refinement; monotonicity not inferred'},
      'R045':{'axis':'max_step','values':[None,0.1],'translation':'probe hidden crossings and coarse proposal counterexamples'},
      'R021':{'axis':'acceptance','translation':'analytic time/state error and actual callback counts'},
      'R048':{'axis':'scope','translation':'refuse unsupported mass/sequential paths; no arbitrary-root completeness'},
    }
    assert all(i in byid for i in mapping)
    candidates=[{'id':f'C{i:02d}','subdivisions':n,'root_method':r,'max_step':h} for i,(n,r,h) in enumerate(itertools.product(mapping['R001']['values'],mapping['R004']['values'],mapping['R045']['values']),1)]
    dump('CANDIDATES.json',{'schema':'m1.event.candidates.v1','harness_routes':[{**byid[k],'mapping':v} for k,v in mapping.items()],'candidates':candidates,'status':'PROPOSAL_ONLY','selection':'reject failed expected root/time/transaction contracts, minimize measured RHS+event calls on training, holdout untouched'})
    print('Generated',len(candidates),'actual Rust policy combinations from',len(mapping),'catalog mappings')
def select():
    c=json.loads((P/'CANDIDATES.json').read_text())['candidates']
    rows=[json.loads(l.split('CELL ',1)[1]) for l in (P/'training.log').read_text().splitlines() if 'CELL {' in l]
    assert len(rows)==(len(c)+1)*6*2,len(rows)
    assert len({(r['candidate'],r['case'],r['rtol']) for r in rows})==len(rows)
    totals=[]
    for p in c:
        rs=[r for r in rows if r['candidate']==p['id']]
        assert len(rs)==12
        valid=all(r['pass'] for r in rs)
        totals.append({**p,'qualified':valid,'pass_count':sum(r['pass'] for r in rs),'rhs':sum(r['rhs'] for r in rs),'event_calls':sum(r['event_calls'] for r in rs),'cost':sum(r['rhs']+r['event_calls'] for r in rs),'failures':[{k:r[k] for k in ['case','rtol','expected','roots','error']} for r in rs if not r['pass']]})
    qualified=[r for r in totals if r['qualified']]
    assert qualified,'no qualified candidate'
    best=min(qualified,key=lambda r:(r['cost'],r['id']))
    def decode(x):return 4*min(2,max(0,round(x[0])))+2*min(1,max(0,round(x[1])))+min(1,max(0,round(x[2])))
    seen=[]
    def obj(x):
        r=totals[decode(x)];seen.append(r['id'])
        return -float(r['cost']) if r['qualified'] else -1e9+float(r['pass_count'])
    x,v,n=bo_maximize(obj,[0.,0.,0.],[2.,1.,1.],random.Random(1729),n_init=8,n_iter=16,x0=[1.,0.,1.])
    proposed=totals[decode(x)]
    pareto=[r['id'] for r in qualified if not any(a['rhs']<=r['rhs'] and a['event_calls']<=r['event_calls'] and (a['rhs']<r['rhs'] or a['event_calls']<r['event_calls']) for a in qualified)]
    simple=[r for r in rows if r['case'] in ['harmonic-terminal','harmonic-crossings','decay','stiff-decay','earliest-ties']]
    simple_totals=[(sum(q['rhs']+q['event_calls'] for q in simple if q['candidate']==p['id']),p['id']) for p in c if all(q['pass'] for q in simple if q['candidate']==p['id'])]
    result={'schema':'m1.event.selection.v1','training_cells':len(rows),'legacy_rows':sum(r['candidate']=='legacy' for r in rows),'candidate_rows':len(rows)-12,'candidates':totals,'bo_guidance':{'method':'MathHarness GP-UCB bo_maximize on measured lookup; categorical rounding, not an online reduction in runs','seed':1729,'calls':n,'visited':seen,'proposal':proposed['id'],'score':v,'matches_exhaustive':proposed['id']==best['id']},'pareto':pareto,'selected':best,'simple_case_cost_minimum':min(simple_totals),'objective':'unit cost per RHS and event callback; a proxy, not wall time','holdout':'NOT_RUN'}
    dump('SELECTION.json',result)
    print('Qualified',len(qualified),'of',len(c),'BO proposal',proposed['id'],'exhaustive selected',best['id'],'cost',best['cost'],'Pareto',pareto)
if __name__=='__main__':
    {'generate':generate,'select':select}[sys.argv[1]]()
