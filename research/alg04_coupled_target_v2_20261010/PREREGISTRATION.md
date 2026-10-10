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

## Results (appended after the recorded run; the registered text above is unchanged)

**Recorded commits** (branch `alg/alg04-alg06`, from `b8964fc`).
- `90ac0b1`: test-only harness `crates/rodas5p-integrators/tests/alg04_alg06.rs` with the registered cell lists.
- `b7f1cb3`: implementation. `DupFix` (`GmresIntoOptions::skip_final_residual`), `CoupledGuarded2` (staged-solver
  `small_system_exhaustion`) and the ALG06 options, with contract tests.
- `19dfa96`: `export_runs_alg04` and `tools/alg04_coupled_target_v2_check.py`, committed before any run.
- `896a32a`: `RUNS.json` (produced on `19dfa96`, clean tree) and `RESULTS_FIRST.json` (committed as `RESULTS.json`).
- `12dbb2d`: checker fix for a reported-only metric and the rerun `RESULTS.json` (see Deviations).

**Commands** (release, `RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1`, `CARGO_INCREMENTAL=0`): the registered ones,
unchanged.

    ALG04_RUNS=research/alg04_coupled_target_v2_20261010/RUNS.json cargo test --release -p rodas5p-integrators --locked --test alg04_alg06 -- --ignored --nocapture --test-threads=1 export_runs_alg04
    python3 tools/alg04_coupled_target_v2_check.py --base research/alg01_coupled_stage_target_20261008/BASE.json --runs research/alg04_coupled_target_v2_20261010/RUNS.json --output research/alg04_coupled_target_v2_20261010/RESULTS.json

The recorded run took 167 s (plus a 2.5 min release build). Contract tests: `cargo test -p rodas5p-krylov -p
rodas5p-integrators --all-targets --locked` passes. Clippy `-D warnings` is clean with and without
`audit2-research`.

**Gate (arm `CoupledGuarded2`): FAIL.**

