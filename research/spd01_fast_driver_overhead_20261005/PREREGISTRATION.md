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
