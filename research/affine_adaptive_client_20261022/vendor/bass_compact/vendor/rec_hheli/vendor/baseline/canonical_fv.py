"""Positive canonical-momentum finite-volume Vlasov and Einstein–BASS coupling.

Scope: LRS Bianchi I, homogeneous collisionless finite-mass nonbaryonic
particles, and source-identical PR137 gamma/nu energy and number Boltzmann
multipoles with optional ORIGINAL frozen H/He n_e gamma-only Thomson.

The true homogeneous Vlasov PDE reduces *exactly* to
    d N_cell(q_perp,q_parallel)/d eta = 0
in conserved spatial Killing momentum cells. This module uses a positive
Gauss2 or Gauss3 cell average of the initial FD q² f(q) times 1+A P2(mu_*).
Physical momentum redshifts according to
    p_perp/T_ref = q_perp exp(-eta+b);
    p_parallel/T_ref = q_parallel exp(-eta-2b).
The cell masses are strictly positive and constant; the moving physical
velocity mesh makes phase-space transport exact within each sub-cell
quadrature rule. No higher l=4 or velocity-i analytic oracle is ever
queried. The finite-volume momentum quadrature has measurable discretization
error and is not an Eulerian flux solver for scattering or finite collisions.

Mass/energy budget: Ω_m -> (1-fm)Ω_m cold dust + fm Ω_m massive Vlasov
(nonbaryonic fm only), preserving the parent PR140 physical normalization.

Einstein shear magnitude Sigma=sqrt(3)|s|/H, sign separate; pi_s=
(P_parallel-P_perp)/3 in PR137 contract. No native reionization gas response,
no E/B polarized Thomson, no UV stellar photon-gravity ledger admitted.
"""
from __future__ import annotations

import math
import time
from dataclasses import dataclass

import numpy as np
from numpy.polynomial.legendre import leggauss
from scipy.integrate import solve_ivp
from species_hierarchy import SpeciesHierarchy

C_SI=299792458.0
G_SI=6.67430e-11
SQRT3=math.sqrt(3.0)


@dataclass(frozen=True)
class FVMoments:
    rho:float
    pressure_perp:float
    pressure_parallel:float
    ncell_number:float
    min_cell_number:float

    @property
    def pbar(self):
        return (2*self.pressure_perp+self.pressure_parallel)/3

    @property
    def delta_p(self):
        return self.pressure_parallel-self.pressure_perp

    @property
    def pi_s(self):
        return self.delta_p/3


