# Preregistration: stage budget guard, second test (ALG06)

Follow-up to ALG03 (L-0092). Branch `audit/rvj-algorithmic-directions-20261008`, registered on top of `4c980c5`. The
user approved registering this follow-up on 2026-10-10. It builds on ALG04's `CoupledGuarded2` target, registered in
the same commit.

**Claim boundary.** As in ALG03: counted work and endpoint accuracy only, no instruction-count or wall-time claim, and
every change opt-in.

## Question

ALG03's guard removed the Robertson-to-4e10 livelock, but that run ended at 3.32x the twin's error at rtol 1e-5. The
fallback had accepted iterates whose residual met production's rule but not the coupled target, and nothing charged
that gap to the step's error. The overrun prediction also used the nominal cycle length 40 where only n = 3 columns
exist.

With three changes, does the guarded large budget keep the twin's accuracy on every stress cell?

- Predict overrun with the effective cycle length.
- Allow the attainable-accuracy floor at every confirmation, not only after a slow cycle.
- Charge every fallback-accepted stage's residual to the step's error estimate.

## Changes (opt-in; arms are cumulative)

1. **`G1`: effective cycle length.** The overrun prediction uses
   `m_eff = min(restart, n, budget - used)` in place of `restart`.
2. **`G2`: attainable-accuracy floor at every confirmation.** At each confirming true residual, the solve is accepted
   if `||r|| <= 1024 eps (||D b|| + ||D x|| + ||D(x - W x)||)`, the stall rule's backward-error floor. This no longer
   requires a slow cycle first.
3. **`G3`: fallback charged to the error estimate.**
   - For every stage i accepted through the production fallback, add `tau_e,i ||r_i||_WRMS` to the attempt's
     embedded error before the accept/reject decision.
   - `tau_e` is the tableau's residual-to-embedded-error transfer constant, already computed for the coupled target.
   - If the charged error exceeds 1, the attempt is rejected through the ordinary local-error path, not the 0.2 cut.
   - First-order frozen-J reasoning: a fallback stage's residual contaminates the error estimate by at most that
     amount when mu <= 0.

**Arms**, all on the `CoupledGuarded2` target with budget 2,000, the stagnation guard, the production fallback and the
Integral controller:

| Arm | Contents |
|---|---|
| `B2` | ALG03's arm, reported |
| `B3a` | `G1` |
| `B3b` | `G1` + `G2` |
| `B3` (gated) | `G1` + `G2` + `G3` |

**Rival:** `Rbig` (`Legacy`, budget 2,000).

## Cells, base

- **Cells:** ALG03's D1-D5, with its references, twins and error metric.
- **Base:** no new base export. ALG03's `BASE.json` (`Legacy`, budgets 200 and 2,000) was recorded on the unmodified
  source, and `Rbig` in RUNS must reproduce its budget-2,000 rows.

## Commands

    ALG06_RUNS=research/alg06_guard_v2_20261010/RUNS.json cargo test --release -p rodas5p-integrators --locked --test alg04_alg06 -- --ignored --nocapture --test-threads=1 export_runs_alg06
    python3 tools/alg06_guard_v2_check.py --base research/alg03_stage_budget_guard_20261008/BASE.json --runs research/alg06_guard_v2_20261010/RUNS.json --output research/alg06_guard_v2_20261010/RESULTS.json

## Gate (arm `B3`)

**PASS** if all of the following hold.

