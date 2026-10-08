"""Per-attempt cost decomposition and the kappa = F_LU / S switching rule (EXPLORATORY).
For each (problem, rtol) the per-attempt flops of direct vs lag/lin/dst are decomposed; kappa_star is the
factor/solve ratio of the PRECONDITIONER-SIDE matrix above which the PC arm would beat direct with the measured
iteration/refresh rates:   direct/att = F_LU(W) + 8 S(W) + rest_d ;  pc/att = r F_P + p S_P + rest_p
   => pc < direct  <=>  F_LU(W) (1) > r F_P + p S_P + rest_p - 8 S(W) - rest_d.
Reported: measured per-attempt ratio pc/direct, and kappa_W = F_LU(W)/S(W), kappa_star = break-even F_LU(W)/S(W)
holding everything else at its measured value (expressed in units of S(W))."""
import json, os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from analyze import load

PROBS = ['semilin2d-32', 'semilin2d-64', 'semilin2d-128', 'semilin2d-256', 'bruss2d-32', 'bruss2d-64', 'bruss2d-128',
         'bruss2d-256', 'bruss2d-64-a0.1', 'bruss2d-128-a0.1', 'semilin2d-32x48']
RT = [1e-4, 1e-6, 1e-8]


def main():
    print(f"{'problem':17s} {'rtol':6s} {'arm':5s} {'att':>4s} {'LU/att':>6s} {'pcs/att':>7s} {'kW':>6s} {'k*':>6s} "
          f"{'dir MF/att':>10s} {'pc MF/att':>9s} {'ratio':>6s}  per-attempt pc split (LU/PC/JVP/orth/other, MF)")
    for pn in PROBS:
        rows = load(pn)
        if not rows: continue
        for rt in RT:
            d = rows.get((rt, 'direct'))
            if d is None or 'error' in d: continue
            fd = d['flops']; ad = d['att']
            S = fd['lusolve'] / d['solves']           # S(W) per solve
            FLU = fd['lu'] / d['lus']                 # F_LU(W) per factorization
            kW = FLU / S
            rest_d = (fd['total'] - fd['lu'] - fd['lusolve']) / ad
            for arm in ('lag', 'lin', 'dst'):
                o = rows.get((rt, arm))
                if o is None or 'error' in o: continue
                fo = o['flops']; ao = o['att']
                pc_att = fo['total'] / ao; dir_att = fd['total'] / ad
                # break-even F_LU(W): FLU* + 8S + rest_d = pc_att  (pc side held fixed, except that for 'lag' the
                # refresh LU is also F_LU(W): FLU* (1 - r) = pc_att - lu_part - 8S - rest_d)
                r = o['lus'] / ao
                if arm == 'lag':
                    other = pc_att - fo['lu'] / ao
                    kstar = (other - 8 * S - rest_d) / (1 - r) / S
                else:
                    kstar = (pc_att - 8 * S - rest_d) / S
                print(f"{pn:17s} {rt:<6g} {arm:5s} {ao:4d} {r:6.3f} {o['pcs']/ao:7.1f} {kW:6.1f} {kstar:6.1f} {dir_att/1e6:10.2f} {pc_att/1e6:9.2f} "
                      f"{pc_att/dir_att:6.3f}  {fo['lu']/ao/1e6:.2f}/{fo['pc']/ao/1e6:.2f}/{fo['jvp']/ao/1e6:.2f}/{fo['orth']/ao/1e6:.2f}/"
                      f"{(fo['total']-fo['lu']-fo['pc']-fo['jvp']-fo['orth'])/ao/1e6:.2f}")


if __name__ == '__main__':
    main()
