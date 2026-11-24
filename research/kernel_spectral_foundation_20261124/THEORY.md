# Structure-exact stage solves: a foundation that joins the side threads

Run `kernel_spectral_foundation_20261124`, branch `research/stage-lattice-referee-20261124`.
It follows the stage-lattice referee run of the same day (`research/stage_lattice_referee_20261124`).

Claim status: **D** derived here, **I** imported (standard result, cited by name), **A** assumption,
**T** tested in this run (script or Rust test named), **N** negative result, **P** proposal,
**C** conjecture, **O** open.

## 0. The question and the answer in one paragraph

The side threads of VigilODE were Fourier transforms and convolution, involutions, algebraic
topology (Hodge, H^0), singular and algebraic geometry (singular loci, inertia), and dynamical
systems (stiffness, contractivity). Each had produced gates and screens but no shipped
optimization. One numerical object joins them: the RODAS5P stage operator
`W(z) = z I - J` with `z = 1/(h gamma)`, solved eight times per step. Every thread is a statement
about `W` or `J`. Symmetry block-diagonalizes `W`. Topology fixes the kernel of `J` and gives a
conserved functional that checks the solves. Singular geometry says where `W(z)` fails and how to
certify that it has not. Dynamics bounds `spec J` and turns that bound into a route decision. The
optimization is a **structure-exact stage solver**: when the declared structure holds, the
eight stage solves cost two real FFT sweeps and one tridiagonal sweep per Fourier mode,
with no Krylov iteration, and they carry certificates that cost O(n). The previous run showed
that the eight stages cannot be run in parallel (V05, depth 8). Within one stage, the separable
solver has `floor(np/2) + 1` independent mode systems, so parallel width comes from symmetry,
not from the stage loop (§6).

## 1. The stage problem and its cost (D)

One RODAS5P attempt does 8 RHS calls, one Jacobian action or assembly, one preparation of `W(z)`
(factorization or preconditioner setup) and 8 solves with `W(z)`. On the matrix-free
route, each solve is a GMRES run: `k` iterations, each with a JVP, a preconditioner application
and `O(k n)` orthogonalization. On the banded route each solve is two triangular sweeps (the
stage lattice of the previous run). The levers are:
(a) a cheaper solve, by structure;
(b) a shorter critical path, by the stage lattice;
(c) factorization reuse;
(d) certificates that replace verification work.
This run works on (a) and (d) for operators with a symmetry group. It keeps (b) and (c) from the
earlier runs.

## 2. Fourier, convolution and involution: block diagonalization by symmetry

**2.1 (I, Schur / convolution theorem).** If a finite group `G` acts on the cells, and `J`
commutes with every permutation of the action, then `J` preserves each isotypic component of
the representation. For the cyclic group `Z_N`, those components are the Fourier modes, and a
`Z_N`-equivariant operator on `C^N` is a circulant, i.e. a convolution, which the DFT diagonalizes.

**2.2 The sphere grid (D, T: F3).** The latitude-longitude finite-volume heat operator of
`spatial.rs` reads, at ring `r` and longitude `x`,

    (L u)[r,x] = a_r (u[r,x-1] - 2 u[r,x] + u[r,x+1]) + n_r (u[r-1,x] - u[r,x]) + s_r (u[r+1,x] - u[r,x]).

Here `a_r` is the azimuth rate, `n_r` the north rate and `s_r` the south rate; all three are
constant along a ring. The longitude rotations `Z_np` therefore commute with `L`. Take the DFT
`u[r,x] = sum_m U_r(m) e^{2 pi i m x / np}`. It turns `L` into `np` independent tridiagonal
blocks:

    (L_m U)_r = -(n_r + s_r + a_r (2 - 2 cos theta_m)) U_r + n_r U_{r-1} + s_r U_{r+1},
    theta_m = 2 pi m / np.

With `J = L + c I` (a ring-independent linear rate `c`), the stage matrix of mode `m` is the real
tridiagonal

    W_m(z) = tridiag(-n_r, z - c + n_r + s_r + a_r (2 - 2 cos theta_m), -s_r).

The script `verify_foundation.py` builds `L` cell by cell and conjugates it with the DFT. Off-mode
coupling and block mismatch are below 1e-9 (check F3).

