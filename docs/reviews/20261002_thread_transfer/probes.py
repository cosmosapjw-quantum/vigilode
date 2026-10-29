"""Executable review mathematics. Not a native Rust certificate or solver.

All rational inputs are interpreted as exact reals. Coupling intervals in the
source fixture remain intervals. Never promote these results to ODE error or
native timing authority. Python >=3.11; SymPy >=1.12.
"""
from __future__ import annotations
from fractions import Fraction as F
import math
import struct
from typing import Sequence
import sympy as sp

Poly = list[F]  # ascending powers
Matrix = list[list[F]]


def _matrix(h: Sequence[Sequence[F]], a: Sequence[F]) -> tuple[Matrix, list[F]]:
    n = len(a)
    if not n or len(h) != n or any(len(row) != n for row in h):
        raise ValueError("shape: nonempty square matrix required")
    q, x = [[F(v) for v in row] for row in h], [F(v) for v in a]
    if any(q[i][j] != 0 for i in range(n) for j in range(i, n)):
        raise ValueError("strict-lower coupling required")
    if any(v < 0 for row in q for v in row) or any(v < 0 for v in x):
        raise ValueError("nonnegative majorant required")
    return q, x


def matmul(a: Matrix, b: Matrix, stats: dict | None = None) -> Matrix:
    n = len(a)
    out = [[F(0) for _ in range(n)] for _ in range(n)]
    for i in range(n):
        for j in range(n):
            for k in range(n):
                if a[i][k] and b[k][j]:
                    out[i][j] += a[i][k] * b[k][j]
                    if stats is not None:
                        stats["mul"] += 1; stats["add"] += 1
    return out


def matvec(a: Matrix, b: list[F], stats: dict | None = None) -> list[F]:
    out = [F(0) for _ in b]
    for i, row in enumerate(a):
        for j, value in enumerate(row):
            if value and b[j]:
                out[i] += value * b[j]
                if stats is not None:
                    stats["mul"] += 1; stats["add"] += 1
    return out


def path_sum_action(h, a, stats: dict | None = None) -> list[F]:
    """(I+H+...+H**(n-1))a via binary doubling, no sum matrix."""
    q, x = _matrix(h, a)
    levels = (len(x)-1).bit_length()
    for level in range(levels):
        v = matvec(q, x, stats)
        x = [u + w for u, w in zip(x, v)]
        if stats is not None:
            stats["add"] += len(x)
        if level + 1 < levels:
            q = matmul(q, q, stats)
    return x


def path_sum_matrix(h, a, stats: dict | None = None) -> list[F]:
    """Independent sum-matrix formulation matching the source algebra."""
    q, x = _matrix(h, a)
    n = len(x)
    total = [[F(i == j) for j in range(n)] for i in range(n)]
    levels = (n-1).bit_length()
    for level in range(levels):
        add = matmul(q, total, stats)
        total = [[total[i][j]+add[i][j] for j in range(n)] for i in range(n)]
        if stats is not None:
            stats["add"] += n*n
        if level + 1 < levels:
            q = matmul(q, q, stats)
    return matvec(total, x, stats)


def peval(p: Sequence[F], x: F) -> F:
    answer = F(0)
    for c in reversed(p):
        answer = answer*x+c
    return answer


def padd(p: Poly, q: Poly) -> Poly:
    out = [F(0)]*max(len(p),len(q))
    for i, c in enumerate(p): out[i] += c
    for i, c in enumerate(q): out[i] += c
    while len(out) > 1 and out[-1] == 0: out.pop()
    return out


def pscale(p: Poly, c: F) -> Poly:
    return [c*x for x in p]


def affine_mul(p: Poly, a: F, b: F) -> Poly:
    return padd(pscale(p, a), [F(0)]+pscale(p, b))


