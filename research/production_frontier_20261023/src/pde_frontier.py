#!/usr/bin/env python3
"""Weighted conservative graph diffusion: reproducible research prototype.

Not the production VigilODE API. numpy is used for vectorized edge fluxes;
no dense operator is formed by any candidate. The tiny independent sphere
reference diagonalizes only the latitude restriction of a known Fourier mode.
"""
from __future__ import annotations
import argparse
from dataclasses import dataclass
import hashlib
import json
import math
from pathlib import Path
import platform
import sys
import unittest
import numpy as np


@dataclass
class Graph:
    weights: np.ndarray
    left: np.ndarray
    right: np.ndarray
    conductance: np.ndarray
    name: str = "graph"

    def __post_init__(self):
        self.weights = np.asarray(self.weights, dtype=float)
        self.left = np.asarray(self.left, dtype=np.int64)
        self.right = np.asarray(self.right, dtype=np.int64)
        self.conductance = np.asarray(self.conductance, dtype=float)
        n = len(self.weights)
        if (n == 0 or not np.all(np.isfinite(self.weights)) or
                np.any(self.weights <= 0) or not
                (len(self.left) == len(self.right) == len(self.conductance)) or
                not np.all(np.isfinite(self.conductance)) or
                np.any(self.conductance < 0) or
                np.any(self.left < 0) or np.any(self.right < 0) or
                np.any(self.left >= n) or np.any(self.right >= n) or
                np.any(self.left == self.right)):
            raise ValueError("positive finite weights and valid nonnegative edges required")
        self.exit = (np.bincount(self.left, self.conductance, minlength=n)
                     + np.bincount(self.right, self.conductance, minlength=n)) / self.weights
        if not np.all(np.isfinite(self.exit)):
            raise ValueError("exit rates overflow")
        self.q = float(np.max(self.exit))
        self.spectral_bound = 2.0 * self.q

    def apply(self, y):
        """Q y, Q=-W^-1 D^T C D for an oriented incidence matrix D."""
        y = np.asarray(y, dtype=float)
        flux = self.conductance * (y[self.right] - y[self.left])
        return (np.bincount(self.left, flux, minlength=len(y))
                - np.bincount(self.right, flux, minlength=len(y))) / self.weights

    def markov(self, y):
        """(I+Q/q)y written as a convex sum, avoiding subtractive diagonals."""
        if self.q == 0:
            return y.copy()
        neighbor = (np.bincount(self.left, self.conductance * y[self.right], minlength=len(y))
                    + np.bincount(self.right, self.conductance * y[self.left], minlength=len(y)))
        return (1.0 - self.exit / self.q) * y + neighbor / (self.q * self.weights)

    def norm2(self, y):
        return float(np.sqrt(np.dot(self.weights, y * y)))

    def norm1(self, y):
        return float(np.dot(self.weights, np.abs(y)))


def check_time(t):
    if not math.isfinite(t) or t < 0:
        raise ValueError("finite nonnegative time required")


def euler(g, y, t, steps):
    check_time(t)
    if steps < 1 or not isinstance(steps, int):
        raise ValueError("positive integer steps required")
    h = t / steps
    if h * g.q > 1 + 1e-14:
        raise ValueError("Euler positivity/stability bound violated")
    out = np.array(y, dtype=float, copy=True)
    theta = h * g.q
    for _ in range(steps):
        out = (1.0 - theta) * out + theta * g.markov(out)
    return out, {"operator_actions": steps, "macrosteps": steps, "stages": 1}


def rkl_stage_count(z):
    # z = h*Lambda, Lambda bounds the spectrum of -Q.
    s = max(2, int(math.ceil((-1 + math.sqrt(9 + 8*z)) / 2)))
    while s*s + s - 2 < 2*z * (1 - 1e-15):
        s += 1
    return s


def rkl2_step(g, y, h, stages=None):
    check_time(h)
    s = rkl_stage_count(h * g.spectral_bound) if stages is None else stages
    if s < 2 or h * g.spectral_bound > (s*s+s-2)/2 * (1+1e-14):
        raise ValueError("Legendre stability enclosure violated")
    k = s*s+s-2
    b = k / (2*s*(s+1))
    omega = 4.0 / k
    v0 = y.copy()
    v1 = y + omega*h*g.apply(y)
    for j in range(2, s+1):
        v2 = (2*j-1)/j * (v1 + omega*h*g.apply(v1)) - (j-1)/j*v0
        v0, v1 = v1, v2
    return (1-b)*y + b*v1, s


