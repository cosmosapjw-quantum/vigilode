# Preregistration: allocation-free small least squares for the Krylov `into` paths (speed research node SPD04)

Second speed research cycle (2026-10-07), branch `audit/rvj-speed-research-20261005`, base `bdcc903`. Candidate
KRY-LS-NOALLOC of the first cycle's DAG (`docs/reviews/20261005_speed_research/SPEED_RESEARCH_STATUS.md`; both
refuters kept it, their corrections are applied below). **Counted allocator events only; no instruction count (no
Krylov arm exists in `stiff-profile-run`) and no wall-time claim; the timing authority stays on HOLD.**

## Question

After L-0045 (`solve_gmres_into`), the per-cycle allocations of GMRES-into are the small least-squares solve:
`rodas5p_krylov::small::least_squares` builds a fresh faer column-pivoted QR and allocates 11 times per call
(`small.rs:4-26`: `to_faer`, the right-hand-side `Mat`, `ColPivQr::new` with its owned copy, two permutation
vectors, `Q_coeff`, the scratch buffer and the second triangle, `solve_lstsq`'s output and scratch, the output
`Vec`). The recorded `rnext02` families show 11.0 allocations per cycle on Brusselator-50, van der Pol, quadratic-4
and Prothero-Robinson, and the matrix-free U-form driver with `gmres_into` keeps 89.9-93.5 allocations per attempt
(8 stage solves x about 1 cycle x 11). Does a workspace-owned least-squares solver that calls the same faer kernels
with the same parameters give bitwise the same solutions with none of these allocations?

## Change (opt-in; `small::least_squares` and every existing solve unchanged)

- `rodas5p-krylov/src/small.rs`: `LeastSquaresWorkspace` with `solve_into(&mut self, a: &DenseMatrix, b: &[f64],
  out: &mut Vec<f64>) -> CoreResult<()>`. It copies `a` into a reused faer `Mat` resized to exactly the current
  `(m + 1) x m`, factors in place with `faer::linalg::qr::col_pivoting::factor::qr_in_place` using the parameters
  `ColPivQr::new` uses (`get_global_parallelism()`, `recommended_block_size(m, n)` recomputed per call, default
  params), forms the unit-lower and upper triangles as faer's private `split_LU` does (zeroed before each use, since
  reused buffers carry stale values), and solves with `solve_lstsq_in_place_with_conj` into a reused right-hand
  side. Same non-finite check and error as `least_squares`. Buffers grow only when `m` exceeds the previous maximum;
  growth is reported.
- `rodas5p-krylov`: `GmresWorkspace` gains an opt-in `LeastSquaresWorkspace` (`GmresWorkspace::with_ls_workspace()`
  or a setter; default none). `solve_gmres_into` uses it when present and `small::least_squares` otherwise.
- `rodas5p-integrators/src/rodas5p_matrix_free_fast.rs`: `Rodas5pMfFastWorkspace::set_ls_workspace(bool)` (default
  false), effective only together with `gmres_into`.
- GCRO-DR and LGMRES are out of scope (LGMRES's per-column call is node SPD05's subject).

## Systems

1. Contract test (`crates/rodas5p-krylov/tests/spd04_ls_workspace.rs`, not ignored): `solve_into` against
   `small::least_squares` bit for bit on at least 1,000 seeded `(m + 1) x m` upper-Hessenberg systems, `m = 1..64`,
   including rank-deficient and near-breakdown columns, and on reuse sequences in one workspace (growing and shrinking
   `m`, `m + 1` in {8, 16, 24, 32, 40}, a solve with `m` smaller than an earlier one); same error on non-finite input.
2. The 56 `rnext02` frozen families (408 solves; restart 40 and 10) with the control (`solve_gmres_into` as of the
   base) and the candidate (with the workspace), plus the legacy allocating solve as the bitwise reference.
3. The six L-0038 driver problems with the matrix-free U-form driver and `gmres_into` on, control and candidate.

New harness `crates/rodas5p-integrators/tests/spd04_ls_workspace_study.rs` reusing `rnext_common`; the frozen
`rnext02_gmres_into_study.rs` (an input of L-0045) is not edited.

## Commands

    cargo test --release -p rodas5p-krylov --locked --test spd04_ls_workspace
    SPD04_OUTPUT=research/spd04_ls_workspace_20261007/RESULTS_RAW.json cargo test --release -p rodas5p-integrators --locked --test spd04_ls_workspace_study -- --ignored --nocapture --test-threads=1
    python3 tools/spd04_ls_check.py --raw research/spd04_ls_workspace_20261007/RESULTS_RAW.json --output research/spd04_ls_workspace_20261007/RESULTS.json

## Gate

**PASS** if all hold:

1. **Contract.** `solve_into` equals `small::least_squares` bit for bit (solution or error text) on every contract
   system and reuse sequence.
2. **Identity.** On all 408 solves the candidate equals the legacy solve and the control bitwise (solution, residual
   norm, iterations, matvecs, `WorkCounters`, carried state), and on the six driver runs the candidate equals the
   control bitwise (output states, attempts, counters).
3. **Exact allocation removal.** For every family, candidate allocations = control allocations - 11 x
   (least-squares solves), except allocations of workspace growth (reported per family with
   `GmresIntoReport::workspace_grew` and the least-squares workspace's growth flag).
4. **Driver.** Matrix-free driver allocations per attempt <= 0.10 x control on the five non-HIRES L-0038 problems
   (predicted: control - 11 x cycles per attempt, about 2-6 per attempt; HIRES keeps its test JVP's per-application
   allocation and is reported).

Otherwise **FAIL**. Kill: any bit difference on any solve or run (faer's kernels were not called with the high-level
path's parameters); the failing family is recorded.

## Prior information

L-0045 (GMRES-into, 0.025-0.24x allocations per solve), L-0061 (LGMRES-into, 0.94-0.99x). No code of this node
exists before this commit.
