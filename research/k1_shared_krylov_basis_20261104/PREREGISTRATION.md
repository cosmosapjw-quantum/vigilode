# K1: Shared Per-Step Krylov Basis vs Per-Stage GMRES
- written: 2026-11-04 (committed and pushed before the first run)
- question: Does reusing a single per-step polynomial Krylov basis across all 8 shifted RODAS5P stage solves reduce instruction work (Ir per solve) by at least 1.3x at matched achieved error compared to per-stage GMRES without losing to preconditioned per-stage GMRES?
- arms: `per_stage_gmres` (baseline MF RODAS5P), `per_stage_gmres_precond` (MF RODAS5P with existing diagonal preconditioner option), `shared_basis_x2` (MF RODAS5P with X2 SharedBasis opt-in); everything else identical
- cells: `heat_1d_n256` (n = 256), `brusselator_1d_n256` (n = 256), `heat_2d_n10000` (n = 10000), `brusselator_2d_n10000` (n = 10000), each at rtol in {1e-4, 1e-6, 1e-8}
- command: `python3 research/k1_shared_krylov_basis_20261104/eval_k1.py`
- inputs: `research/k1_shared_krylov_basis_20261104/PREREGISTRATION.md`, `research/k1_shared_krylov_basis_20261104/eval_k1.py`, `crates/rodas5p-krylov/src/shared_basis.rs`, `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs`
- metrics: deterministic JVP vector counts (`jvp_vectors`), accepted/rejected steps, and instruction cost per solve (`ir_per_solve` at matched achieved error); wall time diagnostic only (`"timing_authority": "diagnostic_only"`)
- PASS gate: `ir_per_solve` lower by >= 1.3x on >= 3 of 4 families against `per_stage_gmres`, and no family worse than `per_stage_gmres_precond`, with zero uncharged JVPs
- KILL gate: `ir_per_solve` not lower than `per_stage_gmres_precond` on any family, or any accounting gap (JVPs not charged)
- holdout: none
- analysis: `research/k1_shared_krylov_basis_20261104/eval_k1.py` evaluates all 12 cells (4 families x 3 tolerances) and computes family median ratios against `per_stage_gmres` and `per_stage_gmres_precond` without post-hoc threshold adjustment

## Results (appended after the run; nothing above this line changes)
- verdict: FAIL (opt-in retained; default remains per-stage GMRES)
- summary: SharedBasis reduces JVP vectors by 6.22x–6.29x vs unpreconditioned per-stage GMRES with zero uncharged JVPs, but per-stage Householder QR least-squares projection increases total Ir relative to preconditioned per-stage GMRES (median ratio 0.343–0.446 < 1.0).
