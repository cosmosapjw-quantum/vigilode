"""Same BASS21 represented-rate frozen-state proof for a single implicit-Euler Newton correction.

NO actual RODAS stage, NO source-owned moving epoch, NO full IVP or BASS/REI equivalence.
The original BASS AtomicNetwork rate calculation is imported unmodified from the
PR269 source-locked 129-coefficient projection (numeric identity, not raw-file identity).
After evaluation, each represented binary64 rate is interpreted as an exact rational.
"""
from __future__ import annotations
from dataclasses import dataclass
from fractions import Fraction as Q
from pathlib import Path
import hashlib, math, sys
import numpy as np

PR269 = Path(__file__).resolve().parents[2] / 'source' / 'rei_pr269'
sys.path.insert(0, str(PR269 / 'code'))
import native_full_rate as native  # exact original BASS network, projected numeric coefficients

def frac(x):
    if isinstance(x,Q): return x
    if isinstance(x,(int,np.integer)): return Q(int(x))
    a=float(x)
    if not math.isfinite(a): raise ValueError('not finite')
    return Q.from_float(a)

def mat(a):
    return [[frac(z) for z in row] for row in a]

def vec(a):
    return [frac(z) for z in a]

def dot(x,y):
    return sum((u*v for u,v in zip(x,y)),Q(0))

def norm_w(v,w):
    return max((abs(a)/b for a,b in zip(v,w)),default=Q(0))

def outward(x:Q, up:bool) -> float:
    """Correct rational-to-binary64 enclosure using exact binary64 comparisons."""
    a=float(x)
    if not math.isfinite(a): raise ValueError('unrepresentable interval endpoint')
    while Q.from_float(a) < x if up else Q.from_float(a) > x:
        a=math.nextafter(a, math.inf if up else -math.inf)
        if not math.isfinite(a):raise ValueError('outward overflow')
    return a

@dataclass(frozen=True)
class Box:
    source: str
    y: tuple
    d: tuple
    M: tuple
    R: tuple
    q: tuple
    baryon: tuple
    nb: Q
    sigma: Q
    w: tuple
    p: tuple
    F0: tuple
    J0: tuple
    Jrad: tuple
    Frad: tuple
    mu_up: Q
    inv_up: Q
    cwp: Q

    def physical_sample(self,z):
        assert len(z)==len(self.y)
        if not all(abs(a-b)<=d for a,b,d in zip(z,self.y,self.d)):
            raise ValueError('state outside box')
        return evaluate_formula(self.M,self.R,self.q,self.nb,z)

    def residual_bound(self,candidate):
        x=vec(candidate); n=len(self.y)
        if len(x)!=n:raise ValueError('candidate shape')
        rho=Q(0)
        for i in range(n):
            bi=self.sigma*self.F0[i]
            rb=self.sigma*self.Frad[i]
            center=bi-x[i]+self.sigma*dot(self.J0[i],x)
            radius=rb+self.sigma*sum((self.Jrad[i][j]*abs(x[j]) for j in range(n)),Q(0))
            rho=max(rho,(abs(center)+radius)/self.w[i])
        return rho

    def certify(self,x,original_budget,stage_budget):
        rb=self.residual_bound(x)
        eb=self.cwp*self.inv_up*rb
        ok=rb<=frac(original_budget) and eb<=frac(stage_budget)
        return dict(accepted=ok,mu=float(self.mu_up),inverse=float(self.inv_up),
                    original_residual=float(rb),physical_stage_error=float(eb),
                    exact=(rb,eb))

    def export_csv(self,candidate,path):
        """Transfer one *conditional* BASS source box into PR112 N03 standalone Rust.
        Using outward binary64 endpoints; cannot itself prove epoch ownership.
        """
        n=len(self.y);x=vec(candidate)
        lines=['BASS21-FROZEN-PROPER-TIME-INTERVAL-V1',str(n),repr(outward(self.sigma,True))]
        # Sigma is exact binary64 1 in this run. More general sigma requires [sigma] proof.
        if frac(float(self.sigma))!=self.sigma: raise ValueError('sigma not representable')
        for i in range(n):
            for j in range(n):
                x0=self.J0[i][j];r=self.Jrad[i][j]
                lines.append(f'{outward(x0-r,False):.17g} {outward(x0+r,True):.17g}')
        for i in range(n):
            b=self.sigma*self.F0[i];rr=self.sigma*self.Frad[i]
            lines.append(f'{outward(b-rr,False):.17g} {outward(b+rr,True):.17g}')
        for z in self.w:lines.append(f'{float(z):.17g}')
        for z in self.p:lines.append(f'{float(z):.17g}')
        for z in x:lines.append(f'{float(z):.17g}')
        Path(path).write_text('\n'.join(lines)+'\n')


