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

## Results

Appended after the recorded run; the registered text above is unchanged. Branch `alg/alg05` from `b8964fc` (the
registration commit of ALG04-ALG06).

**Verdict: FAIL** (item 5, the single-cell bound). Items 1, 2, 3, 4 and 6 pass, and the item-5 fit statistic passes on
every problem. Counted work only; no wall-time claim.

### Record

| commit | content |
|---|---|
| `1165644` | test only: `crates/rodas5p-cli/tests/alg05_predictive_controller_v2.rs` with the cells and `export_base` |
| `306ecf1` | `BASE.json`: arm `I` on the 276 cells, run on `1165644` (no source change of this node) |
| `1abae92` | `ControllerKind::PredictiveCapped2` in `adaptive.rs`; unit tests in `crates/rodas5p-integrators/tests/alg05_predictive_capped2.rs` |
| `732992c` | `export_runs` (arms `I`, `PREDcap`, `PREDcap2`) and `tools/alg05_controller_v2_check.py`, committed before the recorded run |
| `e601998` | `RUNS.json` (run on `732992c`, clean tree) and `RESULTS.json` |

Commands (with `RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1`):

    ALG05_BASE=research/alg05_predictive_controller_v2_20261010/BASE.json cargo test --release -p rodas5p-cli --locked --test alg05_predictive_controller_v2 -- --ignored --nocapture --test-threads=1 export_base
    ALG05_RUNS=research/alg05_predictive_controller_v2_20261010/RUNS.json cargo test --release -p rodas5p-cli --locked --test alg05_predictive_controller_v2 -- --ignored --nocapture --test-threads=1 export_runs
    python3 tools/alg05_controller_v2_check.py --base research/alg05_predictive_controller_v2_20261010/BASE.json --runs research/alg05_predictive_controller_v2_20261010/RUNS.json --native research/stiff_native_benchmark_20261001/NATIVE.json --output research/alg05_predictive_controller_v2_20261010/RESULTS.json

SHA-256: `BASE.json` `16aee528…73bb44`, `RUNS.json` `76184aa2…018807`, `RESULTS.json` `9b8d0071…7a157050` (full
digests in the commit log; BASE and RUNS also in `RESULTS.json` `inputs`). This is the first and only recorded run;
nothing was changed after it.

Checks at `1abae92`: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and
`cargo test -p rodas5p-integrators -p rodas5p-cli --all-targets --locked` (602 passed, 0 failed, 56 ignored). Clippy
and fmt are also clean at `732992c`.

### Base

All 276 runs complete; LU factorizations equal attempts in every run. van der Pol at rtol 1e-3, seed 3e-6: 181/113/68
attempts/accepted/rejected. Pooled rejection fractions of `I`: van der Pol 28.3 %, HIRES 1.50 %, Robertson 2.94 %,
Brusselator-50 13.5 %. Every endpoint error the checker recomputed from the state bits equals the exported one. One
`I` cell has err/rtol above 10: van der Pol rtol 5.62e-4, seed 3e-6, err/rtol 65.04 (the next largest is 3.24).

### Gate (`PREDcap2` against `I`)

| item | measured | threshold | result |
|---|---|---|---|
| 1. identity | 276 of 276 `I` rows of RUNS equal BASE | all | pass |
| 2. van der Pol gain | frontier 0.685, 0.725, 0.766, 0.811, 0.834 at E = 1e-3, 1e-4, 1e-5, 1e-6, 3e-7 (none excluded) | <= 0.90 at >= 3 of 5 | pass (5 of 5) |
| 3. no regression, HIRES | worst in-range frontier 1.015 (E 1e-11); 15 in-range E | <= 1.06 | pass |
| 3. no regression, Robertson | worst 1.033 (E 3.16e-10); 11 in-range E | <= 1.06 | pass |
| 3. no regression, Brusselator-50 | worst 0.969 (E 1e-4); 6 in-range E | <= 1.06 | pass |
| 4. rejections | van der Pol pooled 6.84 % against 28.27 % (0.242x) | <= 0.5x | pass |
| 5. calibration, fit | median over seeds of max fit ratio: van der Pol 0.978, HIRES 0.950, Robertson 1.207, Brusselator-50 1.563 | <= 2.0 each | pass |
| 5. calibration, cells | 1 of 276 `PREDcap2` cells above 10: van der Pol rtol 5.62e-4, seed 3e-6, err/rtol **65.17** | every cell <= 10 | **FAIL** |
| 6. cap behaviour | `err_zero_after_a_rejection_gives_factor_one`, `a_sliver_landing_keeps_the_rejection_flag` present and passing | both | pass |

