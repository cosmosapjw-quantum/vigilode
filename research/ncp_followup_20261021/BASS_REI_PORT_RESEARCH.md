# BASS / rei_bianchi: conservative phase-space integration research

Date: 2026-10-21 KST. Scope: an implementable first port for **non-tilted LRS Bianchi I**, scalar unpolarized photons, prescribed geometry and cold elastic Thomson scattering. The new executable `BASS_REI_FVM_REFERENCE.py` is an independent first-order finite-volume oracle with analytic JVP. It is not installed into BASS/rei and is not the existing production radiation model. The Rust Schur and remap work in the same follow-up provides separately tested building blocks.

## 1. Actual source contracts and audit of the supplied advice

The inspected rei C15 source is commit `80825585f9fe31d2ee0d5af545ada3fa1f52be72`, tree `beb7594e5a68bb361cfcb813c7cc9fb1a2d6b8f0`; GitHub commit identity was checked on 2026-10-21. Its `coupled.py` retains photon/neutrino/UV/massive stress components, while the C14 source explicitly states unpolarized Thomson, missing E2 polarization and baryon recoil. The read-only BASS split-pilot contract separates matter and radiation anisotropic stress; its recorded BASS source head `106a0153bf113fa4f0beea9743e84a744b65f424` and tree `41a83fb95bce2d5e0d72f4c8ecd8479c55a4946a` were checked against GitHub. These are **identified input snapshots**, not a claim to have scanned every current branch of either repository. `BASS_REI_INPUTS.json` records file-level hashes and the distinction between Git-matched source and local research contract.

| Supplied assertion | Audit and useful replacement |
|---|---|
| Generic Bianchi transport lives on `(E,theta)` | True only after a preserved axial-symmetry reduction. General Bianchi/tilt requires `E × S²` with both angular degrees of freedom or an equivalent harmonic basis. Polarization needs a screen-basis connection and coherency/Stokes transport. |
| Scattering is nearest-neighbour/banded | Angular advection has a local stencil. Cold Thomson redistribution is global in angle; its gain admits a small moment factorization in the chosen unpolarized model. Use banded-plus-low-rank, not banded-only. |
| RODAS5P has six stages | This repository's coefficient object and drivers use **eight** stages. Query the coefficient table; do not hard-code six in limiter or workspace logic. |
| One-step restart immediately preserves fifth order and the old step size | One-step methods avoid transporting multistep history, a real advantage [R3]. Remap error, new spatial operator, limiter changes, stage residuals and stiff order conditions remain. Rebind geometry/Jacobian/preconditioner/dense output and re-estimate the next step. |
| BDF must always cold restart at order one after every mesh change | An implementation may choose this policy; it is not a universal theorem about multistep AMR. Compatible projected history and restart strategies exist. Compare actual amortized cost, including remap and operator rebuild. |
| Fixed capacity plus slicing gives zero allocation for changing dimension | It can give bounded allocation for individual kernels. All stage/dense/error/preconditioner/event buffers and cache identities must support the new dimension and topology. The current kernel implementation cannot by itself establish end-to-end zero allocation. |
| Schur preconditioning guarantees 3–5 GMRES iterations | Exact elimination and perturbation bounds provide constructive conditional guarantees; iteration count depends on the preconditioned defect, spectrum/non-normality, restart and rounding. See the Schur study. |
| Smooth or frozen limiter preserves fifth order | Smoothness helps differentiability; it does not prove fifth-order stiff convergence or positivity. Freezing the nonlinear RHS changes the ODE. Keep the physical RHS intact and assess Jacobian approximations under the actual Rosenbrock/W order conditions. |
| A several-hour history will run in minutes | This is a performance hypothesis. Benchmark on matching physical accuracy, resolution, radiation coupling and hardware. |

## 2. Conservative photon variable, units and geometry

Use signature `(-,+,+,+)`, proper time `t` in seconds, photon energy `E` in joules, `q=ln(E/E_ref)` dimensionless, and orthonormal direction `e∈S²`. Let dimensionless `f` be occupation **per polarization state**, with degeneracy `g=2` for unpolarized photons. Define

\[
\kappa=\frac{g}{(2\pi\hbar)^3c^3},\qquad
n_\gamma=\kappa\int E^3 f\,dq\,d\Omega,\qquad
\rho_\gamma=\kappa\int E^4 f\,dq\,d\Omega.
\]