| Item | Result | Numbers |
|---|---|---|
| 1. Identity | PASS | `Legacy` equals ALG01 BASE on all 65 adaptive cells, all 18 rungs at budget 20,000 (BASE `legacy_pilot_budget`) and all 18 at 200 (BASE `legacy`). `DupFix` equals `Legacy` in states, attempts, accepted and rejected steps on all 65 cells and 18 rungs. Its `jvp_vectors` are `Legacy`'s minus `Legacy`'s `diagnostic_matvecs` everywhere, and every other counter except `jvp_calls` and `linear_matvec_vectors` is equal. Twins and LU rungs reproduce |
| 2. Accuracy (<= 1.5x twin) | **FAIL** | 31 cells evaluated, 30 pass: C1 0.996-1.000x, C2 0.999-1.000x, E-05 0.015-1.043x, vigb-k10 0.995-1.002x, vigb-k20 0.549-1.009x, vig1b-k20 1e-4 1.000x, 1e-6 1.000x. **Fail: `vig1b-k20` 1e-8, 4.18x** (2.62e-9 against the twin's 6.26e-10). Excluded as in ALG01: HIRES 1e-10 and Robertson 1e-10 (reference uncertainty; the arm is 1.01x and 1.00x there) and HIRES 1e-11 (twin fails) |
| 3. Ladders (budget 20,000) | PASS | diagpr128 k = 3, 4, 5: 0.997, 1.001, 0.997x LU. semilin128 k = 3, 4, 5: 0.998, 1.000, 1.000x LU. semilin64 1e-6 observed slopes 5.15 and 5.13 |
| 4. Robustness | PASS | 52 cells (C1, C2 without Robertson 1e-11, C3, 18 rungs). Failures are at most `Legacy`'s everywhere (E-05 s = 0 1e-4: 27 vs 53, s = 1 1e-4: 30 vs 31; 0 vs 1-2 at 1e-6) |
| 5. Target gain vs `ProjL2` | PASS | geometric mean **0.738**. Cells: Bruss-50 1e-6 0.667, 1e-8 0.874; Bruss-160 1e-6 0.613, 1e-8 0.828 |
| 6. Small n vs `DupFix` | PASS | HIRES 0.980 / 0.949, Robertson 0.935 / 0.906, van der Pol 1.006 / 1.002 (1e-6 / 1e-8) |

**Reported, not gated.**
- **Failing cell.** On `vig1b-k20` 1e-8 every arm is above the twin. `Legacy` and `DupFix` are 1.73x (they would fail
  item 2 too), `ProjL2` 1.18x, `CoupledGuarded` 4.36x and `CoupledGuarded2` 4.18x. The twin needs 1,037 attempts with
  466 rejections there. Exhaustion repaired 1e-4: ALG01's 37.14x is now 1.00x at the same attempt count as the twin.
  It did not repair 1e-8, where 1,656 of 8,000 solves are stall acceptances and the round-off floor binds in 5,439.
- **Attribution: JVPs per accepted step, `DupFix` / `Legacy`.**
  - Brusselators: 0.976-0.987.
  - HIRES 0.888 / 0.889, quadratic-4 0.82, Robertson 0.764 / 0.773, van der Pol 0.750, Prothero-Robinson
    0.634 / 0.647.
  - So the duplicate diagnostic residual alone explains most of the small-n "savings" of ALG01.
- **Attribution: JVPs per accepted step against `DupFix`.**

  | Problem (1e-6 / 1e-8) | `ProjL2` | `CoupledGuarded2` |
  |---|---|---|
  | Bruss-50 | 0.479 / 0.303 | 0.319 / 0.265 |
  | Bruss-160 | 0.740 / 0.631 | 0.454 / 0.522 |
  | HIRES | 0.856 / 0.674 | 0.980 / 0.949 |
  | Robertson | 0.895 / 0.849 | 0.935 / 0.906 |
  | van der Pol | 0.995 / 0.992 | 1.006 / 1.002 |
  | Prothero-Robinson | 1.000 / 1.000 | 1.157 / 1.094 |

- **Orthogonalization plus nu-guard SVD flops per accepted step against `Legacy`** (2n per inner product and per
  vector update; the SVD flops are not in the counters).
  - `CoupledGuarded2`: Brusselators 0.08-0.38, HIRES 2.03 / 1.84, quadratic-4 1.79 / 1.84, van der Pol 1.62,
    Prothero-Robinson 1.62 / 1.53, Robertson 1.20 / 1.16.
  - `CoupledGuarded`: HIRES 1.19 / 1.11, van der Pol 1.61.
  - Exhaustion more than doubles the nu evaluations (537,620 against 317,059 over all adaptive cells; SVD flops
    7.54e9 against 7.37e9).
- **Stage statistics, `CoupledGuarded2`, all adaptive cells.** 243,401 solves, of which 203,192 exhausted. 11,090 stall
  acceptances, 27 fallback acceptances and 57 failed solves (all in E-05 1e-4, as in ALG01). The threshold was
  tightened by nu in 231,621 solves (max nu 2.05e6).
- **C5 frontiers, JVPs, regression / cheapest-run.**
  - Bruss-50 `CoupledGuarded2`: 0.274 / 0.283 against `Legacy`, 0.280 / 0.290 against `DupFix`; `ProjL2` 0.351 /
    0.423 against `Legacy`.
  - HIRES `CoupledGuarded2`: 0.860 / 0.962 against `Legacy`, 0.968 / 1.083 against `DupFix`; `CoupledGuarded` 0.734 /
    0.806 against `Legacy`.
- **Reproduction of ALG01's RUNS.** `Legacy`, `ProjL2`, `L2Coupled` and `CoupledGuarded` equal ALG01's RUNS bit for bit
  on all 65 cells and on all 18 rungs at budget 20,000. `CoupledGuarded2` equals ALG01's `CoupledGuarded` on all 31
  cells with n > 40.
- **Other arms against the gate items.**
  - `CoupledGuarded` fails item 2 (vig1b-k20 1e-4 37.14x, 1e-8 4.36x).
  - `ProjL2` fails items 2, 3, 4 and 5.
  - `L2Coupled` fails items 2, 3 and 4.
  - `DupFix` fails items 2, 3 and 5.
  - All pass item 6.
- **Predictions.**
  - Item 5 0.738 (predicted about 0.73; cells 0.667, 0.874, 0.613, 0.828 against 0.66, 0.87, 0.62, 0.82).
  - vig1b-k20 within 1.5x: right at 1e-4 and 1e-6, wrong at 1e-8 (4.18x).
  - HIRES item 6: 0.98 / 0.95 (predicted near 1.0-1.1).

**Interpretations fixed before the run** (checker docstring, committed in `19dfa96` before the run).
- **Exhaustion.** An in-cycle confirmation is taken only at a column that ends the Krylov space: happy breakdown, a
  projected residual <= 16 eps ||D b||, or the cycle's last column.
- **nu guard.** ALG01's code is unchanged. nu is evaluated at every column whose projected residual meets the
  (tightened) threshold outside the confirmation gap, also between the first would-pass column and the end of the
  space.
- **Ladders.** Every arm runs them at budget 20,000. `Legacy` also runs at budget 200 for the identity with BASE's
  `legacy` rows.
- **Items 5 and 6** use the C1 cells and `jvp_vectors` per accepted step.

**Deviations.**
- **A reported-only metric was wrong in the first RESULTS.** The ratio
  `orthogonalization_and_svd_flops_per_accepted_vs_legacy_c1` added operation counts to SVD flops (mixed units).
  - Fixed in `12dbb2d`. The checker was rerun on the same RUNS.json, and the first output is kept as
    `RESULTS_FIRST.json`.
  - No gate item, ratio or verdict changed; the two files differ only in that block and the schema id.
- **No other deviation.**
