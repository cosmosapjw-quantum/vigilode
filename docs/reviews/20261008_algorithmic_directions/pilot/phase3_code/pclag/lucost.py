"""Sparse LU with explicit flop accounting (PROBE B4, exploratory).

Orderings: 'nd'  geometric nested dissection of the nx x ny cell grid (separator = middle grid line of the
                 longer side, recursively; cells of a leaf box ordered naturally; b unknowns per cell kept
                 together), factored with SuperLU permc_spec='NATURAL' on the symmetrically permuted matrix;
           'colamd', 'mmd_at_plus_a': SuperLU's own column orderings on the unpermuted matrix.
Flop model (exact for the produced fill, no cancellation):
   factor flops = sum_k [ l_k + 2 l_k u_k ],  l_k = nnz strictly below the diagonal in column k of L,
                                              u_k = nnz strictly right of the diagonal in row k of U
   solve flops  = 2 (nnz_strict(L) + nnz_strict(U)) + n        (forward + backward substitution)
"""
import numpy as np
import scipy.sparse as sp
from scipy.sparse.linalg import splu


def nd_cells(nx, ny, leaf=4):
    out = []
    def rec(i0, i1, j0, j1):
        w, h = i1 - i0, j1 - j0
        if w <= 0 or h <= 0:
            return
        if w * h <= leaf * leaf or (w <= 2 and h <= 2):
            for j in range(j0, j1):
                for i in range(i0, i1):
                    out.append(i + nx * j)
            return
        if w >= h:
            m = i0 + w // 2
            rec(i0, m, j0, j1); rec(m + 1, i1, j0, j1)
            for j in range(j0, j1): out.append(m + nx * j)
        else:
            m = j0 + h // 2
            rec(i0, i1, j0, m); rec(i0, i1, m + 1, j1)
            for i in range(i0, i1): out.append(i + nx * m)
    rec(0, nx, 0, ny)
    out = np.array(out)
    assert len(out) == nx * ny and len(np.unique(out)) == nx * ny
    return out


def nd_perm(nx, ny, b):
    cells = nd_cells(nx, ny)
    return (cells[:, None] * b + np.arange(b)[None, :]).ravel()


def lu_flops(lu):
    L = lu.L.tocsc(); U = lu.U.tocsr()
    lk = np.diff(L.indptr) - 1
    uk = np.diff(U.indptr) - 1
    n = L.shape[0]
    fac = float(np.sum(lk) + 2.0 * np.sum(lk.astype(float) * uk.astype(float)))
    nnzs = int(np.sum(lk) + np.sum(uk))
    sol = float(2 * nnzs + n)
    return fac, sol, nnzs + n


class SparseLU:
    """Factor M (csr/csc) with the chosen ordering; .solve(r); .fflops, .sflops, .nnz"""
    def __init__(self, M, ordering='nd', perm=None, thresh=0.01):
        self.ordering = ordering
        if ordering == 'nd':
            p = perm
            self.p = p
            Mp = M.tocsr()[p][:, p].tocsc()
            self.lu = splu(Mp, permc_spec='NATURAL', diag_pivot_thresh=thresh,
                           options=dict(SymmetricMode=True))
        else:
            self.p = None
            spec = {'colamd': 'COLAMD', 'mmd_at_plus_a': 'MMD_AT_PLUS_A', 'mmd_ata': 'MMD_ATA'}[ordering]
            self.lu = splu(M.tocsc(), permc_spec=spec, diag_pivot_thresh=(thresh if ordering == 'mmd_at_plus_a' else 1.0),
                           options=(dict(SymmetricMode=True) if ordering == 'mmd_at_plus_a' else None))
        self.fflops, self.sflops, self.nnz = lu_flops(self.lu)

    def solve(self, r):
        if self.p is None:
            return self.lu.solve(r)
        x = np.empty_like(r)
        x[self.p] = self.lu.solve(r[self.p])
        return x


if __name__ == '__main__':
    import sys, time
    sys.path.insert(0, '.')
    import probs2d
    for nm in sys.argv[1:]:
        p = probs2d.build(nm)
        J = p['J'](0.0, p['y0']); n = p['n']
        nx, ny = p['grid']
        perm = nd_perm(nx, ny, p['b'])
        for hg in (1e-3, 1e-2, 1e-1):
            W = (sp.identity(n, format='csr') - hg * J).tocsr()
            for o in ('nd', 'colamd', 'mmd_at_plus_a'):
                t0 = time.time()
                try:
                    F = SparseLU(W, o, perm=perm)
                except Exception as e:
                    print(nm, o, 'FAIL', e); continue
                dt = time.time() - t0
                r = np.random.default_rng(0).standard_normal(n)
                t1 = time.time(); x = F.solve(r); ds = time.time() - t1
                res = np.linalg.norm(W @ x - r) / np.linalg.norm(r)
                print(f"{nm:16s} hg={hg:g} {o:14s} nnzLU={F.nnz:9d} ({F.nnz/n:6.1f}/row) factor={F.fflops:.3e} "
                      f"solve={F.sflops:.3e} ratio={F.fflops/F.sflops:6.1f}  t_fac={dt:.3f}s t_sol={ds*1e3:.1f}ms res={res:.1e}", flush=True)