def rkl2(g, y, t, steps):
    check_time(t)
    if steps < 1 or not isinstance(steps, int):
        raise ValueError("positive integer steps required")
    out = np.array(y, dtype=float, copy=True)
    actions = 0
    s = rkl_stage_count(t/steps*g.spectral_bound)
    for _ in range(steps):
        out, used = rkl2_step(g, out, t/steps, s)
        actions += used
    return out, {"operator_actions": actions, "macrosteps": steps, "stages": s}


def uniformization(g, y, t, relative_bound=1e-10):
    """Positive normalized Poisson polynomial, with analytic truncation bound.

    The bound excludes floating-point roundoff. It multiplies the input norm,
    in any Markov-contractive norm (weighted L1/L2 or sup norm here).
    """
    check_time(t)
    if not math.isfinite(relative_bound) or not 0 < relative_bound < 1:
        raise ValueError("relative_bound must lie in (0,1)")
    out = np.array(y, dtype=float, copy=True)
    if g.q == 0 or t == 0:
        return out, {"operator_actions": 0, "macrosteps": 0, "relative_truncation_bound": 0.0}
    splits = max(1, math.ceil(t*g.q/32))
    mu = t*g.q/splits
    target_tail = relative_bound/(2*splits)
    actions = 0
    bound_sum = 0.0
    for _ in range(splits):
        state = out.copy()
        p = math.exp(-mu)
        accum = p*state
        total = p
        k = 0
        while True:
            next_p = p*mu/(k+1)
            ratio = mu/(k+2)
            tail = next_p/(1-ratio) if ratio < 1 else math.inf
            if tail <= target_tail:
                break
            k += 1
            state = g.markov(state)
            p = next_p
            accum += p*state
            total += p
            actions += 1
            if k > 100000:
                raise RuntimeError("Poisson truncation failed to converge")
        out = accum/total
        bound_sum += 2*tail
    return out, {"operator_actions": actions, "macrosteps": splits,
                 "relative_truncation_bound": bound_sum,
                 "roundoff_included_in_bound": False}


def periodic_grid(n, dimension, diffusion=0.1):
    if n < 3 or dimension not in (2, 3) or not diffusion > 0:
        raise ValueError("periodic n>=3, dimension2/3 and positive diffusivity required")
    shape = (n,)*dimension
    size = n**dimension
    left, right, conductance = [], [], []
    volume = n**(-dimension)
    for idx in np.ndindex(shape):
        i = np.ravel_multi_index(idx, shape)
        for axis in range(dimension):
            nxt = list(idx)
            nxt[axis] = (nxt[axis]+1) % n
            left.append(i)
            right.append(np.ravel_multi_index(tuple(nxt), shape))
            conductance.append(diffusion*volume*n*n)
    g = Graph(np.full(size, volume), left, right, conductance, f"periodic{dimension}d_n{n}")
    mode = np.ones(shape)
    for axis in range(dimension):
        view = [1]*dimension
        view[axis] = n
        mode *= np.cos(2*math.pi*(np.arange(n)+0.5)/n).reshape(view)
    mode = mode.reshape(-1)
    eigenvalue = -4*diffusion*n*n*dimension*math.sin(math.pi/n)**2
    return g, mode, eigenvalue, -4*math.pi**2*diffusion*dimension


