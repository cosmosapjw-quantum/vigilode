# Preregistration: per-attempt clock and controller overhead of the fast RODAS5P drivers (speed research node SPD01)

Speed research cycle of 2026-10-05 (branch `audit/rvj-speed-research-20261005`, base `4de1f88`). Counted
instructions and work only; the timing authority is on HOLD and nothing here is a wall-time claim.

## Question

After the lean driver (L-0032), v2 (L-0033) and the small-n static driver (L-0041), the instructions per attempted
step on the small problems are 3,569 (van der Pol, n = 2), 5,047 (Robertson, n = 3) and 12,302 (HIRES, n = 8) for
`rodas5p-fast-small`, and 9,135 / 11,437 / 20,576 for `rodas5p-fast` (this host, release build with line tables,
valgrind 3.22, rtol 1e-6; `BASE_PROFILE.json`). Hairer's RODAS, profiled once in L-0030, needs 2,947 and 9,157 on
van der Pol and HIRES. Line- and call-level attribution of the baseline (exploratory, this session, before this
registration) puts a fixed, n-independent block of about 1,350 instructions per attempt outside the stage
arithmetic:

- the step is landed toward the span end three times per attempt for the same target: `adaptive_end_step` ->
  `land_capped` (`output.rs:243`), then `OutputCollector::limit_step` -> `end_step_capped` on the already
  represented step (`output.rs:743`) and `land_capped` toward the next due time (`output.rs:758`), which is the
  span end on the benchmark schedule `[t0, tf]`: three `land_capped` calls (170 Ir each) and six `step_to` calls
  (54 Ir each) per attempt, about 510 + 324 Ir plus 93 Ir of `limit_step` itself;
- `AdaptiveStepConfig::validate` (12 guarded comparisons) runs on every attempt because `propose_factor` validates
  its configuration first (`adaptive.rs:247`), although both fast drivers validate once at entry
  (`rodas5p_fast.rs:749`, `rodas5p_fast_small.rs:340`): 160 Ir per finite-error attempt;
- one libm `pow` per attempt (106 Ir) and the controller body: not touched by this node (removing the `pow` would
  change the step sequence).

Do two opt-in changes, each provably identical to the existing composition on the paths the corpus exercises,
remove this work with bit-for-bit the same trajectories?

## Changes (opt-in; every existing entry point and default unchanged)

`crates/rodas5p-integrators/src/rodas5p_fast.rs`, `rodas5p_fast_small.rs`, `output.rs`, `adaptive.rs`,
`crates/rodas5p-cli/src/stiff_benchmark.rs`:

- `Rodas5pFastOptions { prevalidated_controller: bool, fused_landing: bool }` with `Default` all false, and the
  entry points `integrate_rodas5p_fast_observed_with_options`, `integrate_rodas5p_fast_banded_observed_with_options`
  and `integrate_rodas5p_fast_small_observed_with_options`. The existing entry points call these with the default
  options. The driver id reported in the result names the options (`rodas5p-fast-transformed-v2`,
  `...-v2-val`, `...-v2-land`, `...-v2-ovh`; the small driver likewise).
- **A. Controller prevalidated.** Crate-private `propose_factor_prevalidated` (the body of `propose_factor` after
  its `validate()` call) and `rodas_next_step_after_attempt_prevalidated`, used only when
  `prevalidated_controller` is set. The opt-in entry points validate the configuration once at entry, as today.
  The public `propose_factor` and `adaptive_next_step_after_attempt` keep validating, so every other caller is
  unchanged. The arithmetic is the same code, so the step sequence is identical by construction.
- **B. Fused landing.** Crate-private `OutputCollector::limit_landed_step(t, h)`, used only when `fused_landing`
  is set: it keeps `limit_step`'s finite-input checks, the `due(next_index)` lookup, the `next <= t` error and
  `require_progress`; when the next due time is the collector's own span end it returns `(h, false)` without
  re-landing; otherwise it skips only the re-landing toward the span end and performs `land_capped(t, h, next,
  max_step)` toward the interior due time. The driver has already produced `h` by `adaptive_end_step` with the
  same `StepCap` (`collector.with_max_step(adaptive.step_cap())`); a debug assertion checks that. Fixed-grid
  callers (`bdf.rs`, `integrate.rs`, `radau.rs`, dense output) keep `limit_step`. With the fused landing the driver
  also calls `accept_prechecked`, which skips `accept`'s finiteness rescan of the state the driver has just
  checked (`rodas5p_fast.rs:518-522`, `rodas5p_fast_small.rs:217-221`).

  Why B is expected to be identical: for a landed step (`t + h == tf`) the re-landing takes the `proposed >=
  to_target` branch of `land_nominal` and `represent` returns `step_to(t, tf)` again; for an interior step, `h` is
  already `step_to(t, fl(t + nominal))`, so `fl(t + h)` is the same float, the gap and residue are the same unless
  the residue is capped by `h * 2^-10` (only when `h < 2^16 eps max(|t|, |tf|)`), and `represent(t, h)` returns
  `h`; `shortened` is false on both paths. The one known non-idempotent case (class C): a step shortened by the
  post-rejection rule (`output.rs:244-256`) whose end lies within the residue of `tf`, which today's re-landing can
  extend back to `tf`; the fused path keeps the shortened step, which is what the R4 time rule asks for (after a
  rejection the next represented step is strictly shorter). That case is reported, not resolved, here.
