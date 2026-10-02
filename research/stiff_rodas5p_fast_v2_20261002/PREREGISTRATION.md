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

---

## Results (appended after the run at `df151d2`)

Outputs:

- `RUST.json`: the sequential and fast arms on 5 problems x 7 tolerances, 7 timed repetitions each;
- `NATIVE.json`: the native arms, rerun in the same session;
- `ANALYSIS.json`: errors and matched-error costs;
- `EVALUATION.json`: identity with v1, instruction counts and the gate.

**Gate: FAIL.**

| Gate item | Outcome |
|---|---|
| 1. Valid benchmark | holds: parity bit for bit, reference uncertainty at most 3.8e-13, every arm deterministic, all 245 runs completed |
| 2. v2 reproduces v1 exactly | holds at all 35 problems and tolerances: final states, accepted and rejected steps, and right-hand-side, Jacobian and factorization counts equal v1's in L-0032 |
| 3a. Brusselator n = 400, at most 0.6 of v1 | holds: **0.397** (3.74 M against 9.41 M instructions per attempt) |
| 3b. HIRES and van der Pol, at most 0.95 of v1 | **fails**: HIRES **1.011** (20,720 against 20,490), van der Pol **0.973** (9,227 against 9,479) |

The changes remove operations on exact zeros. These are many on a banded 400-component matrix and almost none on
dense 2x2 and 8x8 matrices. On HIRES, the in-place Jacobian writes its pattern every step, where v1 filled a freshly
zeroed matrix. That slightly outweighs the saved allocation. The prediction that loop restructuring would also help
the small problems was wrong.

**Matched-error wall time** (descriptive; median ms of v2, and the other arms as a multiple of it):

| Problem | E | v2 (ms) | sequential | cvode-bdf | cvode-bdf-lapack | hairer-radau5 | hairer-rodas |
|---|---|---|---|---|---|---|---|
| Robertson | 1e-3 | 0.025 | 8.6 | 6.4 | 6.2 | 0.63 | 0.55 |
| Robertson | 1e-5 | 0.037 | 8.4 | 10.3 | 10.3 | 0.93 | 0.50 |
| Robertson | 1e-7 | 0.11 | 9.0 | 6.4 | 7.0 | 0.41 | 0.94 |
| HIRES | 1e-3 | 0.13 | 6.5 | 5.3 | 6.0 | 0.95 | 0.64 |
| HIRES | 1e-5 | 0.26 | 7.2 | 6.1 | 9.4 | 1.07 | 1.03 |
| HIRES | 1e-7 | 0.87 | 7.1 | 3.9 | 4.9 | 0.75 | 1.87 |
| van der Pol | 1e-3 | 0.21 | 10.2 | 3.4 | 3.5 | 0.57 | 0.42 |
| van der Pol | 1e-5 | 0.43 | 10.1 | 15.0 | 8.9 | 0.47 | 0.57 |
| van der Pol | 1e-7 | 0.86 | 10.1 | - | - | 0.45 | 1.00 |
| Brusselator n=100 | 1e-3 | 2.1 | 8.1 | 3.9 | 2.9 | 2.4 | 1.03 |
| Brusselator n=100 | 1e-5 | 5.0 | 7.5 | 2.5 | 1.9 | 1.24 | 1.21 |
| Brusselator n=100 | 1e-7 | 9.8 | 7.9 | 3.3 | 2.3 | 1.24 | 1.40 |
| Brusselator n=400 | 1e-3 | 26 | 11.7 | 4.0 | 2.9 | 2.9 | 1.65 |
| Brusselator n=400 | 1e-5 | 55 | 13.0 | 3.1 | 2.4 | 2.2 | 1.60 |
| Brusselator n=400 | 1e-7 | 116 | 12.8 | 3.7 | 2.5 | 1.87 | 2.17 |

**Reading.** The wall readings are diagnostics: one host, single thread.

- **Brusselator.** On both sizes, every native arm now took 1.03 to 4.0 times v2's time at matched error. The
  sequential path took 7.5 to 13 times.
- **Small problems.** v2 is where v1 was, with Hairer's codes 0.41 to 1.9 times its time. The remaining gap there is
  per-step overhead that this change did not touch:
  - the inlined driver loop;
  - the represented-clock helpers `land_capped` and `step_to`, 2.5% to 9% of instructions;
  - the right-hand-side wrapper's finiteness check, about 4%.
- **The v2 code is kept despite the FAIL.** It reproduces v1 exactly, and it removes per-step allocation and the
  n = 400 cost. The FAIL records that its preregistered small-problem gain did not occur.
