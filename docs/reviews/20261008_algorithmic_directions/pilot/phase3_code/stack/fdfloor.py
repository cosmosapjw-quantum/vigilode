"""FD-JVP attainable residual floor of restarted GMRES(40) on Bruss-50 stage-1 systems (scaled form), vs exact JVP."""
import sys, math, numpy as np
sys.path.insert(0,'.')
import stack, run
from scipy.integrate import solve_ivp
p = run.problem('bruss50'); f=p['f']
for rtol in (1e-6, 1e-8):
  atol=rtol
  for tt,hh in ((0.01,1e-3),(0.5,0.05),(2.0,0.1),(5.0,0.3)):
    s=solve_ivp(f,(0,tt),p['y0'],method='Radau',rtol=1e-10,atol=1e-10); y=s.y[:,-1]
    J=p['J'](tt,y); f0=f(tt,y); hg=hh*stack.g; ynorm=np.linalg.norm(y); n=len(y)
    D=1/(atol+rtol*abs(y)); b=hg*f0; Db=D*b
    def Jv(v):
        nv=np.linalg.norm(v)
        if nv==0: return np.zeros_like(v)
        sg=stack.SQEPS*(1+ynorm)/nv; return (f(tt,y+sg*v)-f0)/sg
    Ws=lambda v: D*((v/D) - hg*Jv(v/D))
    We=lambda v: D*((v/D) - hg*(J@(v/D)))
    for name,W in (('fd',Ws),('exact',We)):
        x=np.zeros(n); hist=[]
        for cyc in range(5):
            r=Db-W(x); rn=np.linalg.norm(r); hist.append(rn)
            if not (rn>0 and np.isfinite(rn)): break
            k=40; V=np.zeros((n,k+1)); H=np.zeros((k+1,k)); V[:,0]=r/rn; used=0
            for j in range(k):
                w=W(V[:,j])
                for _ in range(2):
                    for i in range(j+1):
                        h2=V[:,i]@w; H[i,j]+=h2; w-=h2*V[:,i]
                H[j+1,j]=np.linalg.norm(w); used=j+1
                if not (H[j+1,j] > 1e-14*np.linalg.norm(H[:j+2,j])): break
                V[:,j+1]=w/H[j+1,j]
            e1=np.zeros(used+1); e1[0]=rn
            yk=np.linalg.lstsq(H[:used+1,:used],e1,rcond=None)[0]; x=x+V[:,:used]@yk
        U=x; WU=W(U); rex=np.linalg.norm(Db-We(U))
        scale=np.linalg.norm(Db)+np.linalg.norm(U)+np.linalg.norm(U-WU)
        print(f'rtol={rtol:g} t={tt} h={hh} {name:5s} ||Db||={np.linalg.norm(Db):.1e} hist/||Db||=', ' '.join('%.1e'%(h_/np.linalg.norm(Db)) for h_ in hist), ' final/scale=%.1e exact-true/||Db||=%.1e'%(hist[-1]/scale, rex/np.linalg.norm(Db)), ' hg||J||=%.1e'%(hg*np.linalg.norm(J,2)), flush=True)
