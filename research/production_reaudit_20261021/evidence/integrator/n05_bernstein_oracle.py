#!/usr/bin/env python3
"""Independent exact-rational Bernstein bounds for actual represented tableau."""
from fractions import Fraction as Q
from pathlib import Path
import hashlib, json, math
ROOT=Path(__file__).resolve().parents[4]
SNAP=ROOT/'fixtures/rodas5p_coefficients_snapshot.json'
d=json.loads(SNAP.read_text()); parse=lambda x:Q(float(x))
g=parse(d['gamma']); A=[[parse(x) for x in r] for r in d['A']];C=[[parse(x) for x in r] for r in d['C']]
b=list(map(parse,d['b_code']))
def add(a,b):return [(a[i] if i<len(a) else Q(0))+(b[i] if i<len(b) else Q(0)) for i in range(max(len(a),len(b)))]
def linear(p,c,l):
    out=[Q(0)]*(len(p)+1)
    for i,v in enumerate(p):out[i]+=c*v;out[i+1]+=l*v
    return out

def polys(output):
    p=[[v] for v in output]
    for i in reversed(range(8)):
        for j in range(i+1,8):p[i]=add(p[i],linear(p[j],-A[j][i],g*C[j][i]+A[j][i]))
    return [[Q(0)]+v for v in p]
def split(b):
    left=[b[0]];right=[b[-1]]
    while len(b)>1:
        b=[(x+y)/2 for x,y in zip(b,b[1:])];left.append(b[0]);right.append(b[-1])
    return left,list(reversed(right))
def bound(b,depth):
    if not depth:return max(map(abs,b))
    left,right=split(b)
    return max(bound(left,depth-1),bound(right,depth-1))
def bernstein(p):
    n=len(p)-1
    b=[sum((p[j]*Q(math.comb(k,j),math.comb(n,j)) for j in range(k+1)),Q(0)) for k in range(n+1)]
    return bound(b,4)
def cayley(p):
    c=[Q(0)]*len(p)
    for k,v in enumerate(p):
        for j in range(k+1):c[j]+=v*Q(math.comb(k,j),2**k)
    return sum(map(abs,c))
rows={}
for name,output in [('output',b),('embedded',[Q(0)]*7+[Q(1)])]:
    pp=polys(output);bb=list(map(bernstein,pp));cc=list(map(cayley,pp))
    rows[name]={'bernstein_bounds':[float(v) for v in bb],'bernstein_exact':[str(v) for v in bb],
                'cayley_bounds':[float(v) for v in cc],'cayley_exact':[str(v) for v in cc],
                'bernstein_sum':float(sum(bb)),'cayley_sum':float(sum(cc))}
result={'schema':'vigilode.n05.exact_oracle.v1','status':'PASS_EXACT_IDENTITIES',
 'snapshot_sha256':hashlib.sha256(SNAP.read_bytes()).hexdigest(),
 'norm':'fixed Hilbert norm','model':'frozen linear represented tableau',
 'bernstein_depth':4,'arithmetic':'exact rational from binary64 parsed coefficients',
 'rows':rows,'wall_speedup_claim':False}
Path(__file__).with_suffix('.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'status':result['status'],'rows':{k:{q:v for q,v in row.items() if q.endswith('_sum') or q.endswith('_bounds')} for k,row in rows.items()}},indent=2))
