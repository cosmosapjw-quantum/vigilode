#!/usr/bin/env python3
"""Exact-rational enclosure of complex damped exponential-convolution moments.
Source inputs are exact represented IEEE754 binary64 (Fraction.from_float).
All interval operations are Fraction exact; only exported endpoints are rounded outward.
Neither this method nor its results certify a floating-point ODE trajectory.
"""
import csv, functools, math, sys
from fractions import Fraction as Q
from pathlib import Path
from dataclasses import dataclass

BITS=1280
SERIES=80
PHASE_TERMS=18
TWO=Q(2)

def F(x):
    assert math.isfinite(x)
    return Q.from_float(float(x))

@dataclass(frozen=True)
class Iv:
    lo: Q
    hi: Q
    def __post_init__(self):
        assert self.lo<=self.hi
    @classmethod
    def of(cls,x): return cls(x,x)
    def __add__(self,o):
        o=as_iv(o);return Iv(self.lo+o.lo,self.hi+o.hi)
    __radd__=__add__
    def __neg__(self):return Iv(-self.hi,-self.lo)
    def __sub__(self,o):return self+(-as_iv(o))
    def __mul__(self,o):
        o=as_iv(o)
        e=(self.lo*o.lo,self.lo*o.hi,self.hi*o.lo,self.hi*o.hi)
        return Iv(min(e),max(e))
    __rmul__=__mul__
    def __truediv__(self,o):
        o=as_iv(o)
        assert o.lo>0 or o.hi<0
        return self*Iv(1/o.hi,1/o.lo)
    def padded(self,r):assert r>=0;return Iv(self.lo-r,self.hi+r)
    def clamp0(self):return Iv(max(self.lo,0),max(self.hi,0))

def as_iv(x): return x if isinstance(x,Iv) else Iv.of(x)

def quantize(x,sign):
    a=x*(1<<BITS)
    k=a.numerator//a.denominator
    if sign>0 and a.numerator % a.denominator:k+=1
    return Q(k,1<<BITS)

@functools.lru_cache(None)
def expneg(x):
    assert x>=0
    if x==0:return Iv.of(Q(1))
    m=0;y=x
    while y>Q(1,4): y/=2;m+=1
    term=Q(1);lower=Q(1)
    for k in range(1,SERIES+1):
        term=term*y/k;lower+=term
    next_term=term*y/(SERIES+1)
    upper=lower+next_term/(1-y/Q(SERIES+2))
    I=Iv(quantize(1/upper,-1),quantize(1/lower,1))
    for _ in range(m): I=Iv(quantize(I.lo*I.lo,-1),quantize(I.hi*I.hi,1))
    return I

def positive_series_integral(r,c,h,positive=False):
    # I_r=integral s^r exp(±c s) ds for c>=0, positive=True for positive exponent.
    N=200
    assert c*h<Q(50), 'fixed source validity: c*h < 50 for Taylor branch'
    term=h**(r+1)/Q(r+1)
    val=term
    for k in range(1,N+1):
        term=term*c*h*Q(r+k)/Q(k*(r+k+1))
        val+=term if positive or k%2==0 else -term
    next_term=term*c*h*Q(r+N+1)/Q((N+1)*(r+N+2))
    rem=next_term/(1-c*h/Q(N+2))
    if positive:return Iv(val,val+rem)
    return Iv(val-rem,val+rem).clamp0()

def ir_list(c,h,top):
    assert c>=0
    if c==0: return [Iv.of(h**(r+1)/Q(r+1)) for r in range(top+1)]
    if c*h<=2*(top+2):return [positive_series_integral(r,c,h) for r in range(top+1)]
    decay=expneg(c*h)
    prev=(Iv.of(Q(1))-decay)/Iv.of(c)
    out=[prev.clamp0()]
    for r in range(1,top+1):
        prev=(prev*Q(r)-decay*(h**r))/Iv.of(c)
        out.append(prev.clamp0())
    return out

def j_list(a,b,h,top):
    assert min(a,b,h)>=0
    if b>=a:
        return [expneg(a*h)*q for q in ir_list(b-a,h,top)]
    d=a-b
    if d*h<=2*(top+2):
        return [expneg(a*h)*positive_series_integral(r,d,h,True) for r in range(top+1)]
    # endpoint recurrence anchored at dominant s=h, stable for d*h>>top
    eb=expneg(b*h)
    ea=expneg(a*h)
    prev=(eb-ea)/Iv.of(d)
    out=[prev.clamp0()]
    for r in range(1,top+1):
        prev=(eb*(h**r)-prev*Q(r))/Iv.of(d)
        out.append(prev.clamp0())
    return out

@dataclass(frozen=True)
class Z:
    re: Iv
    im: Iv
    def __add__(self,o):return Z(self.re+o.re,self.im+o.im)
    def __mul__(self,o):return Z(self.re*o.re-self.im*o.im,self.re*o.im+self.im*o.re)
    @classmethod
    def zero(cls):return cls(Iv.of(Q(0)),Iv.of(Q(0)))


