"""Constructive JVP-only subspace + all-stage parallel homotopy prototype.
Synthetic quadratic diagonal model; not native VigilODE, not a speed benchmark.
Only V(n,m), projected J(m,m), and stage Jacobian(8m,8m) are stored.
Full-dimensional stage RHS and stage derivative projections run concurrently.
"""
import json, threading, time
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
import numpy as np
from probe import A,C,G,BOUT,OUT,Toy,save,rms

class ParallelProjected:
    def __init__(self,n=12288,eps=1.,h=.01,workers=4):
        assert n%3==0
        self.a=np.tile(np.array([-1.,-20.,-100.]),n//3)
        self.y=np.full(n,.1);self.eps=eps;self.h=h;self.j=self.a+2*eps*self.y
        self.n=n;self.counts=Counter();self.lock=threading.Lock();self.active=0;self.peak=0
        self.pool=ThreadPoolExecutor(max_workers=workers)
    def jvp(self,v):
        self.counts['base_JVP_vectors']+=1
        return self.j*v
    def basis(self):
        f0=self.a*self.y+self.eps*self.y**2;b=np.linalg.norm(f0)
        V=np.zeros((self.n,9));H=np.zeros((9,8));V[:,0]=f0/b
        for k in range(8):
            w=self.jvp(V[:,k])
            for _ in range(2):
                for i in range(k+1):z=V[:,i]@w;H[i,k]+=z;w-=z*V[:,i]
            H[k+1,k]=np.linalg.norm(w)
            if H[k+1,k]<1e-11:break
            V[:,k+1]=w/H[k+1,k]
        self.V=V[:,:k+1];self.J=H[:k+1,:k+1];self.m=k+1
        self.b=np.zeros((8,self.m));self.b[:,0]=self.h*G*b
        self.L=np.kron(np.eye(8),np.eye(self.m)-self.h*G*self.J)-np.kron(G*C,np.eye(self.m))-np.kron(self.h*G*A,self.J)
        return np.linalg.solve(self.L,self.b.ravel()).reshape(8,self.m)
    def stage(self,x):
        with self.lock:self.active+=1;self.peak=max(self.peak,self.active)
        try:
            # exact nonlinear remainder for this model, including its derivative
            # projection. No sleep/barrier/artificial work to force overlap.
            N=self.eps*x*x
            dJ=(self.V.T*(2*self.eps*x))@self.V
            return N@self.V,dJ
        finally:
            with self.lock:self.active-=1
    def eval(self,z,lam):
        X=(A@z)@self.V.T
        out=list(self.pool.map(self.stage,[X[i] for i in range(8)]))
        self.counts['parallel_stage_batches']+=1;self.counts['rhs_stage_vectors']+=8
        self.counts['projected_derivative_stage_JVP_equivalents']+=8*self.m
        N=np.array([r[0] for r in out])
        H=self.L@z.ravel()-self.b.ravel()-lam*self.h*G*N.ravel()
        K=self.L.copy()
        for i in range(8):
            for j in range(i):K[i*self.m:(i+1)*self.m,j*self.m:(j+1)*self.m]-=lam*self.h*G*A[i,j]*out[i][1]
        return H,K
    def run(self,nodes):
        z=self.basis();steps=[]
        for lam in nodes:
            for it in range(8):
                F,K=self.eval(z,lam)
                residual=np.linalg.norm(F)/np.linalg.norm(self.b)
                if residual<1e-12:break
                delta=np.linalg.solve(K,-F);z+=delta.reshape(8,self.m)
                self.counts['small_all_stage_linear_solves']+=1
                steps.append({'lambda':lam,'iteration':it,'projected_residual_before':float(residual)})
            else:raise RuntimeError('projected convergence budget exhausted')
        U=z@self.V.T
        # Independent full-space target residual, not only projected residual.
        f0=self.a*self.y+self.eps*self.y**2
        r=U*(1-self.h*G*self.j)-G*C@U-self.h*G*self.j*(A@U)-self.h*G*f0-self.h*G*self.eps*(A@U)**2
        t=Toy(self.a,self.eps,self.y,self.h);ref=t.exact_target()
        result={'n':self.n,'subspace_dimension':self.m,'maximum_projected_system_dimension':8*self.m,
                'full_space_residual_l2':float(np.linalg.norm(r)),
                'full_space_relative_residual':float(np.linalg.norm(r)/np.linalg.norm(self.h*G*np.tile(f0,(8,1)))),
                'output_WRMS_vs_sequential_stage_root':t.normed(U,ref),'counts':dict(self.counts),
                'observed_concurrent_stage_tasks':self.peak,'lambda_nodes':nodes,'newton_trace':steps,
                'full_n_by_n_matrix_constructed':False,'full_W_linear_solves':0,
                'actual_parallelism':'ThreadPoolExecutor stage tasks; no acceleration ratio claimed',
                'scope':'invariant three-mode synthetic diagonal quadratic system; not generic nonlinear PDE or native Rust validation'}
        self.pool.shutdown()
        return result
if __name__=='__main__':
    result=[]
    for nodes in [[1.],[.25,.5,.75,1.]]:
        r=ParallelProjected().run(nodes);assert r['full_space_relative_residual']<1e-10
        assert r['output_WRMS_vs_sequential_stage_root']<.01;result.append(r)
    save('projected_parallel.json',result)
    print(json.dumps(result,indent=2))