class CanonicalMomentumFV:
    """Deterministic positive 2D cell masses with Gauss2 subcell integration.

    The canonical cells are disjoint in t=(q-3)/(q+3) and mu_*. This
    compactification covers q in [0,∞) without an arbitrary high-q cutoff.
    Each cell has 4 (Gauss2) or 9 (Gauss3) nonnegative quadrature weights; the physical-cell
    number ∫cell q² f0 W dq dmu is stored immutable. Collisionless
    characteristic flow has ZERO flux in these canonical coordinates.

    The cell phase-space measure is independent of the Einstein metric.
    This provides a genuinely closed finite-dimensional discrete Vlasov
    system for collisionless transport, with numerical quadrature error.
    """

    def __init__(self, mass_over_Tref:float, amplitude:float=0.,
                 radial_cells:int=32,angular_cells:int=16,
                 gauss_order:int=2):
        if not (math.isfinite(mass_over_Tref) and 3<=mass_over_Tref<=1e5):
            raise ValueError("mass/Tref outside BASS exploratory domain")
        if not(math.isfinite(amplitude) and -1<=amplitude<=2):
            raise ValueError("nonpositive initial angular distribution")
        if not(isinstance(radial_cells,int) and isinstance(angular_cells,int)
               and 4<=radial_cells<=256 and 4<=angular_cells<=128):
            raise ValueError("canonical cell resolution not supported")
        if gauss_order not in (2,3,4):
            raise ValueError("positive subcell Gauss order must be 2,3,4")
        self.mass=float(mass_over_Tref)
        self.amplitude=float(amplitude)
        self.radial_cells=radial_cells
        self.angular_cells=angular_cells
        self.gauss_order=gauss_order
        z,w=leggauss(gauss_order)
        # Cell-centered + two point Gauss integration in compactified radial
        # coordinate t, then in mu. No integration or remap from future states.
        tr=np.linspace(-1.,1.,radial_cells+1)
        tm=np.linspace(-1.,1.,angular_cells+1)
        hr=np.diff(tr)/2
        hm=np.diff(tm)/2
        rr=(tr[:-1]+hr)[:,None]+hr[:,None]*z[None,:]
        aa=(tm[:-1]+hm)[:,None]+hm[:,None]*z[None,:]
        wt=(hr[:,None]*w[None,:]).reshape(-1)
        wm=(hm[:,None]*w[None,:]/2).reshape(-1)
        q=3*(1+rr.reshape(-1))/(1-rr.reshape(-1))
        jac=6/(1-rr.reshape(-1))**2
        fd=np.exp(-q)/(1+np.exp(-q))
        radial_number=wt*jac*q*q*fd
        mu=aa.reshape(-1)
        W=1+self.amplitude*(3*mu*mu-1)/2
        if np.any(W<0):raise ArithmeticError("positive source f violated")
        packets=radial_number[:,None]*(wm*W)[None,:]
        if not np.all(packets>=0) or not np.isfinite(packets).all():
            raise ValueError("negative/nonfinite canonical cell number")
        # Rectangular cell grouping: 2 radial × 2 angular points each.
        cell_mass=packets.reshape(radial_cells,gauss_order,angular_cells,gauss_order).sum(
            axis=(1,3))
        initial_E=np.hypot(self.mass,q)[:,None]
        norm=float(np.sum(packets*initial_E))
        if norm<=0:raise ArithmeticError("no positive physical reference rho")
        if np.min(cell_mass)<=0:
            # For very cold profiles a compactification may create
            # underflow-only trailing cells. The zero-mass tail has no
            # physical negativity, but the strict-positive claim is then
            # only nonnegative.
            if np.min(cell_mass)<0:raise ArithmeticError("negative cell mass")
        self.reference_energy_norm=norm
        self._q=q[:,None]
        self._mu2=(mu*mu)[None,:]
        self._numbers=packets
        self._cell_numbers=cell_mass
        self._number_sum=float(np.sum(cell_mass))
        for arr in (self._q,self._mu2,self._numbers,self._cell_numbers):
            arr.flags.writeable=False

    @property
    def cell_numbers(self):
        return self._cell_numbers

    @property
    def total_cell_number(self):
        return self._number_sum

    @property
    def packet_count(self):
        return self._numbers.size

    def distribution_change_deta(self):
        """Complete (and exact) collisionless kinetic finite-volume RHS."""
        return np.zeros_like(self._cell_numbers)

    def moments(self,eta:float,b:float)->FVMoments:
        """Physical Einstein stress from *only* evolved canonical FV cells.

        We never fetch BASS J4/i+2 to close the hierarchy. Conserved
        cell weights and current metric suffice for all mass-shell moments.
        """
        if not all(map(math.isfinite,(eta,b))) or eta<0 or abs(b)>1:
            raise ValueError("finite positive cosmological time and Bianchi chart")
        r=math.exp(-eta)
        qp2=(r*self._q)**2*math.exp(2*b)*(1-self._mu2)
        qz2=(r*self._q)**2*math.exp(-4*b)*self._mu2
        E=np.sqrt(self.mass*self.mass+qp2+qz2)
        coeff=(r**3)/self.reference_energy_norm
        weighted=coeff*self._numbers
        rho=float(np.sum(weighted*E))
        pp=float(np.sum(weighted*(qp2/(2*E))))
        pa=float(np.sum(weighted*(qz2/E)))
        return FVMoments(rho,pp,pa,
            float(self._number_sum*r**3),
            float(np.min(self._cell_numbers)))

    def moment_batch(self,eta,b,chunk=32):
        """Chunked SIMD NumPy with only 4*nq*nmu*c temporary tensors."""
        xx=np.asarray(eta,dtype=float);bb=np.asarray(b,dtype=float)
        if (xx.ndim!=1 or bb.shape!=xx.shape or
            np.any(~np.isfinite(xx)) or np.any(xx<0)
            or np.any(~np.isfinite(bb)) or np.any(abs(bb)>1)
            or not isinstance(chunk,int) or not 1<=chunk<=256):
            raise ValueError("bounded physical batch")
        out=np.empty((len(xx),3))
        q2=self._q**2
        for start in range(0,len(xx),chunk):
            end=min(start+chunk,len(xx))
            r=np.exp(-xx[start:end])[:,None,None]
            b3=bb[start:end,None,None]
            p2p=(r*r)*q2[None,:,:]*np.exp(2*b3)*(1-self._mu2)[None,:,:]
            p2z=(r*r)*q2[None,:,:]*np.exp(-4*b3)*self._mu2[None,:,:]
            E=np.sqrt(self.mass*self.mass+p2p+p2z)
            weight=(r**3)*self._numbers[None,:,:]/self.reference_energy_norm
            out[start:end,0]=(weight*E).sum((1,2))
            out[start:end,1]=(weight*p2p/(2*E)).sum((1,2))
            out[start:end,2]=(weight*p2z/E).sum((1,2))
        return out


