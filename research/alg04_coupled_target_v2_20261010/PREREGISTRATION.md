# Preregistration: coupled stage target, second test (ALG04)

Follow-up to ALG01 (L-0090, corrected by L-0093). Branch `audit/rvj-algorithmic-directions-20261008`, registered
on top of `4c980c5`. The user approved registering this follow-up on 2026-10-10.

**Claim boundary.** As in ALG01: counted work (WorkCounters) and endpoint accuracy only, no instruction-count or
wall-time claim. Every change is opt-in, and `StageTargetPolicy::Legacy` stays bit for bit.

## Question

ALG01 reproduced the coupled target's Brusselator work cut but failed for three reasons:

- **The single nonnormal 2x2 block.** On `vig1b-k20`, with ||W^-1|| about 5e5, the arm was 37x the twin.
- **A registration defect.** The ladders could not finish within 200 columns for any arm.
- **Attribution.** Its small-n savings came from omitting production's duplicate diagnostic residual, not from the
  target.

With the defect removed, the attribution separated, and a rule for small systems added, does the coupled target keep
accuracy everywhere and still beat the in-cycle exit at the production target on the Brusselators?

## Changes (opt-in)

1. **`DupFix` arm.** The `Legacy` stage solve (GMRES into) without the final diagnostic true residual. The solution
   and trajectory must equal `Legacy` bit for bit; only the counters change. This is a programming-level arm, used for
   attribution.
2. **`CoupledGuarded2` (gated)**: ALG01's `CoupledGuarded` with two changes.
   - **Small-system exhaustion.** When the dimension satisfies n <= restart (40), each stage solve runs until happy
     breakdown, or until the round-off floor `16 eps ||D b||`, before the confirming true residual. A full Krylov space
     costs at most n columns. The rationale: for small n the nu estimate from a partial Krylov space can miss the
     amplification that only the last direction reveals, which was the `vig1b-k20` failure.
   - **The nu-guard rule is stated as implemented in ALG01.** nu is re-evaluated at every would-pass projected exit
     outside the confirmation gap (`used >= next_check`), keeping the running maximum. This is the rule the ALG01 run
     used. It is now registered, not assumed.
3. **Ladder budget.** Every arm runs the C4 ladders with a budget of 20,000 columns, which removes the ALG01
   registration defect. Adaptive cells keep budget 200.

Arms: `Legacy`, `DupFix`, `ProjL2`, `L2Coupled`, `CoupledGuarded` (ALG01 rule, reported), `CoupledGuarded2`
(gated).

## Cells, references, twins

As registered in ALG01: C1 (SPD07 14 cells), C2 (HIRES and Robertson at 1e-9, 1e-10 and 1e-11), C3 (`vigb-k10`,
`vigb-k20`, `vig1b-k20`, `e05-s0`, `e05-s1` at 1e-4, 1e-6 and 1e-8), C4 (the fixed-step ladders) and C5 (the reported
ladders). Definitions, references, the uncertainty exclusion rule, the twins and the error metric are those of the
ALG01 test (`tests/alg01_common/mod.rs`).

## Base

No new base export is needed. ALG01's `BASE.json` was recorded on the unmodified source and contains `Legacy` on
every cell, including ladder rows at the pilot budget. `Legacy` in this node's RUNS must reproduce it.

## Commands

    ALG04_RUNS=research/alg04_coupled_target_v2_20261010/RUNS.json cargo test --release -p rodas5p-integrators --locked --test alg04_alg06 -- --ignored --nocapture --test-threads=1 export_runs_alg04
    python3 tools/alg04_coupled_target_v2_check.py --base research/alg01_coupled_stage_target_20261008/BASE.json --runs research/alg04_coupled_target_v2_20261010/RUNS.json --output research/alg04_coupled_target_v2_20261010/RESULTS.json

## Gate (arm `CoupledGuarded2`)

**PASS** if all of the following hold.

1. **Identity.**
   - `Legacy` reproduces ALG01's `BASE.json` bit for bit, on its adaptive and ladder rows.
   - `DupFix` equals `Legacy` bit for bit in states, attempts, accepted and rejected steps.
   - `DupFix`'s JVPs equal `Legacy`'s minus exactly the diagnostic residuals counted.
2. **Accuracy.** In every C1, C2 (except Robertson 1e-11) and C3 cell where the twin succeeds and the reference is
   admissible: `err <= 1.5 err(twin)`.
3. **Ladders** (budget 20,000).
   - At every rung of `diagpr128` and `semilin128`: `|y - y_ref| <= 3 |y_LU - y_ref| + 1e-13`.
   - On `semilin64` at rtol 1e-6: two consecutive order slopes of at least 4.5.
4. **Robustness.** The arm completes every cell `Legacy` completes, with no more linear-solve failures.
5. **The target's own work gain.** Over the four Brusselator cells (Bruss-50 and Bruss-160 at 1e-6 and 1e-8), the
   geometric mean of `CoupledGuarded2` / `ProjL2` JVPs per accepted step is <= 0.85. Each cell is also <= 1.0.
6. **Small n is no worse than the attribution arm.** On HIRES, Robertson and van der Pol at 1e-6 and 1e-8, JVPs per
   accepted step are <= 1.10x `DupFix`'s.

Everything else is **FAIL**, with every ratio preserved.

**Reported:** every arm on every cell, the C5 frontiers, the nu and stall statistics, and the nu-guard SVD flops,
which are not in the counters.

**Predictions** (from ALG01's RUNS, where `CoupledGuarded` and `CoupledGuarded2` coincide for n > 40).
- Item 5: about 0.73 (cells 0.66, 0.87, 0.62, 0.82).
- Item 2 on `vig1b-k20`: within 1.5x, because the solves are exhausted.
- Item 6: HIRES may come in near 1.0-1.1x, because exhaustion costs n columns. This is uncertain.

## Prior information

- ALG01 RUNS/RESULTS and its correction.
- No code of this node exists before this commit.
