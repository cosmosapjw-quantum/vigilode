# Preregistration: instruction count of the INT-03 banded pipeline through a CLI arm (speed research node SPD03)

Speed research cycle of 2026-10-05 (branch `audit/rvj-speed-research-20261005`, base `4de1f88`). Counted
instructions and counted banded operations only; the timing authority is on HOLD and nothing here is a wall-time
claim.

## Question

The INT-03 banded pipeline (L-0054) assembles `W` on the band, pivots over `lower` rows and updates `lower` rows
over `upper + lower` columns; it reproduces v2 bit for bit and its counted operations per attempt grow with slope
1.004-1.007 in n (32,259 at n = 256, 129,795 at n = 1024). It is reachable only from an integration test: no
`stiff-profile-run` arm exists, so it has no instruction count, and v2's 3.81 M instructions per attempt at n = 400
(`BASE_PROFILE.json`) are dominated by n^2 passes the banded kernel does not have. Fitting Ir = A n + B n^2 to v2's
n = 100 and n = 400 profiles gives A ~ 2,090 and B ~ 18.6, so the n-linear work the banded arm still pays (stage
right-hand sides, 64 axpys, copies, scans, controller) is about 0.74-0.84 M at n = 400 before its own LU.

How many instructions per attempt does the banded pipeline take on the CLI Brusselators, and how many instructions
per counted banded operation?

## Change (CLI and measurement only for part A; one opt-in kernel variant for part B)

`crates/rodas5p-cli/src/stiff_benchmark.rs`, `main.rs`, `crates/rodas5p-integrators/src/rodas5p_fast.rs`:

- **A.** A native `BandedJacobian` for the Brusselator (`lower = upper = 2`) writing the same expressions as the
  dense fill (`2uv - 4 - 2c`, `u^2`, `3 - 2uv`, `-u^2 - 2c`, `c`) at band offsets, validated bitwise against the
  dense fill on the parity states in a contract test; the arm `rodas5p-fast-banded` dispatching to
  `integrate_rodas5p_fast_banded_observed` (refused on problems without a band); `profile_run` and the benchmark
  row emit `BandedWork` (`factor_operations`, `solve_operations`, `stored_slots`) for the banded arms beside the
  unchanged six counters; a `brusselator-1d-500` problem (n = 1000) available to `stiff-profile-run` but not in
  the default benchmark selection, reported for the slope only.
- **B (secondary).** A kernel variant whose inner loops run over row slices (`split_at_mut`, zipped ranges) instead
  of `self.at(i, j)` indexing with a width multiply and a bounds check per operand. It performs the same
  floating-point operations in the same order per element (row update for ascending `j`, forward solve for
  ascending `i`, back solve for ascending `j`) and keeps the strict `>` pivot search, so factors and solutions are
  bitwise identical by construction. Selected through a new entry function
  `integrate_rodas5p_fast_banded_observed_with_kernel(.., BandedKernel::Slices, ..)`; `BandedJacobian` and the
  existing entry are unchanged (the struct is built by literals in the INT-03 tests). Arm
  `rodas5p-fast-banded-slices`.

## Systems and measurements

1. Identity: `crates/rodas5p-integrators/tests/spd03_banded_arm.rs` (`--ignored` export) runs `brusselator-1d-50`
   and `brusselator-1d-200` x rtol {1e-3, ..., 1e-9} with the CLI's native band against v2 through the library,
   comparing every output state bitwise, step counts, reuses, clipped steps, counters and success (14 points, both
   kernels); and the six INT-03 cases (Brusselator 32/128/512 cells, Burgers 64/256/1024) with the 41-point grid,
   slices kernel against the indexed kernel, outputs and `BandedWork` bitwise.
2. Band fill contract (`cargo test`): the native band fill equals the dense fill entry for entry on the parity
   states of both Brusselators and of n = 1000.
3. Instructions: `tools/speed_profile.py` at rtol 1e-6 on `brusselator-1d-50`, `brusselator-1d-200` and
   `brusselator-1d-500` for `rodas5p-fast` (n = 100 and 400 only), `rodas5p-fast-banded` and
   `rodas5p-fast-banded-slices`; `BandedWork` per attempt from the JSON line, so Ir per counted operation is
   recorded; by-line attribution of `rodas5p_fast.rs` kept.

