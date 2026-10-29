# Local CPU benchmark model contract

This executable implements every numbered slot in the supplied 24-system proposal. All equations use dimensionless variables. A numbered slot is not evidence that a particular physical application, a named canonical test, or a continuous PDE has been validated. The definitions below distinguish canonical problems, explicitly selected variants, and contradictory claims in the proposal. The two small problems with incorrect switching interpretations remain useful negative controls.

`local_bottleneck_suite --list` supplies the machine-readable catalog. `--describe --case I --size smoke|standard` supplies initial values, dimensions, coefficients, two arbitrary-state RHS probes, and the available endpoint reference. Every full solve starts at time zero from the same initial condition. `--end T` changes only its endpoint; it does not move the initial epoch. The companion campaign runner separates throughput, single-trajectory latency, callback profiling, and independent cold solves at intermediate times.

## Definitions and workload sizes

| Slot | Implemented system | Smoke / standard dimension | Standard endpoint | Qualification |
|---|---|---:|---:|---|
| 1 | Robertson, rates 0.04, 10⁴, 3·10⁷ | 3 / 3 | 40 | Canonical three-species model, initial (1,0,0). Ensembles are independent trajectories, not a spatially coupled reaction PDE. |
| 2 | HIRES | 8 / 8 | 321.8122 | The attachment explicitly permits HIRES as an alternative. Canonical eight equations and initial y₁=1, y₈=.0057; never labelled POLLU. |
| 3 | Pleiades | 28 / 28 | 3 | Canonical seven bodies with masses 1,…,7, original initial data, no softening. A collision is not silently regularized. |
| 4 | Local Kuramoto–Sakaguchi | 16 / 4096 | 10 | Periodic neighbours, radius 4 / 32, coupling 1, lag .2, ωᵢ=.5+.1cos(2πi/N). The self term is excluded explicitly. This is real nonlinear trigonometric work, with no artificial delay or dummy arithmetic. |
| 5 | Adiabatic-flash equations from the attachment | 2 / 2 | 2 | ε=10⁻⁶, initial (0,1). Persistent transverse stiffness: a negative control for unjustified switch-down. |
| 6 | Conservative freeze-out transport | 16 / 1024 | 2 | Constant velocity .1, periodic first-order upwind, global mean relaxation, Γ=10⁵/(1+t/.05)⁴. |
| 7 | Literal proposed ignition equation | 1 / 1 | 10 | Initial .1 and ε=.02. It does **not** ignite over this interval; negative control, not an ignition performance result. |
| 8 | Oregonator | 3 / 3 | 30 | s=77.27, q=8.375·10⁻⁶, w=.161, initial (1,2,3). No claim of a 512-cell ionization-front implementation. |
| 9 | Periodic FPUT β chain | 16 / 512 | 1 | Half positions, half momenta; homogeneous spring 10⁶ and quartic coefficient 1. The attachment's displayed homogeneous equation is used, not an alternating-mass model. |
| 10 | Relativistic uniform-field gyromotion | 6 / 6 | 1 | B=(0,0,1), E=0, ωc=500, initial position 0 and momentum (1,0,.2). This exactly solvable subcase does not cover arbitrary inhomogeneous fields. |
| 11 | Two-scale Lorenz-96 variant | 55 / 396 | 5 | 5 / 36 slow variables, ten fast variables per slow variable; global periodic fast ring and the displayed forward-advection sign. |
| 12 | Kuramoto–Sivashinsky | 16 / 1024 | 10 | Periodic length 32π, ν=1, centered D₁,D₂,D₂²: u′=−uD₁u−D₂u−D₂²u. |
| 13 | Low-Mach primitive acoustic relaxation | 24 / 3072 | .1 | Three fields on 8 / 1024 periodic cells. Explicitly defined primitive model below, not a complete unspecified compressible-Euler/IMEX package. |
| 14 | Advected Cahn–Hilliard | 16 / 4096 | 1 | Length 2π, ε=.02, velocity .1; periodic centered conservative semidiscretization. |
| 15 | Analytic exponential chirp | 2 / 2 | 8 | γ=.8, β=1.2, ω₀=1; initial (1,−.8). Exact state available. |
| 16 | Inspiral chirp | 2 / 2 | 2.49 | First-order Cartesian position, α=.1, ω₀=1, initial (1,0). No redundant velocity variables; t<2.5 is enforced. |
| 17 | Spherical rotation–shear transport | 32 / 2048 | 1 | Equal-μ finite volumes, 4×8 / 32×64, first-order upwind, zero polar face flux and periodic azimuth. |
| 18 | Spherical anisotropic diffusion–absorption | 32 / 4096 | .1 | 4×8 / 32×128; Dμ=.01, Dφ=.1(1+μ²/2), opacity 1001 in fixed patch and 1 outside. |
| 19 | Linear spherical modal waves | 48 / 3072 | 1 | Three real variables per spherical mode, 16 / 1024 modes. Linear nonrotating acoustic pairs plus hyperviscosity; not nonlinear SWE or a Rossby–Haurwitz test. |
| 20 | Normalized multipole hierarchy | 16 / 1024 | 2 | Fixed-m antisymmetric streaming chains; top moment zero, prescribed bath, Γ=10⁴/(1+t)⁴. Not a fully coupled baryon–photon system. |
| 21 | Periodic 2D Brusselator | 32 / 8192 | 1 | A=1, B=3.4, diffusion .01, signed x derivative +.1Dₓ, grids 4² / 64². Canonical BRUSS2D boundary/source terms are not implied. |
| 22 | Oblique anisotropic 2D diffusion | 16 / 4096 | .1 | Nine-point centered tensor, κ∥=1, κ⊥=10⁻⁶, b=(1,1)/√2, periodic 4² / 64² grid. |
| 23 | 3D Barkley reaction–diffusion | 128 / 27648 | 5 | Two fields, 4³ / 24³, Du=.001,Dv=0, ε=.02,a=.75,b=.02. The supplied polynomial is Barkley kinetics; the code does not call it Schnakenberg or Gray–Scott. |
| 24 | 3D absorbing-obstacle transport | 64 / 32768 | .1 | Periodic 4³ / 32³, diffusion .01, upwind velocities (.1,.05,.02), opacity 10⁶ in central radius-.2 ball and 1 outside, fixed Gaussian source. |

