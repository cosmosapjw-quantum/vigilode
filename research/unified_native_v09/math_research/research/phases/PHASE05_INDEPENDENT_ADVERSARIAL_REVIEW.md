# Phase 5 — Independent Adversarial Review

| Hypothesis | Role | Adversarial finding |
|---|---|---|
| H-001 | DEFENDED / TRADEOFF | Correctly acknowledges that cheap diagnostics are not proofs. Risk: two lanes can become an excuse to promote empirically tuned heuristics. Require holdout false-accept gate and explicit `rigorous=false` metadata. |
| H-002 | NEEDS_MORE_EVIDENCE | Mathematically plausible, but lower-triangular does not imply small condition number. Must include a Jordan-chain kill screen and compare against a generic full norm bound. |
| H-003 | DEFENDED | BDF1/2 is bounded and high-information. Do not present fixed-step anchors as CVODE equivalence. |
| H-004 | DEFENDED | Exact tableau and stability properties make it a strong oracle. Dense Newton and no embedded estimator limit performance claims. |
| H-005 | DEFENDED | Prevents tolerance gaming. Reference-error uncertainty and failed-run accounting must be first-class. |
| H-006 | NEEDS_MORE_EVIDENCE | Variable-order BDF is a substantial independent project, not a small extension. |
| H-007 | TRADEOFF | Reuses Radau infrastructure but optimized sweeper coefficients must come from the published method or a separate derivation; no arbitrary tuning on evaluation data. |
| H-008 | NEEDS_MORE_EVIDENCE | Strong parallel design but two-step state and variable-step theory complicate events/restarts. |
| H-009 | DEFENDED AS CAPABILITY LANE | ROK/BOROK may dominate only at large N. Dense toy rankings are scientifically irrelevant. |
| H-010 | DEFENDED AS CAPABILITY LANE | Leja is natural for semilinear problems but incomparable on unsupported DAE/mass-matrix cases. |

## Red-team conclusions

1. The largest current blocker is not a missing threshold but missing certified nonlinear remainder information.
2. A universal registry is useful only if unsupported candidate/problem pairs are labelled `NOT_APPLICABLE`, not failed or silently omitted.
3. Native anchors must expose all nonlinear residual/Jacobian/factorization work; otherwise BDF/Radau will appear artificially cheap.
4. Fixed-step anchors can test mathematics, but adaptive-production claims remain forbidden.
