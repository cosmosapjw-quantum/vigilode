"""Exact-coefficient causal packet reference, not a generic ODE solver.

Supported implementation: a_j'=i*width*xi_j*a_j + i*chi*|mean|^2*mean,
positive rational weights, known common carrier removed exactly, kappa=0.
No vector-field derivative, directional finite difference, matrix factorization,
or external numerical library is used. Certificate arithmetic is rational.
"""
from __future__ import annotations
from dataclasses import dataclass, asdict
from fractions import Fraction as F
from math import factorial
from hashlib import sha256
import json

@dataclass(frozen=True)
class C:
    re: F = F(0)
    im: F = F(0)
    def __post_init__(self):
        object.__setattr__(self,'re',F(self.re)); object.__setattr__(self,'im',F(self.im))
    def __add__(self, other):
        other=coerce(other); return C(self.re+other.re,self.im+other.im)
    __radd__=__add__
    def __neg__(self): return C(-self.re,-self.im)
    def __sub__(self,other): return self+-coerce(other)
    def __rsub__(self,other): return coerce(other)+-self
    def __mul__(self,other):
        other=coerce(other)
        return C(self.re*other.re-self.im*other.im,self.re*other.im+self.im*other.re)
    __rmul__=__mul__
    def __truediv__(self,other):
        if isinstance(other,C):
            den=other.norm2()
            if not den: raise ZeroDivisionError
            return (self*other.conj())/den
        v=F(other)
        if not v: raise ZeroDivisionError
        return C(self.re/v,self.im/v)
    def conj(self): return C(self.re,-self.im)
    def norm2(self): return self.re*self.re+self.im*self.im
    def upper(self): return abs(self.re)+abs(self.im)
    def __complex__(self): return complex(float(self.re),float(self.im))

def coerce(x): return x if isinstance(x,C) else C(F(x))
Z=C(); ONE=C(1); I=C(0,1)

@dataclass
class Work:
    moment_terms:int=0
    poly_multiply_terms:int=0
    volterra_terms:int=0
    nonlinear_evaluations:int=0
    output_horner_terms:int=0
    def dict(self): return asdict(self)

def trim(p):
    p=list(p)
    while len(p)>1 and p[-1]==Z: p.pop()
    return p or [Z]

def add(a,b):
    o=[Z]*max(len(a),len(b))
    for j,c in enumerate(a): o[j]=o[j]+c
    for j,c in enumerate(b): o[j]=o[j]+c
    return trim(o)

def scale(p,c): return trim([x*c for x in p])

def mul(a,b,work=None):
    o=[Z]*(len(a)+len(b)-1)
    for j,x in enumerate(a):
        for k,y in enumerate(b):
            if work: work.poly_multiply_terms+=1
            o[j+k]=o[j+k]+x*y
    return trim(o)

def force(p,chi,work=None):
    if work: work.nonlinear_evaluations+=1
    return scale(mul(mul(p,[c.conj() for c in p],work),p,work),I*chi)

def volterra_monomial(j,k):
    if j<0 or k<0: raise ValueError('nonnegative degrees required')
    return j+k+1,F(factorial(j)*factorial(k),factorial(j+k+1))

def conv(a,b,work=None):
    """Integral_0^t a(t-s)b(s)ds; NOT circular or pointwise multiplication."""
    o=[Z]*(len(a)+len(b))
    for j,x in enumerate(a):
        for k,y in enumerate(b):
            if work: work.volterra_terms+=1
            ell,w=volterra_monomial(j,k)
            o[ell]=o[ell]+x*y*w
    return trim(o)

def evaluate(p,x,work=None):
    v=Z
    for c in reversed(p):
        v=v*x+c
        if work: work.output_horner_terms+=1
    return v

def bound(p):
    """Safe complex modulus bound on [0,1], via coefficient absolute sums."""
    return sum((c.upper() for c in p),F(0))

def qround(x,bits):
    if bits is None: return x
    if bits<16: raise ValueError('reference precision must be at least 16 bits')
    return F(round(x*(1<<bits)),1<<bits)

