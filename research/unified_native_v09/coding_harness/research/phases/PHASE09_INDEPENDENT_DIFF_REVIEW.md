# Phase 9 — Independent Diff Review

Review base: `7aeda15` (common dense nonlinear contract) plus the bounded BDF/Radau implementation through the current feature commit.

## Blocking findings resolved

### Important — complete integrators entered RODAS-stage-only screens

New `Bdf`/`RadauIrk` execution variants made stage-only matches non-exhaustive and risked comparing mathematically different objects in the same one-step stage certificate. The fix introduces explicit candidate-domain classification, excludes complete integrators from stage-only execution, and routes them to `run_native_integrator_gates`.

### Important — redundant exact-linear Newton iteration

The common Newton solver required both a small residual and a small preceding correction. An exact linear Newton step therefore performed a second Jacobian factorization solely to obtain a zero correction. The fix accepts a converged residual after one correction only when the unscaled residual is at the roundoff floor; nonlinear roots still require correction convergence unless that stronger condition is met. A regression test proves one factorization for an exact linear root while preserving the nonlinear square-root accuracy test.

## Remaining nonblocking concerns

1. BDF2 is constant-step only. A shortened last step intentionally falls back to BDF1, so production order claims require a variable-step formula and controller.
2. Dense full Newton scales cubically and is a reference anchor, not a sparse/JFNK production implementation.
3. The operational residual tolerance is not a rigorous output-error certificate for ill-conditioned nonlinear systems.
4. Radau IIA3 has no embedded estimator or adaptive controller.
5. The native scientific gate is intentionally narrow; global-error/total-cost Pareto comparisons and broader stiff/nonnormal/DAE suites remain future work.
6. Candidate catalog growth now spans multiple mathematical tiers. Future additions must declare their comparison tier before becoming executable.

## Verdict

`PASS WITH SCOPE AND PERFORMANCE HOLD`. No critical or important finding remains unresolved within the stated fixed-step anchor scope.