def signed_iq(i,q):
    m=i%4
    return Z(Iv.of(q if m==0 else -q if m==2 else Q(0)),
             Iv.of(q if m==1 else -q if m==3 else Q(0)))

def moment(a_float,b_float,theta_float,h_float,r,P=PHASE_TERMS):
    a,b,theta,h=map(F,[a_float,b_float,theta_float,h_float])
    assert a>=0 and b>=0 and h>=0 and abs(theta*h)<=Q(1,4)
    J=j_list(a,b,h,r+P+1)
    inner=Z.zero();phase=Z.zero()
    for q in range(P+1):
        coef=(-theta)**q/Q(math.factorial(q))
        inner=inner+signed_iq(q,coef)*Z(J[r+q],Iv.of(Q(0)))
        phase=phase+signed_iq(q,(theta*h)**q/Q(math.factorial(q)))
    inner_err=(abs(theta)**(P+1)/Q(math.factorial(P+1)))*J[r+P+1].hi
    inner=Z(inner.re.padded(inner_err),inner.im.padded(inner_err))
    phase_err=abs(theta*h)**(P+1)/Q(math.factorial(P+1))
    phase=Z(phase.re.padded(phase_err),phase.im.padded(phase_err))
    return phase*inner

def outward_float(x,side):
    f=float(x)
    assert math.isfinite(f)
    exact=Q.from_float(f)
    if side<0 and exact>x:return math.nextafter(f,-math.inf)
    if side>0 and exact<x:return math.nextafter(f,math.inf)
    return f

def produce(path):
    rows=[]
    # Actual two-rate rank2 cluster exponents b: .75,1000.5,2000.25,3000
    cases=[(.25,.75),(.25,1000.5),(.25,2000.25),(.25,3000.),
           (1000.,.75),(1000.,1000.5),(1000.,2000.25),(1000.,3000.)]
    for a,b in cases:
        for theta in (-.5,0.,.125):
            for r in range(10):
                M=moment(a,b,theta,.25,r)
                rows.append(dict(a=a,b=b,theta=theta,h=.25,degree=r,
                    re_lo=outward_float(M.re.lo,-1),re_hi=outward_float(M.re.hi,1),
                    im_lo=outward_float(M.im.lo,-1),im_hi=outward_float(M.im.hi,1),
                    re_width=outward_float(M.re.hi-M.re.lo,1),im_width=outward_float(M.im.hi-M.im.lo,1)))
    path=Path(path);path.parent.mkdir(parents=True,exist_ok=True)
    with path.open('w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
    return rows

def verify(path):
    """Independent mpmath closed-form recurrence, with direct quadrature as diagnostic only."""
    import mpmath as mp
    mp.mp.dps=160
    def mpF(f):
        x=F(f);return mp.mpf(x.numerator)/x.denominator
    with Path(path).open() as f:rows=list(csv.DictReader(f))
    failures=[];widths=[];quad_witness=[]
    cache={}
    for i,row in enumerate(rows):
        a,b,th,h,r=(float(row[k]) if k!='degree' else int(row[k]) for k in ('a','b','theta','h','degree'))
        aa,bb,tt,hh=map(mpF,(a,b,th,h))
        key=(a,b,th,h)
        if key not in cache:
            z=-aa+1j*tt
            eta=z+bb
            exact0=mp.exp(z*hh)*(-mp.expm1(-eta*hh))/eta if eta else mp.exp(z*hh)*hh
            cache[key]=[exact0]
        v=cache[key]
        eta=-aa+1j*tt+bb
        while len(v)<=r:
            ell=len(v)
            v.append((ell*v[-1] - hh**ell*mp.exp(-bb*hh))/eta)
        val=v[r]
        ilo,ihi=map(mpF,(float(row['re_lo']),float(row['re_hi'])))
        jlo,jhi=map(mpF,(float(row['im_lo']),float(row['im_hi'])))
        if not (ilo<=val.real<=ihi and jlo<=val.imag<=jhi):
            failures.append((i, str(val), row))
        widths.append(max(float(ihi-ilo),float(jhi-jlo)))
        if a==1000 and b==1000.5 and th==-0.5 and r==0:
            quadr=mp.quad(lambda ss:mp.exp((-aa+1j*tt)*(hh-ss)-bb*ss),[0,hh])
            quad_witness=[dict(case='a=1000,b=1000.5,theta=-0.5,degree=0',relative_difference=float(abs(quadr-val)/abs(val)),
               interpretation='quadrature convergence diagnostic; not ground truth')]
    return dict(total=len(rows),pass_count=len(rows)-len(failures),failures=failures[:3],max_width=max(widths),median_width=sorted(widths)[len(widths)//2],status='PASS' if not failures else 'FAIL',quadrature_diagnostic=quad_witness,reference='Independent mpmath160 analytic integration-by-parts recurrence; Fraction enclosures derived from Taylor remainders')

if __name__=='__main__':
    dst=Path(sys.argv[1] if len(sys.argv)>1 else 'evidence/EXACT_DYADIC_MOMENTS.csv')
    rows=produce(dst)
    import json
    status=verify(dst)
    print(json.dumps(status,indent=2))
    assert status['status']=='PASS'
