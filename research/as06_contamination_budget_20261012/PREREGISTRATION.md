# Preregistration: causal stage chain and committed contamination budget (AS06)

Node AS06 of `research/reaudit_accuracy_speed_20261010/NEXT_DEVELOPMENT_DAG.json` (finding F103 and the AS06 section
of `docs/reviews/20261010_accuracy_speed/REVIEW_KO.md`; MATHEMATICS_PORTING_KO.md sections 2, 4.1-4.3 and 5). Branch
`audit/rvj-reaudit-remaining-20261011`, registered on top of `0692791`. Dependencies:
- **AS04, L-0110, FAIL at item 5 only.** Telemetry on/off bitwise, no hidden JVP, every accepted residual above its
  target labelled `BudgetExceeded`. Item 5 failed because the two registered debits conflict: on 8 of 784 attempts of
  Robertson-4e10 1e-9 `B3`, fallback-accepted stages meet the requested target below the nu-tightened threshold, so
  `debit_all` (non-`Met` stages) < `debit_fallback_only` (G3). AS06 uses the records as accounting input only.
- **AS05, L-0107, PASS.** `rodas5p_core::residual_gain` is `Verified` only for represented 2x2, isolated 1x1/2x2
  blocks, shifted `I - h gamma J` with declared entrywise assembly, and the log-norm route at n <= 8. Usefulness is
  on HOLD (setup 9-13x nominal LU at n = 2).
- AS03's validator `tools/evidence_schema_v2.py` (SHA-256 `1d2bfe92...`) is used by the checker.

**Claim boundary.**
- A `Verified` row bounds the **contamination** of one committed step: the distance between the candidate step and the
  exact-real step of the represented U-form tableau from the same start state (target 2 of MATHEMATICS_PORTING_KO.md
  section 2). It is not a local truncation, global ODE error or order statement. A local linear witness alone cannot
  establish global fifth order (DAG kill rule).
- The sampled `rodas5p_stage_transfer_constants()` (1,501 imaginary-axis points) stays an explicitly **heuristic**
  diagnostic. No tau value enters a bound.
- Outside AS05's `Verified` scope the per-stage `G_i` is `Unavailable`. The step row, its window and the run budget
  then report `Unavailable`, never a number. No generic matrix-free claim.
- Opt-in under the non-default feature `audit2-stage-certificate`. No decision reads the ledger; no default promotion.
  No wall-time claim: counts are counted operations.
- If the verified bounds are too loose to save work, the ledger is kept as a certificate/diagnostic and every speed
  follow-up stops (DAG kill rule). Published ALG04/ALG06/AS04/AS05 results are not re-scored and their campaigns are
  not rerun.

## Question

For the fast U-form driver with a frozen metric and a declared model, does the causal recurrence

    q_i = up{ c_i + G_i xi_i + sum_{j<i} [ min(G_i |h gamma| L_i, G_i + 1 + G_i |h gamma| Lambda_i) |a_ij|
                                          + G_i |gamma C_ij| ] q_j }

bound every stage error `||U_hat_i - U*_i||`, so that output, embedded estimate and committed steps can be debited
against an immutable per-window budget, with every floor, stall and fallback acceptance charged and reconciled with
AS04's records? Here:
- `c_i <= G_i rho_i` is AS05's `VerifiedCorrection` of `W u = b_hat_i` for the floating right-hand side the solver used;
- `xi_i` encloses the assembly/evaluation error `||b_hat_i - b_i(U_hat)||`, including a callback `f_t` against the
  declared `f_t`;
- `L_i` is the causal-tube Lipschitz bound of `f`, and `Lambda_i` the tube bound of `||f'(z) - J||` for the target `J`.

The first branch of the min is the DAG/review form. The second is the exact-linear transfer: because
`W^-1 (h gamma J) = W^-1 - I`, the linear part costs `G_i + 1` and only the remainder `Lambda_i` carries `|h gamma|`.

## Change

