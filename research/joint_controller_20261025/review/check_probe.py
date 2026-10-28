#!/usr/bin/env python3
"""Independent exact rational oracle for emitted native binary64 results."""
from fractions import Fraction as F
import hashlib,json,struct,sys
from pathlib import Path
def q(word):
    return F.from_float(struct.unpack('>d',bytes.fromhex(word))[0])
def qs(words): return list(map(q,words.split(',')))
source=Path(sys.argv[1]); rows=source.read_text().splitlines(); checks=0; bounds=0; costs=0; preferred=0
for line in rows:
    fields=line.split('|')
    if fields[0]=='B':
        _,idx,b,d,w,e,out,v,a,budget=fields
        b,d,w,e,v,a=map(qs,(b,d,w,e,v,a)); out=q(out);budget=q(budget)
        exact=[]
        for i in range(4):
            exact.append(d[i]+sum(b[4*i+j]*exact[j] for j in range(i)))
            assert e[i]>=exact[i],(idx,'stage',i); checks+=1
        assert out>=sum(wi*ei for wi,ei in zip(w,exact)),(idx,'output'); checks+=1
        vi=[F(0)]*4
        for i in reversed(range(4)):
            vi[i]=w[i]+sum(b[j*4+i]*vi[j] for j in range(i+1,4))
            assert v[i]>=vi[i],(idx,'adjoint',i);checks+=1
        assert sum(vi[i]*a[i] for i in range(4))<=budget,(idx,'allowance'); checks+=1
        bounds+=1
    else:
        _,idx,inc,cand,t,n,gain,chosen=fields
        if chosen=='true':
            assert q(cand)+q(t)/int(n)<=(1-q(gain))*q(inc),(idx,'false positive cost choice')
            preferred+=1;checks+=1
        costs+=1
print(json.dumps({'schema':'vigilode.independent-exact-review.v1','status':'pass_scoped',
    'causal_cases':bounds,'cost_boundary_cases':costs,'native_prefer_candidate_cases':preferred,
    'exact_rational_checks':checks,'raw_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),
    'scope':'Actual native binary64 outputs against exact Fraction recurrences and sufficient-cost inequality; not model-provider correctness or future wall-time coverage'},indent=2))