The smoke endpoints are deliberately shorter and are exported by `--describe`. Smoke validates implementation and plumbing; it does not test long-time chaos, all switching transitions, or the full standard-size memory hierarchy.

For #13, with centered periodic derivatives, the selected equations are

\[
\rho'=-uD_x\rho-\rho D_xu,\quad
u'=-uD_xu-\frac{D_xp}{10^{-6}\rho},\quad
p'=-uD_xp-1.4pD_xu-10^3(p-\rho).
\]

Here the middle variable is velocity \(u\), not viscosity. Initial density and pressure are \(1+.001\sin(2\pi x)\), initial velocity .1. Positive density/pressure and finite evaluations are enforced through recoverable callback errors.

For #17, \(\mu=\cos\theta\), \(a=1/\sqrt2\),
\(v_\mu=a\sqrt{1-\mu^2}\sin\phi+.1\mu(1-\mu^2)\) and
\(v_\phi=a[1-\mu\cos\phi/\sqrt{1-\mu^2}]\).
Conservative face fluxes telescope exactly in real arithmetic. This proves total discrete mass conservation, not nonnegativity under an arbitrary high-order time integrator.

For #20, the streaming coefficient from degree ℓ−1 to ℓ is
\(a_{\ell m}=\sqrt{\ell^2-m^2}/\sqrt{4\ell^2-1}\), with the opposite sign for the transpose connection. Thus \(K^T=-K\), and with scattering weights \(w_0=0,w_2=.9,w_\ell=1\) otherwise,
\(\frac12(d/dt)\|y\|_2^2=-\Gamma\sum w_\ell y_{\ell m}^2\le0\).
This is an independently tested semidiscrete energy identity.

