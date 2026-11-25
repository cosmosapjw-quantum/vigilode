"""Map genuine MathHarness proposals to implemented routes; measured-cost choice."""
import json,random,sys
from pathlib import Path
from mathharness_general.search.primitives import bo_maximize
P=Path(__file__).resolve().parent
def dump(n,x):(P/n).write_text(json.dumps(x,indent=2,sort_keys=True,allow_nan=False)+'\n')
def generate():
 d=json.loads((P/'HARNESS_ROUTES.json').read_text());ids={v['candidate_id']:v for v in d['routes']}
 mapping={'R001':'directed interval Jacobian and contraction','R002':'exact scalar root count','R005':'actual factor/linear solve routes','R021':'original constraint residual and derivative bounds','R042':'fixed differential state, immutable source, branch authority','R045':'rank loss and ambiguity controls','R049':'dictionary Bernstein screen with original exact root authority'}
 routes=[('Direct',['direct_solve']),('Reuse',['LuFactorization','LuSolveWorkspace']),('FaerBatch',['LuFactorization::solve_rows']),('PreparedSerial',['retro_foundation::PreparedRhs','SerialReuse','ExactSupport']),('PreparedBatch',['retro_foundation::PreparedRhs','FaerBatch','ExactSupport']),('PreparedRayon',['retro_foundation::PreparedRhs','RayonChunks','ExactSupport']),('ScalarRoot',['retro_event_poly::frozen_scalar/isolate','retro_dictionary','conditioned_polynomial','direct_solve']),('Adaptive',['retro_portfolio::choose','all compatible original-contract routes'])]
 dump('CANDIDATES.json',{'routes':[{'id':k,'modules':v} for k,v in routes],'mappings':[{**ids[k],'mapping':v} for k,v in mapping.items()],'authority':'all candidates use same interval regularity and derivative verification; WARP not a generic algebraic nonlinear root solver'})
 print('generated',len(routes))
def select():
 d=json.loads((P/'TRAINING_AUDIT.json').read_text());c=json.loads((P/'CANDIDATES.json').read_text())['routes'];out={}
 for g in sorted({r['group'] for r in d['rows']}):
  out[g]={}
  for key,tight in [('loose',False),('tight',True)]:
   vals=[]
   for v in c:
    rs=[r for r in d['rows'] if r['group']==g and r['candidate']==v['id'] and r['tight']==tight];assert rs
    vals.append({'route':v['id'],'qualified':all(r['qualified'] for r in rs),'cases':len(rs),'pass_count':sum(r['qualified'] for r in rs),'cost_ns':sum(r['median_ns'] for r in rs)})
   good=[v for v in vals if v['qualified']];best=min(good,key=lambda v:(v['cost_ns'],v['route'])) if good else None
   visited=[]
   def obj(x):
    v=vals[min(len(vals)-1,max(0,round(x[0])))];visited.append(v['route'])
    return -float(v['cost_ns']) if v['qualified'] else -1e30+v['pass_count']*1e20
   x,score,n=bo_maximize(obj,[0.],[float(len(vals)-1)],random.Random(1729),n_init=8,n_iter=16,x0=[0.])
   bo=vals[min(len(vals)-1,max(0,round(x[0])))]
   out[g][key]={'selected':best['route'] if best else None,'bo_proposal':bo['route'] if bo['qualified'] else None,'matches_exhaustive':(best is not None and best['route']==bo['route']) or (best is None and not bo['qualified']),'calls':n,'visited':visited,'routes':vals}
   print(g,key,out[g][key]['selected'],[(v['route'],v['pass_count'],v['cases']) for v in vals])
 dump('SELECTION.json',{'groups':out,'method':'same-contract branch/residual/derivative/rank checks first, then measured whole-init cost; GP-UCB on measured finite lookup','holdout':'NOT_RUN'})
if __name__=='__main__':{'generate':generate,'select':select}[sys.argv[1]]()
