#!/usr/bin/env python3
"""Outward binary64 enclosure of a fixed RODAS stage target; exact root oracle.

This is a research prototype, not the Rust controller. Floating candidates have
no accuracy assumption. The enclosure uses nextafter outward interval operations,
an exact rational 1x1/2x2 inverse witness, and validates against a rational root.
No ODE discretization-error or hardware speedup claim is made.
"""
from pathlib import Path
from fractions import Fraction as F
import argparse, hashlib, json, math, platform, sys, time, struct
import numpy as np

OP_COUNTS = {"interval_add": 0, "interval_mul": 0, "upper_add": 0, "upper_mul": 0}

def finite(x):
    if not math.isfinite(x): raise ArithmeticError("nonfinite enclosure; fail closed")
    return x
def down(x): return finite(math.nextafter(finite(x), -math.inf))
def up(x): return finite(math.nextafter(finite(x), math.inf))
def au(a,b):
    OP_COUNTS['upper_add'] += 1
    if not a: return b
    if not b: return a
    return up(a+b)
def mu(a,b):
    OP_COUNTS['upper_mul'] += 1
    if a == 0 or b == 0: return 0.0
    return up(a*b)
def upward_fraction(x):
    y=finite(float(x))
    return up(y) if F(y)<x else y
def upper_sum(x):
    z=0.
    for t in x: z=au(z,t)
    return z
def upper_dot(a,b): return upper_sum(mu(x,y) for x,y in zip(a,b))
def upper_matmul(a,b):
    bt=list(zip(*b));return [[upper_dot(row,col) for col in bt] for row in a]

class I:
    __slots__=('lo','hi')
    def __init__(self,lo,hi=None):
        self.lo=finite(float(lo)); self.hi=finite(float(lo if hi is None else hi))
        assert self.lo<=self.hi
    @staticmethod
    def as_i(x): return x if isinstance(x,I) else I(x)
    def __add__(self,r):
        r=I.as_i(r);OP_COUNTS['interval_add']+=1
        if self.lo==self.hi==0:return r
        if r.lo==r.hi==0:return self
        return I(down(self.lo+r.lo),up(self.hi+r.hi))
    __radd__=__add__
    def __neg__(self): return I(-self.hi,-self.lo)
    def __sub__(self,r): return self+-I.as_i(r)
    def __rsub__(self,r): return I.as_i(r)+-self
    def __mul__(self,r):
        r=I.as_i(r);OP_COUNTS['interval_mul']+=1
        if self.lo==self.hi==0 or r.lo==r.hi==0:return I(0)
        q=[finite(a*b) for a in (self.lo,self.hi) for b in (r.lo,r.hi)]
        return I(down(min(q)),up(max(q)))
    __rmul__=__mul__
    def abs_upper(self): return max(abs(self.lo),abs(self.hi))
    def includes(self,x):return F(self.lo)<=x<=F(self.hi)

def dot(a,b): return sum((x*y for x,y in zip(a,b)),F(0))
def idot(a,b): return sum((I.as_i(x)*I.as_i(y) for x,y in zip(a,b)),I(0))
def matvec(a,b):return [dot(row,b) for row in a]
def inverse_small(a):
    n=len(a)
    if n==1:
        if a[0][0]==0:raise ArithmeticError('singular exact W')
        return [[1/a[0][0]]]
    assert n==2
    det=a[0][0]*a[1][1]-a[0][1]*a[1][0]
    if det==0:raise ArithmeticError('singular exact W')
    return [[a[1][1]/det,-a[0][1]/det],[-a[1][0]/det,a[0][0]/det]]