- CLI arms (accepted by `stiff-profile-run` and `stiff-benchmark`, absent from `ARMS` and from the default
  selection): `rodas5p-fast-val`, `rodas5p-fast-land`, `rodas5p-fast-ovh` (both), `rodas5p-fast-small-val`,
  `rodas5p-fast-small-land`, `rodas5p-fast-small-ovh`.

## Base export (recorded before any source change)

`BASE_PROFILE.json`: `tools/speed_profile.py` on the unmodified source at the registration commit, arms
`rodas5p-fast` (five problems) and `rodas5p-fast-small` (three small problems), rtol 1e-6, with the determinism
repeat. Its sha256 is recorded in the ledger row. The legacy arms of the new binary must reproduce it (G2).

## Systems and measurements

1. Identity: an integration test (`crates/rodas5p-integrators/tests/spd01_overhead.rs`, `--ignored` export) runs
   the five benchmark problems x rtol {1e-3, ..., 1e-9} with the dense driver (legacy and the three option sets)
   and the three small problems x the same tolerances with the small driver (legacy and the three option sets),
   comparing every output state bitwise, `attempts`, `accepted_steps`, `rejected_steps`, `jacobian_reuses`,
   `output_clipped_steps`, the full `WorkCounters` and the success flag. The six INT-03 banded cases with their
   41-point output grid (interior due times, `next < tf`) are run with the banded driver, legacy against fused.
2. Instructions: `tools/speed_profile.py` at rtol 1e-6 on the three small problems for all eight arms and on
   `brusselator-1d-200` for the four dense arms, with callgrind call records (`calls=`) of `land_capped`,
   `step_to`, `limit_step`, `limit_landed_step`, `AdaptiveStepConfig::validate` and `propose_factor` per attempt.
3. Property test (`#[cfg(test)]` in `output.rs`, since the helpers are crate-private): 1,000,000 seeded random
   `(t, h, tf, cap, last_rejected)` with epochs `t` in {0, 1e3, 1e6, 1e12, 1e100} and relative steps 1e-16..1,
   both `MaxStepPolicy` variants, unreachable targets and the represent-below-cap path, comparing the fused rule
   with the legacy triple composition; discrepancies are classified A (exact landing), B (interior step), C
   (rejection-shortened step within the residue of `tf`).

## Commands

    CARGO_TARGET_DIR=/home/user/target-speed CARGO_PROFILE_RELEASE_DEBUG=line-tables-only cargo build --release -p rodas5p-cli --locked
    SPD01_IDENTITY=research/spd01_fast_driver_overhead_20261005/IDENTITY.json cargo test --release -p rodas5p-integrators --locked --test spd01_overhead -- --ignored --nocapture --test-threads=1
    cargo test --release -p rodas5p-integrators --locked --lib output::  (property test)
    python3 tools/speed_profile.py --rodas5p /home/user/target-speed/release/rodas5p --arms rodas5p-fast,rodas5p-fast-val,rodas5p-fast-land,rodas5p-fast-ovh --problems van-der-pol-mu1000,robertson,hires,brusselator-1d-200 --scratch <scratch> --output research/spd01_fast_driver_overhead_20261005/PROFILE_DENSE.json
    python3 tools/speed_profile.py --rodas5p /home/user/target-speed/release/rodas5p --arms rodas5p-fast-small,rodas5p-fast-small-val,rodas5p-fast-small-land,rodas5p-fast-small-ovh --problems van-der-pol-mu1000,robertson,hires --scratch <scratch> --output research/spd01_fast_driver_overhead_20261005/PROFILE_SMALL.json
    python3 tools/spd01_overhead_check.py --base research/spd01_fast_driver_overhead_20261005/BASE_PROFILE.json --dense ...PROFILE_DENSE.json --small ...PROFILE_SMALL.json --identity ...IDENTITY.json --output research/spd01_fast_driver_overhead_20261005/RESULTS.json