def evaluate_formula(M,R,q,nb,y):
    """Exact real polynomial model, native represented coefficients as exact rationals."""
    n=len(y);ne=nb*dot(q,y)
    r_y=[dot(R[i],y) for i in range(n)]
    f=[dot(M[i],y)+ne*r_y[i] for i in range(n)]
    j=[[M[i][k]+ne*R[i][k]+nb*r_y[i]*q[k] for k in range(n)] for i in range(n)]
    return tuple(f),tuple(tuple(row) for row in j)


def build_native_box(sigma=1.0,nb=1e6,rad_den=256):
    if not(math.isfinite(float(sigma)) and float(sigma)>=0):raise ValueError('sigma')
    # BASS owner: original AtomicNetwork, full21, frozen externally supplied T,UV,nb.
    net=native.new_native_network()
    native.source_manifest_check()
    y=np.asarray(net.initial_Y,float).copy()
    # Interior, physically nonnegative initial-source-compatible test state.
    # Only redistributes within each isotope charge block; no new isotope mass.
    for inds in ([1,2],[3,4],[5,6],[7,8,9],[10,11,12],[13,14,15,16],[17,18,19,20]):
        total=float(y[inds].sum())
        if total>0:
            y[inds]=total*.001/len(inds)
            y[inds[0]]+=total*.999
    T,UV=native.original_bass_bath_at_z(3.603019593287379)
    rate=net.rates(T,UV)
    M=mat(np.asarray(rate['G0'])+np.asarray(rate['D']))
    R=mat(rate['Gne'])
    q=vec(net.q);A=vec(net.A)
    yy=vec(y);d=[z/Q(rad_den) for z in yy]
    n=len(yy); nbq=frac(nb);s=frac(sigma)
    F,J=evaluate_formula(M,R,q,nbq,yy)
    dq=dot([abs(v) for v in q],d)
    rrad=[dot([abs(v) for v in row],d) for row in R]
    Jr=[[nbq*(abs(R[i][k])*dq+abs(q[k])*rrad[i]) for k in range(n)] for i in range(n)]
    Fr=[dot([abs(v) for v in J[i]],d)+nbq*dq*rrad[i] for i in range(n)]
    # Deliberately different w and p to exercise the norm-transport factor.
    w=[Q(2) if i%5==0 else Q(1) for i in range(n)]
    p=[Q(1) for _ in range(n)]
    mu=max(J[i][i]+Jr[i][i]+sum((max(abs(J[i][k]-Jr[i][k]),abs(J[i][k]+Jr[i][k]))*w[k]/w[i] for k in range(n) if k!=i),Q(0)) for i in range(n))
    margin=1-s*mu
    if margin<=0: raise ValueError('unproved resolvent margin')
    result=Box('BASS-PR234-NUMERIC-RATE-CONTENT-FROZEN',tuple(yy),tuple(d),
               tuple(tuple(row) for row in M),tuple(tuple(row) for row in R),tuple(q),tuple(A),nbq,s,
               tuple(w),tuple(p),F,J,tuple(tuple(row) for row in Jr),tuple(Fr),mu,1/margin,
               max(a/b for a,b in zip(w,p)))
    return net,rate,result,T,UV
