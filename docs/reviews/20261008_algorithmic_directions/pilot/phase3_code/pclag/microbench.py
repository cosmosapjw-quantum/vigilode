"""Per-operation wall-time model (compiled kernels: SuperLU factor/solve, scipy sparse matvec, numpy BLAS-1/2
orthogonalization), single thread, min over repeats. Gives a second cost model next to flops:
   T = n_LU t_fac + n_solve t_sol + n_jvp t_op + n_cjvp t_spmv + (dots+axpys+norms) t_vec + n_dst t_dst ...
EXPLORATORY. usage: python3 microbench.py prob1 prob2 ..."""
import os
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS", "MKL_NUM_THREADS"):
    os.environ[_v] = "1"
import sys, json, time
import numpy as np
import scipy.sparse as sp
import probs2d
from lucost import SparseLU


def tmin(fn, rep):
    best = 1e9
    for _ in range(rep):
        t0 = time.perf_counter(); fn(); best = min(best, time.perf_counter() - t0)
    return best


out = {}
fn = os.environ.get('MBOUT', 'microbench.json')
if os.path.exists(fn): out = json.load(open(fn))
for pn in sys.argv[1:]:
    p = probs2d.build(pn); n = p['n']
    J = p['J'](0.0, p['y0']); I = sp.identity(n, format='csr')
    hg = 0.01
    W = (I - hg * J).tocsr()
    rng = np.random.default_rng(0); v = rng.standard_normal(n); D = 1.0 / (1e-6 + 1e-6 * np.abs(p['y0']))
    rep_f = 5 if n > 20000 else 10
    from scipy.sparse.linalg import splu
    from lucost import lu_flops
    def fixed(Mx):
        # ordering computed ONCE (MMD_AT_PLUS_A), then symbolic+numeric factorization of the symmetrically
        # permuted matrix with NATURAL order is what is timed (production reuses the ordering)
        F0 = SparseLU(Mx, "mmd_at_plus_a"); pc = np.argsort(F0.lu.perm_c)
        Mp = Mx.tocsr()[pc][:, pc].tocsc()
        fac = lambda: splu(Mp, permc_spec='NATURAL', diag_pivot_thresh=0.01, options=dict(SymmetricMode=True))
        lu = fac(); ff, sf, nz = lu_flops(lu)
        return tmin(fac, rep_f), tmin(lambda: lu.solve(v), 50), ff, sf, F0
    t_fac, t_sol, ffx, sfx, F = fixed(W)
    Al = p['Alin'](0.0); M = (I - hg * Al).tocsr()
    t_facl, t_soll, ffl, sfl, Fl = fixed(M)
    print(f'   fixed-order fill check: W flops {ffx:.3e} vs MMD {F.fflops:.3e}; lin {ffl:.3e} vs {Fl.fflops:.3e}')
    t_op = tmin(lambda: D * ((v / D) - hg * (J @ (v / D))), 100)     # scaled shifted JVP (sparse model)
    t_spmv = tmin(lambda: J @ v, 100)
    V = rng.standard_normal((6, n)); w = rng.standard_normal(n)
    def cgs2():
        ww = w.copy()
        for _ in range(2):
            hh = V @ ww; ww -= hh @ V
    t_cgs = tmin(cgs2, 100) / (2 * 2 * 6)        # per dot-or-axpy (6 basis vectors, 2 passes, dot+axpy)
    t_rhs = tmin(lambda: p['f'](0.3, p['y0']), 50)
    t_dst = tmin(lambda: p['dst'].solve(v, hg), 30) if p['dst'] is not None else None
    t_lufl = None
    r = dict(n=n, t_fac=t_fac, t_sol=t_sol, t_facl=t_facl, t_soll=t_soll, t_op=t_op, t_spmv=t_spmv, t_vec=t_cgs, t_rhs=t_rhs,
             t_dst=t_dst, fflops=F.fflops, sflops=F.sflops, fflops_l=Fl.fflops, sflops_l=Fl.sflops,
             kappa_flop=F.fflops / F.sflops, kappa_time=t_fac / t_sol, kappa_flop_l=Fl.fflops / Fl.sflops, kappa_time_l=t_facl / t_soll,
             gflops_fac=F.fflops / t_fac / 1e9, gflops_sol=F.sflops / t_sol / 1e9)
    out[pn] = r
    print(pn, json.dumps({k: (round(v_, 7) if isinstance(v_, float) else v_) for k, v_ in r.items()}), flush=True)
    json.dump(out, open(fn, 'w'), indent=1)
