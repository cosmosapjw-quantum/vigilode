# E-05 Krylov stress — results
Status: **FAIL on the stale-reuse criterion; PASS on residual-reporting criteria.**
Pre-registered FAIL rule (PLAN Phase 2b): ratio reported/recomputed not in [0.5,2], or converged=true with true residual > threshold, or stale reuse.

## What was run
Predecessor run (main 96 rows + children) is complete and reused; the stale scenarios were re-run on a convergent matrix (`--stale-only`, s=1, cond 1e2) because the predecessor ran them on the s=10 matrix where no solver converges (uninformative). Binary built with the measurement profile; RAYON_NUM_THREADS=1.

## Key numbers
- 96 main rows, 42 converged: ratio reported/recomputed in [1.000000000000000, 1.000000000000001]; 0 rows converged=true with true residual over threshold.
- No false breakdown at ||b||=1e-14 (identical iteration counts across 1e-14/1/1e14 for s=0).
- s=10, s=100: every solver fails closed (4000-vector budget). s=1: 18/24 converge; gmres/jacobi and gmres_givens/jacobi succeed or fail depending only on the RHS scale (3200 / >4000 / 3440 iterations at 1e-14 / 1 / 1e14).
- Stale suite (s=1, cond 1e2): LGMRES fails closed in 3/6 PC-swap steps (budget exhausted; fresh solve converges in 1028 it). GCRO-DR reuses previous_solution across an operator change (352 vs 264 it, residual verified) and with a mutated same-token operator burns 4126 matvecs before failing closed.
- Child process: GCRO-DR state reused across a dimension change 32->16 **panics** at `crates/rodas5p-krylov/src/gcrodr.rs:463` (`copy_from_slice` length mismatch) -> abort. LGMRES survives (clears previous_solution on identity mismatch, lgmres.rs:69-78).

## Files
e05_raw.json (main rows, predecessor stale rows, children), e05_stale_raw.json (re-run stale suite), e05_matrices.json, stdout/stderr, results.json.