Here `n_gamma` is m⁻³, `rho_gamma` is J m⁻³, and `d³p=E² dE dΩ/c³`. If the source variable is already summed over polarization, omit `g`; if it is intensity or energy density, convert once at the adapter. A photon count cell integral is not the unweighted cell average of `f`.

For non-tilted, vorticity-free Bianchi I in a nonrotating orthonormal frame,

\[
\dot q=-H-\sigma_{ab}e^ae^b=:v_q,\qquad
\dot e^a=-\sigma^a{}_b e^b+(\sigma_{bc}e^be^c)e^a=:v_\Omega^a.
\]

`H,sigma` have s⁻¹ units. The sphere-tangent drift is minus one half of the spherical gradient of `S(e)=sigma_ab e^a e^b`. Since trace-free `S` is an `ell=2` harmonic, `Δ_S² S=-6S`, hence

\[
\nabla_{S^2}\!\cdot v_\Omega=3S,\qquad
3H+3v_q+\partial_qv_q+\nabla_{S^2}\!\cdot v_\Omega=0.
\]

Multiplying `∂t f+v_q∂q f+v_Ω·∇_Ω f=C_t[f]` by `a³ κ E³`, with `dot a/a=H`, therefore gives the exact number-conservative equation

\[
\partial_t U+\partial_q(v_qU)+\nabla_{S^2}\cdot(v_\Omega U)
   =a^3\kappa E^3C_t[f],\quad U=a^3\kappa E^3 f.
\]

`C_t` is the collision rate in proper time; a covariant affine-parameter collision integral must first be divided by the corresponding time component. This identity is directly derived here; [R1,R2] supply the broader conservative kinetic-theory framework.

For LRS Bianchi I, `sigma=diag(-s,-s,2s)`, `mu=cos(theta)`, and azimuthal symmetry implies `dΩ=2π dmu`:

\[
v_q=-H-s(3\mu^2-1),\qquad v_\mu=-3s\mu(1-\mu^2),\qquad
\partial_\mu v_\mu=3s(3\mu^2-1).
\]

Store `N_ij=∫cell 2π U dq dmu`, with `a` dimensionless normalized to the fixed source convention. Then `sum N/a³` is physical photon number density. `BASS_REI_FVM_REFERENCE.py` evolves these **already weighted cell contents**; no missing extra `2π` may be added by the solver. At `mu=±1`, angular flux vanishes exactly. At the energy boundaries, physical outflow must enter a ledger; artificially closed energy boundaries are a separate test configuration.

A comoving energy coordinate `x=ln(aE/E_ref)` removes only the isotropic drift: `dot x=-S`. Anisotropic redshift remains. Ionization thresholds fixed in physical energy move in `x`; cross sections and threshold quadrature must track that motion. Generic class-B Bianchi systems need the full spacetime-current divergence: spatial homogeneity of tetrad components alone does not authorize deleting geometric spatial-flux terms.

## 3. Finite-volume, ALE and collision constructions

The executable oracle uses a first-order upwind flux shared by adjacent cells. For q faces it uses the exact angular-cell average of `3mu²-1`; for mu faces it uses `v_mu(mu_face)`. Each interior flux is added once with opposite signs, proving `sum dN/dt = - outward_energy_flux` in the absence of collisions. This is number conservation for the semi-discrete equation, independent of time-step choice. Forward Euler is positive if `dt*max_i(sum outgoing rates_i)≤1`; the supplied `transport_cfl` computes this sufficient bound. A general Rosenbrock update does not inherit this positivity theorem.

For a moving cell `K(t)`, Reynolds transport gives

\[
\frac{d}{dt}\int_{K(t)}U\,dV
 =-\int_{\partial K(t)}U(v-w)\cdot n\,dS+\int_{K(t)}S\,dV.
\]

Thus the relative velocity is `v-w`, with the same moving-face geometry used for the volume derivative `dot V=∮w·n`. For `v=0` and constant density, `dot N=U dot V`: this is the discrete geometric conservation law test. Evolving cell averages adds a `-U dot V/V` term; evolving cell integrals avoids forgetting that term but still requires coherent geometry. A time-dependent mass matrix or geometry state must be represented consistently through all eight stages.

Cold unpolarized elastic Thomson scattering in the LRS reduction has the kernel

\[
K(\mu,\mu')=1+\tfrac12P_2(\mu)P_2(\mu'),\quad
C=\Gamma\left[\int_{-1}^1K\,f(\mu')\frac{d\mu'}2-f(\mu)\right],\quad
\Gamma=c\sigma_Tn_e.
\]

