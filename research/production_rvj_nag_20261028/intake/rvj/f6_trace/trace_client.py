"""All-root trace observables, a controlled coupled ODE, exact slab certificates.
No selected physical branch can be reconstructed by this API.
"""
from dataclasses import dataclass, asdict
from fractions import Fraction as Q
from math import comb
import numpy as np


def power_sums(a,b,kmax=6):
    if type(kmax) is not int or kmax<0: raise ValueError('nonnegative integer degree required')
    known=[4,0,4*a,-3,4*a*a+4*b,-10*a,4*a**3+12*a*b+3]
    out=known[:kmax+1]
    for k in range(7,kmax+1):
        out.append(2*a*out[k-2]-out[k-3]-(a*a-b)*out[k-4])
    return out


@dataclass(frozen=True)
class PolynomialObservable:
    """Sum h(q1,q2) over the length-four complex zero scheme, with multiplicity.
    Terms are (q1_exponent,q2_exponent,rational_coefficient). No filters or labels.
    """
    terms: tuple
    def __post_init__(self):
        if not isinstance(self.terms,tuple): raise TypeError('immutable polynomial terms required')
        for i,j,c in self.terms:
            if type(i) is not int or type(j) is not int or min(i,j)<0:
                raise TypeError('nonnegative integral exponents required')
            if type(c) not in (int,Q): raise TypeError('exact rational coefficients required')
    def trace(self,a,b):
        degree=max((i+2*j for i,j,_ in self.terms),default=0)
        S=power_sums(a,b,degree)
        return sum((c*comb(j,k)*a**(j-k)*(-1)**k*S[i+2*k]
                    for i,j,c in self.terms for k in range(j+1)),0)


def require_observable(observable):
    if not isinstance(observable,PolynomialObservable):
        raise TypeError('only polynomial all-complex-root traces are admissible; branch indicators are rejected')
    return observable


def companion(a,b):
    return [[0,0,0,b-a*a],[1,0,0,-1],[0,1,0,2*a],[0,0,1,0]]


def matmul(A,B):
    return [[sum((A[i][k]*B[k][j] for k in range(4)),0) for j in range(4)] for i in range(4)]


def companion_traces(a,b,kmax):
    M=companion(a,b); N=[[int(i==j) for j in range(4)] for i in range(4)]; out=[]
    for k in range(kmax+1):
        out.append(sum(N[i][i] for i in range(4))); N=matmul(N,M)
    return out


def reference(t): return Q(3,4)+t/8+t**5/64

def reference_derivative(t): return Q(1,8)+5*t**4/64


def aggregate_g(a,b,sums):
    S2,S4,S6=sums
    return (-a+S2/32+S4/128+S6/512,
            -b+S2/64+S4/64+S6/256)


def exact_g(a,b):
    S=power_sums(a,b)
    return aggregate_g(a,b,(S[2],S[4],S[6]))


@dataclass
class Counter:
    rhs_calls:int=0
    trace_batches:int=0
    root_enumerations:int=0
    roots_returned_with_multiplicity:int=0


def rhs(t,state,mode='trace',counter=None):
    if counter is not None: counter.rhs_calls+=1
    a,b=state
    if mode=='trace':
        S=power_sums(a,b); sums=(S[2],S[4],S[6])
        if counter is not None: counter.trace_batches+=1
    elif mode=='roots':
        roots=np.roots([1.,0.,-2*a,1.,a*a-b])
        sums=tuple(float(np.real(np.sum(roots**k))) for k in (2,4,6))
        if counter is not None:
            counter.root_enumerations+=1; counter.roots_returned_with_multiplicity+=len(roots)
    else: raise ValueError('unknown mode')
    g=aggregate_g(a,b,sums); p=reference(t); control=exact_g(p,p); dp=reference_derivative(t)
    return tuple(dp+gi-ci for gi,ci in zip(g,control))


def integrate(n,mode='trace'):
    if type(n) is not int or n<=0: raise ValueError('positive integer slab count required')
    h=2./n; y=(float(reference(Q(-1))),)*2; out=[(Q(-1),y)]; counter=Counter()
    for k in range(n):
        t=-1.+k*h
        k1=rhs(t,y,mode,counter)
        k2=rhs(t+h/2,tuple(v+h*d/2 for v,d in zip(y,k1)),mode,counter)
        k3=rhs(t+h/2,tuple(v+h*d/2 for v,d in zip(y,k2)),mode,counter)
        k4=rhs(t+h,tuple(v+h*d for v,d in zip(y,k3)),mode,counter)
        y=tuple(v+h*(d1+2*d2+2*d3+d4)/6 for v,d1,d2,d3,d4 in zip(y,k1,k2,k3,k4))
        out.append((Q(-1)+Q(2*(k+1),n),y))
    return out,counter

