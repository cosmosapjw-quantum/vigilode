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

---

## Results (appended after the run; source commit recorded in the ledger row)

Binary: release build with line tables, sha256 `3fb97e4dd3f69879adbd85c08ab1dcc503732331d4eef939c770174de18e523c`, valgrind 3.22.0,
`RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1`. Outputs: `IDENTITY.json` (35 rows), `PROFILE.json`, `RESULTS.json`.
Contract tests `spd02_lu_column_extents` 2/2 (2,000 seeded matrices: banded 600, sparse 400, arrow 200, dense 200,
forced swaps 200, degenerate 200, non-finite 200; 744 singular or non-finite outcomes, all equal in both variants),
`rodas5p_fast_allocations` 2/2 (legacy 46 allocations per integration at both tolerances as recorded; column
extents 47 = one `col_end` buffer per integration, step-independent).

**Gate: FAIL** (items 4 and 5; items 1, 2, 3 and 6 hold; the kill condition on item 4 is met).

| Gate item | Outcome |
|---|---|
| 1. Identity | **holds**: the column-extent LU equals the legacy driver bitwise on all 35 points (all output states, step counts, reuses, clipped steps, counters); both arms report the in-place LU on every problem |
| 2. LU contract | **holds**: factors, pivots, extents, outcomes, error texts and solutions bitwise equal on all 2,000 matrices |
| 3. Instructions | **holds**: colext / legacy = **0.517** at n = 400 (3,903,531 -> 2,018,884 Ir per attempt; gate <= 0.60, predicted 0.37-0.45) and **0.773** at n = 100 (403,214 -> 311,601; gate <= 0.80) |
| 4. No small-problem regression | **fails**: 1.036 (van der Pol, n = 2), 1.048 (Robertson, n = 3), 1.062 (HIRES, n = 8) against <= 1.03 (predicted 0.99-1.02): the extent initialisation (two 8-wide scans per row plus the column-span update) and the `col_end` maintenance cost 330-1,300 Ir per attempt where the dense LU itself is cheap |
| 5. Legacy reproduction | **fails at n = 400**: the legacy arm of the new binary reproduces the base export's work and final states on all five problems, but its Ir per attempt is 1.024x the base on `brusselator-1d-200` (3,903,531 vs 3,811,738; gate +-2 %); 1.020 at n = 100, 1.006-1.014 on the small problems. The legacy code path is unchanged; the policy match in `factor`/`solve` and the second variant of the LU changed the compiled legacy loops |
| 6. Allocations | **holds** (see above) |

Where the remaining 2.02 M instructions per attempt at n = 400 go (by-line attribution of the colext arm, kept in
`RESULTS.json`): the `W = -J` copy (`rodas5p_fast.rs:450` and its zipped iterator lines, about 0.5 M), the two 8-wide
extent scans (`:736`, `:750` and the `rchunks`/`chunks` iterator lines, about 0.5 M, i.e. the chunk test did not
vectorize into fewer instructions than the plain scans), the stage work and the band-local LU; the two column scans
that cost about 2.4 M in the legacy arm are gone (the pivot-search and zero-test lines fall from 8.3 % + 6.1 % + 4.1 %
of 3.9 M to below 1 % of 2.0 M).

What the FAIL means: the mechanism works where it was aimed (n = 400: 0.517x, bitwise identical, zero allocations per
step), but the registered node also required no regression at n <= 8 and a legacy arm within 2 % of the base, and
both were missed. The code stays opt-in behind `FastLuPolicy::ColumnExtents`; nothing was tuned after the run. Two
follow-ups are preregisterable (not done here): a dimension or density threshold for the policy (the regression is
confined to n <= 8, where the scans cost more than the loops they remove), and an attribution of the legacy arm's
+2.4 % to the policy branch versus code layout, which the by-line data in `RESULTS.json` can start.

**Disclosure.** The LU contract test as first written compared the legacy row extents after factorization (updated
by fill-in) with the column-extent variant's initial extents and failed on its first matrix; the comparison was
corrected to the initial extents of both variants before the recorded run of the test (a test defect, not a code
difference; the matrix in question is in the test's seed sequence). `tools/spd02_colext_check.py` was committed
together with SPD01's recorded run before this node ran. Claim ceiling: counted instructions on these problems; no
wall-time claim.
