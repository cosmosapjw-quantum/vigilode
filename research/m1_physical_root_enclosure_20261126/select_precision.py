from pathlib import Path
import json,random
from mathharness_general.search.primitives import bo_maximize
P=Path(__file__).resolve().parent
d=json.loads((P/'PRECISION_AUDIT.json').read_text());out={}
c=json.loads((P/'CANDIDATES.json').read_text())['routes']
for g in sorted({r['group'] for r in d['rows']}):
 out[g]={}
 for key,t in [('1e-8',1e-8),('1e-10',1e-10)]:
  vals=[]
  for v in c:
   rows=[x for x in d['rows'] if x['group']==g and x['time_tol']==t and x['candidate']==v['id']]
   assert rows
   vals.append({'route':v['id'],'qualified':all(r['audit_pass'] for r in rows),'cases':len(rows),'pass_count':sum(r['audit_pass'] for r in rows),'cost_ns':sum(r['median_ns'] for r in rows),'max_width':max((r.get('width') or 0 for r in rows),default=0)})
  good=[v for v in vals if v['qualified']]
  best=min(good,key=lambda v:(v['cost_ns'],v['route'])) if good else None
  visited=[]
  def obj(x):
   v=vals[min(len(vals)-1,max(0,round(x[0])))];visited.append(v['route'])
   return -float(v['cost_ns']) if v['qualified'] else -1e30+v['pass_count']*1e20
  x,score,n=bo_maximize(obj,[0.],[float(len(vals)-1)],random.Random(1729),n_init=7,n_iter=14,x0=[0.])
  bo=vals[min(len(vals)-1,max(0,round(x[0])))]
  out[g][key]={'selected':best['route'] if best else None,'bo_proposal':bo['route'] if bo['qualified'] else None,'matches':(best is not None and best['route']==bo['route']) or (best is None and not bo['qualified']),'routes':vals,'calls':n,'visited':visited}
  print(g,key,out[g][key]['selected'],[(v['route'],v['pass_count'],v['cases']) for v in vals])
(P/'PRECISION_SELECTION.json').write_text(json.dumps({'groups':out,'status':'frozen before tight holdout','method':'same exact physical contract, groupwise measured cost, no qualification by GP-UCB alone'},indent=2,sort_keys=True)+'\n')
