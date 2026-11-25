import json,random,sys
from pathlib import Path
from mathharness_general.search.primitives import bo_maximize
P=Path(__file__).resolve().parent
def dump(n,v):(P/n).write_text(json.dumps(v,indent=2,sort_keys=True,allow_nan=False)+'\n')
def generate():
 ids={v['candidate_id']:v for v in json.loads((P/'HARNESS_ROUTES.json').read_text())['routes']}
 mapping={'R001':'uniform interval branch self-map','R005':'actual linear pencil and batch solvers','R021':'original residual and physical transport','R042':'immutable source and native packet authority','R045':'rank loss and hidden dense defects','R049':'offline dictionary reconstruction and Green gain'}
 routes=[('Mass',['native mass','retro_dae_init']),('Reduced3',['Schur ODE','retro_dae_init','retro_dictionary::basis(3)']),('Reduced5',['Schur ODE','retro_dae_init','retro_dictionary::basis(5)']),('Prepared5',['Schur ODE','PreparedRhs/FaerBatch','dictionary basis5']),('WarpDirect',['TensorPencil::DirectBlock','conditioned Horner','original projection','basis5']),('WarpFolded',['TensorPencil::Folded','conditioned Horner','original projection','basis5']),('Scalar5',['retro_event_poly exact branch','dictionary basis5']),('Adaptive',['retro_portfolio','same-contract candidates'])]
 dump('CANDIDATES.json',{'routes':[{'id':k,'modules':v}for k,v in routes],'mappings':[{**ids[k],'mapping':v}for k,v in mapping.items()],'common':['original interval residual','uniform branch certificate','separate physical bootstrap','retro_causal gain','retro_green dictionary']})
 print('generated',len(routes))
def select():
 d=json.loads((P/'TRAINING_AUDIT.json').read_text());c=json.loads((P/'CANDIDATES.json').read_text())['routes'];out={}
 for g in sorted({r['group']for r in d['rows']}):
  out[g]={}
  for key,tight in [('loose',False),('tight',True)]:
   vals=[]
   for v in c:
    rs=[r for r in d['rows']if r['group']==g and r['candidate']==v['id']and r['tight']==tight];assert len(rs)==1
    r=rs[0];vals.append({'route':v['id'],'qualified':r['qualified'],'global_qualified':r['global_qualified'],'cost_ns':r['median_ns']})
   good=[v for v in vals if v['qualified']]
   best=min(good,key=lambda v:(v['cost_ns'],v['route']))if good else None
   seen=[]
   def obj(x):
    v=vals[min(len(vals)-1,max(0,round(x[0])))];seen.append(v['route']);return -float(v['cost_ns'])if v['qualified']else -1e30
   x,score,n=bo_maximize(obj,[0.],[float(len(vals)-1)],random.Random(1729),n_init=8,n_iter=16,x0=[0.])
   bo=vals[min(len(vals)-1,max(0,round(x[0])))]
   out[g][key]={'selected':best['route']if best else None,'bo_proposal':bo['route']if bo['qualified']else None,'matches_exhaustive':(best is not None and best['route']==bo['route'])or(best is None and not bo['qualified']),'routes':vals,'visited':seen,'calls':n}
   print(g,key,out[g][key]['selected'],[(v['route'],v['qualified'],v['global_qualified'])for v in vals])
 dump('SELECTION.json',{'groups':out,'method':'minimum measured cost after uniform branch and independent trajectory accuracy; global certificate separately tracked; GP-UCB finite measured lookup','holdout':'NOT_RUN'})
if __name__=='__main__':{'generate':generate,'select':select}[sys.argv[1]]()
