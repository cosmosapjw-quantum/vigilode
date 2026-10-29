# N01-JF preregistration: restore Jacobian-free semantics
2026-10-29. Parent PR115: 8555b68a8053fc081be1195b0a40024d994d5798. Original core source 0fab50efc17d4ce64f43fd0a639245c65f3628c0; adapter c42a9cba71fdc7461165e3dc728f24e95d3eb53a.
Read-only provenance: initial Rust parity spec, 2026-08-10 vectorized/JF path-controller spec, 2026-08-08 unified candidate spec, R-NEXT-07 model epoch, Audit2 matrix-free common W, Oct production closure and structural JF report.

## Observed code defects
1. PR115 n01_mass_action_owned_problem registers both explicit J and JVP; OdeProblem::linearize prioritizes explicit J, so default opt-in route is not Jacobian-free.
2. PR115 N01MassActionOwner::jvp_at constructs a 3x3 J before each product.
3. RHS/JVP callbacks call advance(t,state) and change model_epoch for ephemeral stage probes. R-NEXT-07's epoch is for model generation changes, while time/state are already frozen in the operator. This may cause erroneous invalidation.

## Protected theory and implementation target
Exact Robertson-type polynomial f=(-k1*a+k2*b*c,k1*a-k2*b*c-k3*b*b,k3*b*b), nonnegative rates/state and certified box. Direct JVP via u=k1*va, v=k2*(c*vb+b*vc), q=2*k3*b*vb; Jv=(-u+v,u-v-q,q), symbolically equal to Df times vector, with exact mass tangent conservation. Floating representation is explicitly inexact and independently bounded.
Add read-only validate_evaluation_point(t,state); use it in RHS/JVP, not advance. Keep advance as explicit certificate-state binding and source epoch mutation. Configure ONLY this opt-in factory with jac=None, jvp=Some; preserve standalone explicit Jacobian oracle for auditing. Keep source-owned outward reference residual, eta, deltaJ/deltaB and physical stage forward-error gates unchanged. Do not touch default solver, GMRES, warm starts, counters, tolerances, routing, main or N02.

## Falsifiers
Existing archived mirror 28/28 baseline; red tests for hidden explicit operator on linearize(), callback-induced epoch mutation, stale source refusal, immutable output on callback failure, nonfinite JVP, exact symbolic Jacobian equivalence, no source data contamination between temporary states. Repeat independent exact binary64 Fraction oracle only if code mathematics change. Distinguish mirror tests from real cargo. Benchmark loops 0 (no new timing claim). Stage-specific Github checkpoints and immutable local/Drive+Dropbox backup with actual receipts, if available.

Official DAG unchanged: VigilODE 7/23, REI 28/88, successor 0/12 admitted; N00F audit 1/1 scoped, N01-JF 0/1; production HOLD; R6/C9 independent not yet done.