1. **Identity.**
   - `Rbig` reproduces ALG03's budget-2,000 `Legacy` rows bit for bit.
   - On every D5 cell where neither the guard nor the fallback nor the floor ever fires, `B3` equals the
     `CoupledGuarded2` run at budget 200 (from this node's RUNS) bit for bit.
2. **No livelock.** `B3` completes every D1-D4 cell that `Rbig` or the twin completes, including Robertson to 4e10 at
   1e-5, 1e-7 and 1e-9.
3. **Failures.** `B3`'s linear failures are at most `Rbig`'s in every cell, and 0 on Brusselator-160 1e-4 and on
   Brusselator-300.
4. **Accuracy.** In every D1-D4 cell completed by the twin: `err(B3) <= 1.5 err(twin)`.
5. **Work.** On Brusselator-300 at 1e-4, `B3` uses <= 0.80x `Rbig`'s JVPs per trajectory.

Everything else is **FAIL**, with every ratio preserved.

**Reported:**
- `B2`, `B3a` and `B3b` on every cell;
- guard aborts with their shadow-continuation classification;
- floor acceptances, fallback acceptances, and the charged error per attempt.

**Predictions.**
- Items 1-3 and 5 as in ALG03 (Brusselator-300 about 0.42x `Rbig`).
- Item 4 is **uncertain.** Charging the fallback residual should reject the contaminated attempts on Robertson to
  4e10, but smaller steps tighten the per-unit-step target too. The item may still fail.
- `G1` alone should remove most of the Robertson overrun aborts (1, 15 and 49 in ALG03).

## Prior information

- ALG03 RUNS/RESULTS and its correction.
- No code of this node exists before this commit.

## Results (appended after the recorded run; the registered text above is unchanged)

**Recorded commits** (branch `alg/alg04-alg06`, from `b8964fc`).
- `90ac0b1`: test-only harness `crates/rodas5p-integrators/tests/alg04_alg06.rs` with the D1-D5 cell lists.
- `b7f1cb3`: implementation, with contract tests.
  - `StagedGmresConfig::effective_cycle_overrun` (G1) and `floor_at_confirmations` (G2).
  - `StageTargetOptions::charge_fallback_residual` (G3), on the `CoupledGuarded2` target of ALG04.
  - Classification of fallback-accepted guard aborts.
- `19dfa96`: `export_runs_alg06` and `tools/alg06_guard_v2_check.py`, committed before any run.
- `896a32a`: `RUNS.json` (produced on `19dfa96`, clean tree) and `RESULTS.json`.

**Commands** (release, `RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1`, `CARGO_INCREMENTAL=0`): the registered ones,
unchanged.

    ALG06_RUNS=research/alg06_guard_v2_20261010/RUNS.json cargo test --release -p rodas5p-integrators --locked --test alg04_alg06 -- --ignored --nocapture --test-threads=1 export_runs_alg06
    python3 tools/alg06_guard_v2_check.py --base research/alg03_stage_budget_guard_20261008/BASE.json --runs research/alg06_guard_v2_20261010/RUNS.json --output research/alg06_guard_v2_20261010/RESULTS.json

The recorded run took 33 s. Contract tests: `cargo test -p rodas5p-krylov -p rodas5p-integrators --all-targets
--locked` passes. Clippy `-D warnings` is clean with and without `audit2-research`.

**Gate (arm `B3`): PASS.**

| Item | Result | Numbers |
|---|---|---|
| 1. Identity | PASS | `Rbig` equals ALG03 BASE's `legacy_2000` on all 10 D1-D4 cells. D5: 12 of 14 cells eligible, and `B3` equals `CoupledGuarded2` at budget 200 bit for bit on all 12. Van der Pol at both rtols is not eligible, because the G2 floor accepted 15 solves |
| 2. No livelock | PASS | `B3` completes all 10 D1-D4 cells, including Robertson to 4e10 at 1e-5, 1e-7 and 1e-9 (144, 313 and 784 attempts) |
| 3. Failures | PASS | 0 linear-solve failures in all 24 cells (`Rbig` 0 as well), including Bruss-160 1e-4 and Bruss-300 |
| 4. Accuracy (<= 1.5x twin) | PASS | Bruss 0.980-0.995x, stosc 1.000x, E-05 s = 10 0.939x. Robertson to 4e10: 1e-5 **0.544x**, 1e-7 0.994x, 1e-9 1.001x. All 10 references admissible |
| 5. Work, Bruss-300 1e-4 | PASS | 35,883 JVPs against `Rbig`'s 85,778: **0.418x** (42 accepted steps each) |

**Reported, not gated.**

**Robertson to 4e10, error against the twin by arm (1e-5 / 1e-7 / 1e-9).**

| Arm | Error / twin | Note |
|---|---|---|
| `B2` | 1.412 / 1.024 / 0.972 | ALG03's B2 on the `CoupledGuarded` target was 3.32x at 1e-5 |
| `B3a` | **1.872** / 1.012 / 1.007 | would fail item 4 |
| `B3b` | 1.492 / 1.002 / 1.042 | |
| `B3` | 0.544 / 0.994 / 1.001 | |
| `Rbig` | 1.037 / 0.997 / 0.831 | |

**How fragile the item-4 pass is.**
- **G3 rejected nothing.** The charges are small:
  - at 1e-5, 42 charged attempts with a maximum charge of 3.4e-7;
  - at 1e-9, a maximum of 2.7e-3;
  - no charged error crossed 1 in any cell.
- **Every arm took the same steps.** Attempts, accepted steps and rejected steps on Robertson to 4e10 are the same in
  all five arms, `Rbig` included (144 / 143 / 1, 313 / 309 / 4, 784 / 780 / 4).
- **So the registered mechanism did not operate.** The registration expected G3 to reject the contaminated attempts.
  That never happened.
- **What spreads the 1e-5 results** (0.54x to 1.87x across arms that take the same steps) is how sensitive the end
  point is to small perturbations of the stage solutions and of the error that feeds the controller. G3 adds a
  charge of at most 3.4e-7 to that error.
- **The pass at 1e-5 is real but not robust.**
  - `B3b`, which lacks only the charge, is at 1.49x, just inside the limit.
  - The `CoupledGuarded2` target alone (`B2`) already moves 3.32x to 1.41x. This is the small-system exhaustion on
    n = 3, the ALG04 change.

**G1 (effective cycle length).** Overrun aborts on Robertson to 4e10 went from 2 / 13 / 47 (`B2`) to 0 / 0 / 0
(`B3a`); ALG03 had 1 / 15 / 49. This confirms the prediction. Contraction aborts were 73 / 432 / 1,253 (`B2`) and
76 / 444 / 1,316 (`B3a`).

**G2 (floor at every confirmation).**
- `B3` floor acceptances: 241 / 647 / 2,138 on Robertson to 4e10, 29 / 27 / 36 on stosc, and 15 / 15 on van der Pol
  (D5).
- Over D1-D4, contraction aborts fell from 1,836 (`B3a`) to 1,189 (`B3b`) and stall acceptances from 1,981 to 24.
- On Robertson, `B3b` uses 19-26% fewer JVPs than `B3a`.

**Guard aborts.** All guard aborts in every guarded arm were accepted by the production fallback (0 failed solves).
Their shadow classification, `B3` on D1-D4: 442 false (the uncounted continuation without the guard converged
within 2,000 columns) and 807 true. `B2`: 765 false, 1,055 true.

**Work, `B3` / `Rbig` JVPs.**
- Robertson to 4e10: 0.91 / 1.11 / 1.21.
- stosc: 0.97 / 0.95 / 1.00.
- E-05 s = 10: 0.45.
- Bruss-160 1e-4: 0.39. Bruss-300: 0.418 (1e-4) and 0.505 (1e-6).

**Reproduction of ALG03's RUNS.**
- `Rbig` equals ALG03's `rbig` on all 24 cells.
- `B2` equals ALG03's `b2` on all 11 cells with n > 40, where `CoupledGuarded2` is `CoupledGuarded`.
- On the cells with n <= 40, `B2` differs from ALG03's `b2` on 11 of 13 (the exhaustion; Prothero-Robinson, n = 1,
  is equal). The twins reproduce ALG03's BASE.

