#!/usr/bin/env python3
"""Independent Fraction audit of the source-REPRSENTED N3R algebraic envelope.

All input coefficients and carriers are interpreted as exact real values of Rust
binary64 data. This proves enclosure of the exact-real ANALYTIC RECURRENCE only.
It does NOT certify the floating Picard/exponential evaluation errors.
"""
from __future__ import annotations
from fractions import Fraction as F
from pathlib import Path
import csv, json, math, os

root=Path(__file__).resolve().parent
path=Path(os.environ.get('N3R_NATIVE_CSV',root/'evidence'/'native_windows_upper.csv'))
with path.open() as f:
    rows=list(csv.DictReader(z for z in f if not z.startswith('#')))
groups: dict[tuple[int,int],list[dict]]={}
for r in rows:
    key=(int(r['n']),int(r['window']))
    groups.setdefault(key,[]).append(r)

# Reconstruct source.f64 exactly, including approximate binary64 1/n and chi.
def initial(n):
    z=[]
    for j in range(n):
        xi=2.0*float(j)/float(n-1)-1.0
        z.append((F(0.02+xi/500.),F(((j%3)-1)/200.)))
    return z

def exact_bound(n,h,y,old,p):
    assert len(y)==n and len(old)==n
    w=F(1.0/n)
    W=n*w
    # Both phi channels have absolute value one, so S_alpha = W exactly.
    mass=max(F(1),W)
    k=[F(0.25) if j%2==0 else F(1000) for j in range(n)]
    chi=[F(.05),F(.02)]
    omega=F(.5)
    Q=[min(h,1/v) if v else h for v in k]
    e=max(abs(re)+abs(im) for re,im in y)
    A=sum((w*(abs(re)+abs(im)) for re,im in y),F(0))
    tau=(omega*h)**(p+1)/math.factorial(p+1)
    R=max(e*mass,A*(1+tau))
    assert R<=1
    # |phi|=1 both channels, for exact upper bound with actual weight representation.
    Qrank=sum((w*Q[j] for j in range(n)),F(0))
    b=e**3*mass**3*sum(chi)*Qrank
    row=3*mass*sum(chi)*W
    q=max(row*Q[j] for j in range(n))
    assert 0<=q<1
    D=max(old)
    supdiff=D/(1-q)
    bounds=[]
    for j in range(n):
        local=3*R*R*Q[j]*sum(c*(b+A*tau) for c in chi)
        transport=old[j]/(1+k[j]*h)+Q[j]*row*supdiff
        bounds.append(local+transport)
    return bounds,q,mass

out={};viol=[];total=0;all_over=[]
for n in sorted(set(key[0] for key in groups)):
    state=initial(n)
    old=[F(0)]*n
    window=[]
    for index in (1,2):
        original, q, mass=exact_bound(n,F(1,4),state,old,2)
        rr=groups[n,index]
        assert len(rr)==n and [int(r['mode']) for r in rr]==list(range(n))
        mode_violations=[]
        for j,r in enumerate(rr):
            observed=F(float(r['ideal_bound']))
            total+=1
            if observed<original[j]:
                viol.append({'n':n,'window':index,'j':j,'rational':str(original[j]),'native':str(observed)})
                mode_violations.append(j)
            else:
                all_over.append(float(observed-original[j]))
        window.append({'window':index, 'modes':n,'q_exact_represented':str(q),
                       'q_nominal_21_400':q==F(21,400),
                       'sum_weights_exact_represented':str(mass),
                       'native_below_exact_count':len(mode_violations),
                       'max_rational_bound':str(max(original)),
                       'max_rational_as_float':float(max(original))})
        old=original
        state=[(F(float(r['re'])),F(float(r['im']))) for r in rr]
    out[n]=window
receipt={'status':'PASS' if not viol else 'FAIL',
         'contract':'represented binary64 model exact-real algebraic recurrence only; not full numerical trajectory',
         'total_mode_window_comparisons':total,'violations':len(viol),
         'max_native_over_exact_float':max(all_over,default=None),
         'first_violations':viol[:8], 'runs':out}
print(json.dumps(receipt,indent=2))
(Path(os.environ.get('N3R_EVIDENCE_DIR',root/'evidence'))/'exact_represented_oracle.json').write_text(json.dumps(receipt,indent=2)+'\n')
if viol: raise SystemExit('FAIL: candidate undershoots exact rational recurrence')
