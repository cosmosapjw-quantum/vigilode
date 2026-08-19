# Adaptive Step Control for All Current Rust Integrators — Design Specification

Date: 2026-08-08
Base tag: `unified-fair-measurement-v0.11.0-alpha1`
Feature branch: `adaptive-all-methods-v0.12`

## 1. Goal

Add adaptive step-size integration to every currently executable complete-integrator
path without changing fixed-step behavior:

- Sequential RODAS5P with every existing linear solver;
- SABR5P;
- fixed-schedule Homotopy RODAS with protected fallback;
- BDF1 and BDF2;
- Radau IIA1 and Radau IIA3.

The work also defines, but does not implement, the adaptive contract required by future
SDC, Peer/W, ROK, BOROK, and exponential/Leja candidates.

## 2. Non-goals

- no BDF3–5 implementation;
- no native RADAU5 estimator or continuous output;
- no new homotopy path schedule;
- no sparse/MPI/GPU/index-1 DAE production claims;
- no universal solver ranking;
- no replacement of the v0.11 external global-error authority.

## 3. Architecture

### 3.1 Common controller layer

Create `crates/rodas5p-integrators/src/adaptive.rs` with:

- validated `AdaptiveStepConfig`;
- I and PI controller policies;
- `AdaptiveControllerState` holding only accepted-error history;
- bounded accepted/rejected step factors;
- method-supplied estimator order;
- method-independent step-doubling WRMS utility;
- adaptive diagnostics distinguishing macro attempts, accepted macro steps,
  rejected macro steps, actual internal method steps, clipping, and estimator IDs.

The controller never owns solver state.  It receives a dimensionless error and returns a
bounded dimensionless factor.

### 3.2 Native RODAS lane

Refactor existing Sequential/SABR adaptive functions onto the common controller without
changing their estimator or default numerical trajectory.  Homotopy uses the same embedded
RODAS endpoint error plus its existing certificate/fallback decision.  Rejected trials restore
stage history and recycle state.

### 3.3 BDF reference lane

Keep public fixed-step `bdf_step` behavior unchanged.  Add an internal variable-step mode
using

\[
\frac{1+2r}{1+r}y_{n+1}-(1+r)y_n+
\frac{r^2}{1+r}y_{n-1}=h_nf_{n+1},
\quad r=h_n/h_{n-1}.
\]

Adaptive BDF uses one coarse step and two half steps from cloned histories, accepts the fine
state/history, and counts all trial work.  Until both paths genuinely use BDF2, the estimator
uses `p=1`; thereafter it uses `p=2`.

### 3.4 Radau reference lane

Adaptive Radau uses the same coarse/two-half-step construction.  The estimator order is two
for Radau IIA1 and six for Radau IIA3.  Full dense Newton remains the reference nonlinear
backend.

### 3.5 Output semantics

All adaptive APIs use the v0.11 `OutputSchedule`/`OutputCollector`; requested output times
are reached by step clipping.  No unverified interpolation is introduced.  Observer state is
mutated only after an accepted macro step.

### 3.6 Work semantics

Coarse and fine trials, rejected attempts, Newton iterations, factorizations, RHS calls,
JVPs, and fallback work remain in `WorkCounters`.  Macro-step diagnostics are separate from
actual internal accepted-step counts.

## 4. Public interfaces

New exported types/functions:

```rust
pub enum ControllerKind { Integral, Pi }

pub struct AdaptiveStepConfig { /* validated bounds and tolerances */ }
pub struct AdaptiveRunDiagnostics { /* macro and estimator history */ }
pub struct AdaptiveObservedIntegrationResult {
    pub observed: ObservedIntegrationResult,
    pub diagnostics: AdaptiveRunDiagnostics,
}

pub fn integrate_bdf_adaptive_observed(...)
    -> CoreResult<AdaptiveObservedIntegrationResult>;
pub fn integrate_radau_adaptive_observed(...)
    -> CoreResult<AdaptiveObservedIntegrationResult>;
pub fn integrate_homotopy_adaptive_observed(...)
    -> CoreResult<AdaptiveObservedIntegrationResult>;
```

Existing `integrate_adaptive_observed` remains source-compatible and delegates to the new
controller defaults.

## 5. Error handling and transactional rules

A trial is rejected and rolled back for:

- estimator greater than one;
- candidate-level rejection;
- linear/nonlinear convergence failure classified as recoverable;
- nonfinite endpoint or estimator;
- proposed step below the validated minimum.

Invalid configuration, dimension mismatch, and programming/API errors remain hard errors.
Repeated rejection terminates explicitly at the configured attempt budget.

## 6. Testing

TDD gates:

1. controller validation and exact factor formulas;
2. step-doubling estimator on known synthetic coarse/fine errors;
3. variable-step BDF2 polynomial exactness and constant-step regression;
4. BDF history rollback and fine-history commit;
5. Radau rejection/acceptance and order response;
6. native RODAS regression against the previous adaptive path;
7. homotopy fallback rollback;
8. tolerance ladders and observed convergence order;
9. common output grid and requested-output-only storage;
10. 1-thread/4-thread scientific identity for the adaptive comparison screen.

## 7. Promotion boundary

Promote the adaptive correctness/reference layer when all current executable families have
finite, transactional adaptive runs and the expected convergence order on bounded analytic
screens.  Keep production-efficiency claims on HOLD until matched-global-error Pareto results
show a benefit and native BDF/Radau estimators are implemented.
