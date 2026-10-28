"""Finite-grid contracted-entropy coarsening on S2; research prototype.

The fine grid has equal dOmega cells in (mu=cos(theta), phi). The thermal
massless, zero-log-fugacity energy section uses F(rho)=-kappa*rho**(3/4).
Coarsening conserves directional energy integrated over the sphere, NOT
particle number. A separate joint-MB check shows how retaining (n,rho)
repairs this limitation. This module is not a transport/PDE integrator.

All certificates refer to the supplied finite grid. No continuum error,
unobserved subcell structure, quadrature error or time error is included.
"""
from __future__ import annotations

import argparse
import heapq
import json
import math
import platform
import sys
import time
from dataclasses import dataclass
from pathlib import Path

import numpy as np


def power_bregman(rho, mean, kappa=1.0):
    """Stable F(rho)-F(mean)-F'(mean)*(rho-mean), rho>=0, mean>0."""
    x = np.asarray(rho, dtype=float)
    if not np.isfinite(kappa) or kappa <= 0 or not np.isfinite(mean) or mean <= 0:
        raise ValueError("positive finite mean and kappa are required")
    if np.any(~np.isfinite(x)) or np.any(x < 0):
        raise ValueError("rho must be finite and nonnegative")
    r = (x - mean) / mean
    small = np.abs(r) < 1e-3
    a = .75
    value = np.empty_like(r)
    # Expansion starts at quadratic order: no subtraction of O(r) terms.
    coefficients = []
    c = 1.0
    for degree in range(1, 9):
        c *= (a - degree + 1) / degree
        if degree >= 2:
            coefficients.append(-c)
    s = r[small]
    polynomial = np.zeros_like(s) + coefficients[-1]
    for c in reversed(coefficients[:-1]):
        polynomial = c + s * polynomial
    value[small] = s * s * polynomial
    other = ~small
    with np.errstate(divide="ignore", invalid="ignore"):
        value[other] = a * r[other] - np.expm1(a * np.log1p(r[other]))
    if np.any(value < -32 * np.finfo(float).eps):
        raise ArithmeticError("negative stable Bregman divergence")
    # Only signed floating-point zero is normalized, not negative divergences.
    value[value == 0] = 0.
    return kappa * mean**a * value


def prefix(array):
    return np.pad(np.cumsum(np.cumsum(array, axis=0), axis=1), ((1, 0), (1, 0)))


def rectangle_sum(table, box):
    a, b, c, d = box
    return float(table[b, d] - table[a, d] - table[b, c] + table[a, c])


@dataclass(frozen=True)
class Cell:
    box: tuple[int, int, int, int]
    area: float
    mean: float
    entropy: float


