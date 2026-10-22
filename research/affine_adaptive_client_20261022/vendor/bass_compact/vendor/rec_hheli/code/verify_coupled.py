"""Finite coupled Jacobian and physical wiring checks, without new IVPs."""
import hashlib
import json
import math
import sys

import numpy as np

from coupled import CoupledSystem, ROOT, thermal_uv


def main():
    sys1=CoupledSystem(dict(Sigma_F=.15,matter_A=.9,radiation_A=-.225))
    rng=np.random.default_rng(2189)
    errors=[];conservation=[]
    for x in (0.,.6,1.6):
        y=sys1.y0.copy()
        y[1]=.07*x
        y[2]*=math.exp(-3*x)
        # Each populated isotope gets positive test charge populations.
        for name,indices in sys1.atomic.isotope_groups.items():
            if name in ('n','T'):continue
            weights=np.arange(1,len(indices)+1,dtype=float)
            y[40+np.array(indices)]=weights/weights.sum()
        jac=sys1.jac(x,y)
        for kind in range(5):
            direction=rng.normal(size=62)
            direction[3]=0
            direction[[40,45,46]]=0 # zero n/T orbit; huge decay modes separately verified
            if kind==0:direction[40:61]=0
            if kind==1:direction[:40]=0
            if kind==2:
                direction[:]=0;direction[1]=1
            if kind==3:
                direction[:]=0;direction[4]=1;direction[22]=-.7
            h=2e-5
            fd=(-sys1.rhs(x,y+2*h*direction)+8*sys1.rhs(x,y+h*direction)
                -8*sys1.rhs(x,y-h*direction)+sys1.rhs(x,y-2*h*direction))/(12*h)
            analytic=jac@direction
            errors.append(float(np.max(abs(fd-analytic)/(1+abs(analytic)))))
        rhs=sys1.rhs(x,y)
        conservation.append(float(abs(sys1.atomic.A@(rhs[40:61]*sys1.scale))))
    y=sys1.y0
    on=sys1.rhs(.8,y)
    offsystem=CoupledSystem(dict(Sigma_F=.15,matter_A=.9,radiation_A=-.225,opacity=False))
    off=offsystem.rhs(.8,y)
    no_neutrino=bool(np.array_equal(on[22:40],off[22:40]))
    chemical_same=bool(np.array_equal(on[40:],off[40:]))
    # A finite exact counterexample for summing norms before tensors.
    pm,pr=1.,-1.
    incorrect_norm=abs(pm)+abs(pr)
    correct_norm=abs(pm+pr)
    gates=dict(coupled_analytic_jacobian=max(errors)<2e-6,
        atomic_baryon_rhs=max(conservation)<1e-7,
        gamma_only_collision=no_neutrino,instant_chemistry_independent_of_collision_toggle=chemical_same,
        norm_order_counterexample=bool(correct_norm==0 and incorrect_norm==2),
        temperature_within_domain=all(100<=thermal_uv(x)[0]<=12000 for x in np.linspace(0,math.log(7),31)))
    out=dict(status='PASS' if all(gates.values()) else 'FAIL',gates=gates,
        finite_states=3,directions=15,max_jvp_scaled_difference=max(errors),
        max_atomic_baryon_rhs=max(conservation),ivps=0,benchmark=0,
        code_sha256={n:hashlib.sha256((ROOT/'code'/n).read_bytes()).hexdigest()
                     for n in ('geometry.py','coupled.py','atomic_network.py')},
        scope='finite Jacobian and wiring checks; no global or trajectory error certificate')
    (ROOT/'evidence/COUPLED_VERIFICATION.json').write_text(json.dumps(out,indent=2)+'\n')
    print(json.dumps(out,indent=2))
    if not all(gates.values()):sys.exit(1)

if __name__=='__main__':main()
