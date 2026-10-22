#!/usr/bin/env python3
"""BASS PSTF Boltzmann → operational non-tilted LRS Bianchi I Legendre adapter.

Exact homogeneous angular Liouville polynomial operator projected to L<=Lmax.
Unpolarized elastic Thomson angular redistribution is the E=0 slice of the
BASS photon collision hierarchy, not the complete E/B/Thomson polarisation.

Frozen source:
  BASS 'PLAN-H-boltzmann-hierarchy.md' T1/T7/T8/T9 + collision K0,K2,K>=3
  htt_base/src/pstf/hierarchy.rs (reference registry, not directly invoked)
  BASS PR159 positive primordial W=1+kappa P2 and exact moments.

Energy-integrated comoving frame brightness F(mu,x)=a^4 I/I_ref a_ref^4:
  ∂_x F = (s/H)[3 mu (1-mu²)∂_mu F-4(3mu²-1)F] + C_T/F_H
J_l=<F P_l>, F=sum(2l+1)J_l P_l.
Thomson C0=0,C2=-9/10 Γ J2,C_l>=3=-ΓJ_l (unpolarized);
with polarization the E_2 coupling must be explicitly restored, not guessed.
"""
from __future__ import annotations
import math
from dataclasses import dataclass
from pathlib import Path
import numpy as np
from numpy.polynomial.legendre import Legendre,leggauss
from scipy.interpolate import PchipInterpolator,CubicSpline
from scipy.integrate import solve_ivp

G_SI=6.67430e-11
C_SI=299792458.
SIGMA_T_CM2=6.6524587051e-25
C_CGS=2.99792458e10
GAMMA_PER_NE=C_CGS*SIGMA_T_CM2

@dataclass(frozen=True)
class SourceLock:
    h0:float
    omega_m:float
    omega_r:float
    omega_lambda:float
    aref:float
    aend:float
    def __post_init__(self):
        assert self.h0>0 and self.aref>0 and self.aend>self.aref
    @property
    def href(self):
        return self.h0*math.sqrt(self.omega_m/self.aref**3+
            self.omega_r/self.aref**4+self.omega_lambda)
    @property
    def rhocrit_j_m3(self):
        return 3*C_SI**2*self.h0**2/(8*math.pi*G_SI)
    @property
    def xmax(self):
        return math.log(self.aend/self.aref)

def legendre_matrix(lmax:int,order:int|None=None,power:int=4)->np.ndarray:
    if power not in (3,4):raise ValueError('photon number or energy transport power only')
    if lmax<2 or lmax>32 or lmax%2:
        raise ValueError("even Lmax in [2,32] for source LRS tower")
    quad_order=order if order is not None else max(64,lmax+8)
    if quad_order<lmax+4:raise ValueError("underintegrated BASS shear operator")
    mu,w=leggauss(quad_order)
    w=w/2
    d=np.empty((lmax+1,lmax+1))
    for ell in range(lmax+1):
        p=Legendre.basis(ell)(mu)
        for m in range(lmax+1):
            leg=Legendre.basis(m)
            f=(2*m+1)*leg(mu)
            df=(2*m+1)*leg.deriv()(mu)
            directional=3*mu*(1-mu*mu)*df-power*(3*mu*mu-1)*f
            d[ell,m]=np.dot(w*p,directional)
    # Integrate by parts exactly; monopole energy work = -2 times quadrupole J2.
    analytical=np.zeros(lmax+1)
    analytical[2]=-2.0 if power==4 else 0.0
    if np.max(abs(d[0]-analytical))>4e-12:
        raise RuntimeError("NONCONSERVING_TRUNCATED_BASS_SHEAR_MONOPOLE")
    if max(abs(d[1,m]) for m in range(0,lmax+1,2))>2e-12:
        raise RuntimeError("ODD_PARITY_LEAK_FROM_AXIAL_SYMMETRY")
    return d

def thomson_coeffs(lmax):
    # Axisymmetric, linear unpolarized Thomson: forward gain monopole and
    # p2 angular amplitude 1/10 (because gain=J0+1/2 J2 P2).
    # BASS hierarchy actual K2=-Gamma(9/10 F2+sqrt6/10 E2);
    # here E2=0 only. Collision quadrupole contributes no energy.
    c=np.ones(lmax+1)
    c[0]=0
    c[2]=.9
    return c

