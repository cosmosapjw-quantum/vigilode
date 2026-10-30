#!/usr/bin/env python3
"""Independent 180-digit exact-represented-source Picard oracle, not a formal proof.
Analytic complex exponential moments, no Rust N4 interval code reused.
"""
import csv, json, math
from collections import defaultdict
from pathlib import Path
import mpmath as mp

root=Path(__file__).resolve().parent
rows=list(csv.DictReader((root/'evidence/N4A_FROZEN_NATIVE_CARRIER.csv').open()))

def mpf_f64(x):
    num,den=float(x).as_integer_ratio()
    return mp.mpf(num)/den

def cm_f64(re, im):return mp.mpc(mpf_f64(re),mpf_f64(im))

def poly_mul(a,b):
    r=[mp.mpc(0)]*(len(a)+len(b)-1)
    for k,x in enumerate(a):
        for l,y in enumerate(b):r[k+l]+=x*y
    return r

def complex_moment(a,b,theta,h,ell):
    r=(a-b)-1j*theta
    z=r*h
    if abs(z) < 4:
        res=mp.mpc(0);term=mp.mpc(1)
        for k in range(310):
            v=term*h**(ell+1)/(ell+k+1)
            res+=v
            term*=z/(k+1)
            if abs(v)<mp.mpf('1e-155')*max(abs(res),mp.mpf('1e-200')) and k>ell+10:break
    else:
        res=mp.expm1(z)/r
        for q in range(1,ell+1):
            res=(h**q*mp.exp(z)-q*res)/r
    return mp.exp((-a+1j*theta)*h)*res

def calculation(n, prec):
    with mp.workdps(prec):
        t=list(filter(lambda x:int(x['n'])==n,rows)); t.sort(key=lambda x:int(x['j']))
        w=mpf_f64(1./n);omega=mpf_f64(0.5); h=mpf_f64(0.25)
        chi=[mpf_f64(0.05),mpf_f64(0.02)]
        xjs=[mpf_f64(2.*j/(n-1)-1.) for j in range(n)]
        th=[omega*xi for xi in xjs]
        kappa=[mpf_f64(.25 if j%2==0 else 1000.) for j in range(n)]
        phi=lambda alpha,j: mpf_f64(1. if alpha==0 or j%2==0 else -1.)
        y=[cm_f64(r['source_re'],r['source_im']) for r in t]
        p=2
        P=[[[mp.mpc(0)]*(p+1) for c in range(2)] for alpha in range(2)]
        for alpha in range(2):
            for j in range(n):
                c=j%2; base=w*phi(alpha,j)*y[j]
                for ell in range(p+1):
                    P[alpha][c][ell]+=base*(1j*th[j])**ell/mp.factorial(ell)
        triples=[]
        for alpha in range(2):
            for c in range(2):
                for d in range(2):
                    for e in range(2):
                        g=poly_mul(poly_mul(P[alpha][c],[mp.conj(x) for x in P[alpha][d]]),P[alpha][e]);
                        rate=mp.mpf([.25,1000.][c])+mp.mpf([.25,1000.][d])+mp.mpf([.25,1000.][e])
                        triples.append((alpha,rate,g))
        result=[]
        for j in range(n):
            val=mp.exp((-kappa[j]+1j*th[j])*h)*y[j]
            cache={}
            for alpha,rate,g in triples:
                if rate not in cache:cache[rate]=[complex_moment(kappa[j],rate,th[j],h,ell) for ell in range(3*p+1)]
                val+=1j*chi[alpha]*phi(alpha,j)*sum((g[ell]*cache[rate][ell] for ell in range(3*p+1)),mp.mpc(0))
            result.append(val)
        return result

out=[]
for n in sorted({int(r['n']) for r in rows}):
    a=calculation(n,180)
    b=calculation(n,220)
    subset=sorted((r for r in rows if int(r['n'])==n),key=lambda x:int(x['j']))
    max_excess=0.; max_native_gap=0.;max_ref_disagree=0.; included=0
    for j,(r,z,z2) in enumerate(zip(subset,a,b)):
        lo_re=mpf_f64(r['exact_picard_re_lo']);hi_re=mpf_f64(r['exact_picard_re_hi'])
        lo_im=mpf_f64(r['exact_picard_im_lo']);hi_im=mpf_f64(r['exact_picard_im_hi'])
        ok=lo_re <= z.real <= hi_re and lo_im <= z.imag <= hi_im
        included+=int(ok)
        if not ok:
            print('NOT_INCLUDED',n,j, mp.nstr(z,20),r)
        ref_gap=max(abs(z.real-z2.real),abs(z.imag-z2.imag))
        max_ref_disagree=max(max_ref_disagree,float(ref_gap))
        native=cm_f64(r['float_re'],r['float_im']); err=abs(native.real-z.real)+abs(native.imag-z.imag)
        max_native_gap=max(max_native_gap,float(err))
        bound=mpf_f64(r['float_gap_l1_upper'])
        if err>bound: max_excess=max(max_excess,float(err-bound))
    out.append(dict(n=n,rows=n,independent_180vs220_max=float(max_ref_disagree),included=included,
                    max_reference_vs_native_l1=float(max_native_gap),violating_bound_excess=float(max_excess),pass_all=(included==n and max_excess==0)))
print(json.dumps(out,indent=2));(root/'evidence/N4A_INDEPENDENT_180_220_DPS.json').write_text(json.dumps(out,indent=2)+'\n')
assert all(o['pass_all'] for o in out),'interval inclusion failed'