## Commands

    CARGO_TARGET_DIR=/home/user/target-speed CARGO_PROFILE_RELEASE_DEBUG=line-tables-only cargo build --release -p rodas5p-cli --locked
    SPD03_IDENTITY=research/spd03_banded_arm_instructions_20261005/IDENTITY.json cargo test --release -p rodas5p-integrators --locked --test spd03_banded_arm -- --ignored --nocapture --test-threads=1
    cargo test --release -p rodas5p-integrators --locked --test spd03_banded_arm
    cargo test --release -p rodas5p-cli --locked  (band fill contract)
    python3 tools/speed_profile.py --rodas5p /home/user/target-speed/release/rodas5p --arms rodas5p-fast,rodas5p-fast-banded,rodas5p-fast-banded-slices --problems brusselator-1d-50,brusselator-1d-200 --scratch <scratch> --output research/spd03_banded_arm_instructions_20261005/PROFILE.json
    python3 tools/speed_profile.py --rodas5p /home/user/target-speed/release/rodas5p --arms rodas5p-fast-banded,rodas5p-fast-banded-slices --problems brusselator-1d-500 --scratch <scratch> --output research/spd03_banded_arm_instructions_20261005/PROFILE_N1000.json
    python3 tools/spd03_banded_check.py --base research/spd01_fast_driver_overhead_20261005/BASE_PROFILE.json --profile ...PROFILE.json --n1000 ...PROFILE_N1000.json --identity ...IDENTITY.json --output research/spd03_banded_arm_instructions_20261005/RESULTS.json

(`RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1` for every run.)

## Gate

**PASS** if all hold:

1. **Identity.** Both banded arms equal v2 bitwise on all 14 points, and the slices kernel equals the indexed
   kernel bitwise (outputs and `BandedWork`) on the six INT-03 cases.
2. **Band fill.** The native band fill equals the dense fill on every parity state.
3. **Instructions at n = 400.** Ir per attempt `rodas5p-fast-banded` / `rodas5p-fast` <= 0.40 on
   `brusselator-1d-200` (predicted 0.25-0.40: the n-linear floor plus about 50 k counted operations at 6-12
   instructions each).
4. **Counted operations.** `BandedWork` operations per attempt between n = 100 and n = 400 grow with a slope in
   [0.95, 1.10] (log-log), as in L-0054.
5. **Legacy reproduction.** `rodas5p-fast` on the new binary reproduces `BASE_PROFILE.json` on both Brusselators
   (attempts, counts, counters, final-state hash; Ir per attempt within +-2 %).

Otherwise **FAIL**. Kill: any identity or fill difference; banded / v2 > 0.55 at n = 400 (the kernel's instruction
overhead per counted operation would exceed 20x and the pipeline's benefit is not what its operation counts
suggest).

Secondary, reported with its own pass/fail but not deciding the verdict: the n = 100 ratio (expected 0.66-0.85,
stage work dominates there), the n = 1000 point and the Ir-per-counted-operation of both kernels, and part B:
slices / banded <= 0.95 at n = 400 (an estimate of 0.73-0.90; if it is not bitwise identical it is dropped and
reported).

## Prior information and disclosure

L-0054 (parity and counted operations), L-0033 (v2). The fit A n + B n^2 and the 0.74-0.84 M floor are from the
exploratory line profiles of this session on the unchanged baseline. Two independent reviewers corrected the
prediction (0.25-0.40 instead of 0.15-0.30), moved the n = 100 ratio and part B to reported items, required the
same-binary v2 numbers and full-state identity through the library. A PASS applies to problems with a declared
band; the default benchmark set contains only `brusselator-1d-50`, where the gain is modest. Registered before
the full synthesis finished. No code of this node exists before this commit.

## Amendment before the run (2026-10-07, before any measurement of this node)

As in SPD01, the CLI crate has no library target, so the 14-point CLI identity export lives in the CLI crate's unit
tests (`crates/rodas5p-cli/src/stiff_benchmark.rs`, test `spd01::spd03_identity_export`, command
`SPD03_IDENTITY=... cargo test --release -p rodas5p-cli --locked --bin rodas5p -- --ignored spd03_identity_export`),
and the band-fill contract is the CLI unit test `spd01::spd03_band_fill_equals_dense_fill`.
`crates/rodas5p-integrators/tests/spd03_banded_arm.rs` covers the six INT-03 cases, slices kernel against indexed
kernel (export `SPD03_INT03=...`, file `INT03.json`). `tools/spd03_banded_check.py` takes `--int03` for that file and
gates item 1 also on equal `BandedWork` of the two kernels on the 14 CLI points. The node shares
`Rodas5pFastOptions` and the CLI with SPD01/SPD02, whose recorded runs precede this one; the legacy-reproduction item
(5) compares against SPD01's `BASE_PROFILE.json` as registered. Gate items and thresholds are unchanged.