1. **`outward_certificate.rs`** (additive, `#[cfg(feature = "audit2-stage-certificate")]`).
   - `certify_u_form_step` returns `StepContamination {Verified(StepBound), Unavailable(ContaminationRejection)}`.
     `StepBound` has private fields and no `Deserialize`.
   - **Target.** Binds `y`, `t`, `h`, `gamma`, the U-form coefficient snapshot digest (`a`, `C`, `gamma_i`, `b_code`
     bits read as exact rationals), the declared model, the frozen stage metric `s = atol + rtol |y|`, the model epoch
     and the ledger's `T`.
   - **Exact-linear transfer first.** One AS05 witness per attempt on the `Shifted {J, h, gamma}` target. Routes
     (a)-(c) give `G` = `residual_gain_inf_upper` and use the weighted infinity norm; route (d) gives
     `G = 1 / denominator_lower` and uses WRMS. One norm is used per attempt, and a weighted-infinity bound is also
     reported as a WRMS bound.
   - **Nonlinear tube remainder, bounded separately.** The causal box of stage `i` is `y + sum_j a_ij U_hat_j`, widened
     by `sum_j |a_ij| q_j` (already proved, so it contains the exact argument by induction). `Lambda_i` and `L_i` are
     enclosed outward over that box, which must lie inside the model's declared domain (`TubeOutsideDomain`
     otherwise).
   - **Transport.** Output: `E_y = sum_j |b_j| q_j + ||y_hat - enclosure(y + sum_j b_j U_hat_j)||`. Embedded: the
     driver's error vector is `U_hat_(s-1)`; the exact step's embedded norm lies in `[N_hat - D_e, N_hat + D_e]`,
     `D_e = kappa q_(s-1) + rounding(N_hat)`, `kappa = up(max_k s_stage,k / s_err,k)`,
     `s_err = atol + rtol max(|y|, |y_hat|)`; an interval containing 1 is flagged `DecisionUnverified`. Debit
     `d_n = max(kappa E_y, D_e)` in the error metric of step n.
   - **Global amplification (exact-linear runs only).** `E_(n+1) <= kappa_n S_n E_n + d_n`, with
     `S_n <= 1 + sum_j |b_j| p_j`, `p_i = (G + 1)(1 + sum_j |a_ij| p_j) + G sum_j |gamma C_ij| p_j`, and `kappa_n`
     moving metric n to metric n+1 outward. Nonlinear runs report `Unavailable(GlobalNonlinear)`.
2. **`rodas5p_matrix_free_fast.rs`** (feature-gated, opt-in).
   - `ContaminationLedger::new(T, windows = 8)` fixes `T = t_end - t_0` bits at construction and has no setter. Any
     attempt whose `stage_span` bits differ gets `Unavailable(SpanMismatch)`.
   - New entry point `integrate_rodas5p_mf_fast_observed_with_contamination_ledger`, which also returns AS04's
     `StageResidualTelemetry`. Only committed attempts are debited. Rejected and failed attempts go to a work ledger.
   - **One coherent debit (resolves L-0110 item 5).** The ledger debits `d_n` per committed attempt from all stages
     of every acceptance reason; it does not depend on `Met` or the reason, so the conflict cannot arise. Its
     heuristic column `heuristic_debit = sum over all accepted stages of tau_e,i ||r_i||_WRMS` is a superset sum in
     stage order, hence `>=` both `debit_all` and `debit_fallback_only` in binary64 (rounding is monotone). G3 stays
     a heuristic charge; AS06 changes neither G3 nor any decision.
   - Stage inputs (`b_hat_i`, `U_hat_i`, `s`) are copied only with the ledger on. Their copies are counted in
     `ledger_vector_copies`, outside WorkCounters.
3. **`transactional_q1_q2.rs`.** Additive only: an accessor so that a `QuadraticModel` (`f(z) = A z + q z^2`,
   componentwise) can be declared to the certificate. Its `ModelBinding` rule is reused: a sampled-only binding gives
   `Unavailable(SampledBindingOnly)`.
4. **Tests and checker.** `tests/contamination_budget_contract.rs` (contracts plus the ignored `export_runs`), with a
   `[[test]]` entry `required-features = ["audit2-stage-certificate"]` in the crate's `Cargo.toml` (disclosed
   deviation: not a DAG edit target). `rodas5p-core` is not edited (`residual_gain.rs` pinned at `c8d08e64...`).

## Cases

