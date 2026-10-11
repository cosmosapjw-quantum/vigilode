# Preregistration: PEXPRB54S4 at its own step size with a rigorous defect certificate (EX01)

Node EX01 (P3, kind `research_port`) of `research/reaudit_accuracy_speed_20261010/NEXT_DEVELOPMENT_DAG.json`, from the
October 10 re-audit (`REVIEW_KO.md` F105; `MATHEMATICS_PORTING_KO.md`, section 8). It is the native counterpart of
pilot B3 (`docs/reviews/20261008_algorithmic_directions/pilot/phase3_reports/expo.md`, arm E2). Registered on branch
`audit/rvj-reaudit-remaining-20261011` at base `0ce7c32`. It depends on AS03 (merged).

**Claim boundary.**
- Scope: a declared semilinear, nonnormal, matrix-free niche: f = L(t) y + N(t, y), stiffness carried by L, and
  mu_2(L) + max diag(N') <= 0 certified in O(n) from the declared structure. Outside it the result is EstimateOnly or
  refused.
- DAG kill rule: Simpson quadrature that agrees with dense expm is not a machine certificate. A bound without a
  rigorous quadrature remainder and arithmetic enclosure is rejected.
- Pilot B3 numbers are replica attribution, never native evidence.
- Accuracy, certificate and counted work (Ir per trajectory, JVPs). No wall time; no default promotion.

## Question

Can PEXPRB54S4 run natively at its own step size and meet all of the following?
- A total phi-action certificate: a rigorous defect-integral bound with quadrature remainder and floating-point
  enclosure.
- Independent method and order gates.
- Lower complete work at matched accuracy than its strongest rivals, on a fresh corpus in the declared niche. The
  rivals are the improved matrix-free RODAS5P, the direct arm, and ROCK4.

## Stage 0: feasibility (what must exist before any trajectory study)

**F1. Rigorous defect bound (derivation plus oracle).**
- `docs/reviews/20261011_ex01/DEFECT_BOUND.md` (new) derives the bound. In the 2-norm, with mu_2(A) <= 0, it is
  ||e(tau)|| <= int_0^tau g(s) ds, where g(s) = |c(s)| ||q|| + ||U_flip a(s)|| (the pilot's KIOPS-augmented JAK form).
- Simpson is replaced by an outward upper Riemann sum, sum_k (s_{k+1} - s_k) sup_{[s_k, s_{k+1}]} g. The sup is
  enclosed by an interval evaluation of exp(s H_hat) over each subinterval, using `directed.rs` and the certified
  Taylor machinery of `taylor_phi_total.rs`. There is no smoothness assumption and no remainder term to estimate.
- The bound must also enclose:
  - the floating Arnoldi relation residual A V - V H - h v e^T, computed outward;
  - output assembly V y;
  - substep composition;
  - the O(n) structural mu_2 certificate, with outward rounding.
- Oracle: 50-digit `mpmath` actual errors on 120 fresh fixtures (symmetric, advection-diffusion, rotated 2x2 blocks;
  n = 96, 200; seeds fixed in `cases.json`). There must be 0 bound < actual, **including at the rounding floor**,
  where the pilot's bound had no rounding term.

**F2. Phi-engine parity, before any trajectory.**
- Native KIOPS-type multi-output engine (`exponential.rs`, opt-in), checked on the F1 fixtures against:
  - (a) the dense augmented expm reference: actual error <= 10x tol, the pilot's unit-check criterion;
  - (b) the pilot's `phase3_code/expo/kiops.py`, run unchanged: operator applications within [0.8, 1.25]x per
    fixture (design choice).
- A parity export script is added under `phase3_code/expo` (edit target).

**F3. Rivals exist natively.**
- `mf`: the minimum over {`B3`, `mf-legacy-v2`}. `B3` is the U-form matrix-free driver with the ALG04
  `CoupledGuarded2` target, the ALG06 B3 guard and SP01 v2 accounting, the counterpart of the pilot's improved MF arm;
  `mf-legacy-v2` is the `Legacy` target with SP01 v2 accounting. Both are run on every cell. In item 4 the `mf` Ir is
  the smaller of the two at each matched error; in item 3 the `mf` error is that of the arm with the smaller Ir at the
  cell's rtol.
- `direct`: `sparse_direct` (PC01's F1 kernel) if `crates/rodas5p-core/src/sparse_direct.rs` exists with its contract
  test at the commit of this node's `STAGE0.json`, otherwise the dense fast driver with `colext64`. The choice is
  recorded in `STAGE0.json` (`direct_rival`) before stage 1, and it is independent of PC01's outcome: no PC01
  verdict, stage-0 decision or result is consulted.
- `rock4`: ROCK4 with the 16-column field-of-values step bound, used where its guard admits.
- **No ROCK4 exists in this repository today** (no match in `crates/`).

**F4. Fresh niche corpus: `rotadv2d-N`.**
- Never run by pilot B3; the pilot used the corpus v2 calibration families.
- Equations: c_t = nu Lap c + K c - c^3 + g(x) on the periodic unit square, n = N^2.
- K = (D - D^T)/2 is the skew part of the centred conservative advection by the cellular flow psi = sin(2 pi x)
  sin(2 pi y) / (2 pi). So L + L^T = 2 nu Lap <= 0 exactly, and N' = -3 c^2 <= 0.
- Parameters (design choices): nu = 1e-3, g = sin(2 pi x) cos(2 pi y), c(0) = 0, t in [0, 2], N in {24, 48}
  (n = 576, 2304, both >= 384).
- References: the direct arm at rtol 1e-13 and 1e-12; the uncertainty u is their difference. Cells with error
  < 100 u are reference-limited.

**Stage 0 record.** `tools/ex01_stage0_check.py` (command below) writes a numeric `STAGE0.json`. It lists F1, F2, F3,
F4 and the reference prerequisite R (at most 5% of cells reference-limited), each as `{met, violations, evidence,
reviewer}`: `met` is whether the item is complete (for F1, the derivation), `violations` the count of failed oracle or
parity checks (0 for items without one), `evidence` the committed test or file, and `reviewer` the independent
reviewer of the derivation (F1) or `null`. It also records `direct_rival` (F3).

**Stage 0 outcome.** FEASIBLE, HOLD or FAIL, decided from `STAGE0.json` only:
- **FEASIBLE** iff every entry has `met = true` and `violations = 0`.
- **HOLD** iff any entry has `met = false` or `violations > 0`:
  - **HOLD(certificate):** the F1 derivation is incomplete, or any oracle fixture is not enclosed.
  - **HOLD(phi parity):** F2 fails.
  - **HOLD(rival):** a native ROCK4 with the FoV bound is not available within this audit. Replica numbers never
    substitute.
  - **HOLD(corpus):** F4 is unmet.
  - **HOLD(reference):** more than 5% of cells are reference-limited.
- **FAIL** iff any entry is missing from `STAGE0.json`.
- **Ledger.** FEASIBLE and HOLD are recorded as verdict PASS with claim "decision FEASIBLE" or "decision HOLD(...)",
  covering `STAGE0.json`; a missing entry is recorded as FAIL.
- `STAGE0.json` is INVALID if the checker blob at its source commit differs from the checker that was run, or if any
  recorded tree status is dirty.

## Stage 1: measurement (only if FEASIBLE)

**Cells.** `rotadv2d-N`, N in {24, 48}, on 9 half-decade rungs from 1e-4 to 1e-8, with h0 in {1e-4, 1e-3} (two fixed
seeds, not random samples). Endpoint error, recomputed from state bits; the arms have no common dense output.

**Arms.**
- `E2` (gated): PEXPRB54S4 at its own step size.
  - 4 multi-output 2-norm KIOPS calls per attempt.
  - EPUS phi tolerance (Theta 0.2, e_ref 0.5, p 1.2); Krylov cap clamp(ceil((2n + sqrt(4n^2 + 100 F_jvp))/50), 12,
    100).
  - Order-5 I controller with err = max(time, phi); certificate on every action.
- `E` (WRMS, EstimateOnly) and `Ed` (dense phi control): reported.
- Rivals: `mf` (both `B3` and `mf-legacy-v2`), `direct` and `rock4`, as in F3.

**Order ladder (item 1).** `rotadv2d-24` with dense phi, at the fixed step sizes h_k = h0 / 2^k, k = 0..4, with
h0 = 0.25: 8, 16, 32, 64 and 128 steps, which divide [0, 2] exactly. h0 is a design choice: the largest step that
resolves the cellular flow's unit time scale while keeping the k = 4 error well above the reference floor. The error
is the endpoint error against the reference. The observed order is the slope of the ordinary least-squares fit of
log(error) against log(h) over the 5 points.

**Units.**
- Phi error bound / actual error per action.
- Complete work: same-binary Ir per trajectory with 2-minus-1 repetitions, including the certificate.
- JVPs and orthogonalization counters.
- Reference uncertainty.

## Commands

    cargo test --offline --locked --release -p rodas5p-integrators --test pexprb_defect_certificate_contract
    PYTHONDONTWRITEBYTECODE=1 python3 tools/ex01_stage0_check.py --derivation docs/reviews/20261011_ex01/DEFECT_BOUND.md --cases research/ex01_pexprb_own_step_20261011/cases.json --output research/ex01_pexprb_own_step_20261011/STAGE0.json
    # stage 1 only if STAGE0.json is FEASIBLE:
    EX01_RUNS=research/ex01_pexprb_own_step_20261011/RUNS.json RAYON_NUM_THREADS=1 cargo test --offline --locked --release -p rodas5p-integrators --test pexprb_own_step_holdout -- --ignored --nocapture --test-threads=1 export_runs
    python3 tools/ex01_profile.py --output research/ex01_pexprb_own_step_20261011/PROFILE.json
    python3 tools/ex01_pexprb_check.py --cases research/ex01_pexprb_own_step_20261011/cases.json --runs research/ex01_pexprb_own_step_20261011/RUNS.json --profile research/ex01_pexprb_own_step_20261011/PROFILE.json --output research/ex01_pexprb_own_step_20261011/RESULTS.json

The checker calls `evidence_schema_v2` first (new kind `ex01`). INVALID exits 2, FAIL exits 1. `cases.json` and the
checker are committed before the recorded run.

## Gate (stage 1)

**Validity.** INVALID if any of the following holds:
- the AS03 rules fail, or a record does not match the `ex01` schema;
- the (N, rung, h0, arm) row set or the order-ladder set differs from the registration, or a row is duplicated;
- a recomputed quantity differs from the exported one: an endpoint error from the state bits, a 50-digit action
  error, a work total from its components, or a profiled run from its RUNS record;
- the `direct` rival differs from `STAGE0.json`'s `direct_rival`;
- the checker blob at the RUNS source commit differs from the checker that was run, or any recorded tree status is
  dirty.

**PASS** if all of the following hold.

1. **Method and order.** On the registered order ladder of `rotadv2d-24` with dense phi, the observed global order
   (OLS slope over the 5 points) is in [4.5, 5.5]. Native PEXPRB54S4 gave 5.0 in L-0040.
2. **Total certificate.**
   - The 50-digit dense reference is computed on a deterministic sample, at n = 576 only: in each trajectory, the
     attempts are split into ten deciles by attempt index, and the first and the last `E2` action of each decile
     are sampled.
   - On every sampled action, bound >= 50-digit actual error: 0 violations, rounding floor included.
   - Over the sample, median bound/actual <= 10 and maximum <= 1e4. The maximum is the pilot's criterion; the median
     is a design choice.
3. **No undetected error.** `E2` error / `mf` error at equal rtol is <= 3 on every cell, the pilot's undetected-error
   criterion. There is no lower bound: being more accurate at equal rtol is not penalized, and the work gate below is
   scored at matched error anyway.
4. **Complete work.** Frontier and cheapest-run Ir ratios at the 1e-6 and 1e-8 matched errors, at n = 2304:
   - `E2/mf` <= 0.80 (pilot kill line);
   - `E2/rock4` <= 1.00 on every cell where ROCK4's guard admits. This needs at least one cell where the guard admits
     ROCK4; with none, the `E2/rock4` comparison is reported as vacuous, is not PASS evidence, and item 4 is not met;
   - `E2/direct` is reported. It is in scope only as "matrix-free, no declared pattern"; where a pattern is declared,
     direct is the stated choice.

Everything else is **FAIL**, with every number preserved.

## Prior information (disclosed)

**Pilot B3** (Python replica, flop models).
- E/mf ratios: rotating 0.47-0.71, semilinear 0.20-0.77, forcing 0.56-0.62.
- Certified E2 against mf: about 0.63-0.85 on rotating at n >= 384.
- E/direct: 1.8-14. E/ROCK4: 1.3-2.4 on rotating.
- JAK bound/actual: 1.0-2.2 with mu_2 <= 0. Raw violations occurred at the rounding floor.

**Native state.** `exponential.rs` has PEXPRB54S4 with fused Krylov phi (`PhiConvergenceBasis::ResidualEstimate`,
"not a bound for nonnormal operators"). There is no defect bound in `crates/`.

## Predictions

- **Stage 0: HOLD (most likely).**
  - The main reason is HOLD(rival): no native ROCK4.
  - F1 is feasible but heavy, because the interval exp enclosure and the Arnoldi-relation term are both needed.
  - F2 likely passes.
- **If stage 1 runs:**
  - Item 2 should hold, at a median bound/actual of a few.
  - Item 4 vs `mf` is borderline: certified 0.63-0.85 against a 0.80 bound.
  - Item 4 vs `rock4` is likely FAIL, if the guard admits ROCK4 on this flow (pilot E/ROCK4 1.3-2.4).
- **Overall: HOLD now, FAIL more likely than PASS.**

## Results

Appended after the recorded run. Nothing above this heading changes.
