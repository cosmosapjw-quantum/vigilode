# Preregistration: column-extent LU above a fixed size only (speed research node SPD09)

Second speed research cycle (2026-10-07), branch `audit/rvj-speed-research-20261005`, base `bdcc903`. Follow-up of
SPD02 (L-0082, FAIL). **Counted instructions only; no wall-time claim; the timing authority stays on HOLD.**

## Question

SPD02's column-extent LU is bitwise identical to v2 and cuts the dense-storage fast driver's instructions per attempt
to 0.517x at n = 400 and 0.773x at n = 100, but costs 1.036-1.062x at n <= 8, where the extent bookkeeping outweighs
the scans it removes. Does applying it only above a fixed dimension keep the gain at n >= 100 and remove the loss at
small n?

The threshold is fixed here, before any measurement, at the existing constant `RODAS5P_FAST_SMALL_LU_MAX = 64`
(matrices with at most 64 rows keep the legacy LU). It is not fitted to SPD02's data: SPD02 has no point between
n = 8 and n = 100. Two sizes no earlier node measured test it: n = 60 (just below) and n = 80 (just above).

## Change (opt-in)

- `FastLuPolicy::ColumnExtentsAbove64`: the column-extent LU when `n > RODAS5P_FAST_SMALL_LU_MAX`, the legacy
  in-place LU otherwise. Driver id `rodas5p-fast-transformed-v3-colext64`; CLI arm `rodas5p-fast-colext64`.
- `profile_problems()` gains `brusselator-1d-30` (n = 60) and `brusselator-1d-40` (n = 80); `benchmark_problems()` is
  unchanged.

## Systems

Identity: the five benchmark problems at the seven tolerances (35 points) plus the two new sizes at rtol 1e-6, the
new arm against `rodas5p-fast` through the library (all output states, step counts, reuses, clipped steps,
counters). Instructions: `tools/speed_profile.py` at rtol 1e-6 for `rodas5p-fast`, `rodas5p-fast-colext` and
`rodas5p-fast-colext64` on van der Pol, Robertson, HIRES, brusselator-1d-30, -40, -50 and -200.

## Commands

    CARGO_PROFILE_RELEASE_DEBUG=line-tables-only cargo build --release -p rodas5p-cli --locked
    SPD09_IDENTITY=research/spd09_colext_threshold_20261007/IDENTITY.json cargo test --release -p rodas5p-cli --locked --bin rodas5p -- --ignored spd09_identity_export
    python3 tools/speed_profile.py --rodas5p RELEASE_BINARY --arms rodas5p-fast,rodas5p-fast-colext,rodas5p-fast-colext64 --problems van-der-pol-mu1000,robertson,hires,brusselator-1d-30,brusselator-1d-40,brusselator-1d-50,brusselator-1d-200 --scratch SCRATCH --output research/spd09_colext_threshold_20261007/PROFILE.json
    python3 tools/spd09_threshold_check.py --profile ...PROFILE.json --identity ...IDENTITY.json --output research/spd09_colext_threshold_20261007/RESULTS.json

## Gate (the new arm against `rodas5p-fast`, same binary)

**PASS** if all hold:

1. **Identity.** Bitwise on all 37 points.
2. **No loss at or below the threshold.** Ir per attempt ratio <= 1.005 on van der Pol, Robertson, HIRES and
   brusselator-1d-30 (n = 60).
3. **Gain above it.** Ratio <= 0.85 on brusselator-1d-40 (n = 80), <= 0.80 on brusselator-1d-50 (n = 100) and <= 0.60
   on brusselator-1d-200 (n = 400).

Otherwise **FAIL**. Reported, not gated: the always-on `rodas5p-fast-colext` arm on the two new sizes (where the
crossover lies), and the legacy arm's drift against `BASE_PROFILE.json` (SPD02 recorded +2.4 % at n = 400 from carrying
the second LU; this node does not re-gate it).

## Prior information

L-0082 (SPD02). No code of this node exists before this commit.

## Results (appended after the run; source commit recorded in the ledger row)

Implementation: `00991a1`, merged as `bd62a8b`; the run used `bd62a8b` (the same binary as SPD06, sha256
`5dffdc3e199c831a305f0900204b302cd89a51dc753b8d2ddeb592a147fe6be8`), valgrind 3.22.0. Outputs: `IDENTITY.json`,
`PROFILE.json`, `RESULTS.json`. The non-ignored identity test passed; all callgrind runs were deterministic.

**Gate: PASS.**

| Gate item | Outcome |
|---|---|
| 1. Identity | **holds**: `rodas5p-fast-colext64` equals `rodas5p-fast` bitwise on all 37 points |
| 2. No loss at or below n = 64 | **holds**: 1.0000 van der Pol, 1.0001 Robertson, 1.0000 HIRES, 1.0000 brusselator-1d-30 (n = 60); gate <= 1.005 |
| 3. Gain above n = 64 | **holds**: **0.833** brusselator-1d-40 (n = 80; gate <= 0.85), **0.787** brusselator-1d-50 (n = 100; <= 0.80), **0.531** brusselator-1d-200 (n = 400; <= 0.60) |

Reported, not gated:

- The always-on `rodas5p-fast-colext` arm costs 1.036 / 1.050 / 1.066 at n = 2 / 3 / 8 (SPD02 measured 1.036-1.062)
  but **0.888 at n = 60**: the crossover lies below n = 60, so the threshold 64, fixed before the run and not
  fitted, leaves a gain of about 11 % at n = 60 unused. Where between n = 8 and 60 the crossover is was not measured.
- Legacy drift of `rodas5p-fast` against `BASE_PROFILE.json` with equal work and final states: 1.014 (van der
  Pol), 1.017 (Robertson), 1.022 (HIRES), 1.014 (n = 100), 1.006 (n = 400), from carrying the extra LU code paths
  (SPD02 recorded +2.4 % at n = 400).
- The colext64 driver id is reported also below the threshold, where the legacy LU runs; the result's `lu` field
  says which LU ran.

Claim ceiling: counted instructions of the dense-storage fast driver on the listed problems; no wall-time claim.
