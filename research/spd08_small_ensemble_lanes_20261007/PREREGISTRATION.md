# Preregistration: lane-batched small-n ensemble driver (speed research node SPD08)

Second speed research cycle (2026-10-07), branch `audit/rvj-speed-research-20261005`, base `bdcc903`. Candidate
ENS-LANES of the first cycle's DAG (both refuters kept it; their corrections are applied). **Counted instructions
only; no wall-time claim; the timing authority stays on HOLD.** SSE2 vector instructions count as single
instructions.

## Question

The small driver integrates an ensemble member by member (`stiff-ensemble-run`, L-0041: 64 van der Pol trajectories,
`mu = 1000 (1 + k / 64)`, rtol 1e-6, 1,122,414 Ir per trajectory recorded). Its body at n = 2 is mostly loop control
around about 370 flops per attempt. Does integrating B members concurrently, each with its own step sequence and its
exact scalar arithmetic, while running the elementwise stage work over structure-of-arrays storage, reduce the
instructions per trajectory with every member bitwise unchanged?

## Change (new research driver; the small driver unchanged apart from visibility)

- `crates/rodas5p-integrators/src/rodas5p_fast_small_batch.rs`: `integrate_rodas5p_fast_small_batch::<N, B, P>` with
  B lanes. Each lane has its own `(t, h, y)`, controller state, output collector, counters and freshness flag, as the
  small driver, and takes members from a queue in member order. In each round every active lane makes one attempt at
  its own `(t, h)`; the elementwise stage arithmetic (the A and C combinations with per-lane `c / h` quotients and
  `!= 0.0` guards, the right-hand sides through a `BatchProblem` trait with the scalar problem's expression order,
  the finiteness scans, the error norm) runs over `[[f64; B]; N]` arrays; the LU and the solves call the small
  driver's per-lane routines (made `pub(crate)`: a visibility-only change to `rodas5p_fast_small.rs`, disclosed).
  Every member performs exactly the scalar driver's floating-point operation sequence. A lane whose attempt fails
  (typed linear-solve or non-finite failure) reproduces the scalar driver's partial counter updates and freshness
  reset.
