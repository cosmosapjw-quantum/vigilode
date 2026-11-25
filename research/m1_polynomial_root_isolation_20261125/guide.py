"""Unmodified MathHarness route guidance, grounded in actual Rust candidates."""
from pathlib import Path
import json,sys,random,statistics,itertools
from mathharness_general.search.primitives import bo_maximize
P=Path(__file__).resolve().parent
def dump(n,x):(P/n).write_text(json.dumps(x,indent=2,sort_keys=True,allow_nan=False)+'\n')
def generate():
 r=json.loads((P/'HARNESS_ROUTES.json').read_text())
 ids={a['candidate_id']:a for a in r['routes']}
 # Explicit candidate composition: every declared module is actually called
 # by the Rust path when its degree/representation conditions admit it.
 axes={'screens':['none','native','dictionary'],'hints':['none','companion']}
 mappings={'R001':'directed interval admissible screen','R002':'exact GCD and Sturm count','R005':'faer companion hint','R049':'native or dictionary Bernstein subdivision','R042':'original polynomial authority','R045':'held-out adversarial controls'}
 routes=[]
 for screen,hint in itertools.product(*axes.values()):
  if screen=='native' and hint=='companion':continue # not implemented this node
  key={('none','none'):'Sturm',('none','companion'):'Companion',('native','none'):'NativeBernstein',('dictionary','none'):'DictionaryBernstein',('dictionary','companion'):'DictionaryCompanion'}[(screen,hint)]
  routes.append({'id':key,'screen':screen,'hint':hint,'verifier':'exact-sturm','modules':(['affine_event_certificate','directed'] if screen=='native' else ['retro_dictionary','conditioned_polynomial','directed'] if screen=='dictionary' else [])+(['faer'] if hint=='companion' else [])})
 routes.append({'id':'Adaptive','screen':'adaptive','hint':'adaptive','verifier':'same exact count','modules':['retro_portfolio','retro_dictionary','affine_event_certificate','conditioned_polynomial','faer']})
 dump('CANDIDATES.json',{'routes':routes,'catalog_mappings':[{**ids[k],'mapping':v} for k,v in mappings.items()],'excluded':[{'screen':'native','hint':'companion','reason':'not implemented in this bounded slice'},{'module':'warp_cubic/retro_green/retro_temporal','reason':'different mathematical contract; not root authority'}],'status':'PROPOSAL_ONLY'})
 print('MathHarness executable route combinations:',len(routes))
def select():
 d=json.loads((P/'TRAINING_AUDIT.json').read_text())
 routes=json.loads((P/'CANDIDATES.json').read_text())['routes']
 rows=d['rows']
 groups=sorted({r['group'] for r in rows});out={}
 for group in groups:
  vals=[]
  for c in routes:
   rs=[r for r in rows if r['group']==group and r['candidate']==c['id']]
   assert rs and len(rs)==sum(r['group']==group for r in rows)//len(routes)
   vals.append({'route':c['id'],'qualified':all(r['audit_pass'] for r in rs),'cells':len(rs),'cost_ns':sum(r['median_ns'] for r in rs),'variation_calls':sum(r['variation_calls'] for r in rs),'screens':sum(r['screened'] for r in rs),'dictionary_uses':sum(r['dictionary_uses'] for r in rs)})
  good=[v for v in vals if v['qualified']]
  assert good
  best=min(good,key=lambda v:(v['cost_ns'],v['route']))
  visited=[]
  def f(x):
   i=min(len(vals)-1,max(0,round(x[0])));v=vals[i];visited.append(v['route'])
   return -float(v['cost_ns']) if v['qualified'] else -1e30
  x,score,n=bo_maximize(f,[0.],[float(len(vals)-1)],random.Random(1729),n_init=6,n_iter=12,x0=[0.])
  bo=vals[min(len(vals)-1,max(0,round(x[0])))]
  out[group]={'routes':vals,'selected':best['route'],'bo_proposal':bo['route'],'bo_calls':n,'bo_matches_exhaustive':bo['route']==best['route'],'visited':visited}
 dump('SELECTION.json',{'groups':out,'selection':'min summed warm median ns of complete certified candidates per declared fixture group','timing_authority':'diagnostic only; finite train set, frozen before holdout','bo':'unmodified MathHarness GP-UCB on already measured discrete lookup; no claim of savings'})
 print({k:(v['selected'],v['bo_proposal']) for k,v in out.items()})
if __name__=='__main__':{'generate':generate,'select':select}[sys.argv[1]]()
