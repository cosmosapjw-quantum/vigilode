"""Use actual MathHarness catalog plus GP-UCB on measured candidate receipts."""
import pathlib,json,random,sys
from mathharness_general.search.primitives import bo_maximize
P=pathlib.Path(__file__).resolve().parent
def dump(n,x):(P/n).write_text(json.dumps(x,indent=2,sort_keys=True,allow_nan=False)+'\n')
def generate():
 d=json.loads((P/'HARNESS_ROUTES.json').read_text())
 ids={x['candidate_id']:x for x in d['routes']}
 mapping={'R001':'physical tube interval enclosure','R008':'strict total derivative sign and interval Newton','R021':'source residual a posteriori bound','R042':'original dense polynomial and independent source check','R045':'unstructured grazing and initial root refusal','R049':'native Bernstein residual and dictionary moment weighting'}
 routes=[
  ('Unity',['affine_event_certificate::bernstein','directed']),
  ('AffineCarry',['affine_event_certificate::certify/carry']),
  ('Green',['retro_green::RetroTable','retro_causal::gain','affine_event_certificate::bernstein']),
  ('Weighted',['retro_green::RetroTable','retro_causal::gain','retro_dictionary::moment']),
  ('WeightedHint',['retro_green::RetroTable','retro_causal::gain','retro_dictionary::moment','retro_event_poly::isolate']),
  ('SourceAction',['retro_causal::flow','retro_foundation::nilpotent_green','retro_dictionary::moment','retro_green::RetroTable']),
  ('Adaptive',['retro_portfolio::choose','all compatible physical routes'])]
 dump('CANDIDATES.json',{'routes':[{'id':r,'modules':v} for r,v in routes],'mappings':[{**ids[k],'mapping':v} for k,v in mapping.items()],'authority':'original same-contract physical state tube plus derivative and signed endpoints; no conjectural candidate certifies a root'})
 print('generated',len(routes))
def select():
 d=json.loads((P/'TRAINING_AUDIT.json').read_text());c=json.loads((P/'CANDIDATES.json').read_text())['routes'];out={}
 for g in sorted({r['group'] for r in d['rows']}):
  vals=[]
  for v in c:
   r=[r for r in d['rows'] if r['group']==g and r['candidate']==v['id']]
   assert r
   vals.append({'route':v['id'],'qualified':all(x['audit_pass'] for x in r),'cases':len(r),'pass_count':sum(x['audit_pass'] for x in r),'cost_ns':sum(x['median_ns'] for x in r),'max_width':max((x.get('width') or 0 for x in r),default=0)})
  good=[v for v in vals if v['qualified']];assert good,g
  best=min(good,key=lambda x:(x['cost_ns'],x['route']))
  seen=[]
  def obj(x):
   v=vals[min(len(vals)-1,max(0,round(x[0])))];seen.append(v['route'])
   return -float(v['cost_ns']) if v['qualified'] else -1e30+v['pass_count']*1e20
  x,score,n=bo_maximize(obj,[0.],[float(len(vals)-1)],random.Random(1729),n_init=7,n_iter=14,x0=[0.])
  bo=vals[min(len(vals)-1,max(0,round(x[0])))]
  out[g]={'selected':best['route'],'bo_proposal':bo['route'],'matches':bo['route']==best['route'],'calls':n,'visited':seen,'routes':vals}
 dump('SELECTION.json',{'groups':out,'method':'min summed median certification cost after independent same-contract accuracy; GP-UCB guidance on already measured lookup, not run reduction','holdout':'not run at selection'})
 print({g:(r['selected'],r['bo_proposal']) for g,r in out.items()})
if __name__=='__main__':{'generate':generate,'select':select}[sys.argv[1]]()
