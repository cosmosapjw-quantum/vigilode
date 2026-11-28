"""Postprocess frozen balanced samples; descriptive only."""
from pathlib import Path
import gzip,json,statistics,math
p=Path(__file__).parent
path=p/'arena.log'
if not path.exists():path=Path(str(path)+'.gz')
with (gzip.open(path,'rt') if path.suffix=='.gz' else path.open()) as f:
 rows=[json.loads(l.split('FLAT_IVP ',1)[1])for l in f if 'FLAT_IVP {' in l]
ix={(r['case'],r['schedule'],r['partial'],r['plan'],r['candidate']):r for r in rows}
out={}
for storage in ['Row','Flat']:
 for partial in [False,True]:
  ratios=[];group=[]
  for key,r in ix.items():
   if key[-1]!=storage+'Legacy' or key[2]!=partial:continue
   v=ix[(*key[:-1],storage+'Prevalidated')]
   q=[a/b for a,b in zip(r['samples_ns'],v['samples_ns'])]
   ratios.extend(q);group.append(statistics.median(q))
  out[storage+'/'+str(partial)]={'groups':len(group),'paired_samples':len(ratios),'geomean_legacy_over_prevalidated':math.exp(statistics.mean(map(math.log,ratios))),'median_group_ratio':statistics.median(group),'groups_above_one':sum(x>1 for x in group)}
(p/'TIMING_CONTROLLER.json').write_text(json.dumps(out,indent=2,sort_keys=True)+'\n')
print(json.dumps(out,sort_keys=True))
