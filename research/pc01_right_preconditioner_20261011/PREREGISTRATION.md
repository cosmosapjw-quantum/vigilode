# Preregistration: declared linear-part right preconditioner on a fresh 2-D multi-species problem (PC01)

Node PC01 (P3, kind `research_port`) of `research/reaudit_accuracy_speed_20261010/NEXT_DEVELOPMENT_DAG.json`, from the
October 10 re-audit (`MATHEMATICS_PORTING_KO.md`, section 8). It is the native counterpart of pilot B4
(`docs/reviews/20261008_algorithmic_directions/pilot/phase3_reports/pclag.md`, arm (d)). Registered on branch
`audit/rvj-reaudit-remaining-20261011` at base `0ce7c32`.

Dependencies:
- SP03 (executed; L-0101 FAIL on item 5 only). Its validated `ProblemStructure` declaration path exists and is
  extended here.
- AS03 (merged).

**Claim boundary.**
- Accuracy and counted work (factorizations, applications, true residuals, same-binary Ir per trajectory) on one
  declared problem family. No wall time; the timing authority is on HOLD.
- DAG kill rule: the pilot's 467 runs and its kernel timings are not solver speed. No extrapolation to unstructured
  problems.
- Production preconditioning is left-only. The right path must keep acceptance on the true unpreconditioned residual,
  in WRMS units. Shifted-space reuse under an arbitrary preconditioner is refused, not qualified away.
- No default promotion, and no change to any existing caller's results.

## Question

Can a right preconditioner P = I - h_P gamma A do the following?
- P is built only from the declared linear part A, and factored by a sparse direct LU once per bounded h-window
  rho = h/h_P in [1/2, 2].
- It integrates a fresh 2-D multi-species problem natively, at matched accuracy.
- Its complete counted work beats a strong sparse direct RODAS5P comparator.

## Stage 0: feasibility (what must exist; each item is a committed contract test)

- **F1. Sparse direct kernel.**
  - `faer 0.24.4` is already a workspace dependency with the `sparse-linalg` feature (`Cargo.toml`), and it provides
    `faer::sparse::linalg::lu` (symbolic and numeric LU, COLAMD ordering). No crate in the repository uses it today:
    the only hit is a comment in `routing.rs` ("not a general sparse LU").
  - `crates/rodas5p-core/src/sparse_direct.rs` (new) wraps it with:
    - reuse of the symbolic factorization;
    - numeric refactorization;
    - solve;
    - counted factor and solve work (nnz(L+U), operations).
  - Contract: on 200 seeded sparse matrices (n = 50-5000) and on Brusselator-1D W:
    - backward error ||b - A x|| / (||A|| ||x|| + ||b||) <= 1e-13 (design choice);
    - agreement with the banded LU <= 1e-12 relative.
  - No new dependency and no `Cargo.lock` change are allowed.
- **F2. Right path, true residual.**
  - `operator.rs::Preconditioner` gains a side (`Left` is the default and is unchanged). `gmres_into.rs` solves
    W P^-1 u = b and returns x = P^-1 u.
  - Acceptance recomputes b - W x on the unpreconditioned operator with the caller's `residual_scale`.
  - Every existing left-preconditioned caller is bitwise unchanged; SP01's 44 cells, and the SP02 and SP04 cells (see
    "Coordination with sibling nodes"), are the identity test.
  - With a right preconditioner active, GCRO-DR recycling, shared-shift and Hessenberg-shift reuses return a typed
    refusal.
- **F3. Declared linear part.**
  - SP03's `ProblemStructure` gains `LinearPart { sparse A, nonlinear callback N }`.
  - At (t0, y0) and one seeded perturbation, f(t, y) = A y + N(t, y) is checked to a relative 1e-12, using SP03's
    scaled norms (`safe_l2`). The verification work is charged.
  - A mismatch is a typed refusal before any step.