## Corrections to the supplied proposal

1. **Boundary-layer decay is not disappearance of stiffness (#5).** For \(z=y_1-\cos y_2\), the exact transformed equation is \(z'=-z/\epsilon\). The eigenvalue −1/ε remains after z becomes small. An explicit method can still be stability limited, and roundoff/reconstruction perturbations can excite the fast direction.
2. **The literal ignition formula does not have the advertised endpoint derivative (#7).** At T=1 its derivative is −1, not −exp(1/ε). At T=.1 the exponent is −450. On .1≤T≤.100001, the RHS is below 10⁻¹⁹¹, so a first-exit argument gives |T(t)−.1|<10⁻¹⁹⁰ for 0≤t≤10. The reference .1 is therefore accompanied by this absolute bound; endpoints beyond 10 are unvalidated. A separately named Kapila slow–fast ignition model is a future addition, not a retroactive relabelling of this equation.
3. **Freeze-out exponent (#6).** The overview says −3 while the displayed differential equation uses −4. The executable follows the displayed equation and records exponent 4 in its parameters.
4. **KS signs (#12).** Moving the two even spatial derivatives from the left side to the RHS yields −uₓₓ−νuₓₓₓₓ. Hence long modes may grow while sufficiently short modes decay. Reversing the fourth-order sign would create a different ill-posed semidiscretization.
5. **Unspecified geometry and kinetics need explicit variants.** Fields, ICs, boundaries, truncation conventions and coefficients not supplied in the proposal are fixed above and in the descriptor. In particular #19 is a linear modal experiment, #23 uses Barkley kinetics, and #24 uses absorption rather than a newly meshed impenetrable obstacle boundary.

## Independent endpoint references

Six cases have explicit semidiscrete solutions; one has a rigorous negligible-evolution bound. The remaining seventeen have no independent reference embedded in this executable and are emitted as `UNVALIDATED_REFERENCE` even when the solver reaches the endpoint. Solver self-convergence is not an independent reference. Narrow external adapters may supply separately recorded comparisons after verifying the model contract and RHS probes.

* **#6.** The mean is constant. A Fourier mode θ=2π/N has upwind eigenvalue λ=−.1N(1−e⁻ⁱθ). Its amplitude is multiplied by \(\exp[\lambda t-G(t)]\), where \(G(t)=10^5(.05)/3[1-(1+t/.05)^{-3}]\). This solves the actual periodic semidiscrete ODE, including its numerical diffusion.
* **#7.** The bound above replaces a falsely claimed ignition trajectory. Its 10⁻¹⁹⁰ uncertainty is added to the measured reference difference before admission.
* **#10.** γ=√2.04 is constant and ω=500/γ. Momentum is (cos ωt,−sin ωt,.2); position is (sin ωt/500,(cos ωt−1)/500,.2t/γ).
* **#15.** \(x=e^{-.8t}\cos\Phi\), \(\Phi=(e^{1.2t}-1)/1.2\), with its analytic velocity. Direct differentiation gives \(x''+.4x'+(e^{2.4t}-.32)x=0\).
* **#16.** \(r=(1-.4t)^{1/4}\), \(\phi=4(1-r^{5/2})\), and \((x,y)=r(\cos\phi,\sin\phi)\). These satisfy radial decay \(r'=-.1/r^3\) and angular frequency \(\phi'=r^{-3/2}\), up to but not at the collision time 2.5.
* **#19.** Each acoustic pair rotates with ωℓ=√(ℓ(ℓ+1)) and decays as exp[−10⁻⁴ℓ²(ℓ+1)²t]; the passive third component has the same decay. This validates the chosen linear model, not nonlinear rotating shallow-water equations.
* **#22.** A periodic plane wave with phases θx=2π/N, θy=4π/N is an eigenvector of the centered stencil. Its eigenvalue is \(-4(.5000005)N^2[\sin^2(\theta_x/2)+\sin^2(\theta_y/2)]-.999999N^2\sin\theta_x\sin\theta_y\). The exact constant-plus-wave solution follows by one scalar exponential.

Error is maximum absolute component difference across **every** ensemble member, with the reference bound added where relevant. It is not a WRMS average over a giant concatenated ensemble. The default absolute acceptance threshold 10⁻⁴ is explicit in each row and configurable. This is an endpoint statement; oscillatory phase accuracy requires the separate intermediate-time probes and componentwise interpretation.

## Execution and profiling semantics

Methods are DOPRI5, matrix-free RODAS5P with a supplied directional derivative, dense RODAS5P, and the existing switching facade. Dense operation is refused for dimensions above 256 **before** any dense Jacobian allocation. Matrix-free Jacobian products and nonautonomous fₜ are generated by forward dual arithmetic from the same model expressions and checked against independent finite differences. The explicit method never calls this derivative provider. The RHS implementation is shared across all methods, including its zero-direction dual arithmetic and counters; benchmark values therefore describe this callback implementation, not an ideal hand-specialized physics kernel.

`serial` requires exactly one thread. `data` uses one prebuilt Rayon pool for independent component evaluation within a single trajectory. Reductions inside a component retain a fixed order. `ensemble` uses the pool for independent solves, each with its own error controller; it intentionally uses identical IVPs as a reproducibility/throughput control, not a parameter-distribution survey. The thread budget is 1–128, and the prepared campaign selects 1,2,4,8,12. Nested pools are avoided. Stage parallelism is tested in the separate coupled Rosenbrock experiment and is never credited to this data/ensemble executable.

The clean timer includes solver, provider/workspace construction, rejection, pilots and callbacks. Model construction, pool startup, diagnostic evaluation, references, and serialization are outside it. Pool startup is recorded separately. `--instrument` enables callback clocks and makes a row ineligible for clean speed ranking; without it only counters run. Across ensemble members callback durations overlap, so their sum cannot be subtracted from elapsed wall time to claim an exclusive solver overhead fraction. Dense Jacobian timing includes all column work. There is no fake breakdown for factorization or orthogonalization when those phases are not independently instrumented.

## Verification and remaining limits

Model tests check all 24 RHS/JVP evaluations, directional derivatives against finite differences, nonautonomous partial time derivatives, analytic-equation residuals, discrete mass and energy identities, invalid switching claims, bounded-reference scope, and deterministic 1/2/4-thread component evaluation. These checks are model verification, not a production solver certification.

The initial test run passed six checks and failed the #5 finite-difference fₜ check because subtracting RHS values near 5.4×10⁵ erased the small time-dependent difference. The corrected independent test evaluates the same fₜ on the slow manifold and uses a resolved perturbation; no model equation or expected derivative was changed. Eight checks then passed. A final independent-review hardening rejects nonfinite/overflowed reference vectors and adds a ninth check. Raw build and verification logs and the actual executable smoke result ledger accompany this contract. No claim is made about unexecuted standard-size campaigns.

## Primary-source identity checks

* F. Mazzia and C. Magherini, *Test Set for Initial Value Problem Solvers*, release 2.4 (2008), University of Bari: https://archimede.uniba.it/~testset/report/testset.pdf . Used for canonical test identities and dimensions.
* SciML benchmark implementation, HIRES: https://docs.sciml.ai/SciMLBenchmarksOutput/stable/StiffODE/Hires/ . Canonical equations, the constant .0007 source in the first equation, initial state and endpoint were checked against the executable benchmark code.
* SciML benchmark implementation, Pleiades: https://docs.sciml.ai/SciMLBenchmarksOutput/stable/NonStiffODE/Pleiades_wpd/ . Masses, ordering of positions/velocities, unsoftened force, initial state and endpoint were checked.

All other variants are defined from the supplied proposal plus the declared choices above; they are not advertised as externally standardized named benchmarks. `BENCHMARK_MODELS.json` records source identity, dimensions, model/reference status and verification evidence for machine inspection.