class ElectronHistory:
    """SHA-matched, input-frozen original q=0 HHe electron density per cm3."""
    def __init__(self,a:np.ndarray,ne:np.ndarray):
        a=np.asarray(a,dtype=float)
        ne=np.asarray(ne,dtype=float)
        if (a.ndim!=1 or len(a)<30 or len(a)!=len(ne)
             or not np.isfinite(a).all() or not np.isfinite(ne).all()
             or np.any(a<=0) or np.any(np.diff(a)<=0) or np.any(ne<0)):
            raise ValueError("unphysical source-saved electron opacity")
        self.x=np.log(a)
        self.n=np.maximum(ne,1e-50)
        self.interp=PchipInterpolator(self.x,self.n,extrapolate=False)
    def density(self,a):
        x=np.log(a)
        ne=self.interp(x)
        if not np.isfinite(ne).all() or np.any(ne<0):
            raise ValueError("electron-history extrapolation/negative")
        return ne

class SmoothElectronHistory:
    """C2 positive log-density surrogate fit to *actual* source ne(a).

    Unlike 18k-knot PCHIP, a C2 spline avoids artificial derivative jumps
    that contaminate high-order DOP853 local error inference. Its own source
    interpolation residual is measured against ALL saved real ne samples.
    """
    def __init__(self,a,ne,nodes=8193):
        a=np.asarray(a,dtype=float);ne=np.asarray(ne,dtype=float)
        if (a.ndim!=1 or len(a)<nodes or len(a)!=len(ne) or
                np.any(np.diff(a)<=0) or np.any(ne<=0) or not np.isfinite(ne).all()):
            raise ValueError("invalid actual HHe donor source")
        if nodes<257 or nodes>32769:raise ValueError("validated interpolation-node count")
        x=np.log(a)
        self.x=np.linspace(x[0],x[-1],nodes)
        self.ln=CubicSpline(self.x,np.interp(self.x,x,np.log(ne)),bc_type="natural",extrapolate=False)
        recovered=np.exp(self.ln(x))
        self.max_relative_source_error=float(np.max(abs(recovered/ne-1)))
        self.p999_relative_source_error=float(np.quantile(abs(recovered/ne-1),.999))
        self.nodes=nodes
        if not(np.isfinite(recovered).all() and np.all(recovered>0)):
            raise ValueError("negative source opacity after C2 reconstruction")
    def density(self,a):
        x=np.log(a)
        z=self.ln(x)
        if not np.isfinite(z).all():
            raise ValueError("donor outside compiled source physical epoch")
        return np.exp(z)

