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
