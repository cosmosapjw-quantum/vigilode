# N00F pre-execution research and benchmark preregistration (2026-10-29)

Source: PR104 native `crates/rodas5p-krylov/src/gmres_into.rs`, blob `90bfaaa62afeda9dec27b3dfd594e67cd3034da1`. Stacked from PR109 head `2d430a7aa81070da17a7a5dba60a2d56639b82db`. This is an isolated *audit* not an official DAG promotion or native source patch.

H1: `P^-1(Vy)=sum_j y_j P^-1(v_j)` silently requires fixed linear P^-1. With nonstationary/nonlinear preconditioning, a projected Givens zero can coexist with nonzero original residual. Check an exact 2-vector example.
H2: Flexible right-preconditioning stores `z_j=P_j^-1(v_j)`, forms `A Z_m=V_{m+1} Hbar_m`, and reconstructs `x=x0+Z_m y`. Incremental Givens only triggers true original-residual evaluation, never admits a result by itself.
H3: Under fixed-linear synthetic diagonal systems, delaying candidate/backsolve to projected-trigger, happy breakdown or restart boundary should reduce candidate cost while preserving accepted output bits, Arnoldi iteration counts and original residual.

Execution plan: theory proof → zero-dependency Rust1.94.1 coding/tests → one (maximum three) light benchmark loops with 7 paired ABBA/BAAB blocks for n=16/64/128. Compare always-prefix and trigger-only with identical sources, RHS, tolerances, requested output and achieved errors. Record timing, counters, matched outputs and negative controls. Reject improvement if output, error or source contract differs. Include exact nonlinear PC counterexample, original-residual admission, no-output-mutation and nonfinite negative tests.

Existing inherited DAG counts: core 7/23; central REI 28/88; proposed successor 0/12 admitted. R6/C9 independently authored reviews NOT_RUN. Production HOLD. No default solver mutation, no merge, no force push. Native full-IVP benchmark NOT_RUN. Original physical source/JVP/rate/capsule provenance NOT_RUN. Any later snapshots of stages are recorded as post-execution, not backdated.