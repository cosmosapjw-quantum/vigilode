# PHYS–MATH AUDIT — G4-S5B3 Threshold-Free Early-Defect Geometry

## Authority quantity and conventions

The read-only diagnostic is evaluated at the first dependency level of the
canonical fused `pexprb54s4` implementation, where

\[
c_2=\frac14,
\qquad
U_2=y_n+h c_2\varphi_1(c_2hJ_n)F_n,
\]

\[
D_2=F(U_2)-F_n-J_n(U_2-y_n),
\]

and, on the declared physical component prefix of length \(m\),

\[
\eta_{c_2}
 =\frac{|h|\,\|D_2^{(m)}\|_2}
        {\|U_2^{(m)}-y_n^{(m)}\|_2}.
\]

The campaign uses regular ODEs or time-augmented regular ODEs, full-MGS
Krylov actions, one execution thread, and endpoint-only requested output.
The augmented clock coordinate is excluded from the norm.

## Checks that pass

1. **Dimensions:** \(D_2\) has state/time units, so \(|h|D_2\) and the stage
   increment have state units. Their ratio is dimensionless when the retained
   components share compatible scaling.
2. **Linear limit:** the complex-Dahlquist rows give
   \(\eta_{c_2}\simeq 2.6\times10^{-16}\)–\(4.2\times10^{-16}\), consistent
   with the vanishing nonlinear remainder in the affine limit.
3. **Finite coverage:** all 248 attempts have finite \(\eta_{c_2}\); there are
   no zero-increment degeneracies or nonfinite normalizations.
4. **Outcome coverage:** 241 accepted and 7 error-control-rejected attempts are
   aligned with complete trial-work and controller histories.
5. **No local-error overclaim:** the analysis never treats \(\eta_{c_2}\) as a
   local-error bound and never selects a threshold.

## Load-bearing negative result

A raw global \(\eta_{c_2}\) threshold cannot be an outcome-complete admission
rule across tolerances. The quadratic first trial has the exact same

\[
\eta_{c_2}=9.919814843468982\times10^{-4},
\qquad h=0.0625,
\]

at all three tolerances, but its actual adaptive error norm is

\[
0.003102\;(10^{-4}),\qquad
0.234141\;(10^{-6}),\qquad
23.414145\;(10^{-8}),
\]

so the first two attempts are accepted and the third is rejected. Acceptance
is tolerance-scaled; the raw defect ratio is not. Thus accept/reject outcome is
not a single-valued function of raw \(\eta_{c_2}\).

The complete threshold sweep confirms the operational consequence:

- all-admit baseline: 7 false accepts, 0 false rejects;
- best zero-false-accept point: 0 false accepts but 201/241 accepted attempts
  falsely rejected; only 40 accepted attempts are retained;
- the minimum total false-count is 7 and is achieved only by all-admit.

The descriptive pooled rank AUC is 0.723, but this does not rescue the rule:
there are only seven rejected attempts, attempts are serially dependent, and
count-weighted decision cost is not improved over all-admit.

## Scaling and sampling limitations

- Raw \(L^2\) normalization is not invariant under component-wise unit or scale
  changes.
- 208/248 attempts (83.9%) come from oscillatory-PR; 154/248 (62.1%) come from
  \(\mathrm{rtol}=10^{-8}\).
- Six of seven rejected attempts are oscillatory-PR and one is quadratic;
  advection-diffusion and complex-Dahlquist contain no rejected examples.
- All seven rejected attempts occur at \(t=0\), in attempt index 0 or 1. A
  re-entry-conditioned diagnostic may therefore have different geometry, but
  that view was not sealed in this node and is not promotion evidence.
- The current corpus is small/medium dense; there is no \(N=2048\) or sparse
  holdout.

## Overhead interpretation

The seven alternating paired measurements give a read-only/disabled median
wall and \(\Gamma\) ratio of 0.9973, MAD 0.0117, and observed P95 1.0189.
Ratios below one are timing noise, not a telemetry speedup. The supported claim
is only that no material positive overhead was resolved in this run; this is
not a platform-independent upper bound. JSON serialization and Python analysis
are intentionally outside the solver-path timing.

## Verdict

- **Telemetry mechanism:** PASS.
- **Complete attempt/work geometry:** PASS.
- **Raw \(\eta_{c_2}\) as a blanket global admission threshold:** REJECTED.
- **Threshold choice, early abort, active switching:** NOT AUTHORIZED.
