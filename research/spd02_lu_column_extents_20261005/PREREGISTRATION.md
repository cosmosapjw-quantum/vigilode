# Preregistration: column extents in the in-place zero-skipping LU of the fast driver (speed research node SPD02)

Speed research cycle of 2026-10-05 (branch `audit/rvj-speed-research-20261005`, base `4de1f88`). Counted
instructions only; the timing authority is on HOLD and nothing here is a wall-time claim.

## Question

v2 (L-0033) factors `W = I/(h gamma) - J` in place with row extents and zero-multiplier skipping, which cut the
n = 400 Brusselator to 0.397x of v1 but left 3.74 M instructions per attempt (3.81 M on this host's line-tables
binary, `BASE_PROFILE.json`). The line-level attribution of the baseline (exploratory, this session) shows that
the stage arithmetic is a small part of that: the pivot search `for i in k + 1..n` (`rodas5p_fast.rs:557-563`)
and the zero-multiplier test over every row below `k` (`rodas5p_fast.rs:584-587`) each visit n(n-1)/2 = 79,800
entries per factorization, and together with their inlined iterator and comparison code cost about 2.4 M of the
3.81 M instructions per attempt (64 %), while the actual row updates (`:593`) cost about 6 k. The `W = -J` copy
(`:370`, about 0.39 M) and the per-row `rposition` extent scan (`:379`, about 0.24 M with its iterator code) are the
other n^2 terms.

Does an exact column-extent bound remove the two column scans with bit-for-bit the same factors, pivots and
trajectories?

## Change (opt-in; the existing LU and every default unchanged)

`crates/rodas5p-integrators/src/rodas5p_fast.rs` and the CLI:

- `Rodas5pFastLu::InPlaceColumnExtents`, selected by `Rodas5pFastOptions::lu_policy = FastLuPolicy::ColumnExtents`
  (default `Legacy`; the options struct is introduced by node SPD01 and this node adds the field). Driver id
  `rodas5p-fast-transformed-v3-colext`. CLI arm `rodas5p-fast-colext` (with `-ovh` variants of SPD01 as further
  arms only if SPD01 has run; this node's gate uses `rodas5p-fast-colext` against `rodas5p-fast`).
- `col_end[j]`: the last row index whose column `j` may hold a nonzero. Initialised when `W` is assembled from each
  row's `[first_nonzero, row_end]` span (`col_end[j] = max(col_end[j], i)` for `j` in the span: O(sum of row spans),
  O(n b) for a banded `W`); updated after every row update (`col_end[j] = max(col_end[j], i)` for `j` in
  `k + 1..=pivot_end`) and after a swap of rows `k` and `p` (`col_end[j] = max(col_end[j], p)` over the swapped
  span). The pivot search and the elimination loop then run over rows `k + 1..=col_end[k]` only. Every skipped row
  holds an exact zero (or -0.0) in column `k`: the strict `v > max` with `max >= 0` never selects it and the
  elimination `continue`s on it, so pivots, multipliers, `row_end`, `l_start` and the factors equal those of the
  full loops in every case, including non-finite entries (`NaN != 0.0` is true, so such rows stay inside the
  spans). The `col_end` storage is allocated only under the new policy, so the legacy arm's per-integration
  allocation count stays at its recorded value.
- Secondary, separately attributable: the per-row last-nonzero scan (`:379`) and the new first-nonzero scan are
  written as 8-wide chunk tests (`chunk.iter().fold(false, |a, v| a | (*v != 0.0))`) followed by a scalar search in
  the hit chunk; the index is the same. Whether the chunk test vectorizes under the fixed release profile (SSE2
  baseline) is unknown; the by-line attribution of both arms is kept so the two mechanisms are separable after the
  run.
- Not changed: the `W = -J` copy (n^2, about 0.39 M at n = 400) and the solves. The instruction count stays
  Theta(n^2); for declared bands the INT-03 banded pipeline (node SPD03) removes those terms too. This node is for
  the dense-storage path of fixed sparse patterns without a band declaration, which no current CLI problem with
  n >= 100 represents other than the banded Brusselators.

## Systems and measurements

1. Identity: `crates/rodas5p-integrators/tests/spd02_lu_column_extents.rs` (`--ignored` export) runs the five
   benchmark problems x rtol {1e-3, ..., 1e-9} with `rodas5p-fast` (legacy LU) and the column-extent LU through the
   library, comparing every output state bitwise, step counts, reuses, clipped steps, counters and success.