def primitive_checks():
    vals=[0.,5e-324,-5e-324,1e-300,-1e-100,1.,-1.,math.nextafter(1.,2.),1e100]
    count=0
    for a in vals:
        for b in vals:
            z=I(a)+I(b);assert z.includes(F(a)+F(b));count+=1
            if math.isfinite(a*b):
                z=I(a)*I(b);assert z.includes(F(a)*F(b));count+=1
    try: I(1e308)*I(1e308)
    except ArithmeticError: overflow=True
    else: overflow=False
    assert overflow
    return {'exact_operation_checks':count,'overflow_fails_closed':overflow}

class Problem:
    def __init__(self,c,J,y,h,q):
        self.s=len(c['b']);self.n=len(y);self.J=np.array(J,dtype=float)
        self.y=np.array(y,dtype=float);self.h=float(h);self.q=np.array(q,dtype=float)
        self.gamma=float(c['gamma']);self.al=np.array(c['alpha'],dtype=float)
        self.L=np.array(c['l'],dtype=float);self.b=np.array(c['b']);self.bt=np.array(c['btilde'])
        for x in (self.al,self.L):
            if np.count_nonzero(np.triu(x)):
                raise ValueError('strict-lower coefficient premise failed: no silent truncation')
        self.af=[[F(float(v)) for v in row] for row in self.al]
        self.lf=[[F(float(v)) for v in row] for row in self.L]
        self.jf=[[F(float(v)) for v in row] for row in self.J]
        self.yf=list(map(lambda v:F(float(v)),self.y));self.hf=F(h);self.qf=list(map(lambda v:F(float(v)),self.q))
        self.wf=[[F(i==j)-self.hf*F(self.gamma)*self.jf[i][j] for j in range(self.n)] for i in range(self.n)]
        self.invf=inverse_small(self.wf)
        self.U=[[upward_fraction(abs(v)) for v in row] for row in self.invf]
        self.W=np.eye(self.n)-h*self.gamma*self.J
        # f(y)=J*y-q*y^2; a globally quadratic f(z) has Jacobian J at y
        # when f(z)=(J-2diag(q*y))*z+q*z^2. Its exact remainder is q*delta^2.
        self.gf=[self.hf*(dot(row,self.yf)-qq*yy*yy) for row,qq,yy in zip(self.jf,self.qf,self.yf)]
        self.rhs=np.tile(h*(self.J@self.y-self.q*self.y*self.y),(self.s,1))
    def solve(self,r):return np.linalg.solve(self.W,np.asarray(r).T).T
    def coupling(self,k):return self.h*(self.L@k)@self.J.T
    def residual(self,k):return k@self.W.T-self.coupling(k)-self.rhs-self.h*self.q*(self.al@k)**2
    def candidates(self):
        k=self.solve(self.rhs);k+=self.solve(self.coupling(k)+self.h*self.q*(self.al@k)**2)
        for _ in range(2):
            t0=self.solve(self.residual(k));t1=self.solve(self.coupling(t0));k-=t0+t1
        q1=k.copy();q2=k-self.solve(self.coupling(t1))
        self.solve(self.residual(q2))  # Retain the native protocol's q=0 diagnostic batch in work accounting.
        return [('q1',q1,6),('q2',q2,8)]
    def exact_root(self):
        k=[[F(0)]*self.n for _ in range(self.s)]
        for i in range(self.s):
            d=[dot(self.af[i], [row[a] for row in k]) for a in range(self.n)]
            lc=[dot(self.lf[i], [row[a] for row in k]) for a in range(self.n)]
            g=[self.gf[a]+self.hf*(dot(self.jf[a],lc)+self.qf[a]*d[a]*d[a]) for a in range(self.n)]
            k[i]=matvec(self.invf,g)
        return k
    def residual_interval(self,k):
        wi=[[I(float(i==j))-I(self.h)*I(self.gamma)*I(self.J[i,j]) for j in range(self.n)] for i in range(self.n)]
        gf=[I(self.h)*(idot(self.J[a],self.y)-I(self.q[a])*I(self.y[a])*I(self.y[a])) for a in range(self.n)]
        out=[];dhat=[]
        for i in range(self.s):
            d=[idot(self.al[i],k[:,a]) for a in range(self.n)]
            lc=[idot(self.L[i],k[:,a]) for a in range(self.n)]
            out.append([idot(wi[a],k[i])-I(self.h)*idot(self.J[a],lc)-gf[a]-I(self.h)*I(self.q[a])*d[a]*d[a] for a in range(self.n)])
            dhat.append([x.abs_upper() for x in d])
        return out,dhat
    def certify(self,k,atol,rtol):
        t=time.perf_counter();counts0=OP_COUNTS.copy()
        ri,dh=self.residual_interval(k)
        E=[[0.]*self.n for _ in range(self.s)]
        for i in range(self.s):
            d=[upper_dot(abs(self.al[i,:i]),[x[a] for x in E[:i]]) for a in range(self.n)]
            lc=[upper_dot(abs(self.L[i,:i]),[x[a] for x in E[:i]]) for a in range(self.n)]
            rhs=[]
            for a in range(self.n):
                lin=upper_dot(abs(self.J[a]),lc)
                rem=mu(abs(self.q[a]),au(mu(mu(2.,dh[i][a]),d[a]),mu(d[a],d[a])))
                rhs.append(au(ri[i][a].abs_upper(),mu(abs(self.h),au(lin,rem))))
            E[i]=[upper_dot(row,rhs) for row in self.U]
        # Observable is the actual binary64 projection, including its roundoff.
        yhat=self.y+self.b@k;ehat=self.bt@k
        B=[];Bt=[];projection=[]
        for a in range(self.n):
            yp=I(self.y[a])+idot(self.b,k[:,a]);ep=idot(self.bt,k[:,a])
            dy=(yp-I(yhat[a])).abs_upper();de=(ep-I(ehat[a])).abs_upper()
            B.append(au(upper_dot(abs(self.b),[x[a] for x in E]),dy))
            Bt.append(au(upper_dot(abs(self.bt),[x[a] for x in E]),de))
            projection.append([dy,de])
        # Scale is a frozen exact formula at the returned candidate, not root-dependent.
        scale_exact=[F(atol)+F(rtol)*max(abs(F(float(a))),abs(F(float(b)))) for a,b in zip(self.y,yhat)]
        scales=[float(v) for v in scale_exact]
        def wrms_upper(vec):
            squared=sum((F(v)/ss)**2 for v,ss in zip(vec,scale_exact))/self.n
            x=math.sqrt(float(squared))
            while F(x)*F(x)<squared:x=up(x)
            return x,squared
        outwrms,osq=wrms_upper(B);embwrms,esq=wrms_upper(Bt)
        embtotal=[au(abs(float(x)),r) for x,r in zip(ehat,Bt)]
        embtotalwrms,_=wrms_upper(embtotal)
        combined=au(outwrms,embtotalwrms)
        return dict(E=E,output_bound=B,embedded_difference_bound=Bt,output_wrms=outwrms,
                    embedded_difference_wrms=embwrms,embedded_target_wrms_upper=embtotalwrms,
                    combined_proxy_upper=combined,scale=scales,projection_roundoff=projection,
                    certificate_seconds=time.perf_counter()-t,op_counts={key:OP_COUNTS[key]-counts0[key] for key in OP_COUNTS},
                    residual_intervals=ri,yhat=yhat,ehat=ehat,scale_exact=scale_exact)
    def doubling_certificate(self,k,radius=1e-4):
        """Finite path-sum enclosure with a priori state radii; no root input.

        R_i = radius is a preset domain. If output E closes |alpha|E<=R,
        induction proves the quadratic remainder bound was used in its domain.
        H has 8 strict-lower stage blocks: H^8=0 independently of ||H||.
        S=(I+H)(I+H^2)(I+H^4), built in three mathematical doubling levels.
        """
        ri,dh=self.residual_interval(k);m=self.s*self.n
        H=[[0.]*m for _ in range(m)];a=[]
        for i in range(self.s):
            a+= [upper_dot(row,[v.abs_upper() for v in ri[i]]) for row in self.U]
            for j in range(i):
                for u in range(self.n):
                    for v in range(self.n):
                        inner=[]
                        for w in range(self.n):
                            ell=mu(abs(self.q[w]),au(mu(2.,dh[i][w]),radius))
                            term=mu(abs(self.J[w,v]),abs(self.L[i,j]))
                            if w==v:term=au(term,mu(ell,abs(self.al[i,j])))
                            inner.append(mu(self.U[u][w],term))
                        H[i*self.n+u][j*self.n+v]=mu(abs(self.h),upper_sum(inner))
        Q=H;S=[[float(i==j) for j in range(m)] for i in range(m)]
        for level in range(3):
            QS=upper_matmul(Q,S)
            S=[[au(x,y) for x,y in zip(sr,qr)] for sr,qr in zip(S,QS)]
            if level<2:Q=upper_matmul(Q,Q)
        ef=[upper_dot(row,a) for row in S];E=[ef[i*self.n:(i+1)*self.n] for i in range(self.s)]
        R=[[upper_dot(abs(self.al[i,:i]),[r[v] for r in E[:i]]) for v in range(self.n)] for i in range(self.s)]
        closes=all(x<=radius for row in R for x in row)
        return {'radius':radius,'closes':closes,'E':E,'state_radii':R,'doubling_levels':3,
                'matrix_dimension':m,'dense_matrix_multiplications':5,'scope':'parallelizable algebra only; this Python evaluator executes serially'}

