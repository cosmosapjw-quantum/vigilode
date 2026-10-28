# Preregistration: LGMRES writing into caller storage (review DAG node REV-04)

## Question

R-NEXT-02 (L-0045) gave GMRES a `solve_into` path. The review asked for the same for LGMRES and GCRO-DR. GCRO-DR's
recycling has just been repaired (L-0059) and is not recommended on the systems studied, so this node covers LGMRES.
The existing LGMRES solve allocates on every call:

- a full snapshot of its carried state (`state.clone()`, k + 1 vectors of length n), for rollback on failure;
- a new image vector for each refreshed direction;
- clones of the new direction and its image in every outer cycle;
- clones of the solution for the warm start and for the report.

Can a caller-owned path remove these allocations with bit-for-bit the same results, the same state and the same
rollback?

## Changes (new API; the existing solve unchanged)

`rodas5p-krylov/src/lgmres_into.rs`: `solve_lgmres_into(op, pc, rhs, x0, config, state, residual_scale, output,
workspace, counters) -> CoreResult<LgmresIntoReport>`, with `LgmresIntoWorkspace`. Same algorithm, arithmetic order,
stopping rule, counters and state commit rules as `solve_lgmres_with_workspace_and_residual_scale`. Allocations are
replaced by workspace-owned storage:

- the rollback snapshot is copied into reused buffers;
- vectors for new images and directions come from a pool that dropped directions return to;
- the warm start and the output reuse existing buffers.

The solution is written into `output` only on success.

## Systems

1. INT-01's CDR family (6 sequences of 6 operators with 8 right-hand sides, carried state per sequence), with
   `LgmresConfig` defaults (inner 30, outer k = 8) and rtol 1e-9.
2. The Brusselator-50 trajectory set of L-0046 (40 attempts with 8 stages, carried state), rtol 1e-11.
3. Failure paths: the same systems with `max_outer = 1` and rtol 1e-14, so that most solves fail and roll back.

## Commands

`REV04_OUTPUT=research/rev04_lgmres_into_20261003/RESULTS.json cargo test --release -p rodas5p-integrators --locked --test rev04_lgmres_into -- --ignored --nocapture --test-threads=1`

`cargo test -p rodas5p-krylov --locked --test rev04_lgmres_into_contracts`

## Gate

**PASS** if all hold:

1. **Bitwise identity.** On every solve of systems 1 to 3, the new path matches the existing one in success or
   failure, solution bits, residual norm, iterations, matvecs, the `WorkCounters`, and the carried state after the
   solve (directions, images, previous solution, generation, identity). After a failed solve both states equal their
   pre-solve values.
2. **Output only on success.** On every failed solve the caller's output buffer is unchanged (bitwise).
3. **Fewer allocations.** After one warm-up solve per sequence, the new path allocates strictly less per solve than
   the existing one on every sequence of systems 1 and 2.

Otherwise **FAIL**. Reported, not gated (P1): the allocation ratio per sequence. Remaining allocations, inside the
shared Arnoldi routine, are listed by source. No timing.

## Prior information

L-0045 (GMRES into: bitwise identity and 0.025-0.24x allocations). No code of this node exists before this commit.

---

## Results (appended after the run at `d520918`)

Output: `RESULTS.json`. Ledger row L-0061. Contract tests `rev04_lgmres_into_contracts` 4/4. These cover success,
failure with rollback, a carried direction of the wrong length, and a wrong output length.

**Gate: PASS.**

| Gate item | Outcome |
|---|---|
| 1. Bitwise identity | **holds** on all 1,216 solves (13 sequences, 344 of them failures): same outcome, solution bits, residual norm, iterations, matvecs, `WorkCounters` and carried state; after every failure both states equal their pre-solve value |
| 2. Output only on success | **holds**: the output buffer is unchanged after all 344 failures |
| 3. Fewer allocations | **holds** on every sequence, but the reduction is small: 0.979x to 0.990x on the CDR sequences and 0.950x on the Brusselator trajectory (0.944x to 0.988x on the failing runs) |

What remains, and why: after the warm-up solve, 455 to 4,099 allocations per solve are left. They come from the
shared Arnoldi routine that LGMRES calls, `gmres::arnoldi_augmented_with_workspace`. In every inner iteration it
resizes `hessenberg_prefix` and calls `small::least_squares`, which returns a fresh solution vector and builds its
factorization in fresh storage. Only the last of those solutions is used. The GMRES `solve_into` of L-0045 removed
the same per-iteration least-squares solve for GMRES by solving once per cycle. Doing that for the augmented
routine would change code that LGMRES shares with other solvers, so it is left for its own node.

So the snapshot, image, direction and solution allocations this node targeted are gone, and they are a small share
of LGMRES's total. Claim ceiling: allocation counts on these systems; no timing.