class HierarchyBackground:
    """Coupled Einstein, collisionless neutrino/CMB stress, Thomson photospheric damping.

    Optional donor ne(a) affects angular radiation collisions but NOT its own
    ionization chemistry. A separate source-coupled gas experiment follows.
    """
    def __init__(self,source:SourceLock,L:int,eta:float,kappa:float,sigma_ref:float,
                 orientation:int,ne_source:ElectronHistory|None=None,
                 nu_visc:float=0.0):
        if not(0<=eta<=1 and abs(kappa)<=.5 and 0<=nu_visc<=.1):
            raise ValueError("invalid source-physical radiation or viscosity")
        if not(0<=sigma_ref<math.sqrt(3)):
            raise ValueError("invalid tensor Sigma")
        if (sigma_ref==0 and orientation!=0) or (sigma_ref>0 and orientation not in (-1,1)):
            raise ValueError("Sigma orientation mismatch")
        self.cos=source
        self.L=L
        self.eta=eta;self.kappa=kappa
        self.sigma_ref=sigma_ref;self.orientation=orientation
        self.nu=nu_visc;self.ne_source=ne_source
        self.A=legendre_matrix(L,power=4)
        self.A_number=legendre_matrix(L,power=3)
        self.damp=thomson_coeffs(L)
        self.initial_q=orientation*sigma_ref/math.sqrt(3-sigma_ref**2) if sigma_ref>0 else 0.
        self.eval_count=0
    def initial_state(self):
        # 0=elapsed tHref; 1=b; 2=v=s/Href;3=thermal_u/rhoCrit0; rest exact PSTF J_l
        y=np.zeros(4+2*(self.L+1))
        y[2]=self.initial_q
        y[4]=1.0
        y[4+2]=self.kappa/5
        y[4+self.L+1]=1.0
        y[4+self.L+1+2]=self.kappa/5
        return y
    def diagnostic(self,x,y):
        if not np.isfinite(y).all() or not math.isfinite(x) or x<0 or x>self.cos.xmax+1e-10:
            raise ValueError("NONFINITE_NONPHYSICAL_HIERARCHY")
        a=self.cos.aref*math.exp(x)
        t,b,v,u=y[:4]
        j=y[4:4+self.L+1]
        jnumber=y[4+self.L+1:]
        if j[0]<=0 or u< -1e-9:
            raise ValueError("NEGATIVE_OR_UNPHYSICAL_HIERARCHY_ENERGY")
        s=v*self.cos.href
        dust=self.cos.omega_m/a**3
        rad=self.cos.omega_r/a**4*((1-self.eta)+self.eta*j[0])
        h2=self.cos.h0**2*(dust+self.cos.omega_lambda+rad+u)+s*s
        if h2<=0:raise ValueError("NEGATIVE_GRAVITY_HAMILTONIAN")
        h=math.sqrt(h2)
        piRnorm=(self.eta*self.cos.omega_r/a**4*j[2]/3)
        piMnorm=-2*self.nu*dust*s/h
        ne=0. if self.ne_source is None else float(self.ne_source.density(a))
        gamma=ne*GAMMA_PER_NE
        return {
         "a":a,"H":h,"s":s,"Sigma":math.sqrt(3)*abs(s)/h,
         "orientation":int(np.sign(s)),
         "J0":float(j[0]),"J2":float(j[2]),
         "N0":float(jnumber[0]),"N2":float(jnumber[2]),
         "piR":piRnorm*self.cos.rhocrit_j_m3,
         "piM":piMnorm*self.cos.rhocrit_j_m3,
         "radiation_density":rad*self.cos.rhocrit_j_m3,
         "dust_density":dust*self.cos.rhocrit_j_m3,
         "thermal_density":u*self.cos.rhocrit_j_m3,
         "GammaT_per_s":gamma,
         "ne_proper_cm3":ne,
         "piRnorm":piRnorm,"piMnorm":piMnorm,
         "h2_constraint":h2
        }
    def rhs(self,x,y):
        self.eval_count+=1
        q=self.diagnostic(x,y)
        a=q["a"];h=q["H"];s=q["s"];v=s/h
        j=y[4:4+self.L+1]
        jnumber=y[4+self.L+1:]
        hj=v*(self.A@j) - (q["GammaT_per_s"]/h)*self.damp*j
        hn=v*(self.A_number@jnumber) - (q["GammaT_per_s"]/h)*self.damp*jnumber
        srcpi=(3*self.cos.h0*self.cos.h0/(self.cos.href*h))*(q["piRnorm"]+q["piMnorm"])
        dus=-5*y[3]-6*v*q["piMnorm"]
        deriv=np.r_[self.cos.href/h,v,-3*y[2]+srcpi,dus,hj,hn]
        if not np.isfinite(deriv).all():
            raise RuntimeError("HIERARCHY_DERIVATIVE_NONFINITE")
        return deriv
    def integrate(self,n=301,rtol=5e-11,atol=1e-13):
        start=__import__("time").perf_counter()
        s=solve_ivp(self.rhs,(0.,self.cos.xmax),self.initial_state(),
                    method="DOP853",rtol=rtol,atol=atol,dense_output=True,
                    t_eval=np.linspace(0,self.cos.xmax,n))
        runtime=__import__("time").perf_counter()-start
        if not s.success:raise RuntimeError("HIERARCHY_ODE_DID_NOT_FINISH:"+s.message)
        if np.min(s.y[4,:])<=0 or np.min(s.y[3,:])<-1e-9 or np.max(abs(s.y[4+self.L+1,:]-1))>2e-10:
            raise RuntimeError("NEGATIVE_ENERGY_IN_COUPLED_HIERARCHY")
        return s,runtime
    def minimum_angular_brightness(self,y,order=128):
        mu,_=leggauss(order)
        pol=np.array([(2*l+1)*Legendre.basis(l)(mu) for l in range(self.L+1)])
        return float(np.min(y[4:4+self.L+1]@pol))