This is a diagonal loss plus rank-two gain **per energy group**. Let `d_j=Δmu_j`, `p_j` the exact cell average of `P2`, `M0=sum N_j`, and `M2=sum p_j N_j`. Then

\[
\dot N_j=\Gamma\{(d_j/2)[M_0+\tfrac12p_jM_2]-N_j\}.
\]

Since `sum d_j=2` and `sum d_j p_j=0`, total group number is preserved exactly in algebra. The nonnegative discrete kernel guarantees positivity of this gain; Euler with `dt Γ≤1` is positive. This uses `O(N_mu)` rather than `O(N_mu²)` operations. Piecewise-constant quadrupole damping approaches the continuum value with angular refinement; do not assign the exact continuous eigenvalue to every coarse discretization. Polarized scattering needs the appropriate tensor/low-moment kernel and positivity of the 2×2 coherency matrix; componentwise clipping of signed Q/U/V is physically wrong.

Local transport plus rank-two-per-energy scattering and a small matter border suggests:
`W = I - gamma*h*J = local sparse transport/loss + low-rank angular gains + matter border`.
The rank of all energy groups combined grows with `N_E`; use energy-block factorizations or a nested Schur/Woodbury construction, not a single globally constant-rank assumption.

## 4. Exchange, stress and spatial/temporal error budgets

For a single-photon absorption channel `s` with threshold `chi_s`, use one shared event rate `A_si=c kappa_si n_gamma_i`. The identical rate contributes

\[
\dot n_{\gamma,i}=-A_{si},\quad \dot n_{s^+}=A_{si},\quad
\dot u_{\rm ion}=\chi_sA_{si},\quad
\dot u_{\rm heat}=(E_i-\chi_s)A_{si}.
\]

Then photon-energy loss plus ionization-potential and thermal gain sums to zero. Multiple species, secondary ionization and recombination emissivity require their source-owned branching/energy ledgers; the simple oracle does not replace those models. Physical expansion/shear work is separate from collision exchange. For massless radiation in this Bianchi-I scope,

\[
\dot\rho_r+4H\rho_r+\sigma_{ab}\pi_r^{ab}=Q_r,
\quad \sum_{\rm components}Q=0
\]

for closed internal exchanges. Keep signed tensors `pi_matter` and `pi_radiation` separately; only the Einstein source takes their sum. In LRS, with `D=P_parallel-P_perp`, `pi=diag(-D/3,-D/3,2D/3)` and `sigma:pi=2sD`. Opposite matter/radiation stresses can cancel. The norm of the total is not the sum of the norms. C15 already separates `Pi_gamma`, `Pi_nu`, `Pi_massive`, `Pi_ion`; preserve that ownership and unit convention rather than introducing an ambiguous new Pi scalar.

For a stable step map with Lipschitz amplification `1+Lh`, remap defect `delta_n`, temporal local defect `tau_n`, and linear-solve defect `lambda_n`, a useful recursion is

\[
e_{n+1}\le(1+Lh_n)e_n+\tau_n+\delta_n+\lambda_n.
\]

With `L≥0`, discrete Gronwall bounds the final error by `exp(LT)` times initial error plus all defects. Spatial consistency adds its time-integrated defect. Number conservation does not control these shape/energy/phase errors. Fifth-order temporal convergence on changing meshes requires total remap/space/solve defects to be below the target fifth-order term; a first-order remap at every step generally dominates. Keep spatial refinement, temporal refinement and remap-frequency refinement as separate experiments. Use the same physical reference and original budget on each mesh.

Freezing a limiter coefficient is not the same operation as freezing a Jacobian. For the smooth ODE `y'=y²`, freezing one multiplicative state at `y_n` and solving the modified equation exactly gives `y_n exp(h y_n)` instead of `y_n/(1-h y_n)`. The difference begins at `-h² y_n³/2`, yielding only first-order global convergence. This counterexample narrows the advice: a frozen reconstruction needs a consistency correction or an independently verified order condition. Wolfram returned the same series; the Python oracle observes the expected factor-two global error reduction. This is an elementary diagnostic, not a novel no-go theorem.

## 5. Concrete port contract and development sequence

Use a fixed-grid first port before AMR, with these interfaces (names are proposed contracts, not existing public API):

