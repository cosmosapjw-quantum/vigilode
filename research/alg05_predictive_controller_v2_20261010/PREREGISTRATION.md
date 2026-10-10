# Preregistration: predictive step-size control, second test (ALG05)

Follow-up to ALG02 (L-0091). Branch `audit/rvj-algorithmic-directions-20261008`, registered on top of `4c980c5`. The
user approved registering this follow-up on 2026-10-10.

**Claim boundary.** As in ALG02: counted attempts (LU), RHS and JVPs, and endpoint accuracy. No instruction-count or
wall-time claim. Every change is opt-in; `Integral` and `Pi` stay bit for bit.

## Question

ALG02 confirmed the van der Pol work and rejection gains. It failed on two items:

- **A cheapest-run threshold artefact.** Robertson's cheapest-run ratio was 1.160 at a single E, where the frontier
  ratio was 1.025.
- **A calibration shift at equal rtol.** van der Pol was 2.55x rtol at 5.62e-7.

The independent review also found two literal-reading gaps in `PredictiveCapped`: err = 0 bypassed the cap, and an
accepted sliver landing cleared the rejection flag.

This node fixes the controller gaps and re-measures on fresh initial-step seeds. Score matched accuracy by the
frontier only, and bound the calibration shift with a ladder-level rule. Does the controller pass?

## Change (opt-in)

`ControllerKind::PredictiveCapped2` is `PredictiveCapped` with two changes:

1. **The cap applies when err = 0.** If the previous attempt was rejected or failed, an accepted step with err = 0
   gets `f = min(max_factor, 1) = 1`.
2. **The rejection flag survives sliver landings.** It is cleared only by an accepted, non-sliver step, i.e. a trial of
   at least `CLIPPED_SAMPLE_INFORMATIVE_RATIO` times the request, or a non-clipped trial.

Everything else, including the f_I/f_P formula and the clipped-sample history rule, is as in ALG02.

## Cells

- **Problems, references and error metric.** As ALG02 (dense fast driver; van der Pol mu = 1000, HIRES, Robertson,
  Brusselator-50; references from `NATIVE.json`).
- **Ladders.** As ALG02: 17 points for van der Pol and Brusselator-50, 29 points for HIRES and Robertson.
- **Fresh initial-step seeds:** h0 in {3e-6, 3e-5, 3e-3}. None of these was used in ALG02.
- **Arms:**
  - `I`;
  - `PREDcap`, ALG02's rule, reported;
  - `PREDcap2`, gated.

## Base export (before any source change)

A test-only commit adds `crates/rodas5p-cli/tests/alg05_predictive_controller_v2.rs`. Its `export_base` runs `I` on
all cells with the fresh seeds, on the source at that commit (`BASE.json`).

## Commands

    ALG05_BASE=research/alg05_predictive_controller_v2_20261010/BASE.json cargo test --release -p rodas5p-cli --locked --test alg05_predictive_controller_v2 -- --ignored --nocapture --test-threads=1 export_base
    ALG05_RUNS=research/alg05_predictive_controller_v2_20261010/RUNS.json cargo test --release -p rodas5p-cli --locked --test alg05_predictive_controller_v2 -- --ignored --nocapture --test-threads=1 export_runs
    python3 tools/alg05_controller_v2_check.py --base .../BASE.json --runs .../RUNS.json --native research/stiff_native_benchmark_20261001/NATIVE.json --output research/alg05_predictive_controller_v2_20261010/RESULTS.json

## Scoring

- **Frontier only.** Per seed, fit log(attempts) against log(error) by ordinary least squares over the whole ladder,
  evaluate the fit at E, and take the median over the 3 seeds.
- **In-range E.** E must lie inside both arms' measured error ranges for every seed; other E are excluded.
- **Cheapest-run ratios** are reported, not gated.

## Gate (arm `PREDcap2` against `I`)

**PASS** if all of the following hold.

1. **Identity.** `I` reproduces BASE bit for bit.
2. **van der Pol gain.** Frontier ratio <= 0.90 at >= 3 of E in {1e-3, 1e-4, 1e-5, 1e-6, 3e-7}.
3. **No regression.** On HIRES, Robertson and Brusselator-50, the worst in-range half-decade frontier ratio is <= 1.06.
4. **Rejections.** The pooled van der Pol rejection fraction is <= 0.5x `I`'s.
5. **Calibration.** Per problem and seed, fit log10(err) = a + b log10(rtol) over the ladder for each arm.
   - Over the ladder's rtol range, the median over seeds of max(fit_PREDcap2 / fit_I) must be <= 2.0.
   - In every cell, err / rtol <= 10.
6. **Cap behaviour.** Unit tests show the two changes:
   - err = 0 after a rejection gives factor 1;
   - a sliver landing keeps the rejection flag.

Everything else is **FAIL**, with every ratio preserved.

**Prior information that shaped items 3 and 5 (disclosed).**
- On ALG02's data, the item-5 statistic would have been 1.65 for Brusselator-50, 1.25 for Robertson, 1.05 for van der
  Pol and 1.00 for HIRES, with a largest single-cell err/rtol of 7.9.
- The 2.0 and 10 bounds were set knowing these values. They bound a user-visible calibration shift; they are not
  tuned to pass.
- The fresh seeds are the replication.

**Predictions.**
- Items 2-4 as in ALG02: van der Pol 0.68-0.84, HIRES worst about 1.01, Robertson worst about 1.04, Brusselator worst
  about 0.96, rejections about 0.25x.
- Item 5: Brusselator-50 about 1.6.
- `PREDcap2` equals `PREDcap` on most cells, because the two fixes act only after rejections.

## Prior information

- ALG02 RUNS/RESULTS and its correction.
- No code of this node exists before this commit.
