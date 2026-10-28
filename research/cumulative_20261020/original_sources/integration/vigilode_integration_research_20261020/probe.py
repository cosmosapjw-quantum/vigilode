"""Exploratory raw-U stage homotopy, NOT a production VigilODE integration.
Pinned decimal coefficients transcribed from fixture bc55da4d... at c03ca90f...
Candidate operators act on vectors; explicit 8n matrices appear only in small
reference solves / determinant checks, never in the matrix-free GMRES path.
"""
from __future__ import annotations
import json,math,hashlib,time,sys
from pathlib import Path
from dataclasses import dataclass,field
from collections import Counter
import numpy as np
from scipy.sparse.linalg import LinearOperator,gmres

OUT=Path(__file__).resolve().parent
G=.21193756319429014
A=np.zeros((8,8)); C=np.zeros((8,8))
AR=[[],[3.],[2.849394379747939,.45842242204463923],[-6.954028509809101,2.489845061869568,-10.358996098473584],
[2.8029986275628964,.5072464736228206,-.3988312541770524,-.04721187230404641],
[-7.502846399306121,2.561846144803919,-11.627539656261098,-.18268767659942256,.030198172008377946],
[-7.502846399306121,2.561846144803919,-11.627539656261098,-.18268767659942256,.030198172008377946,1.],
[-7.502846399306121,2.561846144803919,-11.627539656261098,-.18268767659942256,.030198172008377946,1.,1.]]
CR=[[],[-14.155112264123755],[-17.97296035885952,-2.859693295451294],
[147.12150275711716,-1.41221402718213,71.68940251302358],
[165.43517024871676,-.4592823456491126,42.90938336958603,-5.961986721573306],
[24.854864614690072,-3.0009227002832186,47.4931110020768,5.5814197821558125,-.6610691825249471],
[30.91273214028599,-3.1208243349937974,77.79954646070892,34.28646028294783,-19.097331116725623,-28.087943162872662],
[37.80277123390563,-3.2571969029072276,112.26918849496327,66.9347231244047,-40.06618937091002,-54.66780262877968,-9.48861652309627]]
for i in range(8):A[i,:i]=AR[i];C[i,:i]=CR[i]
BOUT=np.array(AR[7]+[1.])

def save(name,obj):
    (OUT/name).write_text(json.dumps(obj,indent=2,sort_keys=True,allow_nan=False)+'\n')

def rms(x):return float(np.linalg.norm(x)/np.sqrt(x.size))
@dataclass
class Toy:
    a:np.ndarray
    eps:float
    y:np.ndarray
    h:float
    counts:Counter=field(default_factory=Counter)
    @property
    def n(self):return self.y.size
    @property
    def j(self):return self.a+2*self.eps*self.y
    @property
    def d(self):return 1-self.h*G*self.j
    def f(self,x):return self.a*x+self.eps*x*x
    def rhs0(self):return np.broadcast_to(self.h*G*self.f(self.y),(8,self.n)).copy()
    def B(self,u):
        # One eight-row batched base-JVP action, no formed spatial matrix.
        self.counts['base_JVP_batches']+=1;self.counts['base_JVP_vectors']+=8
        return G*(C@u)+self.h*G*self.j*(A@u)
    def nonlinear(self,u):
        self.counts['nonlinear_rhs_batches']+=1;self.counts['rhs_vectors']+=8
        x=A@u
        # Analytic cancellation-safe remainder for this known quadratic model.
        return self.eps*x*x
    def residual(self,u,theta=1.,lam=1.):
        eta=theta+lam*(1-theta)
        return u*self.d-eta*self.B(u)-self.rhs0()-lam*self.h*G*self.nonlinear(u)
    def lower_derivative(self,u,v,theta,lam):
        eta=theta+lam*(1-theta)
        av=A@v;au=A@u
        self.counts['stage_JVP_batches']+=1;self.counts['stage_JVP_vectors']+=8
        return eta*self.B(v)+lam*self.h*G*(2*self.eps*au)*av
    def dinv(self,v):
        self.counts['W_inverse_batches']+=1;self.counts['W_inverse_vectors']+=8
        return v/self.d
    def exact_target(self,lam=1.,theta=1.):
        # Reference only: exploit triangular equation; this is the eight-stage
        # sequential target used to check, never an alleged parallel candidate.
        u=np.zeros((8,self.n));eta=theta+lam*(1-theta)
        for i in range(8):
            x=A[i]@u
            u[i]=(self.h*G*self.f(self.y)+G*eta*(C[i]@u)+self.h*G*eta*self.j*x+
                  lam*self.h*G*self.eps*x*x)/self.d
        return u
    def linear_seed(self,theta):
        # Small reference-seed solve. This oracle is explicitly NOT counted as
        # a matrix-free constructed predictor. A separate Arnoldi test below
        # evaluates actual operator-only affine predictor feasibility.
        out=np.zeros((8,self.n))
        for k in range(self.n):
            m=np.eye(8)*self.d[k]-theta*(G*C+self.h*G*self.j[k]*A)
            out[:,k]=np.linalg.solve(m,np.full(8,self.h*G*self.f(self.y)[k]))
        return out
    def normed(self,u,ref):
        yref=self.y+BOUT@ref
        scale=1e-9+1e-6*np.abs(yref)
        return rms((BOUT@(u-ref))/scale)