**Item 5.** The failing cell is not a controller shift. `I` gives 65.04 in the same cell (error 0.03658 against
`PREDcap2`'s 0.03665; equal-rtol error ratio 1.002), and `PREDcap` gives the identical state. The neighbouring seeds of that rung
give 0.56 and 1.22 for `PREDcap2` (1.15 and 0.78 for `I`). The registered bound is absolute, not relative to `I`, so it
fails on a cell where the reference arm itself is far above it. The registered rule stands, and the item fails.

Per-seed maximum fit ratios (seeds 3e-6, 3e-5, 3e-3; the end of the rtol range where the maximum sits): van der Pol
1.675 (1e-3), 0.929 (1e-3), 0.978 (1e-7); HIRES 1.110, 0.890, 0.950 (all 1e-3); Robertson 1.655, 1.207, 1.044 (all
1e-10); Brusselator-50 1.536, 1.563, 1.786 (all 1e-3). Fitted slopes b (`PREDcap2` / `I`) are close on every problem:
van der Pol 0.91-1.09 / 0.92-1.02, HIRES 1.17-1.19 / 1.17, Robertson 0.88-0.90 / 0.91-0.92, Brusselator-50 0.73-0.75
/ 0.65-0.66. The Brusselator-50 shift is the steeper slope: the ratio rises from about 0.77 at rtol 1e-7 to about 1.6
at 1e-3.

### `PREDcap2` against `PREDcap`

All 276 `PREDcap2` rows equal the `PREDcap` rows bit for bit (all fields except the arm name). The two changes cannot
act on these cells:

- the output schedule is `[t0, tf]` and no run had a clipped output landing (`output_clipped_steps` = 0 in every row),
  so there is no sliver;
- no accepted step had err = 0 while a rejection was pending (inferred from the identical rows: such a step would
  change the next request from 5h to h; the runs record no per-step errors).

The prediction "equal on most cells" holds on all cells. Every `PREDcap` number in `RESULTS.json`
(`reported.PREDcap_against_the_items`) therefore equals the `PREDcap2` number, including the item-5 failure.

### Reported, not gated

- **Cheapest-run ratios** (`PREDcap2`/`I`): van der Pol 0.737, 0.650, 0.743, 0.943, 0.813; worst in-range HIRES
  1.024, Robertson 1.107 (E 1e-5), Brusselator-50 1.103 (E 1e-4). Under ALG02's C <= 1.10 rule Robertson and
  Brusselator-50 would fail again by a single threshold step; the frontier at the same E is 1.012 and 0.969.
- **Geometric-mean frontier ratios** over in-range E: van der Pol 0.762, HIRES 0.942, Robertson 1.021, Brusselator-50
  0.940.
- **Pooled rejection fractions** (`I` / `PREDcap2`): van der Pol 28.3 % / 6.84 %, HIRES 1.50 % / 0.65 %, Robertson
  2.94 % / 2.81 %, Brusselator-50 13.5 % / 5.90 %. Per seed for van der Pol: 6.65 %, 6.46 %, 7.40 % against 28.0 %,
  28.2 %, 28.6 %.
- **Ladder attempt totals** (3 seeds): van der Pol 18,885 against 14,888; HIRES 57,181 against 57,240; Robertson 8,618
  against 8,662; Brusselator-50 3,663 against 3,409.
- **Equal-rtol error ratios to `I`** (median [min, max]): van der Pol 1.017 [0.36, 6.66], HIRES 0.966 [0.25, 1.97],
  Robertson 1.000 [0.17, 9.46], Brusselator-50 1.226 [0.78, 2.19]. Cells with err/rtol > 2 (`I` / `PREDcap2`): van der
  Pol 18 / 15, Robertson 0 / 1 (4.01), Brusselator-50 16 / 0.

**Predictions.** All registered predictions held: van der Pol 0.685-0.834 (predicted 0.68-0.84), HIRES worst 1.015
(about 1.01), Robertson worst 1.033 (about 1.04), Brusselator-50 worst 0.969 (about 0.96), rejections 0.242x (about
0.25x), item-5 Brusselator-50 1.56 (about 1.6), `PREDcap2` equal to `PREDcap` (on all cells). The preregistration did
not anticipate the `I` cell at err/rtol 65.

### Interpretations and deviations

- **Item-5 cell bound** is applied to the gated arm's cells (`PREDcap2`), as the gate reads "arm `PREDcap2` against
  `I`" and the disclosed prior value 7.9 is `PREDcap`'s ALG02 maximum (`I`'s ALG02 maximum was 14.7). `I`'s cells are
  reported. Applying it to `I` as well would not change the verdict.
- **Item-5 fit maximum** "over the ladder's rtol range" is taken at the two ends of the range (both fits are linear in
  log10 rtol); the maximum over the ladder points is also recorded and is the same.
- **Checker cross-check before the run.** On ALG02's RUNS (`PREDcap` against `I`, ALG02 seeds) the checker gives the
  disclosed item-5 values: 1.048, 0.998, 1.251, 1.646, largest `PREDcap` cell 7.90.
- **Item 6** is evidenced by `cargo test` at `1abae92`; the checker confirms the two registered tests exist in the test
  file (and records its SHA-256) but does not run them.
- **Frontier medians** require all three seeds' fits (ALG02's checker took the median of the available seeds); every
  fit existed, so this had no effect. The verdict also requires a complete RUNS and no error-recomputation mismatch
  (both hold).