Common settings: the coefficient snapshot of `rodas5p_coefficients()`; autonomous models with declared `f_t = 0`;
metric `s = 1e-8 + 1e-6 |y|` unless stated. Single-attempt candidates (X, Y) are `U_hat_i = fl(solve of b_hat_i) + p_i`
with `p_i` of weighted size `2^-20` (design choice) along seeded signs (seed 20261012), `e2`, or zero.

**X: exact-linear adversaries** (`f = A y`, `J = A`, `Lambda = 0`), each at `h gamma in {1e-3, 1e-1}`, `y = ones`:
X1 `A = diag(-1, -1e4)`; X2 (F105 family) `A = [[-10, K], [0, -10]]`, `K in {0, 1024, 2^20}`; X3 isolated blocks
n = 4 and 8 (copies of X2, `K = 1024`); X4 route (d), `A = -100 tridiag(-1, 2, -1)`, n = 8, metric `1 + |sin k|`
(Verified in AS05). **Missed embedded-error direction:** X2/X3 with direction `e2` and `K >= 1024`; a member
*discriminates* iff its exact embedded contamination exceeds the attempt's `heuristic_debit` (Fraction arithmetic only).

**Y: nonlinear-tube adversaries.** Y1 = X2 (`K = 1024`) and Y2 = X4, each with `q in {-1, -100}` and
`y_k = 1 + 0.1 sin k`; Y3 = Y1 with a stale `J` evaluated at `1.01 y` (W-method mismatch, carried by `Lambda`). The
represented `J = fl(A + 2 diag(q y))` puts its own rounding into `Lambda` as well.

**N: negatives** (none may be `Verified`; registered class in brackets): stale epoch (`StaleEpoch`); stage-metric
mismatch (`MetricMismatch`); JVP-only problem without a declared model (`UndeclaredModel`); sampled-only binding
(`SampledBindingOnly`); dense n = 3 on route (c) (`UndeclaredStructure`); an off-block entry `2^-1074`
(`OffBlockCoupling`); a declared domain smaller than the causal box (`TubeOutsideDomain`); a mid-run
`set_stage_target_span` change (`SpanMismatch` on every later row); a coefficient-digest mismatch (`TargetMismatch`).

**D: driver runs**, each with ledger on and off. `B3` = ALG06 options (stagnation guard, production fallback, G1, G2,
G3, budget 2,000).
- **D-a (fit, anchor).** AS04's D2 fixture: `A = [[-1, 1], [0, -1000]]`, `y0 = [1, 1]`, `T = 1`, atol 1e-8, rtol 1e-6,
  `h0 = 0.5`, `CoupledGuarded2`, budget 200, the same epoch closure.
- **D-b (fit).** `A = [[-1, 1024], [0, -1000]]`, atol 1e-12, rtol 1e-10, `B3`. Chosen to provoke floor/stall
  acceptances.
- **D-c (overrun visibility).** D-a with `B3`, `restart = budget = 1` and production-rule `rtol_lin = 1e-3` (design
  choice), so fallback acceptances are forced. It is not a fit cell.
- **D-d (fit, nonlinear).** n = 8, `A = -100 tridiag(-1, 2, -1)`, `q = -1`, `y0_k = 1 + 0.1 sin k`, `T = 0.1`,
  atol 1e-6, rtol 1e-4, `B3`.
- **D-e (scope negative).** Robertson-4e10 at rtol 1e-5, `B3`, JVP-only: every row `Unavailable`.

**References.** Single attempts (every X, Y and D attempt): an independent Python Fraction oracle on exact rationals
of all bits, polynomial `f` evaluated exactly. Exact-linear trajectories (D-a, D-b, D-c): mpmath at 200 digits,
checked at 260 digits (design choice).

## Commands

    cargo test --offline --locked -p rodas5p-integrators --features audit2-stage-certificate --test contamination_budget_contract
    AS06_RUNS=research/as06_contamination_budget_20261012/RUNS.json cargo test --offline --locked --release -p rodas5p-integrators --features audit2-stage-certificate --test contamination_budget_contract -- --ignored --nocapture --test-threads=1 export_runs
    python3 tools/as06_contamination_budget_check.py --runs research/as06_contamination_budget_20261012/RUNS.json --as04 research/as04_stage_residual_telemetry_20261011/RUNS.json --output research/as06_contamination_budget_20261012/RESULTS.json