- `PhaseSpaceLayout`: energy/angular face arrays, cell measures, `n_active`, `capacity`, mesh epoch, physical variable/unit convention, boundary policy, ordered matter slots.
- `GeometryAtStage(t)`: `H`, shear tensor, angular drift, geometry source, mesh velocity and cell measure from a single clock/frame owner.
- `rhs(t, state, out)`: conservative shared faces, collision gain/loss, shared absorption/emission ledger, separate matter/radiation stresses.
- `jvp(t, state, direction, out)`: analytic fixed-grid transport/collision action plus matter derivatives. Use the supplied linear oracle to test the transport part independently.
- `prepare_preconditioner(h, gamma, mesh_epoch, rates_epoch)` and `apply(rhs,out)`: local block solver + angular gain + matter Schur; reject stale epochs.
- `accepted_step_remap`: build candidate geometry and projected contents in scratch; validate original physical budget, conservation, positivity/PSD, thresholds and energy exchange; commit atomically; invalidate dense output, stage/Jacobian/preconditioner/controller caches tied to the old mesh; select a new initial step. A rejected candidate preserves old state and geometry.

DAG in `BASS_REI_DAG.json`:
1. **P01 completed scoped here:** derive variable/geometry/flux and executable RHS/JVP oracle; 12 checks pass.
2. **P02 next:** Rust fixed-grid scalar adapter and end-to-end VigilODE stage solve; compare with oracle and exact collisionless Bianchi-I characteristics. Use both stiff and nonstiff settings, moving thresholds and mixed component scales.
3. **P03 next:** energy-block Thomson low-rank solve plus matter-border Schur; match dense solution and analytic residual, instrument all work. Benchmark break-even over `N_E × N_mu`, not only total dimension.
4. **P04 next:** live source-owned absorption/heating/recombination and signed pi_m/pi_r adapters; independently close number, ionization potential, thermal and shear-work ledgers.
5. **P05 next:** safe accepted-step remap + epoch rebinding; test repeat refine/coarsen and fixed-grid convergence before adaptive production.
6. **P06 extension:** full sphere and polarized transport, then class-B geometry/tilt/recoil; no theta-only shortcut.

Promotion criterion for P02–P05: fixed physical scenarios and original observable budgets, separate spatial/time/remap refinement, exact or independent references, ledger residuals, typed refusal/rollback, and accuracy-matched cost against the user-authorized VODE/DVODE, diffsol and Julia configurations. Full physical BASS/rei promotion remains pending their live integration and validation.

## References and evidence

- **R1** Cardall & Mezzacappa, *Conservative formulations of general relativistic kinetic theory*, PRD 68, 023006 (2003), https://doi.org/10.1103/PhysRevD.68.023006. Conservative number/energy distinction. Exa retrieved the author's explanatory paper https://arxiv.org/pdf/astro-ph/0404254; its explicit phase-space divergence discussion was read.
- **R2** Cardall, Endeve & Mezzacappa, *Conservative 3+1 General Relativistic Boltzmann Equation*, PRD 88, 023011 (2013), https://arxiv.org/abs/1305.0037 and https://doi.org/10.1103/PhysRevD.88.023011. Author abstract and source metadata retrieved through Exa. Supports general conservative framework; the Bianchi-I specialization above is directly derived.
- **R3** Lang & Verwer, *ROS3P—An Accurate Third-Order Rosenbrock Solver Designed for Parabolic Problems*, BIT 41 (2001), https://ir.cwi.nl/pub/1102/1102D.pdf. Primary manuscript retrieved by parent; page 732 discusses one-step mesh advantages and PDE order issues. This supports motivation, not fifth-order AMR preservation.
- **R4** Sandu, *Rosenbrock methods with an explicit first stage*, DOI https://doi.org/10.1080/00207160.2015.1012837. SciSpace discovery and Exa metadata/abstract: order reduction with nonhomogeneous PDE boundary/source terms, additional order conditions. Abstract-level evidence only.
- **R5** Official OrdinaryDiffEq Rosenbrock documentation, https://docs.sciml.ai/OrdinaryDiffEq/stable/semiimplicit/Rosenbrock/ (read 2026-10-21). Identifies different Rosenbrock variants and supported preconditioner interface; recommendations are implementation guidance, not a proof of solver dominance.
- Actual tests: `python3 research/ncp_followup_20261021/BASS_REI_FVM_REFERENCE.py research/ncp_followup_20261021/BASS_REI_CHECKS.json`; stdout/stderr in `BASS_REI_CHECKS.log`.
