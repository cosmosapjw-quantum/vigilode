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
