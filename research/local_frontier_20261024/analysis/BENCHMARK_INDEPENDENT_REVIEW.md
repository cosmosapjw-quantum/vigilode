# Local benchmark model and execution review

Review date: 2026-10-24. Reviewer: the independent fixed-P continuation worker, who did not implement these benchmark models. Scope: `crates/vigilode/examples/local_bottleneck_suite.rs`, `crates/vigilode/examples/local_bottleneck_support/models.rs`, and `crates/vigilode/tests/local_bottleneck_models.rs`; facade endpoint/output behavior was also read. No model source was edited during this review.

## Decision and limits

No blocking mathematical or classification defect was found in the 24 default smoke/standard model definitions. This supports using the executable for the proposed local measurement campaign. It does **not** establish broad production readiness, a ranking of methods, accurate continuous trajectories, PDE spatial convergence, or advantages from parallelizing Rosenbrock stages.

Seventeen model definitions deliberately require an independent reference before accuracy-qualified speed comparisons. A successful solve on one of these remains `UNVALIDATED_REFERENCE`. The seven analytic or bounded references admit only a maximum-component **endpoint** error comparison. For rapidly oscillating or dissipative problems, small endpoint error can conceal earlier phase or transient errors; those remain separate research questions.

One defensive improvement was requested: reject malformed/nonfinite analytic reference vectors before the `zip`/maximum error reduction. Rust floating maximum can ignore a NaN operand, so reference validity must be explicit. The implementation owner added time-domain/finite-value checks in `Model::exact`, an independent length/finite check in the executable, and a NaN/negative-time/overflow test. These changes were independently read after implementation and their final receipts were inspected. They do not alter the default finite model equations.

## Equation and reference checks

All 24 component functions and their state indexing were read. The dual arithmetic implements the ordinary sum, product, quotient, sine, cosine, exponential, and power derivative rules. A state seed differentiates only state variables; a time seed differentiates only explicit time dependence. The global mean in case 6 carries the mean of the state-direction seed. Dense Jacobian assembly uses one basis seed per column and writes row-major entries `a[i*n+j]`; it is intentionally more expensive than a specialized analytic sparse assembly.

The model-specific discontinuities in cases 17, 18, and 24 are fixed spatial coefficients. They do not create unaccounted state derivatives in the supplied JVP. The hierarchy indexing in case 20 gives each degree `l` exactly `2l+1` values of `m`, pairs its lower and upper couplings with opposite signs, and truncates the upper boundary to zero. Thus the undamped streaming matrix is skew-symmetric in the represented Euclidean normalization. The supplied test checks its quadratic form against the diagonal scattering dissipation.

| Case | Independently checked reference or identity | Scope |
|---|---|---|
| 6, freeze-out transport | The periodic Fourier mode has transport eigenvalue `-0.1*n*(1-exp(-ik))`; damping integral is `(1e5*0.05/3)*(1-(1+t/0.05)^-3)`. The code has the correct real decay and negative phase sign. | Reference is for the discrete upwind ODE, not the continuum PDE. |
| 7, literal ignition control | Starting at `y=0.1`, `f(y)=(1-y) exp(50-50/y)`. On `[0.1,0.10001]`, `0 <= f(y) < exp(-449.95)`. Integration for `t<=10` gives a displacement far below `1e-190`, which closes the interval bootstrap. | Constant state is a bounded approximation, not an exact evolving solution; the explicit `1e-190` bound is added to the measured difference. |
| 10, relativistic gyromotion | Momentum norm is constant, `gamma=sqrt(2.04)`, angular frequency `500/gamma`, and `p=(cos(wt),-sin(wt),0.2)`. Integrating `x'=p/gamma` gives the stated positions and signs. | Uniform-field, zero-electric-field subcase only. |
| 15, exponential chirp | With `y=exp(-0.8t) cos(theta)`, `theta'=exp(1.2t)`, direct differentiation gives `y''=-0.4y'-(exp(2.4t)-0.32)y`. Initial derivative is `-0.8`. | Endpoint reference for the specified nonautonomous ODE. |
| 16, inspiral | The Cartesian equations give `r'=-0.1/r^3`, `theta'=r^-1.5`; hence `r=(1-0.4t)^1/4` and `theta=4*(1-r^2.5)`. | Only before the singularity at `t=2.5`; CLI refuses later endpoints. |
| 19, spherical modal waves | Each pair is a damped rotation with `w=sqrt(l(l+1))`, `d=1e-4*(l(l+1))^2`; the third component only damps. | Linear nonrotating modal model; no nonlinear shallow-water claim. |
| 22, anisotropic conduction | Fourier symbol is `-4*a*n^2*(sin(kx/2)^2+sin(ky/2)^2)-b*n^2*sin(kx)*sin(ky)`, where `a=0.5000005`, `b=0.999999`. This matches the nine-point stencil and initial mode. | Discrete periodic constant-tensor equation; no positivity theorem for the centered mixed-derivative stencil is implied. |

The case-5 slow manifold is `y0=cos(y1)`. Differentiating the transverse residual confirms a persistent stiff transverse derivative near `-1e6`, so post-transient stiffness disappearance is correctly **not** asserted. Case 7 is likewise correctly labeled as a negative ignition control. HIRES replaces an unspecified POLLU model explicitly; the low-Mach, spherical, hierarchy, Brusselator, and Barkley models retain their restricted variant names rather than claiming equivalence to a different full physical model.

