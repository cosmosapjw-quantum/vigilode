# G3 PHYS–MATH AUDIT

## Definitions and dimensions

For regular ODEs

\[
  y'=F(t,y),\qquad Jv=\partial_yF(t,y)v,
\]

`hJ` is dimensionless. Each `phi_k(hJ)` is dimensionless. Terms passed to `fused_phi_linear_combination` have the units of `F`; the final state update is `y+h action`, so the dimensions are consistent.

Current G3 support is identity mass only. Nonidentity mass and DAE claims are forbidden.

## Sign and stability conventions

- negative real eigenvalues represent decay;
- complex Dahlquist blocks use negative damping plus imaginary frequency;
- no sign mutation was observed between dense phi oracle and matrix-free action;
- linear exactness does not imply nonlinear stiff-order robustness.

## Order and limits

The inherited G2 minimum observed order of pexprb54s4 is

\[
  p_{\min}=4.9287704282,
\]

which passes the declared `>=4.8` gate.

Known limits:

- zero input combination returns zero with no operator work;
- happy breakdown/invariant-subspace closure is accepted;
- full-space full-MGS projection is treated as exact up to projected dense arithmetic;
- linear stiff/oscillatory limits match matrix exponential references;
- nonlinear oscillatory PR remains a counterexample to generic performance.

## Error-estimator status

The Arnoldi first-residual term is tied to the projection residual and is available at the first checkpoint. It is not a rigorous upper bound under arbitrary nonnormality. Its use is therefore restricted to adaptive research diagnostics and must be cross-checked against dense/high-precision references on calibration problems.

The nested-dimension difference is retained only as a contrastive diagnostic; it is not the convergence authority.

## Positivity, regularity, and boundary conditions

No generic positivity theorem is claimed. Current problems remain in their regular domains. Nonfinite states fail explicitly. The adaptive failure path returns partial committed outputs without claiming completion.

## Counterexamples and narrowed claims

1. Oscillatory nonlinear PR: step ratio up to 8.333 and same-error speedup below 0.1.
2. Small-state T4 execution: every problem median is below 1.
3. Legacy IOP2: enormous failed work under the nested heuristic.
4. Aggregate speedup is not regime-uniform; favorable linearized regimes dominate the median.

## Verdict

```text
Mathematical correctness foundation     PASS WITH SCOPE
Fifth-order evidence                    PASS on current bounded corpus
Generic stiff/oscillatory superiority   FAIL / HOLD
Phi residual rigor                      RESEARCH ESTIMATOR ONLY
```