**Predictions.**
- Items 1-3 and 5: confirmed (Bruss-300 0.418, predicted about 0.42).
- Item 4 (uncertain): passed, but not by the predicted mechanism (see above).
- G1 removes the overrun aborts: confirmed (all of them).

**Interpretations fixed before the run** (checker docstring, committed in `19dfa96` before the run).
- **`B2`.** It is ALG03's switch set on the `CoupledGuarded2` target ("all on the CoupledGuarded2 target").
- **G1.** It replaces `restart` in the overrun prediction only.
- **G2.** The floor is tested at every true residual that misses the threshold, both in-cycle confirmations and
  restart boundaries. At a restart boundary it comes after the stall rule; the stall rule keeps its label.
- **G3.**
  - `||r_i||_WRMS = ||D r_i||_2 / sqrt(n)`, with the coupled target's weights `D = diag(1 / (atol + rtol |y_n|))`.
  - The stage charges of an attempt are summed linearly.
  - The charged error is the attempt's error everywhere downstream: accept/reject, the controller and `e_hat`.
- **Classification.** Every guard abort is shadow-classified, including the aborts the fallback accepted. A contract
  test shows the classification changes nothing in a run.
- **Item 1b eligibility.** It uses `B3`'s `guard_contraction + guard_overrun`, `fallback_accepted` and
  `floor_accepted`.
