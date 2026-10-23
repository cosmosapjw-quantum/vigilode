# Opt-in RHS-first method switching

When a constant damped-rotation structure is known, select `LawsonDopri5`
before the black-box switching adapter. Pass `DampedRotationFlow` for A and
a `Problem` callback evaluating **only N** in `y'=Ay+N(t,y)`:

```rust
use vigilode::{DampedRotationBlock, DampedRotationFlow, LawsonDopri5, Options, Problem};
let flow = DampedRotationFlow::repeated(
    DampedRotationBlock { damping: 0.1, frequency: 1_000.0 }, 1
)?;
let residual = Problem::new(2, |_t, _y, out| { out.fill(0.0); Ok(()) });
let result = LawsonDopri5::solve(&flow, &residual, (0.0, 1.0), &[1.0, 0.0], &Options::default())?;
assert!(result.integration.success);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The declared block action is computed algebraically in O(n), while adaptive
DOPRI stages evaluate only N. It performs no derivative or matrix operations.
`result.integration.stats.counters.rhs_calls` counts residual callbacks;
`result.quotient_work` records analytic block actions separately. Endpoint
observations and an affine physical norm have explicit facade entry points.
Strong nonlinear or forcing scales remaining in N still require resolution.
The declaration is supplied by the caller; no automatic fit can silently
change the IVP. Automatic competition between this semilinear contract and a
full-RHS black-box contract is a subsequent extension.

`SwitchingSolver` integrates a smooth forward identity-mass initial-value
problem using DOPRI5 first. When that route exhausts a bounded attempt batch,
or completes a window with high observed step/rejection work, the adapter may
enter an explicitly authorized RODAS5P route. It always starts at a state
accepted by the previous driver. Its public API does not change `Rodas5p` or
`Dopri5` defaults.

```rust
use vigilode::{Atol, DerivativeFallback, Jacobian, Options, Problem,
               SwitchingConfig, SwitchingSolver};

let problem = Problem::new(1, |t, y, out| {
    out[0] = -10_000.0 * (y[0] - t.cos()) - t.sin();
    Ok(())
}).jacobian(Jacobian::dense(|_, _, jac| {
    jac[0] = -10_000.0;
    Ok(())
})).time_derivative(|t, _, ft| {
    ft[0] = -10_000.0 * t.sin() - t.cos();
    Ok(())
});
let options = Options {
    rtol: 1e-7, atol: Atol::Scalar(1e-10), ..Options::default()
};
let switching = SwitchingConfig {
    fallback: DerivativeFallback::ProvidedOnly,
    ..SwitchingConfig::default()
};
let solution = SwitchingSolver::solve(
    &problem, (0.0, 1.0), &[1.0], &options, &switching
)?;
assert!(solution.success, "{}", solution.message);
# Ok::<(), vigilode::SwitchingError>(())
```

## Derivative policy

| Policy | Allowed behavior |
|---|---|
| `Forbidden` (default) | RHS-only DOPRI5 throughout. Neither a declared Jacobian/JVP nor a time-derivative callback is entered. |
| `ProvidedOnly` | After observed explicit work pressure, allow the supplied dense/banded Jacobian or JVP route, with either autonomy or a supplied explicit time derivative. Any implicit finite-difference provider is rejected before user callbacks. |
| `AllowFiniteDifference` | Also allow the existing finite-difference Jacobian route, with all its RHS calls charged. |

`ProvidedOnly` covers both state and time derivatives: nonautonomous
problems must supply `df/dt`. Central differences in time require
`AllowFiniteDifference`. A correct `Problem::autonomous()` declaration
removes explicit time-derivative work.

Matrix-free JVP and derivative-free RHS integration have distinct meanings.
The actual route and original counters are retained for every batch.

## Selection and hysteresis

The default divides the interval into eight windows. Window count is a
resource/response setting, not a property of the differential equation.
Within each explicit window, an authorized solve gets at most 32 trial
attempts before route reconsideration. Reaching that limit, using at least
24 attempts in a completed batch, or rejecting at least 30% of attempts
activates the RODAS route. A fatal or malformed callback ends the solve;
another method never retries it.

While on RODAS, the adapter waits at least two completed implicit windows and
four windows since the last recheck. It then tries DOPRI5 from the same
accepted state with at most 16 attempts. The recheck is committed only if
it reaches the window endpoint and its work per unit time is at most 0.8
times the most recent implicit rate. Otherwise its numerical state is
discarded, its work stays charged, and RODAS starts from the unchanged
committed endpoint. A fatal recheck preserves its last accepted partial
state and stops the whole operation.

The deterministic work score is

```
RHS calls + JVP vectors + time-derivative calls + n * Jacobian builds
          + linear solves.
```

This score makes the policy reproducible. It is a heuristic for callback and
solve effort, not a measured time model. It does not capture all dense
factorization, preconditioner, vector-memory or user-callback costs. Rapid
oscillation may also produce high explicit work. A work-triggered switch
therefore does not diagnose stiffness or guarantee an elapsed-time saving.
Compare forced DOPRI5, forced RODAS and switching at the same achieved error.

## Budget and failure contracts

`Options::max_steps` is a single limit for **all** attempts across methods,
including discarded rechecks. Every batch receives at most the remaining
budget. If each driver respects its local limit, induction gives

```
sum(batch.attempts) <= options.max_steps.
```

The full segment list preserves every original driver record. Aggregate
`accepted_steps` counts work accepted inside a driver, including discarded
probes; it is not the count of points committed to `solution.t`.
`segment.committed` identifies that distinction. Fatal failures and budget
exhaustion preserve the last committed finite state and return
`success == false`. A driver-level error that supplies no work record sets
`stats.accounting_complete == false`; such a run cannot be reported as a
complete work measurement.

Possible dense fallback routes are refused above `max_dense_dimension`
(64 by default). A declared band on a dense provider also encounters the cap,
because route verification requests its dense matrix. A true band provider
or JVP provider follows its existing workspace contract. The adapter itself
creates no thread pool.

## Accuracy and scope

Supported requests are finite forward spans, identity mass, scalar or
componentwise absolute tolerance subject to the chosen RODAS route, and
endpoint output. Mass matrices, events and retained dense interpolation are
refused before callbacks. Returned points include window endpoints and
accepted partial endpoints at switches; there is no `at(t)` interpolant.

Every committed segment keeps the selected driver's embedded local error
test. Under the usual smoothness and stability assumptions, if both selected
one-step methods have local order six and their accepted step maps satisfy
`||Phi_h(u)-Phi_h(v)|| <= (1+L*h)||u-v||`, composing any sequence of such
steps gives fifth-order convergence on a fixed finite interval. Indeed,
`e_(k+1) <= (1+L*h_k)e_k+C*h_k^6` yields
`e_N <= exp(L*T) C*T*max(h_k)^5`. This is a conditional consistency result;
the controller's nominal tolerance is not a universal global-error bound.

Window restarts rebuild workspaces and discard DOPRI FSAL/controller history.
Small windows may therefore reduce speed. A persistent step-session API is a
subsequent optimization. Model or discontinuity changes must be represented
by explicitly separate solves; the adapter assumes deterministic callbacks
for a fixed IVP throughout a solve.

Contract tests: `cargo test -p vigilode --locked --offline --test switching_contracts`.
The experiment harness compares this policy with forced methods and retains
unfavorable work-precision results.
