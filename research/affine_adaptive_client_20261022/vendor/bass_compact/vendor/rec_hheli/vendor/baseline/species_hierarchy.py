#!/usr/bin/env python3
"""Physically separated photon and massless neutrino LRS Boltzmann/Einstein hierarchy.

Source: BASS PR159 positive W=1+kappa P2, k=0 homogeneous Liouville,
BASS lowell-Bianchi T1/T7/T8/T9 moment couplings; elastic unpolarized Thomson
only gamma photons, never massless neutrinos. Full E/B/polarized electron
collision matrix is a further separate stage, not claimed here.

Omega_gamma determined by *source-locked* Tcmb0=2.7255K and actual
Stefan-Boltzmann energy density. The rounded source Omega_r is partitioned
without double counting, Omega_nu=Omega_r-Omega_gamma >=0. No kinetic-matter
stress closure; optional explicitly experimental first-order viscous heat.
"""
from __future__ import annotations
import math
from pathlib import Path
import time
import numpy as np
from scipy.integrate import solve_ivp
from scipy.constants import sigma as STEFAN_BOLTZMANN,G,c
from lrs_boltzmann import (
    SourceLock,ElectronHistory,SmoothElectronHistory,legendre_matrix,
    thomson_coeffs,GAMMA_PER_NE
)

