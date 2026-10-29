"""Job runner for PROBE B2 (EXPLORATORY). Usage: python3 run.py <jobs-file> <out.jsonl> [workers]
jobs-file lines: problem arm rtol      (arm: base | mf | direct | rock4 | sw | swng | sw-k0.2 | ...)
Skips jobs already present in out.jsonl. Each result line holds counters, flop parts, endpoint error, events."""
import os
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS", "MKL_NUM_THREADS"):
    os.environ[_v] = "1"
import sys, json, time, traceback
from concurrent.futures import ProcessPoolExecutor, as_completed
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

ARMS = {
    'base': ('base', {}), 'mf': ('mf', {}), 'direct': ('direct', {}), 'rock4': ('rock4', {}),
    'sw': ('sw', {}), 'swng': ('sw', {'guard': False}), 'sw-k0.2': ('sw', {'kappa': 0.2}),
    'rock4-k0.2': ('rock4', {'kappa': 0.2}), 'swd': ('swd', {}), 'swdng': ('swd', {'guard': False}), 'swd-r2': ('swd', {'ratio': 2.0}),
}
_PCACHE = {}


def job(pname, arm, rt):
    import pr, swdrv
    if pname not in _PCACHE:
        _PCACHE[pname] = pr.build(pname)
    p = _PCACHE[pname]
    kind, kw = ARMS[arm]
    t0 = time.time()
    try:
        r = swdrv.integrate(p, rt, swdrv.make_arm(kind, **kw))
    except Exception as e:
        return dict(problem=pname, arm=arm, rtol=rt, ok=False, error=f'{type(e).__name__}: {e}', wall=time.time() - t0)
    y = np.asarray(r.pop('y'))
    err = p['err'](y) if p['ref'] is not None else None
    out = dict(problem=pname, arm=arm, rtol=rt, ok=True, err=err, err_name=p['err_name'], n=p['n'])
    if p['n'] <= 512:
        out['y'] = y.tolist()
    for k, v in r.items():
        if isinstance(v, (np.floating, np.integer)):
            v = v.item()
        out[k] = v
    out['events'] = [{k: (float(v) if isinstance(v, (np.floating,)) else v) for k, v in e.items()} for e in r.get('events', [])]
    out['hfov_hist'] = [[float(a) if a is not None else None for a in q] for q in r.get('hfov_hist', [])]
    out['wall'] = time.time() - t0
    return out


def main():
    jobs = [l.split() for l in open(sys.argv[1]) if l.strip() and not l.startswith('#')]
    outp = sys.argv[2]; nw = int(sys.argv[3]) if len(sys.argv) > 3 else 2
    done = set()
    if os.path.exists(outp):
        for l in open(outp):
            try:
                d = json.loads(l); done.add((d['problem'], d['arm'], float(d['rtol'])))
            except Exception:
                pass
    todo = [(a, b, float(c)) for a, b, c in jobs if (a, b, float(c)) not in done]
    print(f'{len(todo)} jobs ({len(done)} done)', flush=True)
    with ProcessPoolExecutor(max_workers=nw) as ex, open(outp, 'a') as fo:
        futs = {ex.submit(job, *j): j for j in todo}
        for fu in as_completed(futs):
            j = futs[fu]
            try:
                d = fu.result()
            except Exception:
                d = dict(problem=j[0], arm=j[1], rtol=j[2], ok=False, error=traceback.format_exc()[-500:])
            fo.write(json.dumps(d, default=float) + '\n'); fo.flush()
            if d.get('ok'):
                print(f"{d['problem']:12s} {d['arm']:8s} {d['rtol']:.2e} att={d['att']:6d} Mflop={d['flops']['total']/1e6:10.3f} "
                      f"err={d['err']:.3e} hand={d.get('n_handoff')} rev={d.get('n_revert')} rock={d.get('rock_frac_time', 0):.2f} wall={d['wall']:.1f}s", flush=True)
            else:
                print('FAIL', j, d.get('error'), flush=True)


if __name__ == '__main__':
    main()
