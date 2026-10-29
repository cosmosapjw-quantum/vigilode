"""Finite-grid MB remap budget under positive mass-preserving angular maps.

This is a verification of a discrete theorem, not a CMB PDE benchmark.
State entries are masses on equal-area (mu,phi) cells; total mass is one.
The propagator is an exact grid rotation followed by isotropic relaxation.
"""
from __future__ import annotations
import hashlib
import json
import platform
from pathlib import Path
import time
import numpy as np
from scipy.special import kl_div


def hellinger(p, q):
    return float(np.linalg.norm(np.sqrt(p) - np.sqrt(q)))


def poisson_divergence(p, q):
    return float(np.sum(kl_div(p, q)))


def propagate(p, shift, optical_depth):
    a = np.exp(-optical_depth)
    return a * np.roll(p, shift, axis=1) + (-np.expm1(-optical_depth)) * p.sum() / p.size


def project(p, block, offset):
    # Rotating the partition changes the map, but each map is stochastic.
    shifted = np.roll(p, offset, axis=1)
    nmu, nphi = p.shape
    if nmu % block or nphi % block:
        raise ValueError('block must divide both dimensions')
    mean = shifted.reshape(nmu//block, block, nphi//block, block).mean(axis=(1,3))
    coarse = mean.repeat(block, axis=0).repeat(block, axis=1)
    return np.roll(coarse, -offset, axis=1)


def run_case(block, optical_depth, remap_every, steps=40):
    nmu, nphi = 16, 32
    mu = -1 + (np.arange(nmu)+.5)*2/nmu
    phi = (np.arange(nphi)+.5)*2*np.pi/nphi
    xx = np.sqrt(1-mu[:,None]**2)*np.cos(phi[None,:])
    p = 1e-6 + np.exp(16*(xx-1)) + .05*(1+mu[:,None])
    p /= p.sum()
    ref = p.copy(); approx = p.copy()
    bound = 0.; entropy_sum = 0.; history=[]
    initial_mass=float(p.sum())
    kernel=np.sqrt(1-mu[:,None]**2)*np.cos(phi[None,:])
    kernel_range=float(np.ptp(kernel))
    for j in range(1,steps+1):
        ref=propagate(ref,1,optical_depth)
        prior=propagate(approx,1,optical_depth)
        eta=0.
        if j % remap_every == 0:
            approx=project(prior,block,(j//remap_every)%block)
            eta=poisson_divergence(prior,approx)
            local_h=hellinger(prior,approx)
            assert local_h**2 <= eta+1e-14
            bound += np.sqrt(max(0.,eta)); entropy_sum+=eta
        else:
            approx=prior
        distance=hellinger(ref,approx)
        obs=float(abs(np.sum(kernel*(ref-approx))))
        obs_bound=kernel_range*np.sqrt(initial_mass)*bound
        assert distance<=bound+2e-13
        assert obs<=obs_bound+2e-13
        assert abs(approx.sum()-initial_mass)<2e-13
        assert np.min(approx)>=0
        history.append({'step':j,'local_poisson_kl':eta,'hellinger':distance,
                        'accumulated_sqrt_kl_bound':float(bound),'observable_error':obs,
                        'observable_bound':obs_bound})
    return {'block':block,'optical_depth_per_step':optical_depth,'remap_every':remap_every,
            'steps':steps,'fine_cells':p.size,'final_hellinger':distance,
            'sqrt_budget_sum':float(bound),'final_poisson_kl':poisson_divergence(ref,approx),
            'sum_local_poisson_kl':entropy_sum,'observable_error':obs,
            'observable_bound':obs_bound,'history':history}


def main():
    start=time.perf_counter()
    cases=[run_case(b,tau,k) for b in (2,4) for tau in (0.,.2) for k in (1,4)]
    # Independent nonuniform-weight test works in mass coordinates.
    rng=np.random.default_rng(20261023)
    gaps=[]
    for _ in range(30):
        p=rng.random(17);p/=p.sum();q=rng.random(17);q/=q.sum()
        a=rng.random((11,17));a/=a.sum(axis=0,keepdims=True)
        gaps.append(hellinger(p,q)-hellinger(a@p,a@q))
    assert min(gaps)>=-1e-14
    result={'schema':'spherical-remap-budget.v1','execution':'completed','classification':'finite-grid theorem verification',
            'source_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            'command':'python3 src/remap_budget_demo.py','numpy_version':np.__version__,
            'python':platform.python_version(),'wall_seconds':time.perf_counter()-start,
            'cases':cases,'random_stochastic_checks':30,'minimum_contraction_gap':min(gaps),
            'claim':'Hellinger <= sum sqrt(local MB Poisson divergence); bounded-observable certificate',
            'not_tested':['continuum spatial/angular truncation error','DOPRI/RODAS positivity','coupled matter feedback']}
    out=Path(__file__).resolve().parents[1]/'results'/'remap_budget_results.json'
    out.parent.mkdir(exist_ok=True);out.write_text(json.dumps(result,indent=2,allow_nan=False)+'\n')
    print(json.dumps({'cases':len(cases),'step_checks':sum(c['steps'] for c in cases),
                      'random_stochastic_checks':30,'minimum_contraction_gap':min(gaps),
                      'wall_seconds':result['wall_seconds']}))


if __name__=='__main__':
    main()