All run with `RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1`; a relative `AS06_RUNS` is joined to the repository root
(the AS05 lesson). RUNS.json stores every input, bound, intermediate (`G`, `c_i`, `xi_i`, `L_i`, `Lambda_i`, `q_i`,
`kappa`, `p_i`) and ledger value as hex bits, with the AS04 telemetry, source commit and tree status.

**Checker.** `tools/as06_contamination_budget_check.py` and its mutation tests
(`tools/test_as06_contamination_budget_check.py`) are committed **before** the recorded run. It calls
`tools/evidence_schema_v2.py` first and imports nothing from the Rust code. Malformed or unbound evidence is
**INVALID** (exit 2), a gate failure **FAIL** (exit 1), a vacuous item **INCONCLUSIVE** (exit 3), otherwise **PASS**
(exit 0). Precedence: INVALID, then FAIL, then INCONCLUSIVE.

## Gate

**Validity (INVALID)** if: the AS03 validation fails; a registered case, run or twin is missing or duplicated; an
input hex does not decode to the registered input; a pin differs (AS04 `RUNS.json` `e42c7be2...`, `residual_gain.rs`
`c8d08e64...`, `evidence_schema_v2.py` `1d2bfe92...`); the 200- and 260-digit references differ by more than `1e-150`
relative; fewer than 2 X2/X3 members discriminate or D-c has no `FallbackAccepted` stage (fixture-design defects); or,
**G1 rule**, the checker blob at the RUNS source commit differs from the checker that ran, any recorded tree status is
dirty, or the checker commit postdates the run.

**PASS** requires all items. Every count is exact; the thresholds are 0 violations.

1. **No behaviour change.** In every D twin, ledger on and off give bitwise identical states, steps, WorkCounters,
   `StageSolveStatistics`, charge log and AS04 telemetry. D-a's AS04 records equal AS04 `RUNS.json` D2 bitwise; a
   mismatch is a FAIL reported as anchor drift, and the kill rule does not apply to it.
2. **Enclosure.** 0 violations over all `Verified` rows of X, Y and D: `q_i >=` the exact stage error for every `i`;
   `E_y >=` the exact output contamination; the embedded interval contains the exact embedded norm; and on D-a, D-b
   and D-c the global bound `E_n >=` the exact contamination at every committed step.
3. **Missed direction.** Every attempt whose exact and computed embedded norms lie on opposite sides of 1 is
   `DecisionUnverified` or `Unavailable`. If fewer than 2 discriminating members are `Verified`, the item is vacuous
   (INCONCLUSIVE).
4. **Negatives.** 0 of the N cases (and 0 rows of D-e) are `Verified`, and each has its registered class.
5. **Outward.** 0 violations: every recorded upper bound (`q_i`, `E_y`, `D_e`, `d_n`, window and cumulative sums,
   `kappa`, `S_n`, `E_n`) is `>=` the checker's exact-Fraction evaluation of its registered formula on the recorded
   inputs, and every recorded lower bound (`denominator_lower`, window budgets) is `<=` it.
6. **Ledger and AS04 reconciliation.**
   - One ledger row per `Committed` AS04 attempt record, with equal index, `t`, `h` bits and per-reason counts. No row
     for a rejected or failed attempt, and the work ledger equals `work_all_attempts`.
   - Every accepted stage, `Floor`/`Stall`/`Fallback` included, has a `Verified` bound or an `Unavailable` class;
     `heuristic_debit >= max(debit_all, debit_fallback_only)` on every attempt; the `T` bits are equal in every row.
   - 0 numeric window or run values where any contributing row is `Unavailable`. In D-c, the `Exceeded` flags equal
     the checker's recomputation; if no D-c window is `Exceeded`, the visibility check is vacuous (INCONCLUSIVE).