- **The rejection flag** of `PredictiveCapped2` is a new `#[serde(skip)]` field (`rejection_pending`), read and written
  only by the new kind. The existing `last_rejected_trial` is the TIME-DEV-02 landing guard and keeps its meaning. A
  rejected or non-finite/linear failure sets the flag (the drivers report all of them to the shared update as not
  accepted); an accepted non-clipped step, or a clipped one with trial >= 0.5 x request, clears it; a sliver keeps it.
  The err = 0 step still enters the history (err_prev 1e-16, h_acc = h), as in ALG02.
- **Unit tests** for the new kind are in a new integration-test file, so the ALG02 test module in `adaptive.rs` is
  untouched. They include an operation-for-operation model of the pre-ALG05 shared update, checked bit for bit against
  `Integral`, `Pi`, `Predictive` and `PredictiveCapped` on 16,000 generated attempts each (rejections, failures,
  err = 0, slivers and informative clipped samples).
- **No matrix-free cells**; none were registered.

## Correction after the independent review (appended 2026-10-10)

The verdict stays **FAIL**. The review reran the export at the merge head and got a byte-identical RUNS.json; the checker reproduces RESULTS.json. One wording correction applies to ledger row L-0095, which is superseded by L-0098: the claim that the absolute single-cell bound of 10 "was mis-specified" is a post-hoc judgement. The correct statement is factual: the reference arm I also violates the bound in the same cell (65.04 against 65.17). That was foreseeable at registration, because I's maximum in ALG02 was 14.7, while the registration disclosed only PREDcap's maximum of 7.9. Gate item 6 rests on unit tests alone, since the two fixes never acted on these cells.
