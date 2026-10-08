import sys, time
sys.path.insert(0,'.')
import stack, run, numpy as np
p = run.problem('bruss50')
for an in sys.argv[1].split(','):
    t0=time.time()
    try:
        r = stack.integrate(p, float(sys.argv[2]), run.ARMS[an], gapdiag=True, max_att=int(sys.argv[3]))
        print(an, r['att'], r['acc'], r['rej'], r['lin_fail'], r['jvp'], r['fd_rhs'], r['maxit_fail'], run.gap_summary(r['gaps']), f'{time.time()-t0:.1f}s', flush=True)
    except Exception as e:
        print(an, 'ERR', e, f'{time.time()-t0:.1f}s', flush=True)
