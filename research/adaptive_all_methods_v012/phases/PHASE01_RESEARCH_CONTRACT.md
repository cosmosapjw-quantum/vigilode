# Phase 1 — Research Contract: Adaptive Step Control Across All Current Methods

Date: 2026-08-08
Baseline: `unified-fair-measurement-v0.11.0-alpha1`

## PRIMARY_RQ

Can every currently executable integrator family in the Rust registry receive adaptive step-size control under one external global-error authority, while preserving each method's mathematical identity, transactional state, fixed-step baseline, and complete work accounting?

## SUB_RQS

1. Which local-error estimator is defensible for each family: embedded RODAS estimator, nonlinear output certificate, step doubling, multistep history estimator, collocation defect, or cost-aware estimator?
2. How must BDF2 history and coefficients change when consecutive step sizes differ?
3. Can one controller contract be shared without pretending that all estimator orders and costs are identical?
4. How are rejection, output-time clipping, recycle/history rollback, and fallback handled consistently?
5. What adaptivity contract must deferred SDC, Peer/W, ROK/BOROK, and exponential/Leja candidates satisfy when implemented later?

## IN_SCOPE

- Sequential RODAS5P with direct/GMRES/LGMRES/GCRO-DR inner solvers.
- SABR5P and homotopy RODAS endpoints.
- BDF1, variable-step BDF2, Radau IIA1, Radau IIA3.
- A shared bounded I/PI controller contract.
- Step doubling as a method-independent reference estimator.
- Common requested-output observer and external global-error Pareto evaluation.
- Research-only adaptive contracts for deferred method families.

## OUT_OF_SCOPE

- Implementing variable-order BDF3–5.
- Implementing deferred SDC, Peer/W, ROK, BOROK, or exponential/Leja integrators.
- Sparse/MPI/GPU/index-1 DAE production claims.
- Declaring a universal performance winner.
- Treating a nonlinear homotopy output correction as a rigorous error bound.

## CONVENTIONS

- State error is normalized by dimensionless WRMS scaling
  \(s_i=\mathrm{atol}+\mathrm{rtol}\max(|y_{n,i}|,|y_{n+1,i}|)\).
- Step size \(h\) has units of time. Step ratios and normalized errors are dimensionless.
- A method of global order \(p\) uses a local-error controller exponent based on an estimator of order \(p+1\), unless a native embedded estimator has a different documented order.
- Rejected trials commit neither state, multistep history, stage history, recycle space, nor output-observer state.
- Internal tolerance equality is not a cross-family fairness criterion; external global error is.

## EVIDENCE TYPES

1. Primary method papers and official solver documentation/source.
2. Exact algebra for variable-step formulas and estimator scaling.
3. Analytic-limit and manufactured-problem convergence tests.
4. Holdout global-error/work measurements using the v0.11 authority.

## COMPLETION BAR

- A source-audited estimator/controller map for every current and deferred family.
- Exact variable-step BDF2 formula and constant-step limit verified.
- A common adaptive-controller contract with method-specific estimator order.
- A bounded coding plan covering all current executable families.
- Explicit kill gates for order loss, false acceptance, history corruption, and cost blow-up.
