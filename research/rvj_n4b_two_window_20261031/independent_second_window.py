#!/usr/bin/env python3
"""Independent exact-represented source checks, N4B second window.
No N4B/Rust interval code called. Only mathematical formulae inherited from
N3R exact_represented_oracle and N4A independent analytic moment derivation.
Exact Fraction audit: native upper of CONDITIONAL analytic second-window error.
High-precision independent analytic exp-moment: enclosed all second-window Picard values.
"""
from __future__ import annotations
import math, csv, json, os, time
from fractions import Fraction as F
from collections import defaultdict
from pathlib import Path
import mpmath as mp
root=Path(__file__).resolve().parent
source_rows=list(csv.DictReader((root/'evidence/N4B_TWO_WINDOW_NATIVE.csv').open()))
by_key={(int(x['n']),int(x['window']),int(x['j'])):x for x in source_rows}
assert len(source_rows)==716 and len(by_key)==716

def exact_bound(n,h,y,old,p):
    w=F(1./n); mass=max(F(1),n*w)
    k=[F(.25 if j%2==0 else 1000.) for j in range(n)]
    chi=[F(.05),F(.02)];omega=F(.5)
    qmode=[min(h,1/kj) for kj in k]
    e=max(abs(re)+abs(im) for re,im in y)
    a=sum((w*(abs(re)+abs(im)) for re,im in y),F(0))
    tau=(omega*h)**(p+1)/math.factorial(p+1)
    r=max(e*mass,a*(1+tau));assert r <= 1
    qab=sum((w*qmode[j] for j in range(n)),F(0))
    beta=e**3*mass**3*sum(chi)*qab
    row=3*mass*sum(chi)*(n*w)
    q=max(row*x for x in qmode);assert q<1
    oldsup=max(old)/(1-q)
    ans=[]
    for j in range(n):
        local=3*r*r*qmode[j]*sum(c*(beta+a*tau) for c in chi)
        decay=old[j]/(1+k[j]*h)
        feedback=qmode[j]*row*oldsup
        ans.append(local+decay+feedback)
    return ans,q

def mpf_f64(x):
    a,b=float(x).as_integer_ratio();return mp.mpf(a)/b

def poly_mul(a,b):
    out=[mp.mpc(0)]*(len(a)+len(b)-1)
    for j,x in enumerate(a):
        for k,y in enumerate(b):out[j+k]+=x*y
    return out

def cm_f64(re,im):return mp.mpc(mpf_f64(re),mpf_f64(im))

def analytic_moment(a,b,theta,h,l):
    # Independent analytic recurrence. The fast negative exponent is kept exact.
    z=(a-b-1j*theta)*h; w=(a-b)-1j*theta
    if abs(z) < 4:
        total=mp.mpc(0);factor=mp.mpc(1)
        for k in range(400):
            v=factor*h**(l+1)/(l+k+1);total+=v
            factor*=z/(k+1)
            if abs(v) < mp.mpf('1e-165')*max(abs(total),mp.mpf('1e-210')) and k>l+12: break
    else:
        total=mp.expm1(z)/w
        for q in range(1,l+1):total=(h**q*mp.exp(z)-q*total)/w
    return mp.exp((-a+1j*theta)*h)*total

def ideal_second(n, precision):
  with mp.workdps(precision):
    y=[cm_f64(by_key[n,1,j]['float_re'],by_key[n,1,j]['float_im']) for j in range(n)]
    w=mpf_f64(1./n); h=mpf_f64(.25); omega=mpf_f64(.5)
    chi=[mpf_f64(.05),mpf_f64(.02)]
    # Reconstruct native xi rounded to f64 exactly before interpreting as real.
    xi=[mpf_f64(2.*float(j)/float(n-1)-1.) for j in range(n)]
    theta=[omega*x for x in xi];kappa=[mpf_f64(.25 if j%2==0 else 1000.) for j in range(n)]
    def phi(alpha,j):return mp.mpf(1 if alpha==0 or j%2==0 else -1)
    poly=[[[mp.mpc(0)]*3 for _ in range(2)] for _ in range(2)]
    for alpha in range(2):
      for j in range(n):
        cluster=j%2; base=w*phi(alpha,j)*y[j]
        for l in range(3):poly[alpha][cluster][l]+=base*(1j*theta[j])**l/mp.factorial(l)
    triples=[]
    for alpha in range(2):
      for c in range(2):
       for d in range(2):
        for e in range(2):
          co=poly_mul(poly_mul(poly[alpha][c],[mp.conj(q) for q in poly[alpha][d]]),poly[alpha][e]);rate=mp.mpf([.25,1000.][c])+mp.mpf([.25,1000.][d])+mp.mpf([.25,1000.][e]);triples.append((alpha,rate,co))
    out=[]
    for j in range(n):
      z=mp.exp((-kappa[j]+1j*theta[j])*h)*y[j]
      cache={}
      for alpha,rate,co in triples:
        if rate not in cache:cache[rate]=[analytic_moment(kappa[j],rate,theta[j],h,l) for l in range(7)]
        z+=1j*chi[alpha]*phi(alpha,j)*sum((co[l]*cache[rate][l] for l in range(7)),mp.mpc(0))
      out.append(z)
    return out

out=[];start=time.perf_counter()
for n in [12,24,65,257]:
    y=[(F(float(by_key[n,1,j]['float_re'])),F(float(by_key[n,1,j]['float_im']))) for j in range(n)]
    old=[F(float(by_key[n,1,j]['physical_conditional_l1_upper'])) for j in range(n)]
    expected,q=exact_bound(n,F(1,4),y,old,2)
    below=[]
    for j in range(n):
        actual=F(float(by_key[n,2,j]['physical_conditional_l1_upper']))
        if actual<expected[j]:below.append(j)
    a=ideal_second(n,180)
    b=ideal_second(n,220)
    included=0; maxref=mp.mpf(0); maxfloat=mp.mpf(0);worst=[]
    with mp.workdps(225):
      for j,(x,z) in enumerate(zip(a,b)):
        r=by_key[n,2,j]
        ok=(mpf_f64(r['picard_re_lo'])<=x.real<=mpf_f64(r['picard_re_hi']) and mpf_f64(r['picard_im_lo'])<=x.imag<=mpf_f64(r['picard_im_hi']))
        included+=ok
        diff=abs(x.real-z.real)+abs(x.imag-z.imag)
        maxref=max(maxref,diff)
        native=cm_f64(r['float_re'],r['float_im'])
        gap=abs(native.real-x.real)+abs(native.imag-x.imag)
        maxfloat=max(maxfloat,gap)
        if gap>mpf_f64(r['float_gap_l1_upper']):worst.append(j)
    row=dict(n=n,included=int(included),count=n,exact_represented_upper_below=len(below),max_physical_upper=max(float(by_key[n,2,j]['physical_conditional_l1_upper']) for j in range(n)),max_float_gap=float(maxfloat),independent_180_vs_220=float(maxref),out_of_float_gap=worst,exact_q=str(q))
    out.append(row)
    print(json.dumps(row),flush=True)
receipt=dict(status='PASS' if all(r['included']==r['count'] and not r['exact_represented_upper_below'] and not r['out_of_float_gap'] for r in out) else 'FAIL', scope='source-exact 2nd-window frozen represented carrier + inherited full first physical-error metadata only',calculations=out,elapsed_s=time.perf_counter()-start)
(root/'evidence/N4B_INDEPENDENT_180_220_FRACTION.json').write_text(json.dumps(receipt,indent=2)+'\n')
assert receipt['status']=='PASS'
