"""Fidelity: driver.py 'base' arm (sparse J, MGS2) vs SPD07 BASE.json gmres_into_zero counters, Bruss-1d-50/160."""
import json, sys
import numpy as np
import driver, probs2d
B = json.load(open('/home/user/wt-speed/research/spd07_mf_step_warm_start_20261007/BASE.json'))
out = {}
for case, nm in (('brusselator-1d-50', 'bruss1d-50'), ('brusselator-1d-160', 'bruss1d-160')):
    p = probs2d.build(nm)
    for rt in (1e-6, 1e-8):
        row = [r for r in B['rows'] if r['case'] == case and r['rtol'] == rt][0]['gmres_into_zero']
        cR = row['counters']
        for ortho in ('mgs2', 'cgs2'):
            r = driver.integrate(p, rt, driver.make_arm('base', maxit=200, ortho=ortho))
            line = dict(case=case, rtol=rt, ortho=ortho, rust=dict(att=row['attempts'], acc=row['accepted'], rej=row['rejected'],
                        jvp=cR['jvp_vectors'], dots=cR['orthogonalization_inner_products'], axpys=cR['orthogonalization_vector_updates'],
                        rhs=cR['rhs_evaluations'], fails=cR['linear_solve_failures']),
                        rep=dict(att=r['att'], acc=r['acc'], rej=r['rej'], jvp=r['jvp'], dots=r['dots'], axpys=r['axpys'], rhs=r['rhs'], fails=r['lin_fail']))
            print(json.dumps(line), flush=True)
            out[f'{case}-{rt}-{ortho}'] = line
json.dump(out, open('fid_spd07.json', 'w'), indent=1)
