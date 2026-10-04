# PP03 — verified low-rank RHS compression for the shared shift jet (prospective registration)

RVJ DAG node `PP03` (depends on PP02). Base commit: the commit that adds
this file.

## Method

For supplied columns `B = [b_1 .. b_M]`, Householder QR with column pivoting
gives `Q` (n x k) and `C = Q^T B` (k x M) with `k` the numerical rank
(`|R_ii| > 1e-13 |R_11|`). The jet is built on the k columns of `Q`
instead of the M columns of `B`; each output `x_ij = sum_l z_i^l V_l c_j`
keeps its own coefficient column `c_j`. Acceptance is unchanged: the
directed current-target residual of the **original** `b_j`, so the
compression residual `e_j = b_j - Q c_j`, the non-orthogonality of the
computed `Q` and all rounding are inside the certificate. No column is
replaced by another or by a common RHS.

Typed abstention `HighRank` when `k (d+1) >= M (d+1)` would save nothing
(k = M); the uncompressed jet is then used.

## Fixtures (fixed now)

n in {8, 32}; M in {4, 16}; shifts: 9 targets with rho 0.2; sets:
rank-1 (`b_j = a_j q`), rank-2, full rank, near-dependent rank-2 plus
perturbations of size 1e-12 and 1e-6 relative, and unequal plus/minus
pairs (`b_+ = 3 q`, `b_- = -0.25 q`, evaluated at `z` and `-z` so the even
and odd jet parts are shared but the amplitudes differ). tol = 1e-10.

## Gate

G1 Every Certified candidate on n = 8 encloses the exact error (exact
   rational oracle on the exact binary inputs and the original `b_j`).
G2 No amplitude dropped: for the plus/minus set the two outputs differ and
   each is within its certified bound of its own exact solution; for every
   set the per-column outputs equal (to within their bounds) the
   uncompressed jet's outputs.
G3 Charged work: QR (`2 n M k - 2k^2 (n + M)/3`-type count incremented by
   the code), forming `C`, the jet on k columns, reconstruction and the
   residuals are all counted; the LU solve count drops from `M (d+1)` to
   `k (d+1)` exactly when `k < M`.
Prediction (reported): near-dependent 1e-6 sets have rank 3 (no saving
lost) or reject; full-rank sets abstain with `HighRank`.

## Results (append only after the recorded run)
