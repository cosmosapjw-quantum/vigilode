# Preregistration: fixed stage structure in the small-n fast driver (speed research node SPD06)

Second speed research cycle (2026-10-07), branch `audit/rvj-speed-research-20261005`, base `bdcc903`. Candidate
SMALLN-STATIC-STAGES of the first cycle's DAG with the refuters' corrected mechanism. **Counted instructions only;
no wall-time claim; the timing authority stays on HOLD.**

## Question

The small driver (`rodas5p_fast_small.rs`, L-0041) has a compile-time dimension, and its inner length-N loops are
already unrolled and vectorized (the refutation of OVH-STATIC-STAGE). What remains in the stage combination is the
outer loop over heap `Vec<Vec<(usize, f64)>>` coefficient lists: per coefficient entry a runtime `j`, a `u[j]`
bounds check, the iterator step, a `c / h` division and the `!= 0.0` guard, and a store and reload of the stage
vector between consecutive axpys (about 250 Ir per attempt at n = 2 and 470 at n = 8 on the stage lines alone,
`BASE_PROFILE.json`). The RODAS5P transformed `A` and `C` are fully strictly lower triangular (row `i` has exactly `i`
nonzeros) and `b_code` is full. Does a fixed stage structure remove that loop control at bitwise the same results?

## Change (opt-in; the L-0041 driver and its heap lists unchanged)

`crates/rodas5p-integrators/src/rodas5p_fast_small.rs` and the CLI:

- `Rodas5pFastOptions::static_stages: bool` (default false; the small driver only). When set, the workspace keeps
  fixed `[[f64; 8]; 8]` tables for `A` and `C` and `[f64; 8]` for `b_code`, filled from `rodas5p_coefficients()` at
  construction (no literals; one source of truth), and a stage function per stage index (const generic or macro
  expanded) whose loops over `j in 0..I` and over the components have constant trip counts. Each component is
  accumulated in the same order as today's axpy sequence (same operations, same order, hence bitwise identical),
  the `c / h` quotients of a stage are computed once into a fixed array (same values), and the `!= 0.0` guard is kept.
- Secondary sub-arm (`static_solves`, reported with its own identity requirement): the triangular solves of
  `lu_solve_in_place` written as index loops instead of slice and zip constructions, same operations in the same order.
- CLI arms (not in `ARMS`): `rodas5p-fast-small-static` (fixed stages and index solves: the gated arm),
  `rodas5p-fast-small-static-stages` (fixed stages only: reported), `rodas5p-fast-small-static-ovh` (gated arm plus
  SPD01's two options: reported); `stiff-ensemble-run` accepts `rodas5p-fast-small-static`.

## Systems

Identity: van der Pol (N = 2), Robertson (N = 3), HIRES (N = 8) at rtol 1e-3 ... 1e-9 (21 points) against the
L-0041 arm, through the library, all output states, step counts, reuses, clipped steps, counters; the 64-member van
der Pol ensemble checksum. Instructions: `tools/speed_profile.py` at rtol 1e-6 on the three problems for all arms
(by-line attribution kept), and at rtol 1e-8 on Robertson (195 attempts; Robertson's 45 attempts at 1e-6 make
per-integration savings, such as the 17 heap allocations of the lists, look like per-attempt savings).

## Commands

    CARGO_PROFILE_RELEASE_DEBUG=line-tables-only cargo build --release -p rodas5p-cli --locked
    cargo test --release -p rodas5p-integrators --locked --test spd06_small_static_stages
    SPD06_IDENTITY=research/spd06_small_static_stages_20261007/IDENTITY.json cargo test --release -p rodas5p-integrators --locked --test spd06_small_static_stages -- --ignored --nocapture
    python3 tools/speed_profile.py --rodas5p RELEASE_BINARY --arms rodas5p-fast-small,rodas5p-fast-small-static,rodas5p-fast-small-static-stages,rodas5p-fast-small-ovh,rodas5p-fast-small-static-ovh --problems van-der-pol-mu1000,robertson,hires --scratch SCRATCH --output research/spd06_small_static_stages_20261007/PROFILE.json
    python3 tools/speed_profile.py ... --rtol 1e-8 --problems robertson --output research/spd06_small_static_stages_20261007/PROFILE_ROB8.json
    python3 tools/spd06_static_check.py ... --output research/spd06_small_static_stages_20261007/RESULTS.json

## Gate

**PASS** if all hold:

1. **Identity.** The gated arm equals the L-0041 driver bitwise on all 21 points and the ensemble checksum.
2. **Tables.** The fixed tables equal `rodas5p_coefficients()` bit for bit (unit test and a construction-time check
   that returns an error on mismatch).
3. **Instructions** (gated arm against `rodas5p-fast-small`, same binary, rtol 1e-6): <= 0.90 on van der Pol, <= 0.93
   on Robertson, <= 0.97 on HIRES. Each threshold is both the PASS and the kill line; a result above it is FAIL
   ("gain too small; direction closed") with the measured ratio preserved.
4. **Legacy reproduction.** `rodas5p-fast-small` on the new binary reproduces `BASE_PROFILE.json` (SPD01) in work and
   final state; its Ir per attempt is reported (SPD02 showed that carrying a second code path can move it by a few
   percent; that drift is reported, not gated, here).

Predicted: van der Pol 0.85-0.90, Robertson 0.88-0.93, HIRES 0.95-0.96 (the refuters disagreed at HIRES, 0.80-0.88
against 0.95-0.96; the gate takes the conservative side). The secondary arms are reported; the `static_solves`
sub-arm is dropped and reported if it is not bitwise identical. The by-line attribution is descriptive.

## Prior information

L-0041 (small driver), L-0081 (SPD01, clock and controller overhead), the refutation of OVH-STATIC-STAGE (the inner
loops are already vectorized). No code of this node exists before this commit.