(`RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1` for every run.)

## Gate

**PASS** if all hold:

1. **Identity.** Every option set equals the legacy driver bitwise on every point of system 1 (outputs, step counts,
   reuses, clipped steps, counters, success), including the six banded grid cases.
2. **Legacy reproduction.** The legacy arms of the new binary reproduce `BASE_PROFILE.json`: the same attempts,
   accepted/rejected counts, counters and final-state hash on every problem, and Ir per attempt within +-2 %.
3. **A removes exactly the per-attempt validation.** In the `-val` arms the per-integration difference holds exactly
   one `AdaptiveStepConfig::validate` call (the entry call; the legacy arms hold attempts + 1), and Ir per attempt
   falls by 120 to 200 on each of the three small problems in both drivers (predicted 160 plus a few instructions
   of call overhead).
4. **B removes the two re-landings.** In the `-land` arms the per-integration difference holds one `land_capped` call
   and two `step_to` calls per attempt (legacy: three and six), and Ir per attempt falls by at least 500 on each of
   the three small problems in both drivers (predicted about 2 x 170 + 4 x 54 + ~70 of `limit_step` body + the
   `accept` rescan: 615 at n = 2, 690 at n = 8).
5. **Property test.** No class A or B discrepancy with `h >= 2^16 eps max(|t|, |tf|)`. Class C discrepancies are
   counted and reported as a finding on the legacy composition; they do not fail the node.

Otherwise **FAIL**. Reported, not gated: the `-ovh` arms' totals (predicted van der Pol small 3,569 -> about
2,790, Robertson 5,047 -> about 4,260, HIRES 12,302 -> about 11,470; dense van der Pol 9,135 -> about 8,350); the
ratio to Hairer RODAS's L-0030 counts (2,947 on van der Pol, 9,157 on HIRES), which is a cross-node, cross-binary
comparison and never a gate; the n = 400 arms (expected unchanged within 0.1 %).

Kill: any identity difference; a class A or B discrepancy at a relative step >= 1e-9; a `-land` reduction below
400 Ir per attempt on van der Pol small (the removed calls were not where the cost was). Nothing is tuned after the
run; a FAIL is recorded.

## Prior information and disclosure

L-0041 measured these helpers at 17-23 % of the small driver and left them untouched. The attribution above comes
from an exploratory line-level callgrind run of the unchanged baseline in this session (`scratchpad`, same
binary as `BASE_PROFILE.json`); it informed the thresholds and is not a result of this node. The candidate and
its gates were reviewed by two independent reviewers before this registration (their corrections: absolute
removal thresholds instead of ratios, call-count gates instead of per-function Ir, the class-C disclosure, the
scope restriction to drivers that land with the same cap). This node was registered before the synthesis of the
full candidate set finished; it does not depend on that synthesis. No code of this node exists before this commit.

## Amendment before the run (2026-10-05, before any measurement of this node)

The CLI crate has no library target, so an integration test in `rodas5p-integrators` cannot reach the HIRES and
Brusselator benchmark problems. The identity export of system 1 therefore lives in the CLI crate's unit tests
(`crates/rodas5p-cli/src/stiff_benchmark.rs`, test `spd01::spd01_identity_export`, command
`SPD01_IDENTITY=... cargo test --release -p rodas5p-cli --locked --bin rodas5p -- --ignored spd01_identity_export`),
and `crates/rodas5p-integrators/tests/spd01_overhead.rs` covers the six INT-03 banded grid cases (export
`SPD01_BANDED=...`). The property test writes its counts to `SPD01_PROPERTY=...` when run with that variable.
`tools/spd01_overhead_check.py` takes `--banded` and `--property` for those two files. The gate items are
unchanged. `tools/speed_profile.py` gained the `calls=` extraction (per-callee and per-pair call counts in the
2-rep minus 1-rep difference) that gate items 3 and 4 need; the base export predates that addition and is used
for gate item 2 only, as registered.

---

## Results (appended after the run; source commit recorded in the ledger row)

Binary: release build with line tables, sha256 `4501744cebed74290e68e505fc5ec8cc4f6e2f7088c4ba646d3da2ac9987b931`, valgrind 3.22.0,
`RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1`. Outputs: `IDENTITY.json` (35 dense + 21 small identity rows),
`BANDED.json` (six INT-03 grid cases), `PROPERTY.json`, `PROFILE_DENSE.json`, `PROFILE_DENSE_B50.json`,
`PROFILE_SMALL.json`, `RESULTS.json` (first checker run) and `RESULTS_CORRECTED.json` (see the disclosure).

