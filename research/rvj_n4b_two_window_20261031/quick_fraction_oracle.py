#!/usr/bin/env python3
"""Independent exact-rational bound audit for N4B, all 716 mode-window rows.
Source coefficients, accepted carrier and reported bound are interpreted as
EXACT binary64 rationals; independent of Rust outward arithmetic implementation.
This certifies the analytical recurrence, not the float kernel evaluation.
"""
from fractions import Fraction as F
import csv,math,json
from pathlib import Path
p=Path(__file__).resolve().parent
rows=list(csv.DictReader((p/'evidence/N4B_TWO_WINDOW_NATIVE.csv').open()))
by_key={(int(r['n']),int(r['window']),int(r['j'])):r for r in rows}
assert len(rows)==716 and len(by_key)==716

def ideal(n,h,y,old):
    w=F(1./n);W=n*w;S=max(F(1),W)
    k=[F(.25 if j%2==0 else 1000.) for j in range(n)]
    chi=[F(.05),F(.02)]; qmode=[min(h,1/kj) for kj in k]
    e=max(abs(z[0])+abs(z[1]) for z in y)
    A=sum((w*(abs(z[0])+abs(z[1])) for z in y),F(0))
    tau=(F(.5)*h)**3/math.factorial(3)
    R=max(e*S,A*(1+tau));assert R<=1
    qab=sum((w*qmode[j] for j in range(n)),F(0));b=e**3*S**3*sum(chi)*qab
    row=3*S*sum(chi)*W;q=max(row*Q for Q in qmode);assert q<1
    d=max(old)/(1-q)
    return [3*R**2*qmode[j]*sum(c*(b+A*tau) for c in chi)
            +old[j]/(1+k[j]*h)+qmode[j]*row*d for j in range(n)],q

results=[];violations=[]
for n in [12,24,65,257]:
    initial=[]
    for j in range(n):
        xi=2.*float(j)/float(n-1)-1.
        initial.append((F(.02+xi/500.),F((j%3-1)/200.)))
    state=initial;old=[F(0)]*n
    for window in [1,2]:
        exact,q=ideal(n,F(1,4),state,old)
        worst=F(0);count=0
        for j in range(n):
            rec=by_key[n,window,j]
            raw=F(float(rec['physical_conditional_l1_upper']))
            gap=F(float(rec['float_gap_l1_upper']))
            if raw<exact[j]+gap:violations.append([n,window,j,float(exact[j]+gap-raw)])
            else:count+=1;worst=max(worst,raw-exact[j]-gap)
        results.append({'n':n,'window':window,'included':count,'total':n,'q_exact_represented':str(q),'max_reported_upper':max(float(by_key[n,window,j]['physical_conditional_l1_upper']) for j in range(n))})
        state=[(F(float(by_key[n,window,j]['float_re'])),F(float(by_key[n,window,j]['float_im']))) for j in range(n)]
        old=[F(float(by_key[n,window,j]['physical_conditional_l1_upper'])) for j in range(n)]
receipt={'result':'PASS' if not violations else 'FAIL','n_comparisons':716,'n_violations':len(violations),'first_violations':violations[:10], 'data':results}
(p/'evidence/N4B_EXACT_FRACTION_716.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps(receipt,indent=2))
assert not violations