def run_case(c,family,J,y,h,q):
    p=Problem(c,J,y,h,q);t=time.perf_counter();ref=p.exact_root();refseconds=time.perf_counter()-t
    b=list(map(lambda x:F(float(x)),p.b));bt=list(map(lambda x:F(float(x)),p.bt))
    exactout=[p.yf[a]+dot(b,[x[a] for x in ref]) for a in range(p.n)]
    exactemb=[dot(bt,[x[a] for x in ref]) for a in range(p.n)]
    rows=[]
    for method,k,depth in p.candidates():
        cert=p.certify(k,1e-8,1e-6);kf=[[F(float(x)) for x in row] for row in k]
        stage=all(abs(kf[i][a]-ref[i][a])<=F(cert['E'][i][a]) for i in range(p.s) for a in range(p.n))
        outdiff=[abs(F(float(cert['yhat'][a]))-exactout[a]) for a in range(p.n)]
        embdiff=[abs(F(float(cert['ehat'][a]))-exactemb[a]) for a in range(p.n)]
        oq=sum((x/s)**2 for x,s in zip(outdiff,cert['scale_exact']))/p.n
        eq=sum((x/s)**2 for x,s in zip(embdiff,cert['scale_exact']))/p.n
        outok=all(x<=F(B) for x,B in zip(outdiff,cert['output_bound']))
        embok=all(x<=F(B) for x,B in zip(embdiff,cert['embedded_difference_bound']))
        exactr=[]
        for i in range(p.s):
            d=[dot(p.af[i],[x[a] for x in kf]) for a in range(p.n)]
            lc=[dot(p.lf[i],[x[a] for x in kf]) for a in range(p.n)]
            exactr.append([dot(p.wf[a],kf[i])-p.gf[a]-p.hf*(dot(p.jf[a],lc)+p.qf[a]*d[a]*d[a]) for a in range(p.n)])
        residualok=all(cert['residual_intervals'][i][a].includes(exactr[i][a]) for i in range(p.s) for a in range(p.n))
        checks={'inverse_witness_exact':all(abs(p.invf[a][j])<=F(p.U[a][j]) for a in range(p.n) for j in range(p.n)),
                'residual_contains_exact':residualok,'stage_componentwise_exact':stage,'output_projection_exact':outok,
                'embedded_projection_exact':embok,'output_wrms_exact':oq<=F(cert['output_wrms'])**2,
                'embedded_difference_wrms_exact':eq<=F(cert['embedded_difference_wrms'])**2}
        assert all(checks.values()),(family,h,method,checks)
        dc=p.doubling_certificate(k)
        dc['stage_enclosure_checked_exact']=all(abs(kf[i][a]-ref[i][a])<=F(dc['E'][i][a]) for i in range(p.s) for a in range(p.n)) if dc['closes'] else None
        assert not dc['closes'] or dc['stage_enclosure_checked_exact']
        for key in ('residual_intervals','yhat','ehat','scale_exact'):cert.pop(key)
        rows.append(dict(family=family,h=h,method=method,n=p.n,checks=checks,candidate_w_batches=depth,
            candidate_w_vector_solves=depth*p.s,certificate_stage_layers=p.s,certificate=cert,
            exact_output_error_inf=float(max(outdiff)),exact_embedded_difference_inf=float(max(embdiff)),
            exact_output_wrms=float(math.sqrt(float(oq))),exact_embedded_difference_wrms=float(math.sqrt(float(eq))),
            certificate_accept_output_01=cert['output_wrms']<=.1,
            certificate_accept_combined_1=cert['combined_proxy_upper']<=1,
            doubling_certificate=dc,exact_reference_seconds=refseconds,exact_reference_max_numerator_bits=max(x.numerator.bit_length() for row in ref for x in row)))
        print(json.dumps({'family':family,'h':h,'method':method,'output_wrms':cert['output_wrms'],
                          'combined':cert['combined_proxy_upper'],'checks':checks}),flush=True)
    return rows

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--coefficients',type=Path,required=True);ap.add_argument('--out',type=Path,required=True)
    args=ap.parse_args();c=json.loads(args.coefficients.read_text())
    def decode(x):
        if isinstance(x,list):return [decode(v) for v in x]
        if isinstance(x,str):return struct.unpack('>d',bytes.fromhex(x))[0]
        return x
    c={key:decode(c[key]) for key in ('gamma','alpha','l','b','btilde')}
    t=time.perf_counter();checks=primitive_checks()
    rows=[]
    for family,J,y,q in [('affine',[[-1.]], [1.],[0.]),
                         ('nonnormal',[[-1.,100.],[0.,-1.]],[0.,1.],[0.,0.]),
                         ('quadratic',[[-1.]], [1.],[.25]),
                         ('nonnormal_quadratic',[[-1.,10.],[0.,-1.]],[.01,.01],[.05,.05])]:
        for h in [.1,1.,10.]: rows+=run_case(c,family,J,y,h,q)
    result={'schema':'vigilode.r3.homotopy.outward.v1','source_commit':'cc2cd041737e7ff543624d1b59893a3b4397369f',
            'coefficients_path':str(args.coefficients),'coefficients_sha256':hashlib.sha256(args.coefficients.read_bytes()).hexdigest(),
            'runtime':{'python':platform.python_version(),'numpy':np.__version__},'primitive_checks':checks,'rows':rows,
            'all_exact_checks':all(all(r['checks'].values()) for r in rows),'seconds':time.perf_counter()-t,
            'claim_ceiling':'fixed native rounded coefficient polynomial stage target, 1/2-dimensional exact inverse witness; no ODE global error, production admission, or speedup',
            'target_semantics':'native binary64 gamma/alpha/l/b/btilde treated as exact; W=I-h*gamma*J and g=h*(J*y-q*y^2) interpreted over rationals',
            'arithmetic_semantics':'CPython binary64 nearest arithmetic plus nextafter interval widening; exact Fraction inverse witness and WRMS squared evaluation; overflow rejects'}
    args.out.write_text(json.dumps(result,indent=2,allow_nan=False)+'\n');assert result['all_exact_checks']
if __name__=='__main__':main()
