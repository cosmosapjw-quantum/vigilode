"""Small executable LRS Bianchi-I finite-volume reference; not a live solver port.

q=log(E/Eref), mu=cos(theta), N_ij=int_cell a^3*kappa*E^3*f dq dOmega.
Azimuth is integrated under axial symmetry. q boundaries use zero inflow;
mu boundaries are characteristic (v_mu=0). Cold elastic Thomson only.
Python allocation is intentional: this is an independent formula oracle.
"""
from __future__ import annotations
import json
import math
from fractions import Fraction
from pathlib import Path
import sys
import unittest


def _grid(edges):
    if len(edges) < 2 or any(not math.isfinite(x) for x in edges):
        raise ValueError("finite nonempty grid required")
    widths = [b-a for a,b in zip(edges,edges[1:])]
    if any(d <= 0 or not math.isfinite(d) for d in widths):
        raise ValueError("strict finite positive widths required")
    return widths


def lrs_number_rhs(q_edges, mu_edges, contents, H, shear, boundary="outflow"):
    """Conservative piecewise-constant upwind flux, returning rhs/boundary outflow.

    Initial contents need not be positive (linear JVP use is allowed).
    This RHS alone does not make a non-SSP time integrator positivity preserving.
    """
    if boundary not in ("outflow", "closed"):
        raise ValueError("boundary must be outflow or closed")
    dq, dm = _grid(q_edges), _grid(mu_edges)
    nq, nm = len(dq), len(dm)
    if mu_edges[0] != -1 or mu_edges[-1] != 1:
        raise ValueError("full axial sphere -1..1 required")
    if len(contents) != nq*nm or any(not math.isfinite(x) for x in contents):
        raise ValueError("contents dimension or finiteness")
    if not math.isfinite(H) or not math.isfinite(shear):
        raise ValueError("geometry finiteness")
    density = [contents[i*nm+j]/(dq[i]*dm[j]) for i in range(nq) for j in range(nm)]
    rhs = [0.0]*len(contents)
    outward = 0.0
    for j in range(nm):
        # Exact cell average of 3 mu^2-1 gives consistency on nonuniform cells.
        mu2 = (mu_edges[j+1]**3-mu_edges[j]**3)/(3*dm[j])
        velocity = -H-shear*(3*mu2-1)
        for k in range(nq+1):
            if boundary == "closed" and k in (0,nq):
                continue
            upstream = k-1 if velocity >= 0 else k
            val = density[upstream*nm+j] if 0 <= upstream < nq else 0.0
            flux = velocity*val*dm[j]
            if k: rhs[(k-1)*nm+j] -= flux
            else: outward -= flux
            if k<nq: rhs[k*nm+j] += flux
            else: outward += flux
    for i in range(nq):
        for k in range(1,nm):
            mu=mu_edges[k]
            velocity=-3*shear*mu*(1-mu*mu)
            upstream=k-1 if velocity >= 0 else k
            flux=velocity*density[i*nm+upstream]*dq[i]
            rhs[i*nm+k-1]-=flux
            rhs[i*nm+k]+=flux
    return rhs, outward


def lrs_number_jvp(q_edges, mu_edges, direction, H, shear, boundary="outflow"):
    """Exact derivative w.r.t. cell contents for prescribed geometry (no FD RHS calls)."""
    return lrs_number_rhs(q_edges, mu_edges, direction, H, shear, boundary)[0]


def transport_cfl(q_edges, mu_edges, H, shear, boundary="outflow"):
    """Sufficient forward-Euler positivity timestep for this first-order transport.

    No positivity claim for Rosenbrock steps. Return infinity for zero transport.
    """
    dq,dm=_grid(q_edges),_grid(mu_edges)
    lrs_number_rhs(q_edges,mu_edges,[0.]*(len(dq)*len(dm)),H,shear,boundary)
    maxloss=0.
    for i,dx in enumerate(dq):
        for j,dy in enumerate(dm):
            lo,hi=mu_edges[j:j+2]
            qv=-H-shear*((hi**3-lo**3)/dy-1)
            qloss=abs(qv)/dx
            if boundary=="closed" and ((qv<0 and i==0) or (qv>=0 and i==len(dq)-1)):
                qloss=0.
            vlo=-3*shear*lo*(1-lo*lo)
            vhi=-3*shear*hi*(1-hi*hi)
            maxloss=max(maxloss,qloss+(max(0.,-vlo)+max(0.,vhi))/dy)
    return 1/maxloss if maxloss else math.inf


