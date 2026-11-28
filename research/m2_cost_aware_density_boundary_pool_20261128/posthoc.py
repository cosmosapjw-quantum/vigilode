"""Read-only analysis of the frozen M2.38 measured data; no simulated solves."""
from pathlib import Path
import json,gzip,collections,statistics,math
P=Path(__file__).parent
def read(marker):
 p=P/'portfolio.log'
 if not p.exists():p=P/'portfolio.log.gz'
 with (gzip.open(p,'rt') if p.suffix=='.gz' else p.open()) as f:
  return [json.loads(l.split(marker+' ',1)[1]) for l in f if marker+' {' in l]
rows=read('DENSITY_RUN')
setup={(r['case'],r['candidate']):r['allocation']['requested_bytes'] for r in read('DENSITY_SETUP')}
timings=read('DENSITY_TIME')
key=lambda r:(r['case'],r['goal'],r['input'],r['budget'],r['candidate'])
ix={key(r):r for r in rows}
assert len(ix)==len(rows)==780
names=sorted({r['candidate'] for r in rows})
groups=sorted({k[:-1] for k in ix})
assert len(names)==13 and len(groups)==60
S=json.loads((P/'INDEPENDENT_CHECKS.json').read_text())
assert set(S['stats'])==set(names)
summ={}
for name in names:
 a=[r for r in rows if r['candidate']==name]
 assert len(a)==60 and all(r['cold']['qualified'] and r['warm']['qualified'] for r in a)
 assert all(r[p]['allocation']['requested_bytes']==0 and r[p]['allocation']['alloc']==0 and r[p]['allocation']['zeroed']==0 and r[p]['allocation']['realloc']==0 for r in a for p in ('cold','warm'))
 d={}
 for label, filt in [('all',lambda r:True),('prior6',lambda r:r['case']!='holdout_sphere800'),('holdout800',lambda r:r['case']=='holdout_sphere800'),('holdout800osc',lambda r:r['case']=='holdout_sphere800' and r['input']=='oscillatory')]:
  b=[r for r in a if filt(r)]
  d[label]={'strata':len(b),'admitted':sum(r['cold']['qualified'] and r['warm']['qualified'] for r in b)}
  for phase in ('cold','warm'):
   d[label][phase+'_actions']=sum(r[phase]['work']['adjoint_actions'] for r in b)
   d[label][phase+'_rhs']=sum(r[phase]['work']['counters']['rhs_calls'] for r in b)
  d[label].update({'splits':sum(r['cold']['work']['splits']for r in b),'merges':sum(r['cold']['work']['merges']for r in b),'retained':sum(r['cold']['work']['pilot_retained']for r in b),
   'nonzero_repeat_strata':sum(r['warm']['work']['adjoint_actions']>0 for r in b),
   'max_budget_fraction':max(r[ph]['work']['max_signed_error_upper']/r['budget']for r in b for ph in ('cold','warm'))})
 assert d['all']['cold_actions']==S['stats'][name]['cold_actions']
 assert d['all']['warm_actions']==S['stats'][name]['warm_actions']
 assert d['all']['cold_rhs']==S['stats'][name]['cold_rhs']
 summ[name]=d
# Paired variation: keep identical input/goal/budget, not just aggregate means.
comparisons={}
for a,b in [('OnlineMotion8','OnlineTime8'),('OnlineMotion12','OnlineMotion8'),('OnlineMotion8','SubsetStride4'),('OnlineTime8','SubsetStride4'),('SubsetCost4','SubsetLru4'),('SubsetPinned6','SubsetFull16')]:
 v={'different_partition':0,'different_endpoint':0,'less_rhs':0,'equal_rhs':0,'more_rhs':0,'less_bound':0,'equal_bound':0,'more_bound':0,'less_warm_actions':0,'equal_warm_actions':0,'more_warm_actions':0}
 examples=[]
 for g in groups:
  r,s=ix[(*g,a)],ix[(*g,b)]
  x,y=r['cold'],s['cold'];n=len(r['initial'])
  v['different_partition']+=x['times']!=y['times'];v['different_endpoint']+=x['states'][-n:]!=y['states'][-n:]
  for field,p,q in [('rhs',x['work']['counters']['rhs_calls'],y['work']['counters']['rhs_calls']),('bound',x['work']['max_signed_error_upper'],y['work']['max_signed_error_upper']),('warm_actions',r['warm']['work']['adjoint_actions'],s['warm']['work']['adjoint_actions'])]:
   v[('less_' if p<q else 'more_' if p>q else 'equal_')+field]+=1
  if x['times']!=y['times']:
   examples.append({'group':list(g),'a_times':x['times'],'b_times':y['times'],'a_rhs':x['work']['counters']['rhs_calls'],'b_rhs':y['work']['counters']['rhs_calls'],'a_bound':x['work']['max_signed_error_upper'],'b_bound':y['work']['max_signed_error_upper']})
 comparisons[a+'_vs_'+b]={**v,'examples':examples[:4]}
# Exact measured timing in held-out vs previous strata, no new timings.
ti={(r['case'],r['goal'],r['candidate']):r for r in timings}
timing_names=sorted({r['candidate'] for r in timings})
tm={}
for name in timing_names:
 d={}
 for part,filt in [('all',lambda k:True),('prior6',lambda k:k[0]!='holdout_sphere800'),('holdout800',lambda k:k[0]=='holdout_sphere800')]:
  rr=[(k,r) for k,r in ti.items() if k[2]==name and filt(k)]
  out={}
  for phase in ('cold','warm'):
   v=[a/b for k,r in rr for a,b in zip(ti[(k[0],k[1],'OldCostFull')][phase+'_ns'],r[phase+'_ns'])]
   assert len(v)==len(rr)*6 and min(v)>0
   out[phase]={'pairs':len(v),'geomean':math.exp(statistics.mean(map(math.log,v))),'median':statistics.median(v),'min':min(v),'max':max(v)}
  d[part]=out
 tm[name]=d
o={'authority':'post-hoc analysis of unchanged M2.38 measured raw logs; no new native solve or timing, no global optimality claim','summaries':summ,'comparisons':comparisons,'timing':tm,'memory_ratio_vs_oldcostfull':{c:{n:setup[c,n]/setup[c,'OldCostFull'] for n in names}for c in sorted({k[0] for k in setup})}}
(P/'POSTHOC_ANALYSIS.json').write_text(json.dumps(o,indent=2,sort_keys=True)+'\n')
print('HOLDOUT',json.dumps({n:summ[n]['holdout800'] for n in names},sort_keys=True))
print('COMPARE',json.dumps({k:{a:b for a,b in v.items()if a!='examples'}for k,v in comparisons.items()},sort_keys=True))
print('HOLDOUT_TIME',json.dumps({k:v['holdout800']for k,v in tm.items()}))