# Exact rational polynomial operations in local coordinate s in [0,1].
def add(a,b):
    out=[Q(0)]*max(len(a),len(b))
    for i,v in enumerate(a): out[i]+=v
    for i,v in enumerate(b): out[i]+=v
    return out

def scale(a,c): return [v*c for v in a]

def mul(a,b):
    out=[Q(0)]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b): out[i+j]+=x*y
    return out

def power(a,k):
    out=[Q(1)]
    for _ in range(k): out=mul(out,a)
    return out

def derivative(a): return [i*a[i] for i in range(1,len(a))] or [Q(0)]

def bernstein_range(a):
    n=len(a)-1
    bc=[sum((Q(comb(i,k),comb(n,k))*a[k] for k in range(i+1)),Q(0)) for i in range(n+1)]
    return min(bc),max(bc)

def magnitude(a):
    lo,hi=bernstein_range(a); return max(abs(lo),abs(hi))

def hermite(y0,y1,f0,f1,h):
    return [y0,h*f0,3*(y1-y0)-h*(2*f0+f1),2*(y0-y1)+h*(f0+f1)]

def g_polynomials(a,b):
    # Independent expanded form of the reduced trace vector field.
    a2=mul(a,a); a3=mul(a2,a); ab=mul(a,b)
    s4=scale(add(a2,b),4); s6=add(add(scale(a3,4),scale(ab,12)),[Q(3)])
    return (add(add(scale(a,Q(-7,8)),scale(s4,Q(1,128))),scale(s6,Q(1,512))),
            add(add(scale(b,-1),scale(a,Q(1,16))),add(scale(s4,Q(1,64)),scale(s6,Q(1,256)))))

def exact_rhs(t,y):
    p=reference(t); gp=exact_g(p,p); gy=exact_g(*y); dp=reference_derivative(t)
    return tuple(dp+u-v for u,v in zip(gy,gp))

def round_up(q,bits=112):
    q=Q(q); unit=1<<bits
    return Q(-(-(q*unit).numerator//(q*unit).denominator),unit)


def certify(states,budget=Q(1,10**8)):
    """Exact Bernstein residual bounds on every whole closed slab, with tube test.
    Candidate is cubic Hermite through the binary64 nodes, interpreted exactly.
    This is a rational arithmetic certificate, not interval binary64 arithmetic.
    """
    if len(states)<2: raise ValueError("at least one slab required")
    error=max(abs(Q(v)-reference(states[0][0])) for v in states[0][1])
    max_whole=error; max_defect=Q(0); max_direct=Q(0); min_margin=Q(1)
    records=[]
    for (t0,v0),(t1,v1) in zip(states,states[1:]):
        h=t1-t0
        if h<=0: raise ValueError('noncausal slab')
        y0=tuple(map(Q,v0)); y1=tuple(map(Q,v1))
        f0=exact_rhs(t0,y0); f1=exact_rhs(t1,y1)
        paths=[hermite(a,b,c,d,h) for a,b,c,d in zip(y0,y1,f0,f1)]
        tp=[t0,h]; p=add(add([Q(3,4)],scale(tp,Q(1,8))),scale(power(tp,5),Q(1,64)))
        dp=add([Q(1,8)],scale(power(tp,4),Q(5,64)))
        gs=g_polynomials(*paths); gp=g_polynomials(p,p)
        f=[add(dp,add(u,scale(v,-1))) for u,v in zip(gs,gp)]
        residuals=[add(scale(derivative(z),1/h),scale(ff,-1)) for z,ff in zip(paths,f)]
        delta=max(map(magnitude,residuals)); whole=error+h*delta
        ranges=list(map(bernstein_range,paths)); margin=min(min(lo-Q(1,2),Q(1)-hi) for lo,hi in ranges)
        if margin<=whole: raise ArithmeticError('error tube leaves contraction box')
        direct=max(magnitude(add(z,scale(p,-1))) for z in paths)
        end=round_up(error/(1+h/2)+h*delta)
        records.append({'t0':str(t0),'t1':str(t1),'residual_upper':str(delta),
                        'whole_error_upper':str(whole),'end_error_upper':str(end),
                        'exact_path_error_upper':str(direct),'domain_margin':str(margin)})
        max_defect=max(max_defect,delta);max_whole=max(max_whole,whole);max_direct=max(max_direct,direct);min_margin=min(min_margin,margin)
        error=end
    return {'arithmetic':'exact Fraction on binary64 node values; rational Bernstein convex-hull bounds',
            'nodes':[{'t':str(t),'state':list(map(lambda v:str(Q(v)),y))} for t,y in states],
            'slabs':len(records),'budget':str(budget),'budget_pass':max_whole<=budget,
            'max_residual_upper':str(max_defect),'whole_path_error_upper':str(max_whole),
            'end_error_upper':str(error),'direct_exact_path_error_upper':str(max_direct),
            'min_domain_margin':str(min_margin),'slab_records':records}
