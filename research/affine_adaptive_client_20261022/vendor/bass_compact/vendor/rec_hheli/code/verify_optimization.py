"""Bounded same-physics finite-state parity; no IVP or performance benchmark."""
from pathlib import Path
import json
import math
import sys

import numpy as np
from numpy.polynomial.legendre import legvander

from geometry import Geometry, ROOT


def main():
    cases = [(s,m,r) for s in (0.,.03,.6) for m in (-.9,0.,1.8)
             for r in (-.45,0.,.45)]
    max_moment = max_rhs = max_observable = max_batch = 0.
    min_initial_photon = 1.
    prepared_bytes = 0
    count = 0
    for s,m,r in cases:
        ref = Geometry(s,m,r,'reference')
        fast = Geometry(s,m,r,'prepared')
        prepared_bytes = fast.model.momentum.prepared_array_bytes
        xs = np.array([0., .3, math.log(7)])
        bs = np.array([0., .05, -.15])
        batch = fast.model.momentum.moment_batch(xs,bs,chunk=2,max_work_bytes=2*2592*8)
        for i,(x,b) in enumerate(zip(xs,bs)):
            y = ref.initial.copy()
            y[1] = b
            # Frozen admissible states, not trajectories or pseudo-solutions.
            y[2] *= math.exp(-3*x)
            rm = ref.model.momentum.moments(x,b)
            fm = fast.model.momentum.moments(x,b)
            ra = np.array([rm.rho,rm.pressure_perp,rm.pressure_parallel])
            fa = np.array([fm.rho,fm.pressure_perp,fm.pressure_parallel])
            max_moment = max(max_moment,float(np.max(abs(fa-ra)/(1+abs(ra)))))
            max_batch = max(max_batch,float(np.max(abs(batch[i]-ra)/(1+abs(ra)))))
            original_rhs = ref.model.rhs(x,y)
            optimized_rhs = fast.rhs(x,y,0.)
            max_rhs = max(max_rhs,float(np.max(abs(original_rhs-optimized_rhs)/(1+abs(original_rhs)))))
            max_observable = max(max_observable,float(np.max(abs(ref.normalized(x,y)-fast.normalized(x,y)))))
            count += 1
        mu = np.linspace(-1,1,257)
        min_initial_photon = min(min_initial_photon,float(np.min(1+r*(3*mu**2-1)/2)))
    # Dynamic opacity has the exact source damping law and no neutrino term.
    model = Geometry(.15,.9,.225)
    y = model.initial
    q = model.fields(0.,y)
    off,on = model.rhs_from_fields(y,q,0),model.rhs_from_fields(y,q,200.)
    from lrs_boltzmann import GAMMA_PER_NE
    expected = np.zeros(40)
    for start in (4,13):
        expected[start:start+9] = -200*GAMMA_PER_NE/1e6/q['H']*model.model.thomson*y[start:start+9]
    opacity_error = float(np.max(abs((on-off)-expected)))
    gates = dict(moment_scaled_abs=max_moment<2e-14,
                 bounded_batch_scaled_abs=max_batch<2e-14,
                 original_rhs_scaled_abs=max_rhs<2e-14,
                 signed_observable_abs=max_observable<2e-14,
                 source_opacity_abs=opacity_error<2e-14,
                 initial_photon_positive=min_initial_photon>0,
                 neutrino_collision_exact_zero=bool(np.array_equal((on-off)[22:],np.zeros(18))))
    result = dict(schema='rec.hheli.optimization.finite_states.v1',
        status='PASS' if all(gates.values()) else 'FAIL',gates=gates,
        cases=len(cases),frozen_states=count,source_ivps=0,new_ivps=0,benchmarks=0,
        max_scaled_moment_difference=max_moment,max_scaled_batch_difference=max_batch,
        max_scaled_rhs_difference=max_rhs,max_signed_observable_difference=max_observable,
        opacity_absolute_difference=opacity_error,min_initial_photon_angular_weight=min_initial_photon,
        prepared_array_bytes=prepared_bytes,packet_work_array_bytes=2*2592*8,
        claim='algebraic finite-state parity; no speedup or trajectory accuracy claim')
    path=ROOT/'evidence/OPTIMIZATION_VERIFICATION.json'
    path.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))
    if not all(gates.values()):sys.exit(1)

if __name__=='__main__':main()