def sphere_grid(ntheta, nphi):
    """Unit sphere cell averages, latitude faces in theta and periodic phi.

    mu=-cos(theta); areas are exact (mu_hi-mu_lo)*dphi. Pole faces
    have zero integrated mu flux; there is no duplicate polar node.
    """
    if ntheta < 2 or nphi < 3:
        raise ValueError("ntheta>=2,nphi>=3 required")
    faces = -np.cos(np.linspace(0, math.pi, ntheta+1))
    widths = np.diff(faces)
    centers = (faces[1:]+faces[:-1])/2
    dp = 2*math.pi/nphi
    left, right, c = [], [], []
    for i in range(ntheta):
        for j in range(nphi):
            pos = i*nphi+j
            if i+1 < ntheta:
                left.append(pos); right.append((i+1)*nphi+j)
                c.append((1-faces[i+1]**2)*dp/(centers[i+1]-centers[i]))
            left.append(pos); right.append(i*nphi+(j+1)%nphi)
            c.append(widths[i]/((1-centers[i]**2)*dp))
    weights = np.repeat(widths*dp, nphi)
    g = Graph(weights, left, right, c, f"sphere_{ntheta}x{nphi}")
    return g, faces, centers, widths


def sphere_x_reference(ntheta, nphi, t):
    """Independent exact semidiscrete reference for the cos(phi) mode.

    Dense eigensystem has size ntheta, not ntheta*nphi. Used only as oracle.
    """
    g, faces, centers, widths = sphere_grid(ntheta, nphi)
    dp = 2*math.pi/nphi
    anti = lambda x: 0.5*(x*np.sqrt(np.maximum(0, 1-x*x)) + np.arcsin(x))
    amplitude = (anti(faces[1:])-anti(faces[:-1]))/widths
    cosphi = np.cos((np.arange(nphi)+0.5)*dp)*math.sin(dp/2)/(dp/2)
    a = np.zeros((ntheta, ntheta))
    for i in range(ntheta-1):
        c = (1-faces[i+1]**2)/(centers[i+1]-centers[i])
        a[i,i] -= c/widths[i]; a[i,i+1] += c/widths[i]
        a[i+1,i+1] -= c/widths[i+1]; a[i+1,i] += c/widths[i+1]
    a[np.diag_indices(ntheta)] -= 4*math.sin(dp/2)**2/(dp*dp*(1-centers*centers))
    rootw = np.sqrt(widths)
    symmetric = rootw[:,None]*a/rootw[None,:]
    vals, vec = np.linalg.eigh(symmetric)
    evolved = vec @ (np.exp(t*vals)*(vec.T @ (rootw*amplitude))) / rootw
    mode = (amplitude[:,None]*cosphi).reshape(-1)
    discrete = 1+0.2*(evolved[:,None]*cosphi).reshape(-1)
    continuum = 1+0.2*math.exp(-2*t)*mode
    residual = np.max(np.abs(g.apply(mode)-(a@amplitude)[:,None].__mul__(cosphi).reshape(-1)))
    return g, 1+0.2*mode, discrete, continuum, float(residual)


def row(g, y0, exact, method, control, t):
    fn = {"euler": euler, "rkl2": rkl2, "uniformization": uniformization}[method]
    value, work = fn(g, y0, t, control)
    initial_mass = float(np.dot(g.weights, y0))
    error = g.norm2(value-exact)/g.norm2(exact)
    return {"case": g.name, "cells": len(y0), "edges": len(g.left), "method": method,
            "control": control, "time": t, "q_max_exit": g.q,
            "spectral_upper_bound": g.spectral_bound, "relative_weighted_l2_error": error,
            "max_abs_error": float(np.max(np.abs(value-exact))),
            "relative_mass_drift": abs(float(np.dot(g.weights,value))-initial_mass)/abs(initial_mass),
            "minimum": float(np.min(value)), "weighted_l2_ratio": g.norm2(value)/g.norm2(y0),
            **work}


