import os
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS"): os.environ[_v] = "1"
import sys, time, json, numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pr, swdrv
name = sys.argv[1]; rt = float(sys.argv[2]); arms = sys.argv[3].split(',')
p = pr.build(name)
for a in arms:
    kw = {}
    if ':' in a:
        a, opts = a.split(':')
        for o in opts.split('+'):
            k, v = o.split('=')
            kw[k] = (v == 'True') if v in ('True', 'False') else float(v)
    t0 = time.time()
    r = swdrv.integrate(p, rt, swdrv.make_arm(a, **kw))
    e = p['err'](r['y']) if p['ref'] is not None else float('nan')
    F = r['flops']
    print(f"{name} {rt:g} {a:6s} {kw} att={r['att']} rej={r['rej']} lf={r['lin_fail']} jvp={r['jvp']} rhs={r['rhs']} dots={r['dots']} axpys={r['axpys']} "
          f"e_f={r['e_f']} pow={r['e_pow']} sh={r['sh_f']} g={r['g_jvp']} hand={r['n_handoff']} rev={r['n_revert']} rockfrac={r['rock_frac_time']:.2f} "
          f"Mflop={F['total']/1e6:.3f} err={e:.3e} wall={time.time()-t0:.1f}s", flush=True)
    print('    flops', {k: round(v/1e6,3) for k,v in F.items()})
    if r.get('events'):
        for ev in r['events'][:8]: print('    ', ev)
    if r.get('shadow_summary'): print('    shadow', r['shadow_summary'])