class EntropyGrid:
    def __init__(self, rho, kappa=1.0):
        rho = np.asarray(rho, dtype=float)
        if rho.ndim != 2 or 0 in rho.shape or np.any(~np.isfinite(rho)) or np.any(rho < 0):
            raise ValueError("finite nonnegative nonempty 2D rho required")
        if not np.isfinite(kappa) or kappa <= 0:
            raise ValueError("positive finite kappa required")
        self.rho = rho.copy()
        self.kappa = float(kappa)
        self.weight = 4 * math.pi / rho.size
        self.srho = prefix(rho)
        self.sF = prefix(-kappa * rho**.75)
        self.cache = {}
        self.stable_fallbacks = 0
        self.root = self.cell((0, rho.shape[0], 0, rho.shape[1]))

    def cell(self, box):
        if box in self.cache:
            return self.cache[box]
        a, b, c, d = box
        n = (b-a)*(d-c)
        values = self.rho[a:b, c:d]
        # Prefix moments give O(1) aggregate queries. Recompute when subtraction
        # of nearly equal prefixes compromises the physical mean.
        mass = rectangle_sum(self.srho, box)
        mean = mass/n
        actual_min, actual_max = float(values.min()), float(values.max())
        if n == 1:
            mean = float(values[0, 0])
        elif mean < actual_min or mean > actual_max or mean <= 0:
            mean = float(np.mean(values))
        area = n*self.weight
        H = rectangle_sum(self.sF, box)*self.weight
        eta = H + self.kappa*area*mean**.75
        # Near-constant fields lose the O(variation^2) remainder through
        # Jensen subtraction. Stable pointwise Bregman restores that remainder.
        if n == 1 or mean == 0:
            eta = 0.
        elif eta < 1e-8 * max(abs(H), self.kappa*area*mean**.75):
            mean = float(np.mean(values))
            eta = float(np.sum(power_bregman(values, mean, self.kappa)))*self.weight
            self.stable_fallbacks += 1
        if not np.isfinite(eta) or eta < 0:
            raise ArithmeticError("invalid entropy loss")
        result = Cell(box, area, mean, eta)
        self.cache[box] = result
        return result

    def children(self, cell):
        a, b, c, d = cell.box
        rows = [(a, b)] if b-a == 1 else [(a, (a+b)//2), ((a+b)//2, b)]
        cols = [(c, d)] if d-c == 1 else [(c, (c+d)//2), ((c+d)//2, d)]
        if len(rows)*len(cols) == 1:
            return []
        return [self.cell((i,j,k,l)) for i,j in rows for k,l in cols]

    def refine_to_budgets(self, budgets):
        """Greedy eta removed per added cell; exact realized loss is checked.

        This is a constructive adaptive heuristic, not a globally optimal tree.
        """
        leaves = {self.root.box: self.root}
        heap = []

        def enqueue(cell):
            kids = self.children(cell)
            if kids:
                gain = cell.entropy - math.fsum(x.entropy for x in kids)
                score = gain / (len(kids)-1)
                heapq.heappush(heap, (-score, cell.box, kids))

        enqueue(self.root)
        out = []
        for budget in sorted(budgets, reverse=True):
            if not np.isfinite(budget) or budget < 0:
                raise ValueError("nonnegative finite budget required")
            total = math.fsum(x.entropy for x in leaves.values())
            while total > budget and heap:
                _, box, kids = heapq.heappop(heap)
                old = leaves.pop(box)
                for x in kids:
                    leaves[x.box] = x
                    enqueue(x)
                total += math.fsum(x.entropy for x in kids) - old.entropy
                # Exact leaf sum avoids accumulated cancellation at termination.
                if total <= budget or len(leaves) % 100 == 0:
                    total = math.fsum(x.entropy for x in leaves.values())
            total = math.fsum(x.entropy for x in leaves.values())
            if total > budget:
                raise ArithmeticError("budget not met")
            out.append((budget, list(leaves.values())))
        return out

    def uniform_levels(self):
        levels = [[self.root]]
        while len(levels[-1]) < self.rho.size:
            levels.append([child for cell in levels[-1] for child in (self.children(cell) or [cell])])
        return levels

    def reconstruct(self, cells):
        result = np.empty_like(self.rho)
        for cell in cells:
            a,b,c,d = cell.box
            result[a:b,c:d] = cell.mean
        return result

    def evaluate(self, cells, kernels):
        reconstruction = self.reconstruct(cells)
        total = math.fsum(c.entropy for c in cells)
        direct = 0.
        for cell in cells:
            a,b,c,d = cell.box
            if cell.mean > 0:
                direct += float(np.sum(power_bregman(self.rho[a:b,c:d],cell.mean,self.kappa)))*self.weight
        mass = self.weight*float(np.sum(self.rho))
        mass_after = self.weight*float(np.sum(reconstruction))
        number = self.weight*float(np.sum(self.rho**.75))
        number_after = self.weight*float(np.sum(reconstruction**.75))
        observables = {}
        for name, kernel in kernels.items():
            bound = 0.
            for cell in cells:
                a,b,c,d = cell.box
                r = self.rho[a:b,c:d]
                k = kernel[a:b,c:d]
                if cell.entropy == 0:
                    continue
                lower_hessian = 3*self.kappa/16 * float(r.max())**(-1.25)
                variance_integral = self.weight*float(np.sum((k-np.mean(k))**2))
                bound += math.sqrt(2*cell.entropy/lower_hessian*variance_integral)
            error = self.weight*float(np.sum(kernel*(self.rho-reconstruction)))
            observables[name] = {"absolute_error":abs(error), "bound":bound,
                                 "signed_error":error, "certified":abs(error)<=bound+2e-13*max(1,mass)}
        return {"cells":len(cells),"entropy_loss":total,"direct_entropy_loss":direct,
                "energy_relative_drift":abs(mass_after-mass)/max(mass,np.finfo(float).tiny),
                "number_relative_drift":(number_after-number)/max(number,np.finfo(float).tiny),
                "number_drift_entropy_identity_residual":number_after-number-total/self.kappa,
                "minimum_reconstruction":float(reconstruction.min()),
                "observables":observables}


def sphere_grid(nmu=64,nphi=128):
    mu = -1+(np.arange(nmu)+.5)*2/nmu
    phi = (np.arange(nphi)+.5)*2*math.pi/nphi
    z,p = np.meshgrid(mu,phi,indexing="ij")
    s=np.sqrt(1-z*z)
    return np.stack((s*np.cos(p),s*np.sin(p),z),axis=-1)


def profiles_and_kernels(nmu=64,nphi=128):
    xyz=sphere_grid(nmu,nphi)
    axis=np.array([.7,.2,.6]);axis/=np.linalg.norm(axis)
    axis2=np.array([-.4,.8,-.4]);axis2/=np.linalg.norm(axis2)
    u=xyz@axis;v=xyz@axis2
    profiles={"smooth":1+.15*xyz[...,0]+.10*(3*xyz[...,2]**2-1)/2,
              "localized_beam":1+50*np.exp(80*(u-1)),
              "two_beams":1+20*np.exp(45*(u-1))+10*np.exp(70*(v-1)),
              "near_vacuum":1e-12+np.exp(60*(u-1)),
              "tiny_perturbation":1+1e-8*xyz[...,0]}
    kernels={"dipole_x":xyz[...,0],"quadrupole_z":(3*xyz[...,2]**2-1)/2,
             "localized_response":np.exp(25*(u-1))}
    return profiles,kernels


def joint_mb_check():
    """Joint (n,rho) projection on a finite thermal-chemical MB test array."""
    beta=np.array([.8,1.1,1.7,2.0]);eta=np.array([-.4,.3,-.2,.8]);w=np.array([.2,.3,.1,.4])
    n=2*np.exp(eta)/beta**3;rho=6*np.exp(eta)/beta**4
    nm=float(w@n);rm=float(w@rho)
    def F(n,r): return n*np.log(27*n**4/(2*r**3))-4*n
    grad=np.array([math.log(27*nm**4/(2*rm**3)), -3*nm/rm])
    divergence=F(n,rho)-F(nm,rm)-grad[0]*(n-nm)-grad[1]*(rho-rm)
    H=np.array([[4/nm,-3/rm],[-3/rm,3*nm/rm**2]])
    return {"n_values":n.tolist(),"rho_values":rho.tolist(),"weights":w.tolist(),
            "retained_n":nm,"retained_rho":rm,"jensen_loss":float(w@F(n,rho)-F(nm,rm)),
            "bregman_loss":float(w@divergence),"minimum_pointwise_divergence":float(divergence.min()),
            "hessian_eigenvalues":np.linalg.eigvalsh(H).tolist(),
            "number_conservation_error":float(w@np.full_like(n,nm)-w@n),
            "energy_conservation_error":float(w@np.full_like(rho,rm)-w@rho),
            "scope":"finite-grid joint massless MB equilibrium; no adaptive joint solver implemented"}


def run_experiment(output):
    start=time.perf_counter()
    profiles,kernels=profiles_and_kernels()
    records=[]
    timing=[]
    for name,rho in profiles.items():
        t0=time.perf_counter();grid=EntropyGrid(rho)
        fractions=[.1,.03,.01,.003]
        budgets=[grid.root.entropy*f for f in fractions]
        adaptive=grid.refine_to_budgets(budgets)
        tadapt=time.perf_counter()-t0
        t1=time.perf_counter();levels=grid.uniform_levels();tuniform=time.perf_counter()-t1
        for budget,cells in adaptive:
            uniform=next(level for level in levels if math.fsum(c.entropy for c in level)<=budget)
            a=grid.evaluate(cells,kernels);u=grid.evaluate(uniform,kernels)
            records.append({"profile":name,"budget":budget,"budget_fraction":budget/grid.root.entropy,
                            "root_entropy_loss":grid.root.entropy,"adaptive":a,"uniform":u,
                            "cell_count_reduction":u['cells']/a['cells'],
                            "baseline":"finest supplied discrete state; uniform chooses first nested level meeting same budget"})
        timing.append({"profile":name,"adaptive_diagnostic_seconds":tadapt,
                       "uniform_level_construction_seconds_reusing_cache":tuniform,
                       "cached_cells":len(grid.cache),"stable_fallback_count":grid.stable_fallbacks})
    all_certified=all(v['certified'] for r in records for method in ('adaptive','uniform') for v in r[method]['observables'].values())
    all_budgets=all(r[method]['direct_entropy_loss'] <= r['budget'] for r in records for method in ('adaptive','uniform'))
    result={"schema":"spherical-information.mesh-experiment.v1","nmu":64,"nphi":128,"fine_cells":8192,
            "kappa":1,"normalization":"rho dimensionless normalized energy; dOmega sums to 4pi; nscaled=rho^0.75",
            "algorithm":"greedy quadtree eta reduction per extra cell; no optimality claim",
            "scope":"finite-grid compression of massless one-field thermal energy section, not PDE speedup",
            "certificate_scope":"supplied discrete fine values; excludes continuum quadrature, time, subcell and spectral-shape errors",
            "records":records,"joint_mb":joint_mb_check(),"all_observable_bounds_pass":all_certified,
            "all_direct_entropy_budgets_pass":all_budgets,
            "diagnostic_timings":timing,"wall_seconds":time.perf_counter()-start,
            "environment":{"python":sys.version,"numpy":np.__version__,"platform":platform.platform()},
            "command":"python3 src/spherical_mesh.py --output results/mesh_results.json"}
    if not all_certified or not all_budgets: raise AssertionError("observable or entropy budget check failed")
    path=Path(output);path.parent.mkdir(parents=True,exist_ok=True)
    path.write_text(json.dumps(result,indent=2,allow_nan=False)+'\n')
    return result


if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--output',default='results/mesh_results.json')
    args=parser.parse_args();r=run_experiment(args.output)
    print(json.dumps({'records':len(r['records']),'all_observable_bounds_pass':r['all_observable_bounds_pass'],
                      'wall_seconds':r['wall_seconds'],'output':args.output}))
