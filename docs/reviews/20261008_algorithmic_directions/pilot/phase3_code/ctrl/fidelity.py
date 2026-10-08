"""Replica fidelity vs recorded Rust counters.
Dense: sb_fast.json (stiff-benchmark rodas5p-fast, rtol 1e-3..1e-9, h0 1e-6, I controller).
MF: SPD07 BASE.json gmres_into_zero (Bruss-50/160, vdP, HIRES, Robertson at 1e-6/1e-8), base mode."""
import json, sys, time
import numpy as np
sys.path.insert(0, '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl')
from core import *
OUT = {}
sb = json.load(open('/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/sb_fast.json'))
pmap = {'robertson': 'robertson', 'hires': 'hires', 'van-der-pol-mu1000': 'vdp', 'brusselator-1d-50': 'bruss50'}
rows = []
if 'dense' in sys.argv[1:]:
    for r in sb['rows']:
        if r.get('arm') != 'rodas5p-fast' or r['problem'] not in pmap: continue
        p = PROBS[pmap[r['problem']]]()
        t0 = time.time(); q = dense_integrate(p, r['rtol'], make_ctrl('I'), h0=1e-6); dt = time.time() - t0
        rust_err = endpoint_error(p, np.array(r['final_state']))
        row = dict(problem=pmap[r['problem']], rtol=r['rtol'],
                   rust=(r['accepted_steps'] + r['rejected_steps'], r['accepted_steps'], r['rejected_steps'],
                         r['counters']['rhs_evaluations'], r['counters']['jacobian_builds'], r['counters']['direct_factorizations']),
                   rep=(q['att'], q['acc'], q['rej'], q['rhs'], q['jac'], q['lu']), rust_err=rust_err, rep_err=q['err'], sec=dt)
        row['exact'] = row['rust'] == row['rep']
        rows.append(row)
        print(f"{row['problem']:9s} {r['rtol']:.0e} rust att/acc/rej/rhs/J/LU={row['rust']} rep={row['rep']} exact={row['exact']} "
              f"err rust={rust_err:.3e} rep={q['err']:.3e} [{dt:.2f}s]", flush=True)
    OUT['dense'] = rows
if 'mf' in sys.argv[1:]:
    base = json.load(open('/home/user/wt-speed/research/spd07_mf_step_warm_start_20261007/BASE.json'))
    m2 = {'brusselator-1d-50': 'bruss50', 'brusselator-1d-160': 'bruss160', 'van-der-pol-mu1000': 'vdp', 'hires': 'hires', 'robertson': 'robertson'}
    mrows = []
    for r in base['rows']:
        if r['case'] not in m2: continue
        z = r['gmres_into_zero']; cc = z['counters']
        p = PROBS[m2[r['case']]]()
        t0 = time.time(); q = mf_integrate(p, r['rtol'], make_ctrl('I'), h0=1e-6, mode='base'); dt = time.time() - t0
        rust = (z['attempts'], z['accepted'], z['rejected'], cc['jvp_vectors'], cc['linear_iterations'], cc['orthogonalization_inner_products'], cc['rhs_evaluations'])
        rep = (q['att'], q['acc'], q['rej'], q['jvp'], q['cols'], q['ip'], q['rhs'])
        mrows.append(dict(problem=m2[r['case']], rtol=r['rtol'], rust=rust, rep=rep, sec=dt))
        print(f"MF {m2[r['case']]:9s} {r['rtol']:.0e} rust att/acc/rej/JVP/iters/ip/rhs={rust} rep={rep} "
              f"JVP ratio={q['jvp']/cc['jvp_vectors']:.4f} ip ratio={q['ip']/cc['orthogonalization_inner_products']:.4f} [{dt:.1f}s]", flush=True)
    OUT['mf'] = mrows
json.dump(OUT, open(f'{HERE}/fidelity_{"_".join(sys.argv[1:])}.json', 'w'), indent=1)