- **F4. Fresh native problem `gray-scott-2d-N`.**
  - Never used by pilot B4 (`bruss2d`, `semilin2d`) or by the corpus.
  - Equations: u_t = D_u Lap u - u v^2 + F (1 - u), and v_t = D_v Lap v + u v^2 - (F + k) v.
  - Grid: periodic unit square, 5-point Laplacian, n = 2 N^2.
  - Linear part: A = blockdiag(D_u Lap - F I, D_v Lap - (F + k) I), with the constant F in N.
  - Parameters (design choices fixed now):
    - D_u = 2e-3, D_v = 1e-3, F = 0.04, k = 0.06, on t in [0, 20];
    - u = 1 and v = 0, plus the deterministic bump v += 0.25 exp(-100 |x - (0.5, 0.5)|^2) and u -= that bump.
  - Analytic sparse Jacobian.
  - References: the sparse direct arm at rtol = atol = 1e-13 and 1e-12. The uncertainty u is their difference.

**Calibration problem (not gated).** A native port of the pilot's `bruss2d` (Dirichlet, alpha = 0.02) is used only to
develop F1-F3.

**Stage 0 record.** `tools/pc01_stage0_check.py` (command below) writes a numeric `STAGE0.json`. It lists the
prerequisites F1, F2, F3, F4 and the reference prerequisite R (at most 5% of the gated cells reference-limited, error
< 100 u), each as `{met, evidence}`: `met` is a boolean, and `evidence` names the committed contract test and its
recorded outcome (for R, the reference-limited count and the gated-cell count).

**Stage 0 outcome.** FEASIBLE, HOLD or FAIL, decided from `STAGE0.json` only:
- **FEASIBLE** iff every entry has `met = true`.
- **HOLD** iff any entry has `met = false`, named by the unmet prerequisite:
  - **HOLD(no sparse kernel):** F1 fails or needs a new dependency. No hand-written sparse LU in this audit.
  - **HOLD(residual contract):** F2 cannot keep the left path bitwise unchanged, or cannot keep acceptance on the true
    residual.
  - **HOLD(linear part)** or **HOLD(problem):** F3 or F4 is unmet.
  - **HOLD(reference):** R is unmet.
- **FAIL** iff any entry is missing from `STAGE0.json`.
- **Ledger.** FEASIBLE and HOLD are recorded as verdict PASS with claim "decision FEASIBLE" or "decision HOLD(...)",
  covering `STAGE0.json`; a missing entry is recorded as FAIL.
- `STAGE0.json` is INVALID if the checker blob at its source commit differs from the checker that was run, or if any
  recorded tree status is dirty.

## Stage 1: measurement

**Cells.**
- Gated: `gray-scott-2d-N` with N in {128, 256} (n = 32768, 131072) at rtol in {1e-4, 1e-6, 1e-8}, atol = rtol.
- Reported: N = 64.
- Callgrind cost: Ir at N = 256 is gated at rtol 1e-6 only. Its other rungs are reported (counted work), not scored.

**Arms.**
- `direct-sparse`: the strong comparator.
  - Fast-driver policies: Jacobian reuse after rejection, symbolic factorization reused across the run, and numeric
    refactorization per new W.
  - Analytic sparse J.
- `rpc-lin`, the gated arm:
  - right-preconditioned restarted GMRES(40), zero start, maxit 2000, with SP01 v2 accounting;
  - refresh when rho leaves [1/2, 2] (pilot rule);
  - the stage target is production's.
- Reported arms:
  - `rpc-lin-every` (refactor P every attempt);
  - `mf-v2` (no preconditioner);
  - `banded` (l = u = 2N), at N <= 128 only.

**Units (separate).**
- Factorizations, numeric refactorizations, P applications, JVPs and true-residual checks.
- Ir per trajectory: callgrind 2-minus-1, determinism repeat, line-tables release.
- The true residual / target of every accepted stage solve.

## Commands

    # stage 0
    PYTHONDONTWRITEBYTECODE=1 python3 tools/pc01_stage0_check.py --contracts research/pc01_right_preconditioner_20261011/STAGE0_CONTRACTS.json --output research/pc01_right_preconditioner_20261011/STAGE0.json
    # stage 1 only if STAGE0.json is FEASIBLE:
    PC01_RUNS=research/pc01_right_preconditioner_20261011/RUNS.json RAYON_NUM_THREADS=1 cargo test --offline --locked --release -p rodas5p-integrators --test right_preconditioner_holdout -- --ignored --nocapture --test-threads=1 export_runs
    cargo test --offline --locked -p rodas5p-core --test sparse_direct_contract
    cargo test --offline --locked -p rodas5p-integrators --test right_preconditioner_holdout
    python3 tools/pc01_right_pc_profile.py --output research/pc01_right_preconditioner_20261011/PROFILE.json
    python3 tools/pc01_right_pc_check.py --runs research/pc01_right_preconditioner_20261011/RUNS.json --profile research/pc01_right_preconditioner_20261011/PROFILE.json --output research/pc01_right_preconditioner_20261011/RESULTS.json