7. **Fit.** In every fit run (D-a, D-b, D-d) with a numeric budget, over windows `w = 1..8` of length `T/8` (by step
   start; 8 is a design choice): every window has `up(sum_{n in w} d_n) <= down(0.2 sum_{n in w} |h_n| / T)`, the
   cumulative sum is `<= 0.2` (0.2 is the code's `COUPLED_TARGET_THETA`), and every committed attempt with a floor,
   stall or fallback stage is `Verified` and inside its window. With no `Verified` floor, stall or fallback row in
   these runs, the item is vacuous (INCONCLUSIVE).
8. **Non-regression.** The pre-existing `rodas5p-integrators` and `rodas5p-core` suites pass with and without the
   feature; no pre-existing test file is modified; AS06 items compile only under `audit2-stage-certificate`.

**Kill and hold; ledger verdict mapping.** Allowed verdicts are PASS, FAIL, INCONCLUSIVE and INVALID. HOLD and ABSTAIN
appear only in the claim.
- Item 2, 3, 4 or 5 fails (soundness): **FAIL**, and the certificate is HOLD, not even diagnostic; AS07 stays
  blocked.
- Item 1 fails other than by anchor drift: **FAIL**, and the ledger is not adopted.
- Items 1-6 hold and item 7 fails:
  - if the exact contamination fits but the bounds do not: **FAIL**, claim "bounds too loose; retained as
    certificate/diagnostic; speed promotion stopped (decision HOLD)";
  - if the exact contamination itself overruns: **FAIL**, claim "calibrated target misses its own budget".
- Any other failed item: **FAIL**, with every number preserved.
- All items hold: **PASS, decision HOLD**. There is no default, global-accuracy or speed promotion. If the slack
  `budget / cumulative d_n` is below 2 (design choice) in every fit run, the claim adds "no work-saving follow-up".

**Reported, not gated:** bound/exact ratios per row; which branch of the min binds; slack per window;
`heuristic_debit`, the G3 charge and AS04's two debits against `d_n`; `Unavailable` classes per run; witness setup
flops per attempt against nominal LU; `ledger_vector_copies` and ledger memory.

## Prior information (disclosed)

- **AS04.** D2 gave 39 attempts and 312 stage solves, all `Converged`, 27 `BudgetExceeded` through the roundoff raise.
  Robertson 1e-9 `B3` gave 856 fallback-accepted stages, including the 8 conflicting attempts. The G3 charges are at
  most 2.7e-3 and never acted (L-0097).
- **AS05.** Route (c) bound/error is 4.2-9.6; the `K = 1024` cases with entrywise `delta` are
  `DeterminantContainsZero`; route (d) refused n = 8 Peclet 10 (`1 - a mu_up < 0`); the projected `nu`
  underestimated the error by about 1e3 and 1e6 (F105).
- **ALG04** (L-0094): the strongly nonnormal vig1b-k20 failed at 1e-8 (4.18x). This motivates D-b; ALG04 is not
  rerun.
- The cells were chosen knowing these results. No AS06 code or run exists at registration.

## Predictions

- Items 1, 2, 4, 5 and 6 hold; they follow from outward arithmetic by construction. The anchor reproduces: at
  `0692791` the `rodas5p-integrators` and `rodas5p-krylov` crates are byte-identical to AS04's run commit `bf92e67`.
- **Exact-linear branch.** The `G + 1` branch binds whenever `|h gamma| L > 1 + 1/G` (X1 at `h gamma = 1e-1`, X2,
  D-b). X bound/exact ratios are 1-1e3 for X1 and X4, larger for X2 with `K = 2^20`. The `e2` members with
  `K = 2^20` discriminate; the `K = 1024` ones probably do.
- **Global amplification.** `S_n` ignores cancellation (`|R(z)| <= 1`), so `E_n`/exact grows geometrically: I expect
  more than 1e6 after about 20 steps. It is sound but not useful.
- **Fit.** D-a fits with slack above 10 (uncertain). D-b: floor/stall acceptances are likely, and their verified
  debits are probably still below `0.2 h/T`. D-d may be mostly `Unavailable` (`NonpositiveDenominator` in the
  atol/rtol metric).
- **Most likely non-PASS:** INCONCLUSIVE through item 7's vacuity (no `Verified` floor, stall or fallback row in a fit
  run with a numeric budget). Next most likely is a loose-bound FAIL on D-b.
