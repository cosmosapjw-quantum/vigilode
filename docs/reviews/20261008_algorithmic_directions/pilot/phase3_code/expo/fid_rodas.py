"""Fidelity: my copy of the PROBE B2 RODAS arms reproduces the B2 rows (EXPLORATORY)."""
import os, sys, json, math
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS"): os.environ[_v] = "1"
HERE = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, os.path.join(HERE, 'rodas'))
import numpy as np, pr, swdrv
old = {}
for l in open(os.path.join(HERE, 'res/reg_runs_copy.jsonl')):
    d = json.loads(l)
    if d.get('ok'): old[(d['problem'], d['arm'], round(math.log10(d['rtol'])*2)/2)] = d
out = []
for pn, arm, rt in [('rot-96', 'mf', 1e-6), ('hires-96', 'base', 1e-6), ('bruss1d-50', 'mf', 1e-6), ('semi-96', 'mf', 1e-8)]:
    p = pr.build(pn); r = swdrv.integrate(p, rt, swdrv.make_arm(arm)); o = old[(pn, arm, round(math.log10(rt)*2)/2)]
    e = p['err'](r['y'])
    row = dict(problem=pn, arm=arm, rtol=rt, att=(r['att'], o['att']), jvp=(r['jvp'], o['jvp']), dots=(r['dots'], o['dots']),
               flops=(r['flops']['total'], o['flops']['total']), err=(e, o['err']))
    print(row, flush=True); out.append(row)
json.dump(out, open(os.path.join(HERE, 'res/fid_rodas.json'), 'w'), indent=1, default=float)