- The batch driver mirrors the legacy composition of the small driver (not SPD01's options), so the identity gate
  compares against the same composition.
- CLI: `stiff-ensemble-run` gains `--repetitions` and the arms `rodas5p-fast-small-batch4` and
  `rodas5p-fast-small-batch8` (B fixed before the run; no other B is tried), and `--dump-members` writing per-member
  JSON (attempts, accepted, rejected, reuses, counters, output times, internal and clipped steps, final state bits).
  The checksum is accumulated in member order after all members finish.

## Base export (recorded before any source change)

`BASE_ENSEMBLE.json`: callgrind Ir of `stiff-ensemble-run --arm rodas5p-fast-small --members 64` and `--members 32`
(the L-0041 protocol) on the unmodified source with the line-tables binary, with attempts and checksum.

## Systems and protocol

The 64-member van der Pol ensemble at rtol 1e-6. Instructions per trajectory = (Ir(64 members, 2 repetitions) -
Ir(64 members, 1 repetition)) / 64 for each arm in one binary, with the 1-repetition run repeated as a determinism
check; the L-0041 64-minus-32 figure is reported as a cross-check. A contract test injects failures (an oversized
initial step causing typed rejections, and a non-finite right-hand side in one lane) and compares per-lane counters
and outputs with the scalar driver.

## Commands

    CARGO_PROFILE_RELEASE_DEBUG=line-tables-only cargo build --release -p rodas5p-cli --locked
    cargo test --release -p rodas5p-integrators --locked --test spd08_small_ensemble_lanes
    python3 tools/spd08_ensemble_profile.py --rodas5p RELEASE_BINARY --arms rodas5p-fast-small,rodas5p-fast-small-batch4,rodas5p-fast-small-batch8 --members 64 --scratch SCRATCH --output research/spd08_small_ensemble_lanes_20261007/PROFILE.json
    python3 tools/spd08_ensemble_check.py --base ...BASE_ENSEMBLE.json --profile ...PROFILE.json --output research/spd08_small_ensemble_lanes_20261007/RESULTS.json

## Gate

**PASS** if all hold:

1. **Members identical.** All 64 members equal the scalar small driver bitwise in both batch arms (final state bits,
   attempts, accepted, rejected, reuses, counters, output times, internal and clipped steps).
2. **Checksum.** Equal to the scalar arm's.
3. **Failure masking.** The contract test's injected failures give the scalar driver's per-lane counters and outputs.
4. **Instructions.** Ir per trajectory of `batch8` <= 0.80 x the scalar arm's (same binary, repetition protocol);
   `batch4` reported. Kill: both batch arms above 0.90 x.
5. **Legacy reproduction.** The scalar arm of the new binary reproduces `BASE_ENSEMBLE.json`: attempts 20,083 and
   the checksum exactly, Ir per trajectory (64-minus-32 protocol) within +-2 %.

Otherwise **FAIL**. Predicted: `batch8` 0.65-0.75x. Ceiling: about 1,500 Ir per attempt of per-lane clock and
controller work is not batched, so no ratio below about 0.42 is possible. Claim ceiling: a research ensemble driver
on concurrent same-N trajectories; no client in the repository integrates such ensembles today.

## Prior information

L-0041 (small driver and the 64-member ensemble). No code of this node exists before this commit.

## Results (appended after the run; source commit recorded in the ledger row)

Implementation: `3b758ab`, merged with SPD06 in `364f9f8` and into the branch as `fd998ca`; the run used `fd998ca`.
Binary: release build with line tables, sha256 `2416b7afc2db4b72abca908dab56055e18f7b087a4b45f83cb7a90fe6e6c7103`,
valgrind 3.22.0. Outputs: `CONTRACT.json` (failure-masking outcome, written by the contract test after its
assertions held; 4 tests passed), `PROFILE.json` with the three member dumps `PROFILE.members.*.json`, `RESULTS.json`.
The profiler was run with `--dump-members`, which the registered command line omitted although gate item 1 needs the
dumps (the flag existed in the profiler committed with the base).

**Gate: FAIL (kill).**

| Gate item | Outcome |
|---|---|
| 1. Members identical | **holds**: all 64 members of both batch arms equal the scalar arm (dumps compared field by field) |
| 2. Checksum | **holds**: -47.78788524467381 (bits `c047e4d96c776fff`), attempts 20,083 in all arms |
| 3. Failure masking | **holds**: B = 4 and 8; oversized step: 27 typed linear-solve failures in 3 lanes; a member whose factorizations all fail: 25 failures; a non-finite right-hand side in one lane: 44 failures there, 0 elsewhere; 40 members compared, all identical |
| 4. Instructions | **fails, kill**: Ir per trajectory (2-minus-1 repetitions) batch8 / scalar = **0.924**, batch4 0.937 (gate <= 0.80; kill above 0.90 for both); 64-minus-32 cross-check 0.922 and 0.936 |
| 5. Legacy reproduction | **fails**: the scalar arm costs 1,222,712 Ir per trajectory (64-minus-32) against 1,126,013 in `BASE_ENSEMBLE.json` (1.086; tolerance +-2 %); attempts and checksum exact |

The prediction (0.65-0.75) was wrong by a wide margin. Batching the elementwise stage arithmetic over lanes saves
about 78,000-93,000 Ir per trajectory (of 1.22 M), i.e. about 250-300 Ir per attempt (313.8 attempts per
trajectory); per-lane control flow,
LU, solves, controller and output clock dominate at N = 2. Against the base binary the batch8 arm (1,131,170 Ir per
trajectory) is 1.005x the scalar small driver as recorded before any change: in absolute terms the batch arm only
recovers the scalar arm's drift.

The scalar arm drifted by 8.6 %: the small driver's legacy path is now one generic function shared with SPD06's
static variants (SPD06 measured +5.2 % per attempt on van der Pol in its binary), and the ensemble runner now keeps
every member's result until all finish (for the member-order checksum and dumps). Both changes leave the results
bitwise unchanged. Item 5 is a validity failure of the comparison baseline; with item 4 failing in both binaries'
terms, the direction (lane batching of the small driver at N = 2) is closed. Claim ceiling: counted instructions of
a research ensemble driver; no wall-time claim.

## Corrections after the run (2026-10-07, from the independent review; appended)

- "Against the base binary the batch8 arm is 1.005x" (also in the ledger claim of L-0089) divided the 2-minus-1
  repetitions figure (1,131,170) by the base's 64-minus-32 figure (1,126,013). With the same protocol on both sides
  it is 1,126,931 / 1,126,013 = **1.0008x**. The conclusion stands: in absolute terms the batch arm only recovers the
  scalar arm's drift.
- The drift's attribution to SPD06's shared generic function and to the ensemble runner keeping all member results
  is unverified (see the SPD06 correction: the dense arm drifted 1.4-2.2 % in SPD06/SPD09's binary too).
No verdict and no gated number changes.