def campaign():
    rows = []
    spatial = []
    for dimension, ns in [(2,[8,16,32]), (3,[4,8,12])]:
        for n in ns:
            g, mode, rate, continuum_rate = periodic_grid(n, dimension)
            t = 0.1; y0 = 1+0.2*mode
            exact = 1+0.2*math.exp(rate*t)*mode
            cont = 1+0.2*math.exp(continuum_rate*t)*mode
            spatial.append({"case":g.name,"cells":len(mode),"relative_weighted_l2_spatial_error":g.norm2(exact-cont)/g.norm2(cont),"oracle":"analytic_discrete_Fourier_mode","time":t})
            rows.extend(case_rows(g,y0,exact,t))
    for nt in [6,10,16]:
        np_ = 2*nt
        g, faces, centers, widths = sphere_grid(nt,np_)
        t = 0.1; mode = np.repeat(centers,np_); y0=1+0.2*mode
        exact=1+0.2*math.exp(-2*t)*mode
        rows.extend(case_rows(g,y0,exact,t))
        g,x0,xd,xc,res = sphere_x_reference(nt,np_,t)
        value, work = uniformization(g,x0,t,1e-10)
        spatial.append({"case":g.name,"cells":len(x0),"relative_weighted_l2_spatial_error":g.norm2(xd-xc)/g.norm2(xc),"relative_weighted_l2_temporal_error":g.norm2(value-xd)/g.norm2(xd),"oracle":"independent_latitude_symmetric_eigendecomposition_of_cos_phi_mode","oracle_operator_residual":res,"time":t,"q_max_exit":g.q,**work})
    g,_,_,_=periodic_grid(8,2)
    spike=np.zeros(64);spike[0]=1
    t=2.5/g.q
    bad,s=rkl2_step(g,spike,t,3)
    good,work=uniformization(g,spike,t,1e-12)
    counter={"case":"periodic2d_n8_spike","time":t,"stages":s,
             "h_q":t*g.q,"rkl2_minimum":float(np.min(bad)),
             "exact_neighbor_coefficient":-5/512,
             "uniformization_minimum":float(np.min(good)),
             "uniformization_work":work,
             "interpretation":"RKL2 weighted-L2 stability is not positivity; reject for positivity-required unchecked use."}
    thresholds=[1e-3,1e-4,1e-5]
    selected=[]
    for case in sorted(set(r["case"] for r in rows)):
        for eps in thresholds:
            for method in ["euler","rkl2","uniformization"]:
                eligible=[r for r in rows if r["case"]==case and r["method"]==method and r["relative_weighted_l2_error"]<=eps]
                winner=min(eligible,key=lambda r:r["operator_actions"]) if eligible else None
                selected.append({"case":case,"error_cap":eps,"method":method,"status":"PASS" if winner else "NOT_REACHED_IN_TESTED_CONTROLS","selected":winner})
    return {"schema_version":1,"scope":"autonomous linear conservative scalar diffusion only",
            "source_sha256":hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            "python":sys.version,"numpy":np.__version__,"platform":platform.platform(),
            "timing_collected":False,"rows":rows,"spatial_refinement":spatial,
            "positivity_counterexample":counter,"same_error_cap_selection":selected}


def case_rows(g,y0,exact,t):
    rows=[]
    stable=max(1,math.ceil(t*g.q))
    for factor in [1,2,4,8,16,32,64,128,256,512,1024,2048]:
        steps=stable*factor
        rows.append(row(g,y0,exact,"euler",steps,t))
        if rows[-1]["relative_weighted_l2_error"] <= 1e-5:
            break
    for steps in [1,2,4,8,16,32,64,128]:
        rows.append(row(g,y0,exact,"rkl2",steps,t))
    for tol in [1e-4,1e-6,1e-9]:
        rows.append(row(g,y0,exact,"uniformization",tol,t))
    return rows