class SpeciesHierarchy:
    def __init__(self,source:SourceLock,Tcmb0_K:float,ellmax=8,
            Sigma_ref=0.,orientation=0,kappa_gamma=0.,kappa_nu=0.,
            ne_source:ElectronHistory|SmoothElectronHistory|None=None,
            matter_nu_viscosity:float=0.):
        if not(isinstance(ellmax,int) and ellmax%2==0 and 2<=ellmax<=32):
            raise ValueError("L must be even 2..32")
        if not(math.isfinite(Tcmb0_K) and 2.5<Tcmb0_K<3.):
            raise ValueError("source-locked CMB temperature outside expected physical domain")
        if not(all(math.isfinite(z) and abs(z)<=.5 for z in (kappa_gamma,kappa_nu))):
            raise ValueError("initial W>=0 for both physical radiation species")
        if not(0<=Sigma_ref<math.sqrt(3) and orientation in (-1,0,1)
            and (Sigma_ref==0)==(orientation==0)
            and 0<=matter_nu_viscosity<=.1):
            raise ValueError("signed Sigma or phenomenological viscosity invalid")
        self.source=source
        self.ellmax=ellmax
        self.n=ellmax+1
        self.energy_matrix=legendre_matrix(ellmax,power=4)
        self.number_matrix=legendre_matrix(ellmax,power=3)
        self.thomson=thomson_coeffs(ellmax)
        self.ne_source=ne_source
        self.nu_visc=matter_nu_viscosity
        self.Sigma_ref=Sigma_ref
        self.orientation=orientation
        self.k_gamma=kappa_gamma
        self.k_nu=kappa_nu
        energy0=(4*STEFAN_BOLTZMANN/c)*Tcmb0_K**4
        omegarcrit=source.rhocrit_j_m3
        self.omega_gamma=energy0/omegarcrit
        self.omega_nu=source.omega_r-self.omega_gamma
        if not(0<self.omega_gamma<source.omega_r and self.omega_nu>0):
            raise RuntimeError("rounded source radiation Omega_r and CMB T0 physically inconsistent")
        self.f_gamma=self.omega_gamma/source.omega_r
        self.f_nu=self.omega_nu/source.omega_r
    def initial_state(self):
        n=self.n
        y=np.zeros(4+4*n,dtype=float)
        # 0: elapsedHref, 1:b,2:s/Href,3:u_th/rhocrit0
        y[2]=self.orientation*self.Sigma_ref/math.sqrt(3-self.Sigma_ref**2) if self.Sigma_ref else 0.
        for i,kap in ((0,self.k_gamma),(2,self.k_nu)):
            idx=4+i*n
            y[idx]=1.;y[idx+2]=kap/5
            idx+=n
            y[idx]=1.;y[idx+2]=kap/5
        return y
    def fields(self,x,y):
        a=self.source.aref*math.exp(x)
        tau,b,v,u=y[:4]
        n=self.n
        eg,ng,en,nn=[y[4+i*n:4+(i+1)*n] for i in range(4)]
        if not (np.isfinite(y).all() and 0<=x<=self.source.xmax+1e-12
                and eg[0]>0 and en[0]>0 and u>=-1e-12):
            raise ValueError("invalid radiation or matter Einstein state")
        s=v*self.source.href
        dust=self.source.omega_m/a**3
        rad=(self.omega_gamma*eg[0]+self.omega_nu*en[0])/a**4
        h=math.sqrt(self.source.h0**2*(dust+self.source.omega_lambda+rad+u)+s*s)
        piRnorm=(self.omega_gamma*eg[2]+self.omega_nu*en[2])/(3*a**4)
        piMnorm=-2*self.nu_visc*dust*s/h
        ngas=0. if self.ne_source is None else float(self.ne_source.density(a))
        gamma=ngas*GAMMA_PER_NE
        return {
            "a":a,"b":b,"s":s,"H":h,
            "Sigma":math.sqrt(3)*abs(s)/h,"epsilon_sigma":int(np.sign(s)),
            "E_gamma":eg,"N_gamma":ng,"E_nu":en,"N_nu":nn,
            "Pi_gamma_J_m3":self.source.rhocrit_j_m3*self.omega_gamma/a**4*eg[2]/3,
            "Pi_nu_J_m3":self.source.rhocrit_j_m3*self.omega_nu/a**4*en[2]/3,
            "Pi_matter_J_m3":self.source.rhocrit_j_m3*piMnorm,
            "Pi_radiation_J_m3":self.source.rhocrit_j_m3*piRnorm,
            "GammaT":gamma,"n_e_cm3":ngas,
            "rad_energy_norm":rad,"dust_energy_norm":dust,
            "thermal_energy_norm":u,
            "Pi_radiation_norm":piRnorm,"Pi_matter_norm":piMnorm,
        }
    def rhs(self,x,y):
        w=self.fields(x,y)
        a=w["a"];h=w["H"];s=w["s"];nu=w["Pi_matter_norm"]
        v=s/h
        g=w["GammaT"]/h
        eg,ng,en,nn=(w["E_gamma"],w["N_gamma"],w["E_nu"],w["N_nu"])
        d_eg=v*(self.energy_matrix@eg)-g*self.thomson*eg
        d_ng=v*(self.number_matrix@ng)-g*self.thomson*ng
        d_en=v*(self.energy_matrix@en)
        d_nn=v*(self.number_matrix@nn)
        dv=-3*y[2]+3*self.source.h0**2/(self.source.href*h)*(
                 w["Pi_radiation_norm"]+nu)
        du=-5*y[3]-6*v*nu
        return np.r_[self.source.href/h,v,dv,du,d_eg,d_ng,d_en,d_nn]
    def integrate(self,n=401,rtol=2e-12,atol=8e-14):
        begin=time.perf_counter()
        s=solve_ivp(self.rhs,(0,self.source.xmax),self.initial_state(),method="DOP853",
            rtol=rtol,atol=atol,t_eval=np.linspace(0,self.source.xmax,n))
        wall=time.perf_counter()-begin
        if not s.success:
            raise RuntimeError("SOURCE_SPECIES_EINSTEIN_COUPLING_FAIL:"+s.message)
        vals=[self.fields(float(x),s.y[:,i]) for i,x in enumerate(s.t)]
        if any(max(abs(w["N_gamma"][0]-1),abs(w["N_nu"][0]-1))>3e-12 for w in vals):
            raise RuntimeError("MASSLESS_GAMMA_NU_PHOTON_NUMBER_NOT_CONSERVED")
        return s,vals,wall
