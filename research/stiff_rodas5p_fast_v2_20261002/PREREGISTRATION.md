# Preregistration: RODAS5P fast driver v2 (banded LU extents, in-place Jacobian)

## Question

After the lean driver (L-0032), an exploratory line-level callgrind profile of v1 (rtol 1e-6, see Prior information)
showed what remains:

- **Brusselator n = 400.** The forward and backward substitutions ran over full rows of a banded factor (about 10%
  each). The row updates of the elimination ran to the end of the row (about 14% in the inlined f64 operators). The
  zeroing of the Jacobian the problem allocates each step took 12%.
- **HIRES and van der Pol.** Most of the time went to loop overhead on short vectors, and 4% to 11% went to the
  represented-clock and output helpers.

Does a v2 that removes the operations on exact zeros, and lets the problem fill its Jacobian in place, reproduce v1
exactly while executing fewer instructions?

## The change (driver id `rodas5p-fast-transformed-v2`)

1. **Tracked row extents.** The in-place LU tracks, per row, the last column that can be nonzero (under fill-in and
   row swaps) and the first stored multiplier. Row updates, row swaps and both triangular solves stop there. Every
   skipped operation multiplies or adds an exact zero, so the factors and solutions equal v1's (up to the sign of an
   exact zero).
2. **In-place Jacobian.** `OdeProblem::with_jacobian_into` adds a Jacobian callback that writes a fixed sparsity
   pattern into the matrix it wrote last. The driver keeps one Jacobian matrix across steps and allocates nothing per
   step.
   - Robertson, van der Pol, HIRES and the Brusselator now supply one with the same values as their explicit
     Jacobian. Every other path keeps using the explicit Jacobian.
   - `jvp_only_clone` drops the in-place callback.
3. **Precomputed coefficient entries.** The nonzero entries of the `a`, `C` and `b_code` rows are listed once.
   Each element sees the same operations in the same order as in v1.

The represented-clock and output helpers are left as they are. They are shared, audited rules (re-audits R3 and R4),
and their cost is reported, not changed.

## Commands

- `cargo build --release -p rodas5p-cli --locked`
- `RAYON_NUM_THREADS=1 target/release/rodas5p stiff-benchmark --problems robertson,hires,van-der-pol-mu1000,brusselator-1d-50,brusselator-1d-200 --arms rodas5p,rodas5p-fast --repetitions 7 --warmups 1 --output research/stiff_rodas5p_fast_v2_20261002/RUST.json`
- `python3 tools/stiff_benchmark_native.py --rust research/stiff_rodas5p_fast_v2_20261002/RUST.json --driver target/native_stiff/native_stiff --native-output research/stiff_rodas5p_fast_v2_20261002/NATIVE.json --analysis-output research/stiff_rodas5p_fast_v2_20261002/ANALYSIS.json`
- `python3 tools/stiff_fast_v2_evaluation.py --rodas5p target/release/rodas5p --rust research/stiff_rodas5p_fast_v2_20261002/RUST.json --analysis research/stiff_rodas5p_fast_v2_20261002/ANALYSIS.json --v1-rust research/stiff_rodas5p_fast_20261002/RUST.json --v1-evaluation research/stiff_rodas5p_fast_20261002/EVALUATION.json --output research/stiff_rodas5p_fast_v2_20261002/EVALUATION.json --scratch <scratch dir>`
- Inputs:
  - `crates/rodas5p-integrators/src/rodas5p_fast.rs`, `problem.rs` and `problems.rs`;
  - `crates/rodas5p-cli/src/stiff_benchmark.rs`;
  - the tools;
  - the native driver of L-0029;
  - `Cargo.lock`;
  - the v1 outputs of L-0032.

## Gate

**PASS** if all of the following hold:

1. **Valid benchmark:** the analysis verdict is PASS (parity, reference uncertainty at most 1e-8, every arm
   deterministic, finite errors).
2. **v2 reproduces v1 exactly.** At all 35 problems and tolerances, the v2 run's final state equals v1's in L-0032
   as values. So do the accepted and rejected steps and the right-hand-side, Jacobian and factorization counts.
3. **Instructions per attempted step at rtol 1e-6** (callgrind, one integration), against v1's recorded numbers in
   L-0032:
   - at most 0.95 on HIRES and on van der Pol;
   - at most 0.6 on the 400-component Brusselator.

Otherwise **FAIL**. Matched-error wall times, against v1, the sequential path and the native arms, are reported
descriptively. The statistical authority of timing stays on hold.

## Prior information

- **L-0028..L-0032.**
- **The exploratory profile of v1 behind the question above.** It was taken on a line-tables build of `de45568` with
  `tools/stiff_profile.py`, kept in scratch and not committed:
  - HIRES: 20,510 instructions per attempt;
  - van der Pol: 9,479 per attempt;
  - Brusselator n = 400: 9.41 M per attempt.
- **Before this commit, only these tests ran:**
  - the contract tests of the driver: one step against the sequential step on seven problems, and adaptive runs
    against the sequential driver;
  - an allocation contract. With Robertson's in-place Jacobian the driver made 46 allocations at both rtol 1e-6
    (45 attempts) and 1e-9 (195 attempts), against 220 per attempt for the sequential driver;
  - a test that the in-place Jacobians equal the explicit ones.
- No benchmark, callgrind count or wall time of v2 exists before this commit.
