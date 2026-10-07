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

## Results (appended after the run; source commit recorded in the ledger row)

Implementation: `00991a1`, merged as `bd62a8b`; the run used `bd62a8b`. Binary: release build with line tables, sha256
`5dffdc3e199c831a305f0900204b302cd89a51dc753b8d2ddeb592a147fe6be8`, valgrind 3.22.0, `RAYON_NUM_THREADS=1
OPENBLAS_NUM_THREADS=1`. Outputs: `IDENTITY.json`, `ENSEMBLE_SMALL.json`, `ENSEMBLE_STATIC.json`, `PROFILE.json`,
`PROFILE_ROB8.json`, `RESULTS.json`. The contract test passed (5 tests); all callgrind runs were deterministic.

**Gate: PASS.**

| Gate item | Outcome |
|---|---|
| 1. Identity | **holds**: `rodas5p-fast-small-static` equals the L-0041 driver bitwise on all 21 points (and so do the stages-only, static-ovh and solves-only sub-arms); 64-member ensemble: attempts 20,083 and checksum -47.78788524467381 in both arms |
| 2. Tables | **holds**: the fixed tables equal `rodas5p_coefficients()` bit for bit; the construction-time check refuses a table one bit off (unit test) |
| 3. Instructions, rtol 1e-6 | **holds**: static / legacy = **0.838** van der Pol (3,755.6 -> 3,148.7 Ir per attempt; gate <= 0.90), **0.848** Robertson (5,192.7 -> 4,402.8; <= 0.93), **0.736** HIRES (12,333.4 -> 9,072.7; <= 0.97) |
| 4. Legacy reproduction | **holds** in work and final state on all three problems; Ir per attempt drift of the legacy arm against `BASE_PROFILE.json` (reported): **1.052** van der Pol, 1.029 Robertson, 1.003 HIRES |

The legacy arm drifted: the small driver's existing path is now one generic function shared with the static
variants, and it costs 5.2 % more instructions per attempt on van der Pol than in the SPD01 base binary, with
identical results. Part of the same-binary ratio is therefore this drift. Against the base binary (cross-binary,
reported, not the gate) the static arm is 0.882 (van der Pol), 0.872 (Robertson) and 0.737 (HIRES), still inside the
registered thresholds. The drift of the default small driver is a cost of this change and is not hidden by the
gate; a follow-up could keep the legacy function separate if the default path matters.

Reported, not gated:

- Stages only (`-static-stages`) / legacy: 0.843, 0.854, 0.951. The index-loop solves add little at N = 2 and 3
  (static / static-stages 0.995, 0.993) but 0.774 at N = 8 (HIRES: 2,656 Ir per attempt). The refuters' split
  prediction for HIRES (0.80-0.88 against 0.95-0.96) is resolved this way: stages alone 0.951, with the solves 0.736.
- With SPD01's two options (`-static-ovh`): 0.625 / 0.691 / 0.667 of the legacy arm and 0.795 / 0.819 / 0.716 of
  `-small-ovh` (2,348 / 3,588 / 8,224 Ir per attempt).
- Robertson at rtol 1e-8 (`PROFILE_ROB8.json`): **103 attempts**, not 195 as the registration says (195 is rtol 1e-9; a
  factual error of the registration, found by the implementer before the run; rtol 1e-8 was kept). Static / legacy
  0.862 (5,020.5 -> 4,326.3).
- Allocations per integration (implementer's dev check, not part of the recorded outputs): 40 legacy, 16 static
  (the heap coefficient lists are not built).

Deviations from the registration: the switches are fields of a new `Rodas5pFastSmallOptions { fast, static_stages,
static_solves }`, not of `Rodas5pFastOptions` (adding a field would break the exhaustive struct literals of the frozen
`tests/spd01_overhead.rs`); the dense and banded drivers therefore cannot receive them at all. The checker also takes
`--contract-passed` and requires 64 members and equal rtol in the ensemble files. Claim ceiling: counted
instructions of the small-n research driver at N = 2, 3, 8; no wall-time claim.