**Gate: PASS** (`RESULTS_CORRECTED.json`).

| Gate item | Outcome |
|---|---|
| 1. Identity | **holds**: every option set equals the legacy driver bitwise on all 35 dense and 21 small points (all output states, step counts, reuses, clipped steps, counters, success) and on the six INT-03 banded grid cases (38-39 clipped landings each; `BandedWork` equal too) |
| 2. Legacy reproduction | **holds**: the legacy arms reproduce the base export's work and final states on all eight (arm, problem) pairs; Ir per attempt within 0.0004-1.1 % (small HIRES 1.0108, the largest) |
| 3. Controller prevalidated | **holds**: the `-val` arms hold exactly 1 `validate` call per integration (legacy: attempts + 1 = 470/46/211) and remove 170.6-171.2 Ir per attempt on every small problem in both drivers (predicted 160 + call overhead) |
| 4. Fused landing | **holds**: the `-land` arms hold 1 `land_capped` and 2 `step_to` calls per attempt (legacy 3 and 6; call counts 469/938 vs 1407/2814 on van der Pol) and remove 629-678 Ir per attempt (predicted 615-690) |
| 5. Property test | **holds**: 1,000,000 samples, 616,920 compared (184,565 with an interior due time), 0 class A and 0 class B discrepancies at any step size; 14 class C discrepancies out of 76,051 firings of the post-rejection shortening |

Instructions per attempted step at rtol 1e-6 (same binary, legacy / `-val` / `-land` / `-ovh`):

| Driver | Problem | legacy | `-val` | `-land` | `-ovh` | `-ovh` / legacy |
|---|---|---|---|---|---|---|
| small (n = 2) | van der Pol | 3,579.6 | 3,408.4 | 2,949.4 | **2,778.2** | 0.776 |
| small (n = 3) | Robertson | 5,075.7 | 4,905.1 | 4,434.3 | **4,263.6** | 0.840 |
| small (n = 8) | HIRES | 12,434.8 | 12,263.8 | 11,756.4 | **11,585.5** | 0.932 |
| dense v2 | van der Pol | 9,134.7 | 8,963.5 | 8,505.5 | 8,334.3 | 0.912 |
| dense v2 | Robertson | 11,435.4 | 11,264.7 | 10,794.9 | 10,624.1 | 0.929 |
| dense v2 | HIRES | 20,567.2 | 20,396.3 | 19,889.9 | 19,718.9 | 0.959 |
| dense v2 | Brusselator n = 100 | 395,271 | 395,101 | 394,028 | 393,857 | 0.996 |
| dense v2 | Brusselator n = 400 | 3,811,738 | 3,811,567 | 3,808,623 | 3,808,452 | 0.999 |

The two removals are additive to within a few instructions (`additive_check` in the results: the `-ovh` reduction equals
the sum of the `-val` and `-land` reductions within 1 Ir per attempt). Cross-node, cross-binary comparison, not a gate:
the small `-ovh` driver on van der Pol (2,778 Ir per attempt) is 0.943x the L-0030 count of Hairer's RODAS (2,947)
and on HIRES (11,586) 1.265x (9,157).

**Class C finding for the time-rule owners.** The 14 class C discrepancies are cases where the legacy composition
re-extends a step that the post-rejection rule had shortened (the R4 rule: after a rejection the next represented
step is strictly shorter) back to the span end because its end lay within the rounding residue of `tf`; the fused rule
keeps the shortened step. The corpus never exercises this (`output_clipped_steps` and all trajectories identical), and
it is not resolved here.

**Disclosure.** The first checker run (`RESULTS.json`, verdict FAIL) failed gate items 2 and 4 for two reasons that
were mine and not the measurement's: the dense profile command of this registration omitted `brusselator-1d-50`,
which the base export contains, so item 2 found a missing row; and the checker counted the driver's one per-integration
entry call of `step_to` as per-attempt work, which exceeded its 0.01 tolerance on Robertson's 45 attempts. The missing
problem was then profiled with the same binary (`PROFILE_DENSE_B50.json`, four arms) and the checker was changed to
compare integer call counts (`land_capped == attempts`, `step_to == 2 attempts + 1`) as the gate text says; no measured
number changed and no threshold was moved. `RESULTS.json` is kept as written. Claim ceiling: counted instructions on
these problems; no wall-time claim.