The checker calls `evidence_schema_v2` first (new kind `pc01`). INVALID exits 2, FAIL exits 1. The checker is
committed before the recorded run. It recomputes every error from state bits.

**Pre-declared budget rule.** If the N = 256 callgrind run does not finish within 12 h of wall time, that cell is
MISSING and the node verdict is HOLD, not PASS. In the ledger this is recorded as INCONCLUSIVE ("MISSING cell").

## Gate (stage 1)

**Validity.** INVALID if any of the following holds:
- the AS03 rules fail, or a record does not match the `pc01` schema;
- the (N, rtol, arm) row set differs from the registered cells and arms, or a row is duplicated (a cell MISSING under
  the budget rule is handled by that rule, not here);
- a recomputed quantity differs from the exported one: an error from the state bits, a true residual, a work total
  from its components, or a profiled run from its RUNS record;
- a callgrind determinism repeat is not identical;
- the checker blob at the RUNS source commit differs from the checker that was run, or any recorded tree status is
  dirty.

**PASS** if all of the following hold.

1. **Accuracy.**
   - On every gated cell, `rpc-lin` error / `direct-sparse` error at equal rtol is in [0.5, 2.0]. (Pilot: 0.83-1.15 at
     1e-4, and 0.986-1.011 at 1e-8 and 1e-10.)
   - 0 linear failures and 0 stall acceptances.
2. **True residual.** Every accepted stage solve has a recomputed unpreconditioned residual at or below its target
   (100%, outward checked).
3. **Work.** Matched-accuracy Ir ratio `rpc-lin` / `direct-sparse` is <= 0.90 at N = 128 (frontier rule, all three
   rtols) and <= 0.70 at N = 256 (1e-6). These thresholds are the pilot's proposed gate. The pilot measured
   0.64 / 0.33 in flops and 0.88 / 0.60 in timed kernels.
4. **Accounting.**
   - Factor refreshes, P applications and verification work are all charged.
   - Columns per stage at N = 256 are no more than 1.5x those at N = 128 (pilot: N-independent, 5.4-5.8).

Everything else is **FAIL**, with every number preserved. A FAIL is not re-scored.

## Coordination with sibling nodes

AS04, SP02, SP04 and PC01 all edit `crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs`. Their changes are
merged in the fixed order AS04 -> SP02 -> SP04 -> PC01, each as orthogonal opt-in fields, default off, with no new
entry point per node where a field on the existing options or workspace suffices. PC01 merges last, so its parity
tests include the earlier nodes' options-off identity (AS04 telemetry off, SP02 at `krylov-ws-legacy-v1`, SP04 at
`Configured` / `Carried`). PC01's `gmres_into` identity tests (F2) include the SP02 and SP04 cells, besides SP01's 44
cells, so the right-preconditioner side leaves the GMRES-into paths of both nodes bitwise unchanged.

## Prior information (disclosed)

- Pilot B4 (Python replica, SuperLU with MMD, own 2-D Brusselator).
  - Arm (d): 0.64x direct at N = 128 and 0.33x at N = 256 in flops; 0.88x and 0.60x in timed kernels.
  - 0 failures in 467 runs.
  - It loses on scalar semilinear up to N = 256.
- Threats the pilot listed:
  - flops favour the preconditioned arms;
  - a better ordering helps direct most.
- COLAMD in faer differs from MMD.
- No native code of this node exists.

## Predictions

- **Stage 0: FEASIBLE (moderate confidence).** F1 needs no new dependency. F2 is the riskiest item: it touches the
  shared GMRES kernel, and the left path must stay bitwise identical.
- **Stage 1: likely FAIL on item 3 at N = 128.**
  - Ir sits between the pilot's flop and time models, about 0.85-1.0.
  - N = 256 is borderline at about 0.6-0.8.
  - Gray-Scott's A has no species coupling, like the pilot's winning `lin` arm, which helps.
- Accuracy items 1, 2 and 4: expected to hold.
- Overall: FAIL or HOLD more likely than PASS (about 30% PASS).

## Results

Appended after the recorded run. Nothing above this heading changes.