**2.3 Two involutions (D, T: F3, F8).**
- *Reflection* `x -> -x` commutes with `L`, so `W_m = W_{np-m}`: the blocks depend on `m` only
  through `cos theta_m`. Only `floor(np/2) + 1` distinct tridiagonal matrices exist, so a step
  needs `floor(np/2) + 1` factorizations, not `np`.
- *Complex conjugation* acts on the spectrum of a real field as `U(m) -> conj U(np - m)`, so
  real data have Hermitian spectra. Two consequences:
  (i) each real block `W_m` solves the real and imaginary parts of `U(m)` as two real right-hand
      sides with one factorization;
  (ii) two real rings `u, v` share one complex FFT of `u + i v`, separated by
       `U = (Z + R Z)/2` and `V = (Z - R Z)/(2i)`, where `R Z(m) = conj Z(np - m)`.
  Check F8 confirms (ii) to 1e-12. The packing halves the FFT count. The dihedral group
  `D_np = Z_np ⋊ Z_2` is the full symmetry used. Its real irreducible representations have
  dimension 1 (`m = 0` and, for even `np`, `m = np/2`) and dimension 2 (every other `m`).

**2.4 Periodic box (D).** `Z_nx × Z_ny × Z_nz` acts transitively and `L` has constant
coefficients, so the multidimensional DFT diagonalizes it completely:
`W(k) = z - c + sum_axis rate_axis (2 - 2 cos theta_axis)`. A stage solve is one forward
transform, a pointwise division and one inverse transform.

**2.5 Cost (D).** With ring pairs packed, one separable stage solve on the sphere costs
`nt/2` complex FFTs of length `np` forward and the same number inverse
(`O(n log np)` together), plus `floor(np/2)+1` Thomas solves, each with two right-hand sides
(`O(n)`). Preparing `W(z)` costs `O(n)` and happens once per attempt; all 8 stages reuse it,
because `J` is constant (§7, T3).

## 3. Algebraic topology: the kernel, the conserved mass, the stage mass identity

**3.1 Discrete Hodge form (D, T: F1).** Write `δ: C^0 -> C^1` for the coboundary (differences
across cell faces) and `K` for the diagonal of face conductances. Then
`L = -A^{-1} δ^T K δ`, where `A` is the diagonal of cell areas. The code's rates satisfy
`A_r s_r = A_{r+1} n_{r+1}` (the same face conductance `K_{r+1/2}`), and the azimuthal faces are
symmetric by construction. So `A L` is symmetric negative semidefinite: `L` is self-adjoint in
the area inner product `<u, v>_A = sum A_i u_i v_i`. Check F1 gives a relative asymmetry of
`A L` at round-off and no positive eigenvalue.

**3.2 Kernel = H^0 (D, T: F2).** `ker L = ker δ` is the space of functions that are locally
constant on the cell graph, i.e. the 0-th cohomology. The sphere's cell graph is connected
(the polar caps are rings of cells joined in longitude), so `dim H^0 = 1`. Check F2 finds a
one-dimensional kernel aligned with the constants. The same holds for the torus (periodic
box). The kernel lives in mode `m = 0`, and only there.

**3.3 Conserved mass (D).** `1^T A L = (A L 1)^T = 0`, so `M(u) = sum A_i u_i` is invariant
under pure diffusion.

**3.4 Stage mass identity (D, T: F7, Rust test `separable_stage_solves_are_exact_and_mass_consistent`).**
For `W = z I - (L + c I)`, apply `1^T A` to `W x = b`:

    (z - c) M(x) = M(b).

Every stage solve therefore has an O(n) a-posteriori check in the H^0 direction. The check
does not depend on the solver, so it also verifies the FFT round trip.

**3.5 Linear invariants of RODAS5P (D, T: `separable_route_conserves_mass_and_matches_the_reference`).**
Suppose `1^T A f(u) = 0` for every `u` and `1^T A J = 0`. The U-form stage equations are
`W k_i = f(Y_i) + sum_j (C_ij / h) k_j`. Applying `1^T A` with `c = 0` and using induction over
the stages gives `M(k_i) = 0`, so every RODAS5P step conserves `M` up to round-off when the
solves are exact. With an inexact (Krylov) solve, `M` drifts by the solver residual's mass. The
exact separable solve removes this drift by construction.

**3.6 Remark (I).** On the torus, `H^1` has dimension 2: two harmonic flux fields that are not
gradients. They do not touch the scalar heat equation. They matter for flux-form transport, where
a Hodge-based splitting must carry the harmonic part separately. The sphere has `H^1 = 0`.
Recorded as **O** for advection.

