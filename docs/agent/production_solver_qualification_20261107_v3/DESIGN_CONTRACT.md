# Design contract: VigilODE production qualification v3

## 1. Mathematical object and claim boundary

The baseline problem is
\[
y'(t)=f(t,y(t)),\qquad y(t_0)=y_0,\qquad t\in[t_0,T].
\]
The current public production adapter is limited to smooth forward identity-mass IVPs with endpoint observations, subject to provider restrictions. Do not silently broaden this to DAEs, arbitrary mass matrices, events, backward integration or dense-output APIs.

Keep four mathematical objects distinct:
1. a floating-point numerical trajectory;
2. error at a declared discrete output grid;
3. continuous-time global error over \([t_0,T]\);
4. a validated enclosure that proves existence, uniqueness and containment.

For grid \(G=\{t_k\}_{k=0}^{m}\), define
\[
E_k=\left[\frac1n\sum_{i=1}^n
\left(\frac{y_{h,i}(t_k)-y_{ref,i}(t_k)}
{a_i+r|y_{ref,i}(t_k)|}\right)^2\right]^{1/2},
\qquad E_G=\max_k E_k.
\]
This is a maximum over sampled times of per-time WRMS values. It is not \(\sup_{t\in[t_0,T]}\) unless a continuous enclosure or proved interpolation bound exists.

Benchmark metric distinction:
- `error_wrms_own`: each run's own requested rtol and atol scale.
- `error_common`: tolerance-independent common scale for cross-arm work-precision comparisons.
Never interchange these metrics.

## 2. Work-precision protocol

Admit a matched comparison at common error level \(\epsilon\) only if:
- both arms finish successfully and return the full declared grid;
- both outputs are deterministic across the declared repetitions;
- both achieved-error curves bracket \(\epsilon\);
- log-log interpolation occurs strictly inside observed brackets;
- no extrapolation, reference-disputed row or dropped failure contributes;
- all failed/refused/speculative work remains counted.

The v1 spatial `vigilode` and `vigilode-banded` arms are grid-clipped continuity controls: they land steps at output times. They are not free-stepping work-precision baselines. The benchmark code already has separate `vigilode-dense` and `vigilode-banded-dense` endpoint-only output arms. Include these in v2 and assert `output_clipped_steps == 0`.

Dense Jacobian means a full matrix representation/factorization. Dense output means a continuous interpolant used to observe output times without clipping integration steps. These are not synonyms.

Dense-versus-banded timing inside one solver family is an implementation diagnostic, not a cross-solver speedup. Single-host wall-clock results remain `diagnostic_only` until a preregistered multi-run timing protocol and uncertainty estimate are satisfied.

## 3. Reference uncertainty

The v1 spatial reference compares RADAU5 and CVODE at one tolerance level on 101 points. This is empirical cross-family agreement only.

The v2 protocol must:
1. bind identical model, initial state, grid, units and norm;
2. preregister decreasing tolerance/atol levels;
3. record all solver statuses, states, work and inter-level refinement;
4. add a third family or higher-precision arbiter where feasible;
5. define reference uncertainty before interpreting candidate errors.

Stagnating or disputed reference refinement blocks performance promotion. Never call finite-precision cross-checks a rigorous error enclosure.

## 4. Validated numerical-analysis boundary

A standard floating-point local error estimator does not prove a global enclosure. A validated IVP claim needs:
- sound directed-rounding interval or Taylor-model representation;
- sound interval extensions of all callback/model functions;
- a priori enclosure on each time slab;
- an inclusion/existence/uniqueness argument;
- a sound defect or truncation remainder;
- global propagation with wrapping/dependency control;
- independent analytic and adversarial validation.

A sampled observer, pointwise Gershgorin estimate or ordinary floating-point residual is not a continuous certificate. Keep diagnostic/estimate types separate from certificate types at the consumer seam.

Primary literature:
- N. S. Nedialkov, K. R. Jackson, G. F. Corliss, “Validated solutions of initial value problems for ordinary differential equations”, Applied Mathematics and Computation 105(1), 21–68 (1999), DOI 10.1016/S0096-3003(98)10083-8.
- M. Neher, K. R. Jackson, N. S. Nedialkov, “On Taylor Model Based Integration of ODEs”, SIAM Journal on Numerical Analysis 45(1), 236–262 (2007), DOI 10.1137/050638448.

## 5. Spatial physical-model scope

The spherical benchmark is a fixed-grid finite-volume ODE model in the stated BASS/rei P06 oracle scope:
- non-tilted triaxial Bianchi-I;
- constant benchmark geometry;
- unpolarized rank-six Thomson kernel;
- q, mu, phi cell state and explicit absorber/ledger slots;
- declared energy/angular boundary conditions.

It is not a full polarized, tilted Einstein–Boltzmann solver. Do not infer EE/BB transport, tilt, reionization interfaces or general anisotropic cosmology coverage.

The semilinear advection-diffusion systems are method-of-lines ODEs with the exact stencils and boundary conditions in `crates/vigilode-bench/src/model.rs`. Separate ODE time-integration error from PDE spatial discretization error. Require mesh refinement or a manufactured solution before claiming spatial convergence order.

## 6. Allocation measurement

Separate setup/problem construction, workspace creation, numerical hot loop (including rejected proposals and speculative pilots), output sampling/storage, teardown and reporting. The spatial run's allocator window includes setup, integration, output states and teardown. It cannot establish zero hot-loop allocations. Use a calibrated allocator positive control and measure complete production attempts with setup outside the window.

## 7. Switching and API

The live dispatcher is `DriveState` in `switching.rs`. `HybridRouteAutomaton` is an offline discrete verification model unless a live call path is shown.

`ProductionSolver::solve` may return `Ok(solution)` even when `solution.success == false`. Callers must inspect success, final_time, message and accounting_complete, or use `solve_checked` / `ensure_completed`. Partial accepted prefixes and work counters must remain inspectable on failure.

## 8. Claim admission

Keep scientific claim support, test verdict, work decision, failure kind, result validity, provenance validity and packaging validity orthogonal. Do not relax carried-forward thresholds without current authority.
