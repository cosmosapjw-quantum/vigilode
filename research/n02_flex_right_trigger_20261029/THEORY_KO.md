# N02: source-matched right flexible GMRES certificate (2026-10-29)

Frozen inputs: PR109 `2d430a7aa81070da17a7a5dba60a2d56639b82db`, PR104 native `gmres_into.rs` blob `90bfaaa62afeda9dec27b3dfd594e67cd3034da1`. Preregistered on GitHub before new execution: `3f04fe7d0fabd7911c22bd2ee774f7d66947e8f3`.

## Derived/reused statement

For a fixed original linear operator A on each restart cycle, r0=b-Ax_c, beta=||r0||_2, v1=r0/beta, and *actual* right preconditioner applications z_j=P_j(v_j), retain Z=[z_j] and Arnoldi A Z_k=V_(k+1) Hbar_k+F_k. For a represented candidate xhat=x_c+Z_k y+e_x,

`b-A xhat = V_(k+1)(beta e1-Hbar_k y)-F_k y-A e_x`.

Incremental Givens can cheaply calculate a **projected** trigger, not an original-residual or stage-error certificate. After triggering (or final column/happy breakdown), compute the candidate using Z, then let an original-equation monitor decide Accept/Continue/Abandon. All source/clock/norm/epoch and N01 uncertainty bounds remain owned by the caller. Left preconditioning is not admitted by this code.

For fixed linear P, xhat=x_c+P(V_k y). For variable or nonlinear right P_j, factoring P out of a linear combination is invalid. Exact 2x2 constructive example: A=I, b=(1,1), v0=(1,1)/sqrt(2), P0=diag(1,2), P1=2I, Hbar=[[3/2,0],[1/2,2],[0,0]], y=(2sqrt(2)/3,-1/(3sqrt(2))); flexible x=(1,1), incorrectly factored x=(5/3,1), original residual=(-2/3,0). Exact rational Wolfram plus independent Python Fraction 14/14 PASS.

This adapts literature-established FGMRES, not a new general mathematical discovery. Primary Saad (1993), DOI 10.1137/0914028, and Simoncini–Szyld (2003), DOI 10.1137/S1064827502406415.

## Code map and admissible claims

New opt-in `gmres_into_triggered.rs` stores z_j in `workspace.arnoldi.directions[j]`; Givens trigger precedes expensive prefix correction and candidate assembly, preserving existing output-only-on-Accept, counters/progress and default dispatch. The old `gmres_into.rs` Arnoldi helpers have only crate-local visibility changes. A monitor that sees an unbounded matrix-free callback error or stale source cannot be interpreted as certified. The N01 weighted-infinity physical error bound still requires owner-verified delta_J,delta_b,eta_apply, weights and positive resolvent margin, separately from the Krylov solver.

Native function physically running with real full-IVP, matching achieved accuracy, rigorous original residual and signed physical33 rate/clock/source/capsule remain NOT_RUN/OPEN. Original DAG 7/23 and REI 28/88 unchanged; production HOLD.
