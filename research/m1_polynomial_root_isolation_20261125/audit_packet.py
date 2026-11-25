"""Independent exact Fraction composition using native H/stage rows."""
import json
from fractions import Fraction as F
from pathlib import Path
from audit import audit
P=Path(__file__).resolve().parent
def f(x):return F.from_float(float(x))
def trim(v):
    while v and v[-1]==0:v.pop()
    return v
def add(a,b):
    v=[F(0)]*max(len(a),len(b))
    for i,x in enumerate(a):v[i]+=x
    for i,x in enumerate(b):v[i]+=x
    return trim(v)
def scale(a,b):return trim([x*b for x in a])
def mul(a,b):
    if not a or not b:return []
    v=[F(0)]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b):v[i+j]+=x*y
    return trim(v)
rows=[json.loads(l.split('PACKET ',1)[1]) for l in (P/'final_facade.log').read_text().splitlines() if 'PACKET {' in l]
assert len(rows)==3
result=[]
for r in rows:
    t0,t1=map(f,r['times']);h=t1-t0
    n=r['n'];u=r['stage_rows'];H=r['H'];pol=[]
    for k in range(n):
        d=[sum(f(H[i][j])*f(u[j*n+k]) for j in range(8)) for i in range(3)]
        a,b=f(r['y0'][k]),f(r['y1'][k])
        pol.append(trim([a,b-a+d[0],d[1]-d[0],d[2]-d[1],-d[2]]))
    out=[]
    for term in r['declaration']['terms']:
        q=[f(term['coefficient'])]
        for g in term['factors']:
            v=[f(g['constant'])+f(g['time'])*t0,f(g['time'])*h]
            for i,w in g['state']:v=add(v,scale(pol[i],f(w)))
            q=mul(q,v)
        out=add(out,q)
    assert out==list(map(F,r['coefficients']))
    assert [t0,h]==list(map(F,r['clock']))
    a=audit(r['coefficients'],r['clock'],r['report'])
    result.append({'index':r['index'],'coefficient_count':len(out),'exact_native_composition':True,'roots':a['roots']})
(P/'PACKET_AUDIT.json').write_text(json.dumps(result,indent=2)+'\n')
print('Native Robertson exact coefficient composition',len(result),'/',len(rows),'roots',sum(x['roots'] for x in result))