2. LU contract (`cargo test`, not ignored): 2,000 seeded random matrices (seed recorded in the test) through both
   `lu_in_place` variants and the in-place solve: banded `l, u` in 0..4, random sparse patterns with 1-20 %
   nonzeros, arrow, dense, forced row swaps (`p != k`) with fill-in beyond the original band, repeated swaps across
   many columns, rows whose span grows after a swap, all-zero rows, a diagonal that is exactly 0.0 after the
   `+ inv` add, rows that cancel to zero, injected -0.0, one NaN and one Inf entry; factors, pivots, `row_end`,
   `l_start`, the `Ok`/`Err` outcome and the solution of a random right-hand side must be bitwise equal.
3. Instructions: `tools/speed_profile.py` at rtol 1e-6 on the five problems for `rodas5p-fast` and
   `rodas5p-fast-colext`, by-line attribution kept for `rodas5p_fast.rs`.
4. Allocations: `tests/rodas5p_fast_allocations.rs` extended with the column-extent policy (zero allocations per
   step; the legacy count unchanged).

## Commands

    CARGO_TARGET_DIR=/home/user/target-speed CARGO_PROFILE_RELEASE_DEBUG=line-tables-only cargo build --release -p rodas5p-cli --locked
    SPD02_IDENTITY=research/spd02_lu_column_extents_20261005/IDENTITY.json cargo test --release -p rodas5p-integrators --locked --test spd02_lu_column_extents -- --ignored --nocapture --test-threads=1
    cargo test --release -p rodas5p-integrators --locked --test spd02_lu_column_extents --test rodas5p_fast_allocations
    python3 tools/speed_profile.py --rodas5p /home/user/target-speed/release/rodas5p --arms rodas5p-fast,rodas5p-fast-colext --problems van-der-pol-mu1000,robertson,hires,brusselator-1d-50,brusselator-1d-200 --scratch <scratch> --output research/spd02_lu_column_extents_20261005/PROFILE.json
    python3 tools/spd02_colext_check.py --base research/spd01_fast_driver_overhead_20261005/BASE_PROFILE.json --profile ...PROFILE.json --identity ...IDENTITY.json --output research/spd02_lu_column_extents_20261005/RESULTS.json

(`RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1` for every run.)

## Gate

**PASS** if all hold:

1. **Identity.** All 35 points equal the legacy driver bitwise (outputs, step counts, reuses, clipped steps,
   counters, success).
2. **LU contract.** All 2,000 matrices give bitwise equal factors, pivots, extents, outcomes and solutions.
3. **Instructions.** Ir per attempt colext / legacy <= 0.60 on `brusselator-1d-200` (n = 400) and <= 0.80 on
   `brusselator-1d-50` (n = 100); predicted 0.37-0.45 and 0.58-0.70.
4. **No small-problem regression.** The ratio is <= 1.03 on HIRES, van der Pol and Robertson (expected 0.99-1.02:
   the extent initialisation costs a few hundred instructions at n = 8).
5. **Legacy reproduction.** The legacy arm of the new binary reproduces `BASE_PROFILE.json` (attempts, counts,
   counters, final-state hash) and its Ir per attempt within +-2 % on all five problems.
6. **Allocations.** Zero allocations per step under the new policy; the legacy count unchanged.

Otherwise **FAIL**. Kill: any identity or contract difference; an n = 400 ratio >= 0.90 (the measured scan share
did not translate into instructions); a small-problem ratio above 1.03. A ratio between 0.60 and 0.90 is a FAIL
and is reported with the by-line attribution. Nothing is tuned after the run.

## Prior information and disclosure

L-0033 (v2 row extents: 0.397x at n = 400, no gain at n <= 8) and L-0054 (banded pipeline, counted operations
only). The 64 % scan attribution is from the exploratory line profile of this session on the unchanged baseline
and is not a result of this node; the split between the two scans is inferred from entry counts and is reported
as one combined figure until the post-run attribution is measured. Two independent reviewers corrected the
magnitude (1.4-1.65 M predicted at n = 400), required the swap-exercising contract set (the corpus never swaps
rows on the Brusselators), the legacy-reproduction gate and the allocation guard. Registered before the full
synthesis finished; depends on SPD01 only for the options struct. No code of this node exists before this commit.
