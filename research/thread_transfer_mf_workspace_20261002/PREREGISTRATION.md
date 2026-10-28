# Preregistration: JVP-only raw-stage matrix-free workspace driver (thread-transfer DAG node P1-MF-WORKSPACE)

## Question

The strict matrix-free path (`sequential_matrix_free_step`, `build_step_context_matrix_free`) forms in every stage
a `Gamma` mixture of the previous K stages and applies one extra JVP to it for the right-hand side
(`h J sum_j Gamma_ij K_j`), seven per attempt; it also clones the tableau and allocates stage, state, RHS and report
vectors per attempt. Does a separate driver that solves the raw U form (node P0-MF-TARGET) with the same Krylov
solvers and one workspace remove those seven JVPs and most allocations, at matched accuracy?

## Driver (written after this commit; id `rodas5p-mf-fast-transformed-v1`)

`crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs`:

- requires a user JVP (`supports_matrix_free_jvp`) and the identity mass matrix; any direct factorization or
  explicit-matrix preconditioner request (Direct, Jacobi) is refused (strict MF, fail closed);
- per attempt: `f(t, y)`, `f_t` and the JVP operator at `(t, y)` (reused after a rejection from the same state);
  `W = I - h gamma J` as the counted-JVP shifted operator; stage `i` solves
  `W U_i = h gamma f(t + c_i h, y + sum_j A_ij U_j) + gamma sum_j C_ij U_j + h^2 gamma gamma_i f_t` with the
  configured GMRES, LGMRES or GCRO-DR; `y_new = y + sum_j b_code_j U_j`; embedded error `U_(s-1)`;
- linear tolerances: the K path's `rtol` is applied to the U right-hand side's own norm; `atol` is mapped to
  `|gamma| atol` (P0-MF-TARGET); every Krylov kernel still checks the true residual of the current operator;
- the Krylov recycle state is snapshotted before an attempt and restored on rejection, as in the sequential path;
  the step-size controller, represented-clock and output rules are the sequential driver's functions;
