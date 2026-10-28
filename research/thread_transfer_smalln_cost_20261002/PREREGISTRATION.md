# Preregistration: small-n specialization of the fast driver (thread-transfer DAG node P1-SMALLN-COST)

## Question

L-0033 (FAIL, kept) found that removing operations on exact zeros helps the 400-component Brusselator (0.397 of v1
instructions per attempt) but not the small problems (HIRES 1.011, van der Pol 0.973). The v2 profile attributed the
small-problem cost to loop overhead on short runtime-length vectors, dynamic dispatch of the problem callbacks and
the shared clock/output helpers. Does a specialization with compile-time dimension (stack arrays, unrollable loops)
and statically dispatched problem functions cut the instructions per attempt at identical results?

## Implementation (written after this commit; driver id `rodas5p-fast-small-static-v1`)

`crates/rodas5p-integrators/src/rodas5p_fast_small.rs`:

- a `SmallProblem<const N: usize>` trait (right-hand side, Jacobian fill, `f_t`; no trait objects) and
  `integrate_rodas5p_fast_small_observed::<N, P>`;
- the same transformed-stage arithmetic, coefficient order, partial-pivoting LU with zero-multiplier skipping,
  Jacobian/`f`/`f_t` reuse after rejection, finite checks, controller, represented-clock and output functions as v2,
  on `[f64; N]` and `[[f64; N]; N]` storage;
- implementations for van der Pol (N = 2), Robertson (N = 3) and HIRES (N = 8) in
  `crates/rodas5p-cli/src/stiff_benchmark.rs`, with a new arm `rodas5p-fast-small` (not in the default selection)
  for `stiff-benchmark` and `stiff-profile-run`.

## Commands

- `cargo build --release -p rodas5p-cli --locked`
- `cargo test --release -p rodas5p-integrators --locked --test thread_transfer_smalln` (identity contracts)
- `python3 tools/thread_transfer_smalln_evaluation.py --rodas5p target/release/rodas5p --output research/thread_transfer_smalln_cost_20261002/EVALUATION.json --scratch <scratch dir>`:
  for each problem and rtol in {1e-4, 1e-6, 1e-8}, both arms' final states, steps and counters; callgrind
  instructions per attempt (one integration as the difference of 2 and 1 repetitions, as in L-0032/L-0033) at
  rtol 1e-6, with the top functions; and an ensemble of 64 van der Pol trajectories (mu = 1000 (1 + k/64)) run
  back to back in one process, instructions per trajectory, for both arms.

## Gate

**PASS** if all hold:

1. **Identical results.** At every problem and tolerance, the small driver's final state equals v2's as values, and
   so do the accepted and rejected steps and the right-hand-side, Jacobian and factorization counts.
2. **Fewer instructions.** Instructions per attempted step at rtol 1e-6, against v2: at most 0.8 on van der Pol
   (n = 2) and on HIRES (n = 8). Robertson (n = 3) is reported, not gated.

Otherwise **FAIL**. Reported: Robertson's ratio; the ensemble's per-trajectory instructions against the single
trajectory (throughput is reported apart from one-trajectory latency); the callgrind attribution. No wall time is a
gate item; timing authority stays on HOLD. L-0033 stays FAIL whatever this node finds.

## Prior information

- L-0032/L-0033 numbers: v2 HIRES 20,720 and van der Pol 9,227 instructions per attempt at rtol 1e-6.
- No code of this node exists before this commit.

---

## Results (appended after the run at `8d41b8b`)

Output: `EVALUATION.json`. Ledger row L-0041.

**Gate: PASS.**

| Gate item | Outcome |
|---|---|
| 1. Identical results | holds at all 9 points (3 problems x rtol 1e-4, 1e-6, 1e-8): final states, accepted and rejected steps, RHS, Jacobian and factorization counts; the contract test also compares output times, states and every counter, including runs with rejections and Jacobian reuse |
| 2. Instructions <= 0.8 | holds: van der Pol **0.393** (3,569 vs 9,090 per attempt), HIRES **0.602** (12,302 vs 20,427) |

Reported: Robertson 0.444 (5,060 vs 11,384). Ensemble of 64 van der Pol trajectories (mu = 1000 .. 1984): 1.12 M
against 2.86 M instructions per trajectory (0.393), identical checksums and attempts (20,083). One trajectory at
mu = 1000 costs 1.67 M against 4.26 M. The per-trajectory cost of an ensemble member is lower than that of a single
run because the members differ in mu, not because of shared work.

Attribution (callgrind, rtol 1e-6): in v2 the driver function holds 74-83% of the instructions and the dynamic
problem callbacks with their `OdeProblem` wrappers 6-8%. In the small driver the method's share falls to 58-82%, and
the shared represented-clock helpers (`land_capped`, `step_to`) become the largest remainder on van der Pol (23%) and
Robertson (17%). These helpers are unchanged, audited rules; this node does not touch them.

Reading: the small-problem cost that L-0033 did not remove was runtime-length loop overhead and dynamic dispatch, not
zero operations. With the dimension and the problem known at compile time, the same arithmetic in the same order runs
in 0.39-0.60 of the instructions. This is a specialization, not a replacement: it needs a `SmallProblem` per model and
covers autonomous problems with the identity mass matrix only. L-0033 stays FAIL. Timing authority stays on HOLD, and
no wall time was measured.

Development disclosure: the identity contract test ran once before the recorded run (it passed), and one
`stiff-profile-run` of HIRES served as a smoke check. The evaluation itself ran once, as recorded.
