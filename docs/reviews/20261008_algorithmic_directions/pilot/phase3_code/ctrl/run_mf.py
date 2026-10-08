"""Matrix-free U-form ladder (closed loop): problems x arms x modes x rtols x seeds -> mf_runs.jsonl
modes: proj = proj-stop at the base L2 target max(g*1e-14, 1e-10*||b||); tf1/tf2 = INO-FORCE-ABS uncertified
absolute WRMS targets theta=0.001/0.002 (err_exp 1.2, err_prev, U8 floor) on top of proj-stop; rel4 = uniform
relative forcing eta=1e-4 (negative control); base = production full cycles."""
import sys, json, itertools, time, os
os.environ.setdefault('OPENBLAS_NUM_THREADS', '1')
from multiprocessing import Pool
sys.path.insert(0, '/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/ctrl')
from core import *

MODES = {'proj': dict(mode='proj'), 'base': dict(mode='base'), 'tf1': dict(mode='tf', theta=0.001),
         'tf2': dict(mode='tf', theta=0.002), 'rel4': dict(mode='rel', eta=1e-4)}

def ladder(name):
    if name == 'hires': return [10 ** (-3 - 0.25 * i) for i in range(29)]
    return [10 ** (-3 - 0.25 * i) for i in range(17)]

def job(a):
    name, arm, mode, rtol, seed = a
    p = PROBS[name]()
    h0, xr = seed_h0(p, rtol, seed)
    t0 = time.time()
    r = mf_integrate(p, rtol, make_ctrl(arm), h0=h0, h0_rhs=xr, maxatt=int(os.environ.get('MAXATT', '20000')), **MODES[mode])
    r.pop('rec', None)
    r.update(problem=name, arm=arm, mode=mode, rtol=rtol, seed=seed, h0=h0, sec=time.time() - t0,
             flops10=mf_flops(r, p, 10), flops100=mf_flops(r, p, 100))
    return r

if __name__ == '__main__':
    names = sys.argv[1].split(','); arms = sys.argv[2].split(','); modes = sys.argv[3].split(',')
    seeds = sys.argv[4].split(','); out = f'{HERE}/mf_runs.jsonl'
    done = set()
    if os.path.exists(out):
        for line in open(out):
            d = json.loads(line); done.add((d['problem'], d['arm'], d['mode'], round(d['rtol'], 18), d['seed']))
    jobs = [(n, a, m, r, s) for n in names for a in arms for m in modes for s in seeds for r in ladder(n)
            if (n, a, m, round(r, 18), s) not in done]
    # longest first (tight rtol, big n) for better load balance
    jobs.sort(key=lambda j: (-(320 if j[0] == 'bruss160' else 100 if j[0] == 'bruss50' else 8), j[3]))
    print('jobs', len(jobs), flush=True)
    with Pool(int(os.environ.get('NPROC', '2'))) as pool, open(out, 'a') as fo:
        for k, r in enumerate(pool.imap_unordered(job, jobs, chunksize=1)):
            fo.write(json.dumps(r) + '\n'); fo.flush()
            if k % 50 == 0: print(k, r['problem'], r['arm'], r['mode'], f"{r['rtol']:.2e}", r['seed'], r['att'], r['rej'], r['jvp'], f"{r['err']:.2e}", f"{r['sec']:.1f}s", flush=True)
    print('done', flush=True)
