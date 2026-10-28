import sys, time
sys.path.insert(0,'.')
import stack, run, numpy as np
p = run.problem('bruss50')
for an in sys.argv[1].split(','):
    t0=time.time()
    r = stack.integrate(p, float(sys.argv[2]), run.ARMS[an], gapdiag=True, max_att=int(sys.argv[3]), cap_ok=True)
    print(an, 'capped' if r['capped'] else 'done', r['att'], r['acc'], r['rej'], r['lin_fail'], r['jvp'], r['fd_rhs'], 'maxit_fail',r['maxit_fail'],'stalls',r['stalls'], run.gap_summary(r['gaps']), f'{time.time()-t0:.1f}s', flush=True)