- **Item 3** covers all 24 cells against `Rbig`.
- **Item 4** uses ALG03's admissibility rule.
- **Item 5** is per trajectory.

**Deviations.** None.

## Correction after the independent review (appended 2026-10-10)

The registered verdict stays **PASS** by the letter of the rules. **Item 4 is not robust, and this pass must not be cited as evidence for G1-G3.** Ledger row L-0096 is superseded by L-0097.

**What the reviewer did.** An independent reviewer reran the registered export at the merge head and got a RUNS.json byte-identical to the committed one; the checker reproduces RESULTS.json. The reviewer then ran an unregistered post-hoc experiment, in their own worktree and not committed: Robertson to 4e10 again, with the initial step h0 multiplied by (1 + k 1e-9) for k = 0..23. k = 0 reproduces the recorded run exactly. At rtol 1e-5:

| Run | Error against the twin |
|---|---|
| Dense twin | unchanged: 1.791e-6 in every run |
| `Rbig` | 0.95-1.06x |
| `B3` (gated) | 0.011-5.70x, median 2.39x; **15 of 24 runs above 1.5x** |
| `B3a` | median 1.60x; 14 of 24 above 1.5x |
| `B3b` | median 1.67x; 13 of 24 above 1.5x |

At 1e-7 and 1e-9 every arm is stable (0.94-1.09x).

**Reading.**
- The gated arm's typical error at this cell is about 2.4x the twin. The recorded 0.544x was a favourable draw.
- G3 could hardly have acted. The fallback accepts at production's 1e-10 relative rule, so `tau_e ||r||_WRMS` stays at most 3.4e-7 (rtol 1e-5) and 2.7e-3 (1e-9) against a local-error budget of 1, and no charge crossed 1. The ALG03 shortfall was against the coupled target's per-unit-step budget, not the local error, so G3 as registered had no realistic route to act.
- A follow-up must gate accuracy over several h0 perturbations (median or quantile), not over one trajectory.

**Wording corrections to the Results above.**
- "Every arm took the same steps" means the same *numbers* of attempts, accepted and rejected steps. The step sequences differ: the B3 and B3b charge logs diverge after t = 767.39.
- "The pass at 1e-5 is real but not robust" should read "not robust".

**Minor.**
- The G3 rejection branch is never exercised. The contract test does not assert a crossing, and no recorded run had one.
- G3's WRMS weights use |y_n|, while the embedded error norm uses max(|y_n|, |y_new|).
