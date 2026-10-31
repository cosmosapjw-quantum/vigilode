"""Nonaxial observer Lorentz boost of original source-owned BASS21 Q24 sky.

Metric (-,+,+,+), photon propagation n, b=v/c, bolometric d=4.
Frame b is a counterfactual observer at each original saved source epoch.
No owner IVP, and no replacement for generic PSTF hierarchy.
"""
from __future__ import annotations
import io,math,hashlib,zipfile
from functools import lru_cache
from pathlib import Path
import numpy as np
from scipy.special import lpmv,gammaln
from numpy.polynomial.legendre import leggauss

PARENT_SHA='e8faa022fb6e567a97f17f73cca4571f071ffa37de5ea8e9597b0c4d98b088bc'
PR151_SHA='6b1da36684dac122b53d98d8576d657bbf9ea4df5d038fcd6b5dbd332cb41124'
E_NPZ_SHA='b087a54bcd3bdbfaef5d91165634bf5c7590e6c43417112e4de0168327b3366b'
ELL_INPUT=np.arange(2,25,2,dtype=int)
OUTPUT_MODES=[(ell,m) for ell in range(2,9) for m in range(-ell,ell+1)]
FRAMES={
 'rest':(0.,0.,0.),
 'x001':(.001,0.,0.),
 'x005':(.05,0.,0.),
 'negx005':(-.05,0.,0.),
 'tilt45':(.05/np.sqrt(2.),0.,.05/np.sqrt(2.)),
}

def sha(b): return hashlib.sha256(b).hexdigest()

def load_original(parent_path):
    data=Path(parent_path).read_bytes()
    if sha(data)!=PARENT_SHA: raise ValueError('PARENT_PR153_SHA_MISMATCH')
    with zipfile.ZipFile(io.BytesIO(data)) as za:
        nested=za.read('source/PARENT_PR151_FULL_SOURCE.zip')
        if sha(nested)!=PR151_SHA: raise ValueError('PR151_SHA_MISMATCH')
        with zipfile.ZipFile(io.BytesIO(nested)) as zb:
            q=zb.read('source/PR151_ORIGINAL_SOURCE_FULL.zip')
            with zipfile.ZipFile(io.BytesIO(q)) as zc:
                w=zc.read('source/PR148_SOURCE_FULL.zip')
                with zipfile.ZipFile(io.BytesIO(w)) as zd:
                    t=zd.read('source/PR145_CORRECTED_FULL_SOURCE.zip')
                    with zipfile.ZipFile(io.BytesIO(t)) as ze:
                        payload=ze.read('results/HIGH_E_HIERARCHY_50x193.npz')
                        if sha(payload)!=E_NPZ_SHA: raise ValueError('OWNER_E_NPZ_SHA_MISMATCH')
                        with np.load(io.BytesIO(payload),allow_pickle=False) as f:
                            vals={k:f[k].copy() for k in ('cases','x','l','E_l0_plus')}
    if vals['E_l0_plus'].shape!=(50,193,12) or not np.array_equal(vals['l'],ELL_INPUT):raise ValueError('WRONG_CASE_SHAPE')
    return vals

def k_ell(l):
    return np.sqrt(4*np.pi * math.factorial(l+2)/((2*l+1)*math.factorial(l-2)))

@lru_cache(None)
def _prefactors(ell,m,mp):
    """Small Wigner d^ell_{m,mp}(theta) terms, fixed m first index.
    d_{0,-2} = P_l^2/k_l / sqrt((2l+1)/(4pi)).
    """
    j=ell
    logpref=.5*(gammaln(j+m+1)+gammaln(j-m+1)+gammaln(j+mp+1)+gammaln(j-mp+1))
    out=[]
    for k in range(max(0,mp-m),min(j+mp,j-m)+1):
        # Wigner d_{m,mp} standard formula with FIRST (m), SECOND (mp)
        # k from 0, (j+mp-k)!, (m-mp+k)!, (j-m-k)!
        a=j+mp-k;b=k;c=m-mp+k;d=j-m-k
        if min(a,b,c,d)<0:continue
        powc=2*j+mp-m-2*k
        pows=m-mp+2*k
        if min(powc,pows)<0:continue
        lc=logpref-(gammaln(a+1)+gammaln(b+1)+gammaln(c+1)+gammaln(d+1))
        out.append((float((-1)**(k+m-mp)*np.exp(lc)),powc,pows))
    return tuple(out)

def wigner_d(ell,m,mp,mu):
    mu=np.asarray(mu)
    c=np.sqrt(np.maximum(0.,(1+mu)/2));s=np.sqrt(np.maximum(0.,(1-mu)/2))
    y=np.zeros_like(mu,dtype=np.float64)
    for A,i,j in _prefactors(ell,m,mp):y+=A*c**i*s**j
    return y