def picard(t:Toy,start,steps=8):
    u=start.copy();ref=t.exact_target();rows=[]
    for k in range(steps+1):
        rows.append({'round':k,'output_WRMS_vs_stage_root':t.normed(u,ref),
                     'residual_l2':float(np.linalg.norm(t.residual(u)))})
        if k<steps:u=t.dinv(t.rhs0()+t.B(u)+t.h*G*t.nonlinear(u))
    return rows

def newton_path(t:Toy,theta,lam_nodes,q):
    t.counts.clear()
    u=t.linear_seed(theta)
    ref=t.exact_target();history=[];success=True
    for lam in lam_nodes:
        for it in range(12):
            r=t.residual(u,theta,lam)
            rel=np.linalg.norm(r)/max(np.linalg.norm(t.rhs0()),1e-30)
            if rel<2e-12:break
            old=u.copy()
            def apply(v):
                z=v.reshape(8,t.n)
                return (z*t.d-t.lower_derivative(old,z,theta,lam)).ravel()
            def prec(v):
                term=t.dinv(v.reshape(8,t.n));total=term.copy()
                for _ in range(q):
                    term=t.dinv(t.lower_derivative(old,term,theta,lam));total+=term
                return total.ravel()
            m=LinearOperator((8*t.n,8*t.n),matvec=apply,dtype=float)
            p=LinearOperator((8*t.n,8*t.n),matvec=prec,dtype=float)
            iters=[]
            delta,info=gmres(m,-r.ravel(),M=p,rtol=1e-11,atol=1e-14,restart=min(64,8*t.n),maxiter=4,
                             callback=lambda res:iters.append(float(res)),callback_type='pr_norm')
            t.counts['Krylov_iterations']+=len(iters)
            true_res=float(np.linalg.norm(apply(delta)+r.ravel()))
            if info!=0 or not np.isfinite(delta).all():success=False;break
            u+=delta.reshape(8,t.n)
            history.append({'lambda':lam,'outer_iteration':it,'inner_iterations':len(iters),
                            'true_linear_residual':true_res,'output_WRMS':t.normed(u,ref)})
        else:success=False
        if not success:break
    return {'success':success,'output_WRMS':t.normed(u,ref),'target_relative_residual':float(np.linalg.norm(t.residual(u))/max(np.linalg.norm(t.rhs0()),1e-30)),
            'outer_corrections':len(history),'counts':dict(t.counts),'trace':history,
            'affine_seed_cost':'excluded oracle; cannot use totals as full algorithm timing',
            'parallel_execution':'batched operator prototype; no CPU stage parallelism measured in this test'}


def arnoldi_affine(diag,y,h,max_m=32):
    # Physical Krylov V=K_m(J,f0); no n*n Jacobian, W, or full stage matrix.
    n=len(diag);f0=diag*y;beta=np.linalg.norm(f0);V=np.zeros((n,max_m+1));V[:,0]=f0/beta
    H=np.zeros((max_m+1,max_m));records=[];xlast=None
    def jvp(v):return diag*v
    exact=np.zeros((8,n));d=1-h*G*diag
    for i in range(8):exact[i]=(h*G*f0+G*(C[i]@exact)+h*G*diag*(A[i]@exact))/d
    for k in range(max_m):
        w=jvp(V[:,k])
        for _ in range(2):
            for j in range(k+1):z=V[:,j]@w;H[j,k]+=z;w-=z*V[:,j]
        H[k+1,k]=np.linalg.norm(w)
        if H[k+1,k]>1e-14*np.max(np.abs(diag)):V[:,k+1]=w/H[k+1,k]
        m=k+1;smallJ=H[:m,:m]
        # Only 8m-dimensional projected system is formed. The small dense solve
        # is a reference implementation for a future bounded projected backend.
        M=np.kron(np.eye(8),np.eye(m)-h*G*smallJ)-np.kron(G*C,np.eye(m))-np.kron(h*G*A,smallJ)
        b=np.zeros((8,m));b[:,0]=h*G*beta
        z=np.linalg.solve(M,b.ravel()).reshape(8,m);U=z@V[:,:m].T
        r=U*d-G*C@U-h*G*diag*(A@U)-h*G*f0
        err=rms((BOUT@(U-exact))/(1e-9+1e-6*np.abs(y+BOUT@exact)))
        records.append({'m':m,'JVP_basis_vectors':m,'residual_validation_JVP_vectors':8,
                        'raw_rhs_rank':1,'full_relative_stage_residual':float(np.linalg.norm(r)/np.linalg.norm(b)),
                        'output_WRMS_vs_stage_root':err})
        xlast=U
        if np.linalg.norm(r)/np.linalg.norm(b)<1e-11:break
        if H[k+1,k]<=1e-14*np.max(np.abs(diag)):break
    return records