def feasible_radius(polynomials: Sequence[Sequence[F]], bits: int = 40) -> dict:
    """Certified rational point in intersection p_i(D)<=D, D>=0.

    p_i must have nonnegative coefficients. Exact Sturm isolation is a
    review/oracle implementation, NOT a proposal to run CAS in every step.
    A tangency with no rational witness is explicitly indeterminate.
    """
    ps = [[F(c) for c in p] for p in polynomials]
    if not ps or any(not p or any(c < 0 for c in p) for p in ps):
        raise ValueError("nonnegative nonempty polynomials required")
    if not 8 <= bits <= 200:
        raise ValueError("isolation bits outside [8,200]")
    if all(p[0] == 0 for p in ps):
        return {"status":"feasible", "radius":F(0), "certificate":"all constant terms zero"}
    x = sp.Symbol("D")
    safe_lo, safe_hi, outer_lo, outer_hi = F(0), None, F(0), None
    intervals = []
    for index, p in enumerate(ps):
        p = p[:]
        while len(p)>1 and p[-1]==0: p.pop()
        p0, p1 = p[0], p[1] if len(p)>1 else F(0)
        if p0 > 0 and p1 >= 1:
            return {"status":"infeasible", "radius":None,
                    "certificate":{"constraint":index,"type":"positive-intercept-nonnegative-slope","p0":str(p0),"p1":str(p1)}}
        if len(p)<=2:
            if p1 < 1:
                low = p0/(1-p1)
                safe_lo=max(safe_lo,low); outer_lo=max(outer_lo,low)
            elif p0 == 0 and p1 == 1:
                pass
            else:
                safe_hi=F(0) if safe_hi is None else min(safe_hi,F(0))
                outer_hi=F(0) if outer_hi is None else min(outer_hi,F(0))
            continue
        if p0>0 and p[2]>0 and (p1-1)**2 - 4*p[2]*p0 < 0:
            return {"status":"infeasible", "radius":None,
                    "certificate":{"constraint":index,"type":"positive-quadratic-lower-bound", "p0":str(p0),"linear":str(p1-1),"quadratic":str(p[2]),"discriminant":str((p1-1)**2-4*p[2]*p0)}}
        g=p[:]; g[1]-=1
        polynomial=sp.Poly.from_list([sp.Rational(c.numerator,c.denominator) for c in reversed(g)],x)
        roots=sp.polys.polytools.intervals(polynomial,eps=sp.Rational(1,2**bits),inf=sp.Rational(0))
        positive=[]
        for ((left,right), multiplicity) in roots:
            l,r=F(left),F(right)
            if l==r==0: continue
            positive.append((l,r,multiplicity))
        if p0 == 0 and p1 < 1:
            if len(positive)!=1: raise ArithmeticError("unexpected convex root count")
            l,r,_=positive[0]
            lo_l=lo_r=F(0); hi_l,hi_r=l,r
        elif p0==0 and p1>=1:
            lo_l=lo_r=hi_l=hi_r=F(0)
        elif not positive:
            return {"status":"infeasible", "radius":None,
                    "certificate":{"constraint":index,"type":"sturm-no-positive-root","degree":polynomial.degree(),"positive_root_count":0}}
        elif len(positive)==1 and positive[0][2]%2==0:
            l,r,_=positive[0]
            if l!=r:
                return {"status":"indeterminate-tangency", "radius":None,"constraint":index,"root_bracket":[str(l),str(r)]}
            lo_l=lo_r=hi_l=hi_r=l
        elif len(positive)==2:
            lo_l,lo_r,_=positive[0]; hi_l,hi_r,_=positive[1]
        else:
            raise ArithmeticError("unexpected convex root configuration")
        intervals.append({"constraint":index,"lower":[str(lo_l),str(lo_r)],"upper":[str(hi_l),str(hi_r)]})
        safe_lo=max(safe_lo,lo_r); outer_lo=max(outer_lo,lo_l)
        safe_hi=hi_l if safe_hi is None else min(safe_hi,hi_l)
        outer_hi=hi_r if outer_hi is None else min(outer_hi,hi_r)
    if outer_hi is not None and outer_lo>outer_hi:
        return {"status":"infeasible", "radius":None,"certificate":{"type":"disjoint-certified-intervals","outer_lower":str(outer_lo),"outer_upper":str(outer_hi)},"intervals":intervals}
    if safe_hi is not None and safe_lo>safe_hi:
        return {"status":"indeterminate-isolation", "radius":None,"intervals":intervals}
    d=(safe_lo+safe_hi)/2 if safe_hi is not None else max(F(1),2*safe_lo)
    if not all(peval(p,d)<=d for p in ps):
        raise ArithmeticError("independent radius verification failed")
    # Check the proposed binary64 point too; never silently round a witness.
    try: d64=F(float(d)); rounded_ok=all(peval(p,d64)<=d64 for p in ps)
    except (OverflowError, ValueError): d64=None; rounded_ok=False
    return {"status":"feasible","radius":d,"binary64_radius":d64 if rounded_ok else None,"safe_interval":[str(safe_lo),None if safe_hi is None else str(safe_hi)],"certificate":"all exact inequalities re-evaluated","intervals":intervals}


