"""Descriptive paired analysis of frozen actual native measurements."""
from pathlib import Path
import gzip,json,math,statistics,collections
P=Path(__file__).parent
def read(tag):
 p=P/'portfolio.log'
 if not p.exists():p=P/'portfolio.log.gz'
 with (gzip.open(p,'rt') if p.suffix=='.gz' else p.open())as f:
  return [json.loads(l.split(tag+' ',1)[1])for l in f if tag+' {' in l]
rows=read('MONITOR_RUN')
key=lambda r:(r['case'],r['goal'],r['input'],r['budget'],r['candidate'])
ix={key(r):r for r in rows};assert len(rows)==len(ix)==306
groups=sorted({k[:-1]for k in ix});assert len(groups)==34
names=sorted({r['candidate']for r in rows});assert len(names)==9
summary={}
for c in names:
 a=[r for r in rows if r['candidate']==c]
 assert len(a)==34
 s={'rows':len(a),'cold_admitted':sum(r['cold']['qualified']for r in a),'repeat_admitted':sum(r['warm']['qualified']for r in a),'pilot_admitted':sum(r['pilot_work']['accepted']for r in a)}
 for phase in ['cold','warm']:
  s[phase+'_rhs']=sum(r[phase]['work']['counters']['rhs_calls']for r in a)
  s[phase+'_actions']=sum(r[phase]['work']['adjoint_actions']for r in a)
  s[phase+'_monitor_actions']=sum(r[phase]['work']['monitor_adjoint_actions']for r in a)
  s[phase+'_zeroheap']=sum(sum(r[phase]['allocation'][k]for k in ['alloc','zeroed','realloc'])==0 for r in a)
 for field in ['splits','merges','pilot_packets','pilot_retained','replay_rounds']:
  s[field]=sum(r['cold']['work'][field]for r in a)
 pair=collections.Counter()
 for g in groups:
  x,y=ix[(*g,c)],ix[(*g,'Time8')]
  n=len(x['initial']);a,b=x['cold'],y['cold']
  pair['different_partition']+=a['times']!=b['times']
  pair['different_endpoint']+=a['states'][-n:]!=b['states'][-n:]
  for k,v,w in [('rhs',a['work']['counters']['rhs_calls'],b['work']['counters']['rhs_calls']),('upper',a['work']['max_signed_error_upper'],b['work']['max_signed_error_upper'])]:
   pair[k+('_lower' if v<w else '_equal' if v==w else '_higher')]+=1
 s['vs_time8']=pair
 summary[c]=s
responses=read('RESPONSE_RUN');assert len(responses)==12
resp=collections.defaultdict(list)
for r in responses:
 w=r['work'];resp[r['case']].append({'scale':r['scale'],'upper':w['max_signed_error_upper'],'rhs':w['counters']['rhs_calls'],'admitted':w['accepted']})
for a in resp.values():
 a.sort(key=lambda r:-r['scale'])
 for i,r in enumerate(a):
  if i:r['upper_log_slope']=math.log(r['upper']/a[i-1]['upper'])/math.log(r['scale']/a[i-1]['scale'])
o={'authority':'descriptive raw native log aggregation; signed and absolute upper bounds from actual solver; independent reference in separate verifier','summary':summary,'response':resp}
(P/'POSTHOC_ANALYSIS.json').write_text(json.dumps(o,indent=2,sort_keys=True)+'\n')
print(json.dumps(o,sort_keys=True))
