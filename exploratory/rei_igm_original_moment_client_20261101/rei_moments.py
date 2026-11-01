"""VigilODE-owned exact three-moment reduction of REI's frozen original photon source.

This is a deliberately smaller physical IVP, NOT the original 26-event REI owner IVP.
One original proper-time tick and the original four groups are held constant.
"""
from __future__ import annotations
import json
from pathlib import Path
import numpy as np

ROOT=Path(__file__).resolve().parent

class MomentSource:
    def __init__(self, fixture=None):
        f=json.loads((Path(fixture) if fixture else ROOT/'frozen_pr332_source.json').read_text())
        self.f=f
        self.S0=np.array(f['global_neutral_particle_counts_S0'],float)
        self.J=np.array([g['original_photon_current_per_s'] for g in f['groups']],float)
        self.p0=np.array([g['baseline_species_absorption_fraction'] for g in f['groups']],float)
        self.e0=np.array([g['baseline_external_escape_fraction'] for g in f['groups']],float)
        self.h=float(f['dt_s'])
        assert self.p0.shape==(4,3) and np.all(self.S0>0) and np.all(self.J>=0)
        assert np.all(self.p0>=0) and np.all(self.e0>=0)
        assert np.allclose(np.sum(self.p0,axis=1)+self.e0,1.,atol=3e-14,rtol=0)

    def rates(self,w):
        w=np.asarray(w,float)
        if w.shape!=(3,) or not np.all(np.isfinite(w)) or not np.all(w>0):raise ValueError('non-positive neutral moments')
        den=self.e0+self.p0@w
        if np.any(den<=0):raise ValueError('vanished group photon sink')
        p=self.p0*w[None,:]/den[:,None]
        pe=self.e0/den
        return np.sum(self.J[:,None]*p,axis=0),p,pe

    def rhs(self,u,w):
        r,_,_=self.rates(w)
        sdot=np.array([-r[0],-r[1],r[1]-r[2]],float)
        return self.h*sdot/self.S0

    def jvp(self,u,w,v):
        v=np.asarray(v,float)
        if v.shape!=(3,):raise ValueError('bad tangent dimension')
        rate,p,_=self.rates(w)
        # Reuse same photons in direct+feedback derivative (no matrix assembled).
        share_tangent=np.sum(p*(v/w)[None,:],axis=1)
        dp=p*((v/w)[None,:]-share_tangent[:,None])
        dr=np.sum(self.J[:,None]*dp,axis=0)
        return self.h*np.array([-dr[0],-dr[1],dr[1]-dr[2]])/self.S0

    def full_jacobian(self,u,w):
        eye=np.eye(3)
        return np.column_stack([self.jvp(u,w,eye[:,j]) for j in range(3)])

    def rhs_aug(self,u,y):
        y=np.asarray(y,float)
        if y.shape!=(6,):raise ValueError("augmented photoionization state must have six components")
        w=y[:3]
        r,_,_=self.rates(w)
        rate=self.h*r/self.S0
        dydt=np.zeros(6)
        dydt[:3]=np.array([-rate[0],-rate[1],(self.S0[1]/self.S0[2])*rate[1]-rate[2]])
        dydt[3:]=rate
        return dydt

    def jvp_aug(self,u,y,v):
        y=np.asarray(y,float);v=np.asarray(v,float)
        if y.shape!=(6,) or v.shape!=(6,):raise ValueError("augmented tangent must have six components")
        w=y[:3]
        tangent=v[:3]
        r,p,_=self.rates(w)
        share_tangent=np.sum(p*(tangent/w)[None,:],axis=1)
        dp=p*((tangent/w)[None,:]-share_tangent[:,None])
        dr=self.h*np.sum(self.J[:,None]*dp,axis=0)/self.S0
        out=np.zeros(6)
        out[:3]=np.array([-dr[0],-dr[1],(self.S0[1]/self.S0[2])*dr[1]-dr[2]])
        out[3:]=dr
        return out

    def jac_aug(self,u,y):
        eye=np.eye(6)
        return np.column_stack([self.jvp_aug(u,y,eye[:,j]) for j in range(6)])

    def totals(self,y):
        """Cumulative original photoionization events from positive integral states."""
        return np.asarray(y[3:])*self.S0

    def normalize_source(self,u,w):
        _,p,pe=self.rates(w)
        return np.max(np.abs(np.sum(p,axis=1)+pe-1.0))