def qup(x,bits):
    scaled=x*(1<<bits)
    return F(-(-scaled.numerator//scaled.denominator),1<<bits)

def round_poly(p,bits): return trim([C(qround(c.re,bits),qround(c.im,bits)) for c in p])

def kernel_tail(width,horizon,order):
    if order<0 or F(horizon)<0: raise ValueError('invalid degree or horizon')
    return (abs(F(width))*F(horizon))**(order+1)/factorial(order+1)

@dataclass(frozen=True)
class Model:
    xi:tuple[F,...]
    weights:tuple[F,...]
    initial:tuple[C,...]
    width:F=F(1)
    chi:F=F(1,8)
    carrier:F=F(0)
    def __post_init__(self):
        object.__setattr__(self,'xi',tuple(F(x) for x in self.xi))
        object.__setattr__(self,'weights',tuple(F(x) for x in self.weights))
        object.__setattr__(self,'initial',tuple(coerce(x) for x in self.initial))
        for n in ['width','chi','carrier']: object.__setattr__(self,n,F(getattr(self,n)))
        if not self.xi or len(self.xi)!=len(self.weights) or len(self.xi)!=len(self.initial):
            raise ValueError('dimension mismatch')
        if any(abs(x)>1 for x in self.xi): raise ValueError('xi must be in [-1,1]')
        if any(w<=0 for w in self.weights) or sum(self.weights)!=1:
            raise ValueError('positive normalized weights required')
        if self.energy()>1: raise ValueError('reference uses the unit weighted-energy ball')
    def energy(self): return sum((w*z.norm2() for w,z in zip(self.weights,self.initial)),F(0))
    def id(self): return sha256(repr(self).encode()).hexdigest()

@dataclass(frozen=True)
class Packet:
    model:Model
    horizon:F
    order:int
    mu:tuple[F,...]
    nu:tuple[C,...]
    free:tuple[C,...]
    kernel:tuple[C,...]


def compile_packet(model,horizon,order,work=None):
    h=F(horizon)
    if h<=0 or order<0: raise ValueError('positive horizon and nonnegative order required')
    mu=[F(0)]*(order+1); nu=[Z]*(order+1)
    for x,w,z in zip(model.xi,model.weights,model.initial):
        power=F(1)
        for k in range(order+1):
            mu[k]+=w*power; nu[k]=nu[k]+z*(w*power)
            if work: work.moment_terms+=1
            power*=x
    b=ONE; free=[]; kernel=[]
    for k in range(order+1):
        if k: b=b*(I*model.width*h)/k
        free.append(b*nu[k]); kernel.append(b*mu[k])
    return Packet(model,h,order,tuple(mu),tuple(nu),tuple(free),tuple(kernel))

@dataclass(frozen=True)
class Candidate:
    packet:Packet
    mean:tuple[C,...]
    degree:int
    sweeps:int
    def id(self): return sha256(repr(self).encode()).hexdigest()


def candidate(packet,degree=10,sweeps=6,bits=112,work=None):
    if degree<packet.order or sweeps<1: raise ValueError('degree must cover kernel; sweeps positive')
    p=list(packet.free)
    for _ in range(sweeps):
        rhs=force(p,packet.model.chi,work)
        p=add(packet.free,scale(conv(packet.kernel,rhs,work),packet.horizon))[:degree+1]
        p=round_poly(p,bits)
        p[0]=packet.nu[0]  # exact original initial mean, never reset to a fitted value
    return Candidate(packet,tuple(trim(p)),degree,sweeps)


def certificate(cand,work=None):
    p=cand.packet; mod=p.model; h=p.horizon
    if bound(cand.mean)>1: raise ValueError('candidate leaves proved complex-mean domain')
    L=3*abs(mod.chi); G=abs(mod.chi)
    if h*L>=1: raise ValueError('simple rational Gronwall bound unsupported')
    f=force(cand.mean,mod.chi,work)
    residual=add(cand.mean,scale(add(p.free,scale(conv(p.kernel,f,work),h)),-1))
    D=bound(residual)
    tau=kernel_tail(mod.width,h,p.order)
    A0=sum((w*z.upper() for w,z in zip(mod.weights,mod.initial)),F(0))
    kernel_model=tau*(A0+h*G/(p.order+2))
    emean=(D+kernel_model)/(1-h*L)
    emodes=tuple(tau*(z.upper()+h*G/(p.order+2))+h*L*emean for z in mod.initial)
    return {'candidate_id':cand.id(),'model_id':mod.id(),'mean_bound':emean,
            'mode_bounds':emodes,'volterra_defect':D,'kernel_model':kernel_model,
            'tail':tau,'candidate_domain_bound':bound(cand.mean),
            'original_energy':mod.energy(),'L':L,'G':G,'normalization':'tau=t/h',
            'horizon':h,'order':p.order,'omitted_high_modes_charged':True}


def reconstruction_coefficients(cand,work=None):
    """Two shared polynomials in xi reconstruct every independent initial state."""
    p=cand.packet; f=force(cand.mean,p.model.chi,work)
    beta=[]; gam=[]; w=ONE
    for j in range(p.order+1):
        if j: w=w*(I*p.model.width*p.horizon)/j
        beta.append(w)
        acc=Z
        for k,c in enumerate(f):
            acc=acc+c*F(factorial(j)*factorial(k),factorial(j+k+1))
        gam.append(w*acc*p.horizon)
    return beta,gam


def reconstruct(cand,work=None):
    beta,gam=reconstruction_coefficients(cand,work)
    return tuple(evaluate(beta,x,work)*a+evaluate(gam,x,work)
                 for x,a in zip(cand.packet.model.xi,cand.packet.model.initial))


def commit(cand,cert,tolerance):
    if cert['candidate_id']!=cand.id() or cert['model_id']!=cand.packet.model.id():
        raise ValueError('stale or mixed candidate certificate')
    if F(tolerance)<=0: raise ValueError('positive tolerance required')
    if max(cert['mode_bounds'])>F(tolerance): raise ValueError('mode error budget not met')
    return reconstruct(cand)


def full_waveform_mean(model,horizon,order,degree,sweeps,work=None):
    """Unhoisted exact baseline. Same truncated-kernel Picard map; no rounding."""
    kernels=[]; paths=[]
    for xi,z in zip(model.xi,model.initial):
        c=ONE;k=[]
        for j in range(order+1):
            if j:c=c*(I*model.width*F(horizon)*xi)/j
            k.append(c)
        kernels.append(k); paths.append(scale(k,z))
    def mean_of(ps):
        ans=[Z]
        for w,v in zip(model.weights,ps): ans=add(ans,scale(v,w))
        return ans
    for _ in range(sweeps):
        f=force(mean_of(paths),model.chi,work)
        paths=[add(scale(k,z),scale(conv(k,f,work),F(horizon)))[:degree+1]
               for k,z in zip(kernels,model.initial)]
    return tuple(mean_of(paths))


def even_odd_pair(poly,x):
    even=evaluate(poly[0::2],x*x)
    odd=evaluate(poly[1::2],x*x)*x
    return even+odd,even-odd


def carrier_enclosure(angle,bits=112):
    """Rational exp(i*angle) enclosure, no unvalidated libm trigonometry."""
    angle=F(angle); nscale=0
    while abs(angle)>F(1,8): angle/=2; nscale+=1
    prec=bits+2*nscale+12; target=F(1,1<<prec)
    c=ONE;term=ONE;n=0
    while True:
        n+=1;term=term*(I*angle)/n;c=c+term
        rem=abs(angle)**(n+1)/factorial(n+1)
        if rem<=target: break
        if n>256: raise ValueError('phase precision work cap')
    q=C(qround(c.re,prec),qround(c.im,prec));err=qup(rem+(q-c).upper(),prec);c=q
    for _ in range(nscale):
        raw=c*c;q=C(qround(raw.re,prec),qround(raw.im,prec))
        err=qup((2+err)*err+(q-raw).upper(),prec);c=q
    return c,err,{'doublings':nscale,'taylor_terms':n,'working_bits':prec}


def physical_output(cand,cert):
    if cert['candidate_id']!=cand.id(): raise ValueError('candidate mismatch')
    carrier,ec,counts=carrier_enclosure(cand.packet.model.carrier*cand.packet.horizon)
    a=reconstruct(cand)
    return tuple(carrier*z for z in a),tuple(e+z.upper()*ec for e,z in zip(cert['mode_bounds'],a)),counts