---

## Results (appended after the run; source commit recorded in the ledger row)

Binary: release build with line tables, sha256 `2e6a119e5a61a84ddfb425bf9651f1d8052ca03a84e2b04a8c621cfc893cfe92`, valgrind 3.22.0,
`RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1`. Outputs: `IDENTITY.json` (14 CLI rows), `INT03.json` (six cases),
`PROFILE.json`, `PROFILE_N1000.json`, `RESULTS.json`. Band-fill contract (CLI unit test) passed.

**Gate: PASS.**

| Gate item | Outcome |
|---|---|
| 1. Identity | **holds**: both banded arms equal v2 bitwise on all 14 CLI points (all output states, step counts, reuses, clipped steps, counters, success) and the two kernels report equal `BandedWork` there; the slices kernel equals the indexed kernel bitwise (outputs and `BandedWork`) on all six INT-03 cases |
| 2. Band fill | **holds**: the native fill equals the dense fill bit for bit on every parity state of both Brusselators and of n = 1000 |
| 3. Instructions at n = 400 | **holds**: banded / v2 = **0.255** (3,836,518 -> 978,338 Ir per attempt; gate <= 0.40, predicted 0.25-0.40) |
| 4. Counted operations | **holds**: log-log slope 1.011 between n = 100 and 400 (12,447 -> 50,547 banded operations per attempt) |
| 5. Legacy reproduction | **holds**: v2 on this binary reproduces the base work and final states; Ir per attempt 1.0064x (n = 400) and 1.0154x (n = 100) of the base |

Secondary, reported: n = 100 banded / v2 = 0.617 (stage work dominates there, as predicted 0.66-0.85; it came out
lower); n = 1000: 2,438,632 Ir per attempt with 126,747 operations per attempt (operation slope 1.003 from n = 400,
instruction slope about 1.0); **part B fails its secondary item**: the slices kernel is bitwise identical but costs
1.012x (n = 400) and 1.017x (n = 100) the indexed kernel's instructions (gate <= 0.95). The `at(i, j)` indexing was not
the cost: the rows are 2-5 entries long and slice setup costs as much as the bounds checks it removes.

"Ir per counted operation" in `RESULTS.json` (19.4 at n = 400, 19.9 at n = 100) divides the whole attempt (stage
right-hand sides, axpys, controller, assembly) by the banded LU and solve operations only; it is not the cost of a
banded operation alone. Attribution of the banded arm at n = 400 (by file): Rust std iterator and indexing code 50 %,
the driver body 36 %, the Brusselator right-hand side and band fill 9.6 %, `memset` 4.5 % (the factors buffer is
zeroed before every factorization, `rodas5p_fast.rs:186`).

Cross-node comparison, not a gate: at n = 400 the banded arm's 0.98 M instructions per attempt is 0.11x the L-0030 count
of Hairer's RODAS (8.83 M, which factors the dense matrix). Claim ceiling: counted instructions on the 1-D Brusselator
with a declared band; no wall-time claim.

## Changes after the run (2026-10-07, from the independent review; appended)

- `brusselator-1d-500` was added to `benchmark_problems()` for this node. That made the SPD01/SPD02 identity exports
  and two existing Jacobian tests iterate the n = 1000 problem, so those exports could no longer be re-run with
  their recorded results. It now lives in a separate `profile_problems()`, used only by `stiff-profile-run` and the
  band-fill test. The recorded commands of this node are unchanged: `stiff-profile-run --problem brusselator-1d-500`
  still finds the problem.
- `factor_slices` now adds each column's operations to `BandedWork` as it goes, as the indexed kernel does. Before
  the change, a factorization that failed partway dropped the operations of the earlier columns. No recorded case
  fails a factorization, so the recorded `BandedWork` is unchanged.
- `integrate_rodas5p_fast_banded_observed_with_kernel` now refuses a band outside the matrix, as the existing banded
  entry does.