The analytic formulas are evaluated in binary64. Except for the explicit case-7 evolution bound, this suite does not supply a rigorous directed-rounding enclosure of transcendental evaluation or of the represented initial data. Its numerical endpoint references are suitable for the stated practical error limits; they are not formal arbitrary-precision error certificates.

## Equality of task and classification

The executable gives every method the same initial vector, full time span, scalar tolerances, and requested final endpoint. The RODAS facade defaults to endpoint output. DOPRI returns an endpoint, while switching retains its own window records; this extra controller work is included in timing and disclosed in metadata. The comparison measures complete implementations, not equal internal stage counts or identical intermediate output storage.

The explicit method never calls the supplied JVP/time-derivative callbacks. The matrix-free and switching paths receive forward-dual JVPs; the dense path receives a Jacobian built from dual columns. Therefore these runs must not be presented as evidence that the measured implicit implementation is fully derivative-free. They isolate known derivative action from derivative approximation error. All callback/provider/workspace construction, rejected attempts and switching probes are included in the solve time; model creation, reusable thread-pool startup, diagnostics and JSON are excluded and disclosed.

A result is accuracy-qualified only if every ensemble member reaches the requested endpoint successfully with finite state, has a reference, and meets the common maximum-component absolute endpoint error limit. Instrumented runs are explicitly ineligible for a speed comparison. Dense dimension greater than 256 is `REFUSED_DIMENSION`, not a failed accuracy test and not a fast successful solve. Missing references remain unvalidated; conservation/energy diagnostics cannot upgrade them. Errors and unsuccessful partial solves remain failures even if their elapsed time is short. A shell exit status of zero alone is not a scientific success criterion; the machine-readable status is authoritative.

Equal `rtol`/`atol` does not guarantee equal actual global error, so the local campaign must compare only within matching case/size/span/endpoint/error-limit keys and use measured error curves. An ensemble run's time cannot be divided by a serial trajectory time and called single-trajectory speedup: compare ensemble throughput at equal member count or report amortized throughput explicitly.

## Actual parallel layers

| Reported layer | Actual code behavior | Excluded interpretation |
|---|---|---|
| `serial` | No Rayon pool is created; CLI requires exactly one configured thread. | No multi-core speedup claim. |
| `data` | A reusable pool of the requested size evaluates independent output rows within RHS/JVP/partial-time callbacks. Case-6 global mean is still reduced sequentially. Dense Jacobian columns are constructed sequentially, with rows within each column parallelized. | Not parallel stages, parallel time steps, or automatically parallel factorization. |
| `ensemble` | The pool schedules independent identical IVPs; each member uses serial callbacks and its own adaptive integration. Accuracy is tested per member, not through an enlarged combined WRMS. | Not parallelization of one causal trajectory. |

Callback instrumentation sums callback wall durations. Different ensemble members overlap, so those sums can exceed solve wall time and cannot be stacked as exclusive wall-time fractions. The metadata correctly states this. Thread-pool startup is separately reported. The independent source inspection found no hidden nested callback pool in the ensemble route.

## Verification evidence examined

The final `model-final-verification.log` reports 9 passing tests: all-model finite values and directional derivatives, analytic state equation checks, conservative telescoping, skew streaming/dissipation, bit-identical parallel row evaluation for 1/2/4 threads, both negative controls, nonautonomous partial-time derivative checks, the bounded reference's time domain, and invalid/overflowed reference refusal. These are practical finite-difference and structural checks, not whole-domain proofs for all nonlinear systems.

The reviewer parsed `evidence/model-executable-smoke.jsonl` independently: 56 results comprise 33 `PASS_SCOPED`, 22 `UNVALIDATED_REFERENCE`, and one dimension refusal. All recorded DOPRI executions have zero JVP, dense-Jacobian, and time-derivative callback counts. No recorded instrumented or non-passing result is marked speed-comparison eligible. These counts describe that retained smoke receipt, not the future local production-size campaign.

The raw executable receipt is bound to binary SHA256 `edf11cef5c5f0484cc326ab127264011352c6f922e4d7559b7e27692ef0f127d`. The 56-run smoke campaign was rerun after the defensive-reference correction with this final binary. `evidence/model-source-identity.json` matches independently computed SHA256 values: executable source `994fa3e67c9e92d275dc0763de4a126eeab4eb811a8090760dfd7dd7bc1d67a1`, model source `288cc9c54b9380dc35a59d52185cd0fe8f0b36947fafb08283048020efbdad2d`, tests `be0792dc4e3db821d5d039151de2a3d6eab5c92d3d3301462be3b8771b42d519`. Final test/build exit is zero. Earlier binary receipts remain historical evidence.

## Remaining obligations

1. Complete the local campaign with the retained model identity/variant and process-level timeout records. Standard-size stiff cases may exhaust resource or iteration budgets; preserve these outcomes.
2. Obtain independent references for the 17 unvalidated definitions before using them in accuracy-qualified solver rankings. The full discrete equations, dimensions, initial state, parameters and derivative availability must match the external implementation.
3. Add selected intermediate-output or phase/invariant checks for oscillatory and dissipative cases. Endpoint accuracy is intentionally narrower than the solver's eventual production contract.
4. Keep full-run and callback-instrumented measurements separate. High counter contention and pool scheduling overhead can dominate very small systems; those are measured implementation costs, not mathematical solver complexity.
5. Do not infer stiffness-switching superiority from the two negative controls, and do not infer spatial discretization accuracy from a time-integrator endpoint reference.

Final defensive-correction validation: completed; source guards independently inspected, 9-test PASS and the rebuilt 56-run smoke receipt verified. No unresolved blocking finding within this review scope.