class Tests(unittest.TestCase):
    def test_invalid_graphs(self):
        for weights,left,right,c in [([0],[0],[0],[1]),([1,1],[0],[1],[-1]),([1],[0],[1],[1]),([1,math.inf],[],[],[])]:
            with self.assertRaises(ValueError):Graph(weights,left,right,c)
    def test_weighted_conservation_and_symmetry(self):
        g=Graph([.1,.3,.8],[0,1,0],[1,2,2],[.2,.7,.04])
        x=np.array([.4,1.2,-.5]);y=np.array([.1,.7,.3])
        self.assertLess(abs(np.dot(g.weights,g.apply(x))),1e-15)
        self.assertLess(abs(np.dot(g.weights*x,g.apply(y))-np.dot(g.weights*y,g.apply(x))),1e-15)
        self.assertTrue(np.all(g.apply(np.ones(3))==0))
        self.assertLessEqual(np.dot(g.weights*x,g.apply(x)),0)
    def test_markov_nonnegative_and_mass(self):
        g=Graph([.1,.3,.8],[0,1],[1,2],[.2,.7]);x=np.array([0.,2.,0.])
        y=g.markov(x)
        self.assertGreaterEqual(float(min(y)),0)
        self.assertLess(abs(np.dot(g.weights,y-x)),1e-15)
    def test_uniformization_bound_dense_oracle(self):
        g=Graph([.1,.3,.8],[0,1,0],[1,2,2],[.2,.7,.04])
        q=np.column_stack([g.apply(np.eye(3)[:,j]) for j in range(3)])
        w=np.sqrt(g.weights);vals,vec=np.linalg.eigh(w[:,None]*q/w[None,:])
        x=np.array([.2,1.3,-.3]);t=2.
        expected=vec@(np.exp(vals*t)*(vec.T@(w*x)))/w
        actual,work=uniformization(g,x,t,1e-8)
        self.assertLessEqual(g.norm2(actual-expected),work["relative_truncation_bound"]*g.norm2(x)+1e-14)
        self.assertLess(abs(np.dot(g.weights,actual-x)),1e-14)
    def test_rkl_second_order(self):
        g,mode,rate,_=periodic_grid(8,2);x=1+.2*mode;t=.1
        exact=1+.2*math.exp(rate*t)*mode
        a,_=rkl2(g,x,t,32);b,_=rkl2(g,x,t,64)
        ratio=g.norm2(a-exact)/g.norm2(b-exact)
        self.assertGreater(ratio,3.8);self.assertLess(ratio,4.2)
    def test_rkl_stability_and_positivity_failure(self):
        g,_,_,_=periodic_grid(8,2);x=np.zeros(64);x[0]=1
        y,_=rkl2_step(g,x,2.5/g.q,3)
        self.assertLessEqual(g.norm2(y),g.norm2(x)*(1+1e-14))
        self.assertAlmostEqual(float(np.min(y)),-5/512,places=14)
        self.assertLess(abs(np.dot(g.weights,y-x)),1e-15)
    def test_geometric_sphere(self):
        g,faces,centers,widths=sphere_grid(10,20)
        self.assertAlmostEqual(float(sum(g.weights)),4*math.pi,places=12)
        mu=np.repeat(centers,20)
        self.assertLess(float(max(abs(g.apply(mu)+2*mu))),1e-12)
        self.assertGreater(float(max(g.weights)/min(g.weights)),5)
    def test_sphere_independent_reference(self):
        g,y,exact,_,res=sphere_x_reference(6,12,.1)
        result,_=uniformization(g,y,.1,1e-11)
        self.assertLess(res,1e-12)
        self.assertLess(g.norm2(result-exact),1e-10)
    def test_periodic_fourier_modes(self):
        for d in [2,3]:
            g,mode,rate,_=periodic_grid(6,d)
            self.assertLess(float(max(abs(g.apply(mode)-rate*mode))),1e-13)
    def test_invalid_controls_and_zero_operator(self):
        g=Graph([1,2],[],[],[]);x=np.array([1.,2.])
        out,work=uniformization(g,x,1.)
        self.assertTrue(np.array_equal(out,x));self.assertEqual(work["operator_actions"],0)
        with self.assertRaises(ValueError):uniformization(g,x,-1)
        with self.assertRaises(ValueError):uniformization(g,x,1,0)
        h,_,_,_=periodic_grid(4,2)
        with self.assertRaises(ValueError):euler(h,np.ones(16),1,1)
        with self.assertRaises(ValueError):rkl2_step(h,np.ones(16),1,2)


def main():
    p=argparse.ArgumentParser();p.add_argument("--test",action="store_true");p.add_argument("--output",type=Path)
    args=p.parse_args()
    if args.test:
        suite=unittest.defaultTestLoader.loadTestsFromTestCase(Tests)
        result=unittest.TextTestRunner(verbosity=2).run(suite)
        raise SystemExit(0 if result.wasSuccessful() else 1)
    if args.output is None:p.error("--output required for campaign")
    result=campaign();args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(result,indent=2,allow_nan=False)+"\n")
    print(json.dumps({"output":str(args.output),"rows":len(result["rows"]),"spatial":len(result["spatial_refinement"]),"positivity_counterexample":result["positivity_counterexample"]}))

if __name__=="__main__":main()
