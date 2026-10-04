# PP10 — Newton-Leja phi-action candidate with an EstimateOnly status (prospective registration)

RVJ DAG node `PP10`. Base commit: the commit that adds this file.

## Candidate backend (new, opt-in, `core/src/leja_action.rs`)

Real Leja points on a verified spectral interval `[a, b]` of a
`SymmetricNonpositiveOperator` (Gershgorin), with the scaling and
`phi_k` Newton divided differences computed in extended care (the
standard scaled recurrence of Caliari et al.); the action `p(hA) w_k` by
the Newton recurrence with `apply_rows` (charged as operator products); a
stopping estimate from the last two Newton terms. Status is always
`EstimateOnly`: no total witness is claimed (spectrum-only bounds are not
certificates, and a complete bound is not available).

## Comparison (fixed now)

The PP08 fixtures (n in {5, 16}, rho in {2, 30, 200}, h in {1e-3, 3e-2,
0.2}, distinct and same-vector inputs) at estimated tolerances 1e-8 and
1e-12: Leja vs Chebyshev `joint_phi_action` on the same target, recording
actual error against the 50-digit reference, operator vector products,
coefficient setups, and the Leja estimate.

## Gate

G1 Status is `EstimateOnly` in every report; no Leja result is admitted
   through any certified API (contract test).
G2 Accuracy of the candidate: actual error <= 10 x the requested
   estimate tolerance in at least 90 % of cases (a property of the
   candidate, not a certificate); every case where the estimate
   under-reports the actual error is listed.
Reported: products vs Chebyshev at matched actual error; no speed claim.

## Results (append only after the recorded run)