def nilpotent_demo():
    rows=[]
    for z in [-.01,-1.,-100.,-1e6]:
        D=1-G*z;T=(G*C+G*z*A)/D
        norms=[float(np.linalg.norm(np.linalg.matrix_power(T,k),np.inf)) for k in range(1,9)]
        residual_growth=[];b=np.ones(8)*G*z;ref=np.linalg.solve(np.eye(8)*D-G*C-G*z*A,b);u=np.zeros(8)
        for k in range(9):
            residual_growth.append(float(np.linalg.norm(u-ref,np.inf)))
            u=T@u+b/D
        rows.append({'z':z,'spectral_radius':float(np.max(np.abs(np.linalg.eigvals(T)))), 'norm_T_powers':norms,'error_per_round':residual_growth})
    return rows

def determinant_test():
    rng=np.random.default_rng(220)
    t=Toy(np.array([-1.,-20.,-100.]),10.,np.array([.01,.02,.03]),.02)
    worst=0.;Jvals=[]
    for theta in [0.,.5,1.]:
        for lam in [0.,.25,.5,1.]:
            U=rng.normal(size=(8,t.n))*.01
            eye=np.eye(8*t.n)
            jac=np.column_stack([(eye[:,k].reshape(8,t.n)*t.d-t.lower_derivative(U,eye[:,k].reshape(8,t.n),theta,lam)).ravel() for k in range(8*t.n)])
            logdet=np.linalg.slogdet(jac)
            exp=8*np.log(t.d).sum();worst=max(worst,abs(logdet[1]-exp))
            Jvals.append({'theta':theta,'lambda':lam,'logdet_error':abs(logdet[1]-exp),
                          'strict_upper_max':float(np.max(np.abs(np.triu(jac,1))))})
    return {'max_logdet_error':worst,'cases':Jvals}

def main():
    save('fixture_numeric_subset.json',{'source_ref':'c03ca90f19a9e64ba7751c069325a48b54b39c2e','source_git_blob':'bc55da4d43549ffbcee3cb25e31be05658507742','gamma':G,'A':A.tolist(),'C':C.tolist(),'b_code':BOUT.tolist(),
        'identity':'numeric subset of authoritative decimal snapshot, not original-byte artifact'})
    save('nilpotence.json',nilpotent_demo());save('determinant.json',determinant_test())
    rows=[];curves=[]
    for h,eps in [(1e-2,0.),(1e-2,1.),(1e-2,10.),(.1,1.),(.1,10.),(.1,50.)]:
        t=Toy(np.array([-1.,-10.,-100.,-1000.]),eps,np.array([.1,.1,.1,.1]),h)
        for theta,nodes,q in [(0.,[1.],0),(0.,[.25,.5,.75,1.],0),(1.,[1.],0),(1.,[1.],1),(1.,[.25,.5,.75,1.],1)]:
            r=newton_path(t,theta,nodes,q)
            rows.append({'h':h,'epsilon':eps,'theta':theta,'lambda_nodes':nodes,'q':q,**r})
        curves.append({'h':h,'epsilon':eps,'cold':picard(t,np.zeros((8,4))),'affine_seed':picard(t,t.linear_seed(1.))})
    save('homotopy.json',rows);save('round_curves.json',curves)
    save('affine_krylov.json',{
        'few_spectral_modes':arnoldi_affine(np.tile([-1.,-20.,-100.],128),np.ones(384),.02,20),
        'broad_spectrum_rank_one':arnoldi_affine(-np.logspace(0,4,64),np.ones(64),.02,32)})
    assert determinant_test()['max_logdet_error']<1e-10
    assert all(r['norm_T_powers'][-1]==0 for r in nilpotent_demo())
    summary={'scope':'lightweight exploratory raw-U probe, not production integration/performance certification',
         'homotopy_configurations':len(rows),'homotopy_completed':sum(r['success'] for r in rows),
         'homotopy_final_output_WRMS_max':max(r['output_WRMS'] for r in rows if r['success']),
         'matrix_free_candidate':'full-target and projected-affine operators use JVP-style actions; small oracle seeds disclosed',
         'timing_claim':False,'production_crates_modified':False,'novelty_claim':False}
    save('SUMMARY.json',summary);print(json.dumps(summary,indent=2))
if __name__=='__main__':main()
