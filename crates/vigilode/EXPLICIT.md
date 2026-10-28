# Nonstiff integration with DOPRI5

`Dopri5` integrates an identity-mass ODE `y' = f(t, y)` using the
Dormand–Prince fifth-order endpoint and fourth-order embedded estimate.
Select it explicitly for a nonstiff problem. A supplied Jacobian or its
absence does not determine stiffness, so this entry point makes no automatic
method-selection claim.

```rust
use vigilode::{Atol, Dopri5, Options, Problem};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A nonautonomous problem with exact solution y(t) = t^2.
    let problem = Problem::new(1, |t, _y, dy| {
        dy[0] = 2.0 * t;
        Ok(())
    });
    let options = Options {
        rtol: 1e-8,
        atol: Atol::Scalar(1e-10),
        h0: 0.01,
        ..Options::default()
    };
    let solution = Dopri5::solve_observed(
        &problem, (0.0, 1.0), &[0.0], &options,
        &[0.0, 0.25, 0.5, 0.75, 1.0],
    )?.ensure_success()?;
    println!("final state: {:?}", solution.final_state);
    println!("RHS evaluations: {}", solution.stats.counters.rhs_calls);
    Ok(())
}
```

The stage workspace is allocated once for a solve. Subsequent accepted
steps reuse the endpoint derivative through FSAL. A rejected step preserves
the derivative at its unchanged base state. The RHS must represent a fixed
function throughout a solve; stateful callback diagnostics may count calls,
but changing the physical model behind an unchanged callback invalidates
derivative reuse. The low-level `Dopri5Config` allows FSAL to be disabled.

The explicit path never evaluates a Jacobian, a JVP, a matrix factorization,
a Newton solve, or the supplied partial time derivative. Time dependence is
handled by evaluating the RHS at the Runge–Kutta stage times. The existing
`Problem::autonomous` and derivative declarations are not needed for this
method. A declared mass matrix, `dense_output: true`, and event requests are
typed refusals before callback entry. Requested observation times are
reached by shortening actual steps. No interior interpolation is used.

`Atol::Scalar` and `Atol::Vector` use component scales
`atol[i] + rtol * max(abs(y_old[i]), abs(y_new[i]))` in the RMS error test.
The accepted difference estimate is local; global accuracy and stability
must be checked for the intended problem and time interval. Dissipative
stiff modes can force very small explicit steps even when their physical
amplitude is small. Use the protected `Rodas5p` path when those costs dominate.

`ExplicitSolution` and `ExplicitStats` have the separate schema
`vigilode.explicit-stats.v1`. The executed method is
`dopri5-5(4)-fsal`. The shared `Options::tier` is preserved as requested input
but is a RODAS5P routing setting; it does not relabel an explicit run as a
protected Rosenbrock run. `interior_error` has no effect because dense output
is refused. Successful output does not carry an interval or global-error
certificate.

The callback policy is explicit:

- `CallbackError::Recoverable` rejects the current trial and permits a
  smaller step. A repeatedly failing initial-state evaluation eventually
  returns a failed partial run.
- `CallbackError::Fatal` stops immediately with the last accepted state.
- A callback returning `Ok` must write every derivative component with a
  finite value. Incomplete, NaN or infinite output stops immediately. A
  domain condition that may recover with a smaller step should return
  `Recoverable` instead.

An integration that exhausts its attempt budget or encounters a fatal
runtime condition returns `Ok(ExplicitSolution { success: false, .. })` so
that accepted values and all costs remain available. Call `ensure_success`
to obtain a typed `ExplicitSolveError::Failed` that still contains that
partial run. `final_time` and `final_state` always identify the last accepted
physical state; the public output arrays also retain this endpoint after a
failure between requested observations.

`stats.counters.rhs_calls` includes failed user calls. `callback_work`
decomposes the same work into attempted, finite-completed, recoverable,
fatal and malformed outcomes. It must not be added to the total again.
Both rejected and accepted attempts remain in the reported counts.

## Dense observations and physical error control (2026-10-23)

`Dopri5::solve_dense_observed(problem, span, y0, options, times)` samples a
quartic continuous extension of accepted steps. Interior output times do not
shorten those steps or add RHS calls. This returns the requested samples; it
does not retain an arbitrary-query interpolant. Keep `options.dense_output`
false: that existing flag requests the retained-output interface and still
returns `ExplicitUnsupported::DenseOutput`. Events and nonidentity mass remain
unsupported. The original `solve_observed` still lands at every observation.

For smooth solutions the extension has local **state interpolation error**
O(h^5), while its differential residual generally has order O(h^4). The
endpoint estimator does not separately enforce an interior error bound.
Dense sampling can therefore be faster and less accurate at the same nominal
tolerance. Choose settings using achieved error at the requested observations.

`AffineErrorNorm::new(n, offset, row_major_lift, physical_atol, physical_rtol)`
constructs the fixed map `v = offset + L u`. `physical_atol` has length one or
`offset.len()`. Call `solve_observed_with_norm(..., times, &norm)` or
`solve_dense_observed_with_norm(..., times, &norm)` to use

```text
sqrt(sum_j ((L e)_j / (atol_j + rtol max(|v_old,j|, |v_new,j|)))^2 / m)
```

for embedded-error acceptance. The returned states are still the reduced `u`.
The constructor stores only nonzero lift entries; successful evaluations
allocate no scratch vectors and cost O(nnz(L)+m+n). The client must establish
that its reduction represents the intended dynamics. Rank deficiency yields a
seminorm on latent coordinates. This option does not certify global error,
positivity, conservation, or the accuracy of a physical model.

The stats schema receives additive `observation_mode` and `error_control`
fields. `options` and `adaptive` retain requested reduced tolerances; the
`error_control` fields record the actual physical tolerances when supplied.
All routes preserve attempted callback work, malformed-output detection and
the last accepted state. One redundant facade poison pass has been removed;
the kernel still poisons reused RHS buffers and both layers retain finite
checks required by their respective contracts.