class Interval:
    """Exact rational interval; no finite-precision rounding is suppressed."""
    def __init__(self, lo, hi=None):
        self.lo=F(lo); self.hi=F(lo if hi is None else hi)
        if self.lo>self.hi: raise ValueError("interval endpoints reversed")
    def __add__(self, other):
        b=other if isinstance(other,Interval) else Interval(other)
        return Interval(self.lo+b.lo,self.hi+b.hi)
    __radd__=__add__
    def __neg__(self): return Interval(-self.hi,-self.lo)
    def __sub__(self,other): return self+- (other if isinstance(other,Interval) else Interval(other))
    def __mul__(self,other):
        b=other if isinstance(other,Interval) else Interval(other)
        v=[self.lo*b.lo,self.lo*b.hi,self.hi*b.lo,self.hi*b.hi]
        return Interval(min(v),max(v))
    __rmul__=__mul__
    def mag(self): return max(abs(self.lo),abs(self.hi))
    def midpoint(self): return (self.lo+self.hi)/2


def decode_bits(s: str) -> F:
    v=struct.unpack('>d',bytes.fromhex(s))[0]
    if not math.isfinite(v): raise ValueError("nonfinite bit input")
    return F(v)


def quadratic_component(target: dict, component: int) -> dict:
    """Source-defined R4 case, frozen binary64 inputs; exact-real replay."""
    u=component
    y0=1.0+0.1*u; q0=-0.05*(1+u%3); j0=(-1.0-u)+2.0*q0*y0; h0=0.05
    f0=j0*y0-2.0*q0*y0*y0+q0*y0*y0
    y,q,j,h,k=map(F,[y0,q0,j0,h0,h0*f0])
    gamma=decode_bits(target["gamma"])
    alpha=[[decode_bits(v) for v in row] for row in target["alpha_rows"]]
    coupling=[[Interval(decode_bits(a),decode_bits(b)) for a,b in row] for row in target["coupling_rows"]]
    s=len(alpha); w=1-h*gamma*j
    if w==0: raise ValueError("singular W")
    inv=1/abs(w); candidate=[k]*s
    h0m=[[F(0) for _ in range(s)] for _ in range(s)]; h1m=[[F(0) for _ in range(s)] for _ in range(s)]
    aa=[]; deltas=[]; polys=[]; root=[]; serial=[]; constraints=[]
    for i in range(s):
        delta=sum(alpha[i][jj]*candidate[jj] for jj in range(i)); deltas.append(abs(delta))
        lin=sum((coupling[i][jj]*candidate[jj] for jj in range(i)),Interval(0))
        rhs=Interval(h*(j*y-q*y*y+q*delta*delta)) + h*j*lin
        residual=Interval(w*candidate[i])-rhs
        a=inv*residual.mag(); aa.append(a)
        pol=[a]
        for jj in range(i):
            h0m[i][jj]=h*inv*(abs(j)*coupling[i][jj].mag()+abs(q)*2*abs(delta)*abs(alpha[i][jj]))
            h1m[i][jj]=h*inv*abs(q)*abs(alpha[i][jj])
            pol=padd(pol,affine_mul(polys[jj],h0m[i][jj],h1m[i][jj]))
        polys.append(pol)
        dpoly=[F(0)]
        for jj in range(i): dpoly=padd(dpoly,pscale(polys[jj],abs(alpha[i][jj])))
        constraints.append(dpoly)
        # Independent exact root for the fixed midpoint member of the interval target.
        rd=sum(alpha[i][jj]*root[jj] for jj in range(i))
        rl=sum(coupling[i][jj].midpoint()*root[jj] for jj in range(i))
        root.append(h*(j*y-q*y*y+j*rl+q*rd*rd)/w)
        # Existing serial certificate algebra; not a proposed new solver.
        ds=sum(abs(alpha[i][jj])*serial[jj] for jj in range(i))
        ls=sum(coupling[i][jj].mag()*serial[jj] for jj in range(i))
        serial.append(inv*(residual.mag()+h*(abs(j)*ls+abs(q)*(2*abs(delta)*ds+ds*ds))))
    return {"component":u,"y":y,"q":q,"j":j,"h":h,"w":w,"candidate":candidate,"root":root,"serial":serial,"H0":h0m,"H1":h1m,"a":aa,"E_polynomials":polys,"constraints":constraints,"alpha":alpha}


def matrix_at(case: dict, d: F) -> Matrix:
    return [[a+d*b for a,b in zip(r0,r1)] for r0,r1 in zip(case["H0"],case["H1"])]


def chart_output_bound(xhat: F, what: F, kappa: F, ex: F, ew: F) -> F:
    """Absolute bound for y=x^2(w+1/kappa) on a product error box."""
    if kappa<=0 or ex<0 or ew<0: raise ValueError("invalid chart/error data")
    return (2*abs(xhat)*ex+ex*ex)*(abs(what)+1/kappa)+(abs(xhat)+ex)**2*ew
