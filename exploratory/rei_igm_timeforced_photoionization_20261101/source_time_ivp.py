"""15-state source-frozen original REI photon-moment IVP and analytic matrix-free JVP.

Genuine original four-group source snapshots from REI PR332 and PR342; linear
forcing *between* five knots is a stated separate assumption. This is not the
original REI 26-event owner or verified cross-parent local-error certificate.
"""
from __future__ import annotations
import json,math
from pathlib import Path
import numpy as np
from scipy.integrate import solve_ivp
ROOT=Path(__file__).resolve().parent
S0=3; ABS=slice(3,6); ESC=slice(6,10); EM=slice(10,14); BIND=14; ND=15
class SourceIVP:
    def __init__(self,fixture=None,frozen=False):
        f=json.loads((Path(fixture) if fixture else ROOT/'five_clock_pr342_original_source.json').read_text())
        self.f=f;self.h=f['h_seconds'];self.S=np.array(f['S0_particles'],float);self.E=np.array(f['ionization_potentials_eV'],float)
        self.times=np.array([x['u'] for x in f['sites']],float)
        self.J=np.array([x['J_per_s'] for x in f['sites']],float)
        self.W=np.array([x['weights'] for x in f['sites']],float)
        self.frozen=frozen
        if not (self.W.shape == (5, 4, 4) and self.J.shape == (5, 4) and np.all(self.J >= 0)): raise ValueError('source_time_ivp.py original source numerical contract line 22')
        if not (np.all(self.S > 0) and np.all(self.W >= 0)): raise ValueError('source_time_ivp.py original source numerical contract line 23')
        if not (np.max(np.abs(self.W.sum(axis=2) - 1)) < 3e-14): raise ValueError('source_time_ivp.py original source numerical contract line 24')
        if not (np.all(np.diff(self.times) > 0)): raise ValueError('source_time_ivp.py original source numerical contract line 25')
    def forcing(self,u):
        if self.frozen:return self.J[0],self.W[0]
        if not np.isfinite(u) or u < -1e-13 or u>1+1e-13:raise ValueError('time outside source knots')
        k=int(np.clip(np.searchsorted(self.times,u,side='right')-1,0,len(self.times)-2))
        lam=np.clip((u-self.times[k])/(self.times[k+1]-self.times[k]),0,1)
        J=self.J[k]*(1-lam)+self.J[k+1]*lam
        W=self.W[k]*(1-lam)+self.W[k+1]*lam
        return J,W
    def rates(self,u,w):
        if np.any(np.asarray(w)<=0):raise ValueError('nonpositive neutral species')
        J,W=self.forcing(u)
        a=W[:,:3]*np.asarray(w)[None,:]
        den=W[:,3]+np.sum(a,axis=1)
        if not np.all(den>0):raise ValueError('no photon sink')
        p=a/den[:,None]
        pe=W[:,3]/den
        r=np.sum(J[:,None]*p,axis=0)
        return r,p,pe,J
    def rhs(self,u,y):
        y=np.asarray(y,float)
        if y.shape!=(ND,):raise ValueError('15 state dimensions required')
        r,p,pe,J=self.rates(u,y[:3]);k=self.h*r/self.S
        out=np.zeros(ND)
        out[:3]=[-k[0],-k[1],(self.S[1]/self.S[2])*k[1]-k[2]]
        out[ABS]=k;out[ESC]=self.h*J*pe/self.S[0];out[EM]=self.h*J/self.S[0]
        out[BIND]=self.h*np.dot(self.E,r)/self.S[0]
        return out
    def jvp(self,u,y,v):
        y=np.asarray(y,float);v=np.asarray(v,float)
        _,p,pe,J=self.rates(u,y[:3]);d=p*(v[:3]/y[:3])[None,:]
        d-=p*np.sum(d,axis=1)[:,None]
        dr=np.sum(J[:,None]*d,axis=0);dk=self.h*dr/self.S
        out=np.zeros(ND)
        out[:3]=[-dk[0],-dk[1],(self.S[1]/self.S[2])*dk[1]-dk[2]]
        out[ABS]=dk
        out[ESC]=-self.h*J*pe*np.sum(p*(v[:3]/y[:3])[None,:],axis=1)/self.S[0]
        out[BIND]=self.h*np.dot(self.E,dr)/self.S[0]
        return out
    def jac(self,u,y):
        return np.column_stack([self.jvp(u,y,np.eye(ND)[i]) for i in range(ND)])
    @staticmethod
    def initial():return np.r_[np.ones(3),np.zeros(ND-3)]
    def invariants(self,y):
        y=np.asarray(y,float);x=y if y.ndim==2 else y[None,:]
        h=x[:,0]+x[:,3]-1
        he1=x[:,1]+x[:,4]-1
        he2=x[:,2]-1-self.S[1]/self.S[2]*x[:,4]+x[:,5]
        counts=x[:,10:14].sum(axis=1)-x[:,6:10].sum(axis=1)-np.sum(x[:,3:6]*self.S[None,:]/self.S[0],axis=1)
        binding=x[:,14]-np.sum(x[:,3:6]*self.E[None,:]*self.S[None,:]/self.S[0],axis=1)
        return np.column_stack([h,he1,he2,counts,binding])
    def solve(self,method='Radau',rtol=2e-12,atol=2e-14):
        samples=sorted(set([float(i)/8 for i in range(9)]+[float(x) for x in self.times if 0<=x<=1]))
        samples=[x for x in samples if x<=1]
        if samples[-1]<1:samples+=[1.]
        params=dict(method=method,rtol=rtol,atol=atol,t_eval=samples,max_step=.13)
        if method in ('Radau','BDF'):params['jac']=self.jac
        r=solve_ivp(self.rhs,(0.,1.),self.initial(),**params)
        if not r.success:raise RuntimeError(str(r.message))
        return r

if __name__=='__main__':
    for frozen in (False,True):
        model=SourceIVP(frozen=frozen);a=model.solve();b=model.solve(method='DOP853',rtol=2e-12,atol=2e-14)
        print('frozen',frozen,'radau',a.nfev,'dop853',b.nfev,'diff',np.max(np.abs(a.y-b.y)),'end',a.y[:,-1], 'invariants',np.max(np.abs(model.invariants(a.y.T))))