## 4. Singular and algebraic geometry: where the stage pencil fails, and inertia

**4.1 Singular locus (D).** The stage pencil `P(z) = z I - J` is singular exactly on
`Σ = spec J`. In parameter space `(h, p)` this is the hypersurface `det(I/(h gamma) - J(p)) = 0`.
Near `Σ`, `κ(W) ~ 1 / dist(z, spec J)`. Every RODAS5P failure that involves the linear algebra
sits near this locus.

**4.2 Inertia certificate (D, T: F4, Rust test `inertia_counts_the_modes_beyond_the_stage_pencil`).**
For `A`-self-adjoint `J`, `A W_m` is symmetric. Thomas elimination of `W_m` without pivoting
gives `W_m = (A^{-1} L A)(A^{-1} D L^T)`, where `A W_m = L D L^T`. Its pivots are therefore
`d_r / A_r`, and they have the signs of `D`. Sylvester's law of inertia (I) then gives

    #{non-positive Thomas pivots over all modes} = #{λ ∈ spec J : λ ≥ z}.

Check F4 confirms the equality in 40 random `(c, z)` trials, including 64 crossings in one trial.
Consequences:
(i) all pivots positive ⇔ `z > λ_max(J)` ⇔ `W(z)` is SPD in `<.,.>_A`, and then
    `||W^{-1}||_A = 1/(z - λ_max) ≤ 1/(z - c)`;
(ii) a non-positive pivot means the step crossed the singular locus in that mode. The solver
     refuses the preparation with a typed linear-solve error, and the driver rejects the
     attempt and shrinks `h`.
The certificate costs nothing beyond the factorization it reads.

**4.3 The pole as a chart singularity (D, T: F6).** The latitude-longitude chart collapses each
polar cap to a point. The polar ring's area is `2 R^2 sin^2(dθ/2) dφ`, and its azimuth rate is
`a_0 = κ dθ / (2 R^2 sin^3(dθ/2) dφ^2) ≈ 4κ/(R^2 dθ^2 dφ^2)`. Relative to the equator,
`a_0 / a_eq → 4 nt^2 / π^2` (F6: quotient 0.99997 at `nt = 256`). This is the classical pole
problem (I). Explicit methods pay for it with an `O(nt^2)` smaller stable step. Krylov methods
pay with `κ(W)` growing like `a_0 / z`. In the mode decomposition, the polar stiffness enters
`W_m` only on the diagonal, as `a_0 (2 - 2 cos θ_m)`. High modes at the polar ring are therefore
damped exactly, by a diagonally dominant row. The separable solve resolves the chart singularity
at no extra cost: the large entries sit where Thomas elimination is most stable.

## 5. Dynamical systems: contractivity, spectral bounds, a stiffness certificate

**5.1 Contractivity (D).** The logarithmic norm of `J = L + cI` in `<.,.>_A` is
`μ_A(J) = λ_max(J) = c`. So `||u(t) - v(t)||_A ≤ e^{ct} ||u(0) - v(0)||_A`. `J` is `A`-normal, so
`||R(hJ)||_A = max_λ |R(hλ)|`. RODAS5P is A-stable (I), so for `c ≤ 0` the discrete map is
`A`-contractive for every `h`.

**5.2 Spectral bound (D, T: F5).** Gershgorin's theorem applied to each `W_m` gives
`spec J ⊂ [c - ρ̄, c]` with `ρ̄ = max_r 2(n_r + s_r) + 4 a_r`. F5 measured `ρ/ρ̄ = 0.982`
on a 9×16 grid. The bound costs `O(nt)`.

**5.3 Stiffness certificate (D, P for the router).** DOPRI5 is stable on `[-β, 0]` of the real
axis with `β ≈ 3.3`. For `c ≤ 0`, an explicit run over `T` needs at least `T ρ̄ / β`
steps of 6 RHS calls. An implicit run with exact separable solves needs `N` steps, each costing
`8 RHS + 8 S + F`, where `S` is a separable solve and `F` a preparation. Implicit integration
pays off when `6 T ρ̄ / β · c_f > N (8 c_f + 8 c_S + c_F)`. The left side is known before the
first step. `SeparableStages::spectral_bounds` returns `ρ̄`, `c` and the explicit step bound
`β / ρ̄`, so the facade or the user can make this choice. Wiring it into the automatic router is
**P** (not done here).

