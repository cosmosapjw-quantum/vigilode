"""Dense fast-driver ladder: problems x arms x quarter-decade rtols x h0 seeds -> dense_runs.jsonl"""
import sys, json, itertools, time, os
os.environ.setdefault('OPENBLAS_NUM_THREADS', '1')
from multiprocessing import Pool
sys.path.insert(0, '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl')
from core import *

SEEDS = ['1e-06', '1e-04', '1e-02', 'auto']
def ladder(name):
    if name in ('hires', 'robertson'): return [10 ** (-3 - 0.25 * i) for i in range(29)]   # 1e-3..1e-10
    if name == 'rotnn96': return [10 ** (-4 - 0.25 * i) for i in range(17)]               # corpus range 1e-4..1e-8
    return [10 ** (-3 - 0.25 * i) for i in range(17)]                                    # 1e-3..1e-7

def job(a):
    name, arm, rtol, seed = a
    p = PROBS[name]()
    h0, xr = seed_h0(p, rtol, seed)
    t0 = time.time()
    r = dense_integrate(p, rtol, make_ctrl(arm), h0=h0, h0_rhs=xr)
    r.pop('rec', None)
    r.update(problem=name, arm=arm, rtol=rtol, seed=seed, h0=h0, sec=time.time() - t0)
    return r

if __name__ == '__main__':
    names = sys.argv[1].split(','); arms = sys.argv[2].split(',') if len(sys.argv) > 2 and sys.argv[2] != 'all' else list(ARMS)
    seeds = sys.argv[3].split(',') if len(sys.argv) > 3 else SEEDS
    out = sys.argv[4] if len(sys.argv) > 4 else f'{HERE}/dense_runs.jsonl'
    done = set()
    if os.path.exists(out):
        for line in open(out):
            d = json.loads(line); done.add((d['problem'], d['arm'], round(d['rtol'], 18), d['seed']))
    jobs = [j for j in itertools.product(names, arms, [None], seeds)]
    jobs = [(n, a, r, s) for (n, a, _, s) in jobs for r in ladder(n) if (n, a, round(r, 18), s) not in done]
    print('jobs', len(jobs), flush=True)
    with Pool(int(os.environ.get('NPROC', '2'))) as pool, open(out, 'a') as fo:
        for k, r in enumerate(pool.imap_unordered(job, jobs, chunksize=4)):
            fo.write(json.dumps(r) + '\n'); fo.flush()
            if k % 200 == 0: print(k, r['problem'], r['arm'], f"{r['rtol']:.2e}", r['seed'], r['att'], r['rej'], f"{r['err']:.2e}", flush=True)
    print('done', flush=True)