def spin_y(ell,m,spin,mu,phi):
    return np.sqrt((2*ell+1)/(4*np.pi))*wigner_d(ell,m,-spin,mu)*np.exp(1j*m*phi)

def n_from_mu_phi(mu,phi):
    r=np.sqrt(np.maximum(0.,1-mu*mu))
    return np.stack((r*np.cos(phi),r*np.sin(phi),mu),axis=-1)

def basis_from_mu_phi(mu,phi):
    r=np.sqrt(np.maximum(0.,1-mu*mu))
    e_theta=np.stack((mu*np.cos(phi),mu*np.sin(phi),-r),axis=-1)
    e_phi=np.stack((-np.sin(phi),np.cos(phi),np.zeros_like(mu)),axis=-1)
    return e_theta,e_phi

def observer_inverse_geometry(mu_p,phi_p,b):
    """Return photon source direction mu,phi and transported dyad angle e^(i psi)."""
    beta=float(np.linalg.norm(b))
    np_vec=n_from_mu_phi(mu_p,phi_p)
    if beta==0.: return mu_p,phi_p,np.ones_like(mu_p,dtype=complex),np.ones_like(mu_p)
    if beta>=1:raise ValueError('SUPERLUMINAL_FRAME')
    bhat=np.asarray(b,dtype=float)/beta
    gam=1/np.sqrt(1-beta*beta)
    bn_p=np_vec@bhat
    Dinv=gam*(1+beta*bn_p)
    vec=(np_vec+((gam-1)*bn_p+gam*beta)[...,None]*bhat)/Dinv[...,None]
    vec/=np.linalg.norm(vec,axis=-1)[...,None]
    mu=np.clip(vec[...,2],-1,1)
    phi=np.arctan2(vec[...,1],vec[...,0])
    e_old,_=basis_from_mu_phi(mu,phi)
    e_new,f_new=basis_from_mu_phi(mu_p,phi_p)
    proj=e_old@bhat
    # boost e_old^mu=(0,e_old), remove boosted temporal component via photon null gauge
    e_transport=e_old+((gam-1)*proj)[...,None]*bhat+(gam*beta*proj)[...,None]*np_vec
    cospsi=np.sum(e_transport*e_new,axis=-1)
    sinpsi=np.sum(e_transport*f_new,axis=-1)
    phase=cospsi+1j*sinpsi
    return mu,phi,phase,1/Dinv

def spatial_grid(n_mu,n_phi):
    mu,w=leggauss(n_mu)
    phi=2*np.pi*np.arange(n_phi)/n_phi
    MU=np.repeat(mu,n_phi);PHI=np.tile(phi,n_mu)
    WEIGHT=np.repeat(w,n_phi)*(2*np.pi/n_phi)
    return MU,PHI,WEIGHT

def kernel_for_frame(b,n_mu=72,n_phi=96,doppler_weight=4):
    mu_p,phi_p,w=spatial_grid(n_mu,n_phi)
    mu,phi,phase,D=observer_inverse_geometry(mu_p,phi_p,np.asarray(b))
    qsource=np.stack([lpmv(2,int(l),mu)/k_ell(int(l)) for l in ELL_INPUT],axis=1)
    mod=D**doppler_weight
    # Q+iU has spin -2 and Q-iU spin +2 (for transported screen convention)
    boost_plus=qsource*((mod*np.conj(phase)**2)[:,None])
    boost_minus=qsource*((mod*phase**2)[:,None])
    E=[];B=[]
    for l,m in OUTPUT_MODES:
        yminus=np.conj(spin_y(l,m,-2,mu_p,phi_p))
        yplus=np.conj(spin_y(l,m,+2,mu_p,phi_p))
        a_minus=(w*yminus)@boost_plus
        a_plus=(w*yplus)@boost_minus
        E.append((a_minus+a_plus)/2)
        B.append((a_minus-a_plus)/(2j))
    return np.array(E).T,np.array(B).T

if __name__=='__main__':
    mu=np.linspace(-.94,.94,50);phi=np.linspace(0,3,50)
    for l in (2,4,6):
        print('BASIS',l,np.max(abs(spin_y(l,0,2,mu,phi)-lpmv(2,l,mu)/k_ell(l))))
    for b in [(0,0,0),(.05,0,0),(0,0,.05),(.05/np.sqrt(2),0,.05/np.sqrt(2))]:
        for d in (4,1):
            e,bb=kernel_for_frame(b,48,64,d)
            E1=e[0,:];B1=bb[0,:]
            print('FRAME',b,'weight',d,'Esource0->l2m0',E1[OUTPUT_MODES.index((2,0))], 'Bmax',np.max(abs(B1)), 'Bcoefs',[(OUTPUT_MODES[i],B1[i]) for i in np.argsort(abs(B1))[-3:]])
