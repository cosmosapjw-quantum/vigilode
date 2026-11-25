import json
from pathlib import Path
P=Path(__file__).resolve().parent
counts={}
for name in ['training','holdout','precision','precision_holdout']:
    rows=[json.loads(l.split('CELL ',1)[1]) for l in (P/(name+'.log')).read_text().splitlines() if 'CELL {' in l]
    n=0
    for r in rows:
        if r['report'] is None:continue
        q=r['report']
        for p in q['proofs']:
            signs=p['endpoint_signs'];lo,hi=p['derivative'];gl,gh=p['event_range'];strict=lo>0 or hi<0
            if p['disposition']=='NoRoot':assert gl>0 or gh<0 or (strict and signs[0]!=0 and signs[0]==signs[1])
            if p['disposition']=='Unique':assert strict and signs[0]*signs[1]==-1
            if p['disposition']=='Unresolved':assert not (gl>0 or gh<0 or (strict and signs[0]!=0 and signs[0]==signs[1]) or (strict and signs[0]*signs[1]==-1))
            n+=1
    counts[name]=n
(P/'DISPOSITION_AUDIT.json').write_text(json.dumps({'counts':counts,'status':'PASS','scope':'proof-record logical admission, not independent analytic proof'},indent=2)+'\n')
print(counts)