- it has no inner forcing (the protected adaptive driver's WRMS forcing and refinement passes are not ported).

## Tests and command

`crates/rodas5p-integrators/tests/thread_transfer_mf_workspace.rs` (with a counting allocator):

`THREAD_TRANSFER_MF_WORKSPACE_OUTPUT=research/thread_transfer_mf_workspace_20261002/RESULTS.json cargo test --release -p rodas5p-integrators --locked --test thread_transfer_mf_workspace -- --nocapture --test-threads=1`

Problems: the JVP-only clones of Robertson, van der Pol (mu = 1000), HIRES and the 1-D Brusselator (n = 100), plus a
nonautonomous forced linear problem and an 8-stage quadratic diagonal problem with nonzero stages.
Configurations: GMRES and LGMRES and GCRO-DR, no preconditioner. The "sequential" comparator is
`sequential_matrix_free_step` with the same configuration, driven by the same adaptive loop in the test.

## Gate

**PASS** if all hold:

1. **RHS-assembly JVPs 7 -> 0.** Per attempt, `jvp_calls - (linear_matvecs + diagnostic_matvecs +
   recycle_refresh_matvecs)` is 7 for the sequential step and 0 for the new driver, on the 8-stage fixture and every
   problem with a nonzero stage. All JVP, matvec and preconditioner counts are reported per attempt.
2. **Strict MF.** No Jacobian build and no factorization in any run; Direct and Jacobi configurations and a mass
   matrix are refused.
3. **One step agrees.** With a tight Krylov tolerance (rtol 1e-12), on every problem, one step from the initial state
   at h in {1e-4, 1e-2} (scaled to the problem) gives `|y_new - y_new_seq| <= 1e-9 (|y| + 1e-6)` componentwise and
   error norms within 1e-6 relative of the sequential step's.
4. **Matched accuracy.** Adaptive runs at rtol 1e-6 (atol 1e-6 times the problem's scale) complete, and the
   final-state error against a tight-tolerance reference is at most 3 times that of the sequential adaptive run
   (and at most 1e-3 relative), on every problem.
5. **Semantics.** Rejected attempts leave the state, output and Krylov recycle state unchanged (checked by a forced
   rejection); output times equal the schedule bit for bit; a zero right-hand side gives zero stages; very small h
   (1e-12) completes; the nonautonomous problem matches a fine-step reference.
6. **Allocations.** Allocations per attempt are reported for both drivers; the new driver allocates at most half as
   many per attempt as the sequential step on every problem.

Otherwise **FAIL**. Reported, not gated: per-attempt RHS calls, JVPs, Krylov iterations and matvecs; accepted and
rejected steps; the gap to the protected (inner-forced) adaptive driver's accuracy and work. No wall time is
measured; timing authority stays on HOLD.

## Stop conditions (DAG)

A transported target error above the P0-MF-TARGET allowance, a missing true-residual check, or total counted work
consistently worse than the sequential MF path stops promotion of this driver.

## Prior information

- The review's 12 Python JVP-only comparisons removed 7 RHS JVPs in every case.
- The fast (explicit-J) driver's raw-stage algebra passed L-0032/L-0033. No code of this node exists before this
  commit.

---

## Results (appended after the run at `55267cf`)

Output: `RESULTS.json`. Ledger row L-0038.

**Gate: FAIL** (items 1, 2 and 5 hold; 3, 4 and 6 fail).

| Gate item | Outcome |
|---|---|
| 1. RHS-assembly JVPs 7 -> 0 | **holds**: 7 per attempt for the sequential step, 0 for the U form, in every one-step and adaptive run |
| 2. Strict MF | **holds**: no Jacobian build or factorization anywhere; Direct, Jacobi and Direct-preconditioner configurations, a mass matrix and a JVP-less problem are refused |
| 3. One step agrees | **fails**: 22 of 36 comparisons miss `1e-9 (|y| + 1e-6)` or 1e-6 relative error norm (classified below) |
| 4. Matched accuracy | **fails** on one of 18 runs: GCRO-DR on the Brusselator completes for neither driver within 5,000 attempts; the other 17 complete, at most 2.6x the sequential error (Prothero-Robinson with GCRO-DR; all others 0.76-1.0x) |
| 5. Semantics | **holds**: a rejected attempt changes the recycle state and the restored snapshot reproduces a clean attempt bit for bit; output times equal the schedule bit for bit through a rejected first attempt; zero RHS gives zero stages; h = 1e-12 completes; Prothero-Robinson error 9.5e-10 at rtol 1e-8 |
| 6. Allocations <= 0.5x | **fails**: 0.32-0.62x on the small problems, 0.81-0.86x on the Brusselator (1.03x for the non-completing GCRO-DR run) |

Adaptive runs (rtol 1e-6; relative final error against the reference; attempts; allocations per attempt U form /
sequential):

| Problem | GMRES | LGMRES | GCRO-DR |
|---|---|---|---|
| Robertson | 3.67e-7 both, 45 att, 0.41 | same, 0.48 | same, 0.54 |
| van der Pol | 7.59e-7 both, 469 att, 0.40 | same, 0.48 | same, 0.53 |
| HIRES | 2.63e-7 vs 3.44e-7, 210/213 att, 0.62 | same, 0.62 | same, 0.70 |
| Brusselator n=100 | 2.14e-6 both, 92 att, 0.81 | same, 0.86 | neither completes (5,000 att) |
| Prothero-Robinson | 5.98e-9 both, 12 att, 0.32 | same, 0.51 | 1.54e-8 vs 5.98e-9, 16/12 att, 0.49 |
| quadratic n=4 | 4.68e-8 both, 13 att, 0.55 | same, 0.58 | same, 0.62 |

Total JVPs per attempt fall by about 7 everywhere except the Brusselator, where the U-form stage systems took more
Krylov iterations (GMRES 328 vs 315 per attempt) and total JVPs rose (351 vs 345). The protected (inner-forced)
driver reaches the same errors on the 17 completing runs (descriptive).

Classification of the failures (descriptive; the gate stands as preregistered):

- **One-step comparisons.** (a) Robertson at h = 1e-2 diverges in both drivers (states near 1e117, error norm 1e6):
  the comparison is meaningless at that step size. (b) Where the embedded error norm is 1e-13 to 1e-9 (h = 1e-4 and
  some h = 1e-2 cases), the states agree to 1e-16 to 1e-10, but the two error norms differ by 1e-6 to 7x relative.
  At those sizes the norms are Krylov noise: a relative 1e-12 residual on a different right-hand side. The
  preregistered relative criterion did not allow for that. (c) GCRO-DR on the Brusselator fails at the tight
  tolerance ("least-squares solve produced NaN/Inf") in both drivers at h = 1e-4, and in the sequential one at
  h = 1e-2.
- **GCRO-DR on the Brusselator.** Unpreconditioned GCRO-DR at rtol 1e-11 alternates between a failed and an
  accepted attempt in the sequential step, in the protected driver and in the U form alike ("Arnoldi budget
  exhausted"; also with maxiter 1000 in a development check). This is a property of that solver configuration on
  that problem, not of the U form. During development the same runs aborted inside faer (scratch too small for a
  2x2 complex pencil). That crash is fixed separately (`crates/rodas5p-krylov/src/small.rs`, commit `01f4266`,
  regression test), and the fixed code is what ran here.
- **Allocations.** The stage loop no longer allocates, but each Krylov solve still returns a fresh solution vector
  and report, and the GCRO-DR/LGMRES states allocate during updates. At n = 100 the Krylov work dominates. Halving
  the allocations needs Krylov kernels that write into caller storage, which this node did not change.

Development disclosure: the test ran several times before the recorded run, with the same driver, to fix test bugs
(an invalid initial step in the semantics check, an iteration budget raised to 4000 for the tight one-step solves,
recording Krylov failures instead of panicking, a 5,000-attempt cap so that non-completing runs end). Those runs showed
the outcomes above. No gate threshold was changed.

Claim ceiling: native counters and correctness of a research driver; no wall-time claim; timing authority stays on
HOLD.

### Note after recording

The research test takes about 2 minutes in release and far longer in the debug build of `cargo test --workspace`,
so it is marked `#[ignore]` (run by the ignored-tests CI job). To reproduce L-0038, add `--ignored` to the command
above. Nothing else in the test changed.
