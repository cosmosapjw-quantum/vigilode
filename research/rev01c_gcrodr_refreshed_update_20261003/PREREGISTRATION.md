# Preregistration: GCRO-DR with the recycle invariant restored after every update (review DAG node REV-01c)

## Question

L-0058's post-hoc diagnostic found `[C V]` losing orthogonality at the moment an Arnoldi vector is created, with no
near-breakdown and with `C` orthonormal. The remaining explanation (H3) is a violated invariant `M^-1 A U = C`:

- A defect is inherited by each recycle update and amplified by the normalization `U = Y R^-1`.
- The correction `U C^T r` then fails to remove `C C^T r`, so the recomputed residual lies almost in `span(C)`.

H3 predicts that recomputing `C = M^-1 A U` after every update (k operator products, charged) and
re-orthonormalizing the pair removes the recycle-induced failures.

## Changes (opt-in; defaults unchanged)

`rodas5p-krylov`: `GcrodrSolveOptions` gains `refresh_after_update: bool`. When it is set, after each cycle's recycle
update the new basis `U` is applied to the operator. The products are charged as refresh products, and the pair is
rebuilt by `orthonormalize_pair(U, M^-1 A U)` exactly as after an operator change. The per-cycle trace gains
`update_refresh_matvecs`. With the flag false, the solve is unchanged bit for bit.

## Controls

B cold GCRO-DR, C recycled, **R recycled with `refresh_after_update`**, RF with the REV-01 start projection as well,
and RE with INT-01's 1e-8 reuse check as well. A (cold GMRES) is the false-convergence reference. Settings are as in
L-0057 and L-0058.

## Systems

1. **Fresh Brusselator-160** (n = 320), primary. Calibration (`tests/rev01c_calibration.rs`, control C only, before
   this commit): 15 failures in the 320 trajectory solves and 1 in the 16 one-step solves; generating run 92
   attempts, success. No other control was run on it.
2. The Brusselator-50, -80 and -120 sets (seen; reported, not gated).
3. INT-01's CDR family (no-harm check).

## Commands

`REV01C_OUTPUT=research/rev01c_gcrodr_refreshed_update_20261003/RESULTS.json cargo test --release -p rodas5p-integrators --locked --test rev01c_gcrodr_refreshed_update -- --ignored --nocapture --test-threads=1`

`cargo test -p rodas5p-krylov --locked --test rev01c_gcrodr_refresh_contracts`

## Gate

**PASS** if all hold:

1. **Defaults unchanged.** With the flag false, the solve is bitwise `solve_gcrodr_with_policy` (contract test).
   The REV-01b, REV-01, INT-01 and R-NEXT-03 contract tests pass.
2. **Failures removed (H3).** Control R has no failure on set 1 where cold GCRO-DR (B) succeeds.
3. **No false convergence and full accounting** (L-0057 rules). A completed cycle may also charge its
   `update_refresh_matvecs`, which must be at most the recycle dimension.
4. **No harm** on set 3: R fails on no more solves than C.

Otherwise **FAIL**. Reported, not gated: every control on sets 1 to 3, operator products against B and C, and
creation-time `|C^T v|` (the L-0058 diagnostic) for R on set 1.

## Prior information

L-0046, L-0052, L-0057, L-0058 and its post-hoc diagnostic, and the calibration above. No code of the option exists
before this commit.
