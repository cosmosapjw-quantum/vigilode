# Preregistration: larger stage budget with stagnation guard and production fallback (ALG03)

Algorithmic-directions cycle, Tier 1 node N3 (`docs/reviews/20261008_algorithmic_directions/ALGORITHMIC_DIRECTIONS.md`,
sections 2.2, 2.4 and 4). Branch `audit/rvj-algorithmic-directions-20261008`, base `a793ecd`. It builds on ALG01's
staged solver and `CoupledGuarded` stage target, registered in the same commit.

**Claim boundary.** As in ALG01: counted work and endpoint accuracy only, no instruction-count or wall-time claim, and
every change opt-in.

## Question

The matrix-free driver gives each stage solve 200 Krylov columns. A solve that does not converge in that budget
becomes a linear-solve failure, which rejects the attempt with h x 0.2. On large or nonnormal problems this produces
failure cascades (Brusselator-300: 118 failures; L-0066).

The pilots found three things:

- A budget of 2,000 columns removes the failures, and the coupled target makes the remaining solves short.
- A stagnation guard (abort at restart contraction q >= 0.98, or on a predicted overrun) bounds hopeless solves.
- That guard livelocks a long badly scaled run (Robertson to t = 4e10) by aborting solves that would have converged.
  A production fallback, which accepts the iterate if it meets the production residual rule, removed the livelock.

Does the Rust implementation reproduce this, without changing any cell where the budget never binds?

## Change (opt-in)

The ALG01 staged solver gets an optional stagnation guard. At each restart boundary it aborts if:

- `q = ||r_k|| / ||r_(k-1)|| >= 0.98`, or
- the columns used plus `40 ceil(log(thr / ||r_k||) / log q)` exceed the budget.

On an abort or on budget exhaustion, the caller's fallback test runs first. The driver's budget (`maxiter`) is set
per arm.

**Arms** (all with the `CoupledGuarded` stage target of ALG01, Integral controller, GMRES restart 40):

| Arm | Budget | Guard | Fallback |
|---|---|---|---|
| `B0` | 200 | no | yes |
| `B1` | 2000 | no | yes |
| `B2` (gated) | 2000 | yes | yes |
| `B2nf` | 2000 | yes | no (control) |

**Rival.** `Rbig`: the `Legacy` stage target with budget 2000.

## Cells

Adaptive configuration as in ALG01.

| Group | Cells |
|---|---|
| D1 | Brusselator-1d-160 at 1e-4; Brusselator-1d-300 (n = 600) at 1e-4 and 1e-6 |
| D2 | Robertson to t = 4e10 at 1e-5, 1e-7 and 1e-9 (max attempts 50,000) |
| D3 | `stosc` (pilot `sprobs.py`: 64 forced blocks `[[-50, -w_k], [w_k, -50]]`, omega = 1e4, T = 2) at 1e-4, 1e-6 and 1e-8 |
| D4 | `e05-s10` (the E-05 operator with s = 10, forced as in the pilot, T = 0.1) at 1e-4 |
| D5 | the 14 SPD07 cells of ALG01 C1 (identity check) |

References and error metric as in ALG01.

## Base export (before any source change)

The ALG01 test-only commit also writes `BASE.json` for this node: `Legacy` with budgets 200 and 2000 on D1-D4.

## Commands

    ALG03_BASE=research/alg03_stage_budget_guard_20261008/BASE.json cargo test --release -p rodas5p-integrators --locked --test alg01_coupled_stage_target -- --ignored --nocapture --test-threads=1 export_base_alg03
    ALG03_RUNS=research/alg03_stage_budget_guard_20261008/RUNS.json cargo test --release -p rodas5p-integrators --locked --test alg01_coupled_stage_target -- --ignored --nocapture --test-threads=1 export_runs_alg03
    python3 tools/alg03_budget_guard_check.py --base .../BASE.json --runs .../RUNS.json --output research/alg03_stage_budget_guard_20261008/RESULTS.json

## Gate (arm `B2`)

**PASS** if all of the following hold.

1. **Neutral where the budget never binds.** On every D5 cell where `B0` never needs more than 200 columns in a
   stage solve and never aborts, `B2` equals `B0` bit for bit.
2. **No livelock.** `B2` completes every D1-D4 cell that `Rbig` or the dense twin completes. In particular it
   completes Robertson to 4e10 at all three rtols.
3. **Failures.**
   - In every cell, `B2`'s linear-solve failures are at most `B0`'s.
   - They are 0 on Brusselator-1d-160 at 1e-4 and on Brusselator-1d-300.
4. **Accuracy.** In every D1-D4 cell completed by the twin: `err(B2) <= 1.5 err(twin)`.
5. **Work.** On Brusselator-1d-300 at 1e-4, `B2` uses at most 0.80x `B0`'s JVPs and at most 0.80x `Rbig`'s.

Everything else is **FAIL**, with every ratio preserved.

**Reported, not gated.**
- `B1` and `B2nf`.
- Guard aborts and their classification: false (a shadow continuation without the guard converges within 2,000
  columns) or true.
- Fallback acceptances.

**Predictions** (pilot).
- `B2nf` livelocks Robertson to 4e10.
- `B2` completes it with accuracy equal to the twin.
- Brusselator-300 1e-4: `B2` about 0.6x `B0` and about 0.6x `Rbig` in JVPs, with 0 failures.
- On `stosc` (omega = 1e4) `B2` gains nothing over `Rbig` (0.97-1.11x).
- The guard never fires on D1.

## Prior information

- Pilot: `docs/reviews/20261008_algorithmic_directions/pilot/phase3_reports/{stack,critic}.md`.
- L-0066 (failure cascades); KRY-BUDGET-PREDICT in `pilot/phase2_candidates/`.
- No code of this node exists before this commit.