## 6. The stage lattice, revisited

The previous run proved that the RODAS5P stage dependency has depth 8 (V05). It also showed
that on the banded route the critical path is the 16 triangular sweeps (SL-1). For a separable
operator the picture inside one stage changes. The FFTs are independent across ring pairs
(width `nt/2`). The mode solves are independent across modes (width `floor(np/2)+1`). Each mode
solve has depth `2 nt` (Thomas), or `O(log nt)` with cyclic reduction (**P**). The eight stages
stay sequential (**N**, unchanged). Parallel width inside a stage is the symmetry group's number
of irreducible blocks. This is the correct form of "parallelizing the 8-stage loop" for
symmetric problems: parallelize inside each stage, across irreducible representations.

## 7. Implementation contract (what the code does, and its claims)

`vigilode::spatial::SeparableStages` (built by `DiffusionGrid::separable_stages(c)`) runs
inside the protected client driver `integrate_rodas5p_fast_client_observed_with_atol` through
`vigilode::unified::solve_separable`. Its claims:

- **T1 exactness.** For `J = L + cI` on the sphere and the box, the separable solve meets
  `||b - W x||_inf ≤ 1e-13 ((|z - c| + ρ̄) ||x||_inf + ||b||_inf)` on random right-hand
  sides (7 grids including odd `nt`, odd `np` and a 3D box, `z` from 1e-3 to 1e4; measured
  maximum 3.4e-16).
- **T2 stage mass.** `|(z - c) M(x) - M(b)| ≤ 1e-13 (Σ A_i) ((|z - c| + ρ̄) ||x||_inf + ||b||_inf)`
  (measured maximum 5.1e-17). The normalization by the backward-error scale, not by
  `Σ A_i |b_i|`, is needed for `z ≪ ρ̄`, where `||x||` exceeds `||b||` by `1/z`.
- **T3 route.** A whole IVP on the separable route matches an independent dense
  matrix-exponential reference within the tolerance envelope. It conserves `M` to round-off for
  `c = 0`, and agrees with the shipped matrix-free route within the same envelope.
- **T4 inertia.** For `c > z`, preparation refuses with a typed error, and the refusal count
  equals the number of eigenvalues above `z` on a small grid.
- **T5 verified mode.** If the problem's true Jacobian differs from the declared `L + cI` (a
  non-constant reaction), `Verified` mode computes the true residual with the problem's JVP and
  runs defect correction `x ← x + W_sep^{-1} r`. This converges when
  `||W_sep^{-1} (J - J_sep)||_A < 1`, guaranteed for `h γ < 1 / (||J - J_sep||_A + c_+)` (D).
  Beyond that bound the solve refuses, and the step shrinks.
- **T6 cost.** Diagnostic timing only, against the shipped JVP + GMRES route, on this 2-vCPU
  host. Owner-host cells are preregistered in `PLAN.json`.

## 8. What does not carry over (N, O)

- Longitude-dependent coefficients break `Z_np` (**N** for exactness). `Verified` mode turns
  the separable solve into a preconditioner with a checked residual (T5).
- Advection is non-normal. The inertia certificate (§4.2) and the contractivity bound (§5.1)
  do not apply: **O**, field-of-values replacement needed.
- Nonlinear reaction `f(u)` with `f'` not ring-constant: same as the first point. A ring-wise
  linearization (`c_r` per ring) keeps the separability, since a ring-constant diagonal commutes
  with `Z_np`. The implementation takes one global `c`; per-ring `c_r` is **P**.

## 9. Where the side threads now sit

| Thread | Object | Use in code |
| --- | --- | --- |
| Fourier / convolution | `Z_np` isotypic decomposition | block-diagonal stage solve |
| Involution | reflection, conjugation | half the factorizations, half the FFTs |
| Algebraic topology | `H^0`, Hodge form | mass identity check, conservation test |
| Singular geometry | stage-pencil locus, pole chart | inertia refusal, polar stiffness in diagonal |
| Algebraic geometry (inertia) | Sylvester, Sturm | pivot-sign certificate |
| Dynamical systems | log norm, Gershgorin | contractivity statement, `spectral_bounds` |
| Stage lattice | critical path | width from symmetry inside a stage |
