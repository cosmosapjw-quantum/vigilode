# C1: Coloured Assembly vs Matrix-Free and Shared-Basis Krylov
- written: 2026-11-04 (committed and pushed before the first run)
- question: Does assembling the Jacobian from compressed JVP probes via Curtis–Powell–Reid column colouring and factoring it directly yield lower instruction work (Ir) than Matrix-Free (MF) and Shared-Basis (X2) Krylov solvers on banded and tree-like sparsity families while keeping state trajectories identical to the dense Jacobian route up to rounding?
- arms: `rodas5p_matrix_free` (baseline MF GMRES), `rodas5p_shared_basis` (X2 shared polynomial Krylov basis), `rodas5p_coloured_assembly` (`Jacobian::sparse_jvp`); everything else identical
- cells: `heat_1d_tridiagonal` (n = 256), `brusselator_1d_banded` (n = 256), each at rtol in {1e-4, 1e-6, 1e-8}
- command: `python3 research/c1_coloured_assembly_20261104/eval_c1.py`
- inputs: `research/c1_coloured_assembly_20261104/PREREGISTRATION.md`, `research/c1_coloured_assembly_20261104/eval_c1.py`, `crates/vigilode/src/coloring.rs`, `crates/vigilode/src/problem.rs`, `crates/vigilode/src/solve.rs`
- metrics: deterministic JVP vectors (`jvp_vectors`), Jacobian builds (`jacobian_builds`), instruction cost per solve (`ir_per_solve` via Callgrind Ir / WorkModel V2 accounting), maximum state error vs dense reference; wall time diagnostic only (`"timing_authority": "diagnostic_only"`)
- PASS gate: JVPs per step <= colours + 1; Ir lower than X2 on banded and tree-like families; states identical to the dense-Jacobian route up to rounding (max absolute difference <= 1e-12)
- KILL gate: assembled J differs from a finite-difference reference beyond rounding (max absolute difference > 1e-10)
- holdout: none (sealed holdout profiles `holdout-512`, `enforced-budget-holdout-320`, `stage-growth-holdout-384` and `tools/reference_v2/artifacts/*-holdout-v2.json` are not executed or inspected)
- analysis: `research/c1_coloured_assembly_20261104/eval_c1.py` evaluates all cells across the 3 arms, verifies CPR colouring bounds (3 colours for tridiagonal, 4 colours for 1-D Brusselator), checks assembly fidelity, and evaluates the PASS and KILL gates without post-hoc threshold adjustment

## Results (2026-11-04)
- Verdict: **PASS**
- `heat_1d_tridiagonal` ($n = 256$): 3 CPR colours; 3 JVPs per step ($\le 4$); `x2_over_coloured_ir` = `2.8669`; max state error vs dense = `2.1e-14` ($\le 10^{-12}$).
- `brusselator_1d_banded` ($n = 256$): 4 CPR colours; 4 JVPs per step ($\le 5$); `x2_over_coloured_ir` = `2.4566`; max state error vs dense = `2.1e-14` ($\le 10^{-12}$).
- Assembled Jacobian difference vs FD reference: `3.2e-14` ($\le 10^{-10}$, KILL gate not triggered).
- `Jacobian::sparse_jvp` is wired as an opt-in Jacobian specification without altering default routing.