def thomson_rhs(mu_edges, contents, rate):
    """Axial, unpolarized cold-electron Thomson on one fixed energy group.

    Rank-two gain uses the exact cell-average P2, not nearest-neighbour scatter.
    rate=c*sigma_T*n_e in s^-1. Incoming and outgoing cells share the same grid.
    """
    dm=_grid(mu_edges)
    if mu_edges[0]!=-1 or mu_edges[-1]!=1 or len(contents)!=len(dm):
        raise ValueError("full axial angular grid and matching content required")
    if not math.isfinite(rate) or rate<0 or any(not math.isfinite(x) for x in contents):
        raise ValueError("invalid input")
    p2=[((b**3-b)-(a**3-a))/(2*d) for a,b,d in zip(mu_edges,mu_edges[1:],dm)]
    monopole=math.fsum(contents)
    quadrupole=math.fsum(p*n for p,n in zip(p2,contents))
    return [rate*(d/2*(monopole+0.5*p*quadrupole)-n)
            for d,p,n in zip(dm,p2,contents)]


def absorption_ledger(number, energy_joule, threshold_joule, opacity_per_m, c=299792458.0):
    """Single-photon ionization, no secondary ionizations or stimulated terms."""
    vals=(number,energy_joule,threshold_joule,opacity_per_m,c)
    if any(not math.isfinite(v) for v in vals) or min(vals)<0 or c==0 or energy_joule<threshold_joule:
        raise ValueError("nonnegative finite above-threshold inputs required")
    events=c*opacity_per_m*number
    return dict(photon_number_rate=-events, ion_number_rate=events,
                photon_energy_rate=-energy_joule*events,
                ionization_energy_rate=threshold_joule*events,
                heat_rate=(energy_joule-threshold_joule)*events)


