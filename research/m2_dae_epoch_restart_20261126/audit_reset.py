from pathlib import Path
import json,mpmath as mp
P=Path(__file__).parent
mp.mp.dps=85
def m(x):return mp.mpf(float(x))
rows=[json.loads(l.split('RESET ',1)[1])for l in (P/'final_tests.log').read_text().splitlines() if 'RESET {' in l]
assert len(rows)==4 and len({r['route'] for r in rows})==4
n=0;rat=mp.mpf(0)
for r in rows:
 c,rad=m(r['center']),m(r['radius'])
 lo,hi=map(m,r['sync_box']);assert c-rad<=lo<=hi<=c+rad
 for i in range(257):
  tau=mp.mpf(1)/2-mp.mpf(1)/128+mp.mpf(i)/(128*256)
  v=(mp.mpf('1.5')-tau)*mp.exp(-(mp.mpf(1)/2-tau))
  assert lo<=v<=hi
  for q in r['queries']:
   t=m(q['t'])
   x=(mp.mpf('1.5')-tau)*mp.exp(-(t-tau));z=3*x-2*t+1
   err=max(abs(x-m(q['state'][0])),abs(z-m(q['state'][1])))
   b=m(q['physical_error_upper']);assert err<=b,(r['route'],i,t,err,b)
   n+=1
   if b>0:rat=max(rat,err/b)
out={'rows':[{'route':r['route'],'sync_box':r['sync_box'],'radius':r['radius'],'max_bound':r['max_bound'],'accepted_count':r['accepted_count']}for r in rows],'checks':n,'max_ratio':float(rat),'scope':'supplied physical root bracket; no root detection or accepted-step rollback'}
(P/'RESET_AUDIT.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps(out,indent=2))