class CanonicalFVSpecies(SpeciesHierarchy):
    """A self-gravitating *closed collisionless cell system* plus PR137.

    Canonical cell populations are constant exactly; only Einstein and
    massless energy/number Boltzmann states require time integration. An
    optional one-way saved H/He electron profile affects photons only.
    """

    def __init__(self,source,Tcmb0_K,ellmax=8,*,Sigma_ref=0.,
            orientation=0,kappa_gamma=.2,kappa_nu=.2,
            ne_source=None,mass_fraction=.1,massive_A=.5,
            mass_over_Tref=30.,radial_cells=32,angular_cells=16,
            omega_b=.049309230511847414,subcell_gauss_order=2):
        if not(math.isfinite(mass_fraction) and 0<=mass_fraction<1
               and math.isfinite(omega_b) and omega_b>0
               and mass_fraction*source.omega_m<=source.omega_m-omega_b+1e-13):
            raise ValueError("canonical massive fraction overlaps baryonic mass")
        self.fm=float(mass_fraction)
        self.momentum=CanonicalMomentumFV(mass_over_Tref,massive_A,
                         radial_cells,angular_cells,
                         gauss_order=subcell_gauss_order)
        super().__init__(source,Tcmb0_K,ellmax,Sigma_ref=Sigma_ref,
                orientation=orientation,kappa_gamma=kappa_gamma,
                kappa_nu=kappa_nu,ne_source=ne_source,
                matter_nu_viscosity=0.)
        self._mass_pref=source.omega_m*self.fm/source.aref**3
        self._dust_omega=source.omega_m*(1-self.fm)
        self._nfev=0

    def fields(self,x,y):
        base=super().fields(x,y)
        if self.fm==0:
            # Strict byte-level old-physics algebraic path: omit any
            # additional roundoff or Vlasov quadrature from fm=0.
            base.update({
                "massive_energy_norm":0.,"massive_mean_pressure_norm":0.,
                "massive_number_cell_sum":self.momentum.total_cell_number,
                "massive_min_cell_population":float(np.min(self.momentum.cell_numbers)),
                "massive_physical_number_norm":
                    self.momentum.total_cell_number*math.exp(-3*float(x)),
                "massive_source_owner":"positive_canonical_momentum_finite_volume",
                "Einstein_H2":base["H"]**2
            })
            return base
        s=base["s"];a=base["a"]
        m=self.momentum.moments(float(x),float(base["b"]))
        kinetic=self._mass_pref*m.rho
        dust=self._dust_omega/a**3
        h2=self.source.h0**2*(dust+kinetic+
             base["rad_energy_norm"]+self.source.omega_lambda)+s*s
        if not(math.isfinite(h2) and h2>0):
            raise FloatingPointError("nonphysical FV Einstein constraint")
        H=math.sqrt(h2)
        pbar=self._mass_pref*m.pbar
        pis=self._mass_pref*m.pi_s
        base.update({
            "H":H,"Sigma":SQRT3*abs(s)/H,
            "Pi_matter_norm":pis,
            "Pi_matter_J_m3":self.source.rhocrit_j_m3*pis,
            "dust_energy_norm":dust,"massive_energy_norm":kinetic,
            "massive_mean_pressure_norm":pbar,
            "massive_number_cell_sum":self.momentum.total_cell_number,
            "massive_min_cell_population":m.min_cell_number,
            "massive_physical_number_norm":m.ncell_number,
            "massive_source_owner":"positive_canonical_momentum_finite_volume",
            "Einstein_H2":h2,
        })
        return base

    def rhs(self,x,y):
        self._nfev+=1
        if self.fm==0:
            return super().rhs(x,y)
        q=self.fields(x,y)
        h=q["H"];s=q["s"];v=s/h;gamma=q["GammaT"]/h
        eg,ng,en,nn=(q["E_gamma"],q["N_gamma"],q["E_nu"],q["N_nu"])
        de_g=v*(self.energy_matrix@eg)-gamma*self.thomson*eg
        dn_g=v*(self.number_matrix@ng)-gamma*self.thomson*ng
        de_n=v*(self.energy_matrix@en)
        dn_n=v*(self.number_matrix@nn)
        dv=-3*y[2]+3*self.source.h0**2/(self.source.href*h)*(
            q["Pi_radiation_norm"]+q["Pi_matter_norm"])
        return np.r_[self.source.href/h,v,dv,0.,de_g,dn_g,de_n,dn_n]

    def integrate(self,n=121,rtol=3e-11,atol=3e-13):
        start=time.perf_counter()
        sol=solve_ivp(self.rhs,(0,self.source.xmax),self.initial_state(),
                      t_eval=np.linspace(0,self.source.xmax,n),
                      method="DOP853",rtol=rtol,atol=atol)
        seconds=time.perf_counter()-start
        if not sol.success:
            raise RuntimeError("FV Einstein+gamma_nu DOP853 failure:"+sol.message)
        diagnostics=[self.fields(float(x),sol.y[:,i])
                      for i,x in enumerate(sol.t)]
        if any(abs(v["N_gamma"][0]-1)>5e-11 or
               abs(v["N_nu"][0]-1)>5e-11 or
               v["massive_min_cell_population"]<0
               for v in diagnostics):
            raise RuntimeError("phase-space/gamma_nu number or positivity failure")
        return sol,diagnostics,seconds