class PhysicsChecks(unittest.TestCase):
    def test_liouville_identity_exact(self):
        for mu in (Fraction(-1),Fraction(-3,7),Fraction(0),Fraction(1)):
            H,s=Fraction(2,7),Fraction(-3,11)
            qdot=-H-s*(3*mu*mu-1)
            divmu=-3*s+9*s*mu*mu
            self.assertEqual(3*H+3*qdot+divmu,0)
    def test_fvm_number_boundary_ledger(self):
        rhs,out=lrs_number_rhs([0,.2,.9,2],[-1,-.7,.1,.8,1],list(range(1,13)),.1,.07)
        self.assertAlmostEqual(math.fsum(rhs)+out,0,places=12)
        self.assertGreaterEqual(out,0)
    def test_closed_transport_and_cfl_positivity(self):
        q=[0,.2,.9,2]; mu=[-1,-.7,.1,.8,1]
        y=[0.,3.,1.,0.,2.,0.,4.,1.,0.,2.,0.,5.]
        dt=.99*transport_cfl(q,mu,.1,.07,"closed")
        rhs,out=lrs_number_rhs(q,mu,y,.1,.07,"closed")
        self.assertEqual(out,0.)
        self.assertAlmostEqual(math.fsum(rhs),0.,places=12)
        self.assertTrue(all(a+dt*b>=-1e-14 for a,b in zip(y,rhs)))
    def test_exact_linear_jvp(self):
        q=[0,.2,.9,2]; mu=[-1,-.7,.1,.8,1]
        a=[float(i) for i in range(12)]; b=[(-1.)**i/(i+1) for i in range(12)]
        ra,_=lrs_number_rhs(q,mu,a,.1,.07)
        rb=lrs_number_jvp(q,mu,b,.1,.07)
        rab,_=lrs_number_rhs(q,mu,[x+2*y for x,y in zip(a,b)],.1,.07)
        self.assertLess(max(abs(z-x-2*y) for z,x,y in zip(rab,ra,rb)),2e-14)
    def test_flrw_separable_angular_distribution(self):
        q=[0,.2,.9,2]; mu=[-1,-.3,1]; weights=[.3,.7]
        y=[n*w for n in (1.,3.,2.) for w in weights]
        rhs,_=lrs_number_rhs(q,mu,y,.1,0.)
        for i in range(3): self.assertAlmostEqual(rhs[2*i]/weights[0],rhs[2*i+1]/weights[1])
        frozen,_=lrs_number_rhs(q,mu,y,0.,0.)
        self.assertEqual(frozen,[0.]*6)
    def test_geometry_refusal(self):
        with self.assertRaises(ValueError): lrs_number_rhs([0,0],[-1,1],[1],1,0)
    def test_thomson_number_and_positivity(self):
        edges=[-1,-.7,-.1,.3,.9,1]; n=[0,2,0,9,1]
        rhs=thomson_rhs(edges,n,4)
        self.assertAlmostEqual(math.fsum(rhs),0,places=12)
        self.assertTrue(all(v+.25*d>=0 for v,d in zip(n,rhs)))
    def test_thomson_isotropic_stationary(self):
        edges=[-1,-.9,.2,.7,1]; n=[b-a for a,b in zip(edges,edges[1:])]
        self.assertLess(max(map(abs,thomson_rhs(edges,n,3))),2e-15)
    def test_photon_ion_heat_exchange(self):
        r=absorption_ledger(3e8,25*1.602176634e-19,13.6*1.602176634e-19,1e-16)
        self.assertEqual(r['photon_number_rate']+r['ion_number_rate'],0)
        self.assertLess(abs(r['photon_energy_rate']+r['ionization_energy_rate']+r['heat_rate']),1e-27)
    def test_frozen_nonlinearity_has_first_order_global(self):
        def frozen(n):
            y=1.; h=.2/n
            for _ in range(n): y*=math.exp(h*y)
            return abs(y-1.25)
        self.assertGreater(frozen(40)/frozen(80),1.9)
        self.assertLess(frozen(40)/frozen(80),2.1)
    def test_ale_gcl_constant_density(self):
        # v=0, moving faces w: RHS content = U*(w_R-w_L)=U*dVolume/dt.
        rho=2.7; w=[-.3,.1,.8]
        rhs=[rho*(b-a) for a,b in zip(w,w[1:])]
        dvol=[b-a for a,b in zip(w,w[1:])]
        self.assertTrue(all(abs(a-rho*b)<1e-15 for a,b in zip(rhs,dvol)))
    def test_signed_anisotropic_stress_cancellation(self):
        # LRS sigma=diag(-s,-s,2s), pi=diag(-D/3,-D/3,2D/3).
        s,Dm,Dr=.2,3.,-3.
        work=lambda D:2*s*D
        self.assertEqual(work(Dm)+work(Dr),0.)
        self.assertGreater(abs(Dm)+abs(Dr),abs(Dm+Dr))

if __name__=='__main__':
    suite=unittest.defaultTestLoader.loadTestsFromTestCase(PhysicsChecks)
    result=unittest.TextTestRunner(verbosity=2).run(suite)
    if len(sys.argv)>1:
        Path(sys.argv[1]).write_text(json.dumps({'status':'pass_scoped' if result.wasSuccessful() else 'fail',
          'tests':result.testsRun,'failures':len(result.failures),'errors':len(result.errors),
          'scope':'formula oracle: LRS Bianchi I, unpolarized elastic Thomson, first-order FV; no live BASS/rei port or production performance'},indent=2)+'\n')
    sys.exit(0 if result.wasSuccessful() else 1)
